#!/usr/bin/env python3
"""Monitor data collector for zellij-claude-monitor.

Reads recent session data from JSONL files, calculates burn rate, cost, etc.
and outputs as JSON for the Zellij WASM plugin to consume.

Usage: python3 monitor-data.py <claude_dir> [plan]
"""
import json
import sys
import os
from datetime import datetime, timezone, timedelta
from pathlib import Path

# 모델별 가격 (per million tokens)
PRICING = {
    "opus": {"input": 15.0, "output": 75.0, "cache_creation": 18.75, "cache_read": 1.5},
    "sonnet": {"input": 3.0, "output": 15.0, "cache_creation": 3.75, "cache_read": 0.3},
    "haiku": {"input": 0.25, "output": 1.25, "cache_creation": 0.3, "cache_read": 0.03},
}

PLAN_LIMITS = {
    "pro": {"tokens": 19_000, "cost": 18.0, "messages": 250},
    "max5": {"tokens": 88_000, "cost": 35.0, "messages": 1_000},
    "max20": {"tokens": 220_000, "cost": 140.0, "messages": 2_000},
}


def get_pricing(model: str) -> dict:
    m = (model or "").lower()
    if "opus" in m:
        return PRICING["opus"]
    if "haiku" in m:
        return PRICING["haiku"]
    return PRICING["sonnet"]


def calc_cost(model: str, inp: int, out: int, cache_c: int, cache_r: int) -> float:
    p = get_pricing(model)
    return (
        (inp / 1e6) * p["input"]
        + (out / 1e6) * p["output"]
        + (cache_c / 1e6) * p["cache_creation"]
        + (cache_r / 1e6) * p["cache_read"]
    )


def parse_ts(ts_str: str):
    try:
        return datetime.fromisoformat(ts_str.replace("Z", "+00:00"))
    except Exception:
        return None


def collect_teams(claude_dir: Path) -> list:
    """팀 설정 + 태스크 상태를 수집한다."""
    teams = []
    teams_dir = claude_dir / "teams"
    tasks_base = claude_dir / "tasks"

    if not teams_dir.exists():
        return []

    for team_dir in sorted(teams_dir.iterdir()):
        config_path = team_dir / "config.json"
        if not config_path.exists():
            continue

        try:
            with open(config_path) as f:
                config = json.load(f)
        except Exception:
            continue

        team_name = team_dir.name
        members = []

        # 해당 팀의 태스크 수집
        task_map = {}  # owner -> list of task subjects
        team_tasks_dir = tasks_base / team_name
        task_counts = {"pending": 0, "in_progress": 0, "completed": 0}

        if team_tasks_dir.exists():
            for task_file in team_tasks_dir.glob("*.json"):
                try:
                    with open(task_file) as f:
                        task = json.load(f)
                    status = task.get("status", "")
                    if status in task_counts:
                        task_counts[status] += 1
                    owner = task.get("owner", "")
                    if owner and status == "in_progress":
                        subject = task.get("subject", "")
                        task_map.setdefault(owner, []).append(subject)
                except Exception:
                    continue

        for member in config.get("members", []):
            name = member.get("name", "")
            agent_type = member.get("agentType", "")
            current_tasks = task_map.get(name, [])
            members.append({
                "name": name,
                "agent_type": agent_type,
                "task": current_tasks[0] if current_tasks else "",
                "busy": len(current_tasks) > 0,
            })

        teams.append({
            "name": team_name,
            "members": members,
            "tasks_pending": task_counts["pending"],
            "tasks_in_progress": task_counts["in_progress"],
            "tasks_completed": task_counts["completed"],
        })

    return teams


def detect_active_agents(
    subagent_files: list, session_files: list, claude_dir: Path, now
) -> list:
    """사전 수집된 파일 리스트로 활성 서브에이전트를 감지한다.

    subagent_files: (path, stat) 튜플 리스트 — subagents/ 하위 JSONL
    session_files: (path, stat) 튜플 리스트 — 부모 세션 JSONL
    """
    active = set()
    session_cutoff = now - timedelta(minutes=5)
    agent_cutoff = now - timedelta(minutes=2)
    tail_bytes = 128 * 1024

    # 1순위: subagent JSONL 직접 스캔
    for sa_file, sa_stat in subagent_files:
        try:
            sa_mtime = datetime.fromtimestamp(sa_stat.st_mtime, tz=timezone.utc)
            if sa_mtime < agent_cutoff:
                continue
            agent_type = _identify_agent_from_meta(sa_file)
            if agent_type:
                active.add(agent_type)
            else:
                active.add("subagent")
        except OSError:
            continue

    # 2순위: 최근 활성 부모 세션의 JSONL에서 미완료 Agent 호출 감지
    for p, st in session_files:
        try:
            mtime = datetime.fromtimestamp(st.st_mtime, tz=timezone.utc)
            if mtime < session_cutoff:
                continue
        except OSError:
            continue
        try:
            active.update(_parse_active_agents(p, st.st_size, tail_bytes))
        except (OSError, PermissionError):
            continue

    # 팀 에이전트 중 실제 in_progress 태스크가 있는 것만 활성으로 판단
    teams_dir = claude_dir / "teams"
    tasks_base = claude_dir / "tasks"
    if teams_dir.exists():
        for config_path in teams_dir.glob("*/config.json"):
            try:
                with open(config_path) as f:
                    config = json.load(f)
                team_name = config_path.parent.name
                busy_owners = set()
                team_tasks_dir = tasks_base / team_name
                if team_tasks_dir.exists():
                    for task_file in team_tasks_dir.glob("*.json"):
                        try:
                            with open(task_file) as tf:
                                task = json.load(tf)
                            if task.get("status") == "in_progress":
                                owner = task.get("owner", "")
                                if owner:
                                    busy_owners.add(owner)
                        except Exception:
                            continue
                for member in config.get("members", []):
                    name = member.get("name", "")
                    at = member.get("agentType", "")
                    if at and name in busy_owners:
                        active.add(at)
            except Exception:
                continue

    return sorted(active)


def _identify_agent_from_meta(sa_jsonl: Path) -> str:
    """meta.json에서 agentType을 읽는다."""
    meta_path = sa_jsonl.with_suffix(".meta.json")
    if meta_path.exists():
        try:
            with open(meta_path) as f:
                meta = json.load(f)
            return meta.get("agentType", "")
        except Exception:
            pass
    return ""


def _parse_active_agents(jsonl_path: Path, file_size: int, tail_bytes: int) -> set:
    """JSONL 파일의 tail에서 활성 Agent를 파싱한다.

    Agent tool_use (name="Agent") → 시작, tool_result (매칭 id) → 완료.
    시작은 있지만 완료가 없는 Agent = 활성.
    """
    active = set()
    agent_starts = {}  # tool_use_id -> subagent_type
    completed_ids = set()

    with open(jsonl_path, "rb") as f:
        offset = max(0, file_size - tail_bytes)
        f.seek(offset)
        data = f.read().decode("utf-8", errors="ignore")

    # offset > 0이면 첫 줄은 잘렸을 수 있으므로 버림
    lines = data.splitlines()
    if offset > 0 and lines:
        lines = lines[1:]

    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            continue

        rec_type = rec.get("type")

        if rec_type == "assistant":
            msg = rec.get("message", {})
            content = msg.get("content", [])
            if isinstance(content, list):
                for block in content:
                    if not isinstance(block, dict):
                        continue
                    if block.get("type") == "tool_use" and block.get("name") == "Agent":
                        inp = block.get("input", {})
                        agent_type = inp.get("subagent_type", "") or inp.get("description", "")
                        agent_starts[block["id"]] = agent_type

        elif rec_type == "user":
            msg = rec.get("message", {})
            content = msg.get("content", [])
            if isinstance(content, list):
                for block in content:
                    if not isinstance(block, dict):
                        continue
                    if block.get("type") == "tool_result":
                        tid = block.get("tool_use_id", "")
                        if tid in agent_starts:
                            completed_ids.add(tid)

    # 시작됐지만 완료 안 된 Agent = 활성
    for tid, agent_type in agent_starts.items():
        if tid not in completed_ids and agent_type:
            active.add(agent_type)

    return active


def find_current_window(entries: list, now, window_hours: int = 5):
    """고정 윈도우 모델로 현재 활성 윈도우의 entries와 윈도우 종료 시각을 반환한다.

    Anthropic rate limit: 첫 메시지의 시각을 시간 단위로 내림(floor)하여 윈도우 시작.
    예: 15:54 시작 → 윈도우 15:00~20:00. 리셋 후 다음 메시지가 새 윈도우를 시작한다.

    Returns:
        (window_entries, window_end) — 활성 윈도우가 없으면 ([], None)
    """
    if not entries:
        return [], None

    # 첫 entry의 시각을 시간 단위로 내림 (Anthropic 윈도우 정렬 방식)
    raw_start = entries[0]["ts"]
    window_start = raw_start.replace(minute=0, second=0, microsecond=0)
    while True:
        window_end = window_start + timedelta(hours=window_hours)
        if window_end > now:
            # 현재 이 윈도우 안에 있음
            window_entries = [e for e in entries if window_start <= e["ts"] < window_end]
            return window_entries, window_end

        # 이 윈도우는 만료됨 → 만료 후 첫 entry가 새 윈도우 시작
        next_entries = [e for e in entries if e["ts"] >= window_end]
        if not next_entries:
            # 만료 후 사용 없음 → 완전히 리셋된 상태
            return [], None
        # 새 윈도우도 시간 단위 내림
        raw_start = next_entries[0]["ts"]
        window_start = raw_start.replace(minute=0, second=0, microsecond=0)


def main():
    claude_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/.claude")
    plan = sys.argv[2] if len(sys.argv) > 2 else "max5"

    projects_dir = Path(claude_dir) / "projects"
    now = datetime.now(timezone.utc)
    # 고정 윈도우 체인을 정확히 계산하려면 충분한 과거 데이터가 필요
    # 5시간 윈도우 × 2 + 여유 = 12시간
    cutoff = now - timedelta(hours=12)

    entries = []

    # 1회 rglob으로 모든 JSONL 파일을 수집 후 분류하여 재사용
    # (기존: main에서 1회 + detect_active_agents에서 2회 = 3회 rglob)
    subagent_files = []  # (path, stat) — subagents/ 하위
    session_files = []   # (path, stat) — 부모 세션 JSONL
    active_session_count = 0  # 최근 5분 이내 활성 세션 수
    session_cutoff_for_count = now - timedelta(minutes=5)

    tail_bytes = 512 * 1024  # 512KB — 최근 entries만 필요

    if projects_dir.exists():
        for jsonl_path in projects_dir.rglob("*.jsonl"):
            try:
                st = jsonl_path.stat()
                mtime = datetime.fromtimestamp(st.st_mtime, tz=timezone.utc)
            except OSError:
                continue

            is_subagent = "subagents" in jsonl_path.parts

            if is_subagent:
                if "acompact" not in jsonl_path.name:
                    subagent_files.append((jsonl_path, st))
                continue

            # 활성 세션 카운트 (5분 이내 수정된 부모 세션 JSONL)
            if mtime >= session_cutoff_for_count:
                active_session_count += 1

            session_files.append((jsonl_path, st))

            if mtime < cutoff:
                continue

            try:
                file_size = st.st_size
                with open(jsonl_path, "rb") as f:
                    offset = max(0, file_size - tail_bytes)
                    if offset > 0:
                        f.seek(offset)
                    raw = f.read().decode("utf-8", errors="ignore")

                lines = raw.splitlines()
                # offset > 0이면 첫 줄은 잘렸을 수 있으므로 버림
                if offset > 0 and lines:
                    lines = lines[1:]

                for line in lines:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        d = json.loads(line)
                    except json.JSONDecodeError:
                        continue

                    if d.get("type") != "assistant":
                        continue

                    ts_str = d.get("timestamp")
                    if not ts_str:
                        continue

                    ts = parse_ts(ts_str)
                    if not ts or ts < cutoff:
                        continue

                    msg = d.get("message", {})
                    usage = msg.get("usage", {})
                    if not usage:
                        continue

                    inp = usage.get("input_tokens", 0) or 0
                    out = usage.get("output_tokens", 0) or 0
                    cache_c = usage.get("cache_creation_input_tokens", 0) or 0
                    cache_r = usage.get("cache_read_input_tokens", 0) or 0
                    model = msg.get("model", "")

                    cost = calc_cost(model, inp, out, cache_c, cache_r)
                    total_tok = inp + out + cache_c + cache_r

                    entries.append({
                        "ts": ts,
                        "tokens": total_tok,
                        "cost": cost,
                        "model": model,
                        "out": out,
                    })
            except (OSError, PermissionError):
                continue

    entries.sort(key=lambda e: e["ts"])

    # Plan limits — token limit은 output tokens 기준
    limits = PLAN_LIMITS.get(plan, PLAN_LIMITS["max5"])
    local_tz = datetime.now().astimezone().tzinfo

    # 고정 윈도우 모델: 첫 사용부터 5시간 윈도우, 만료 후 다음 사용이 새 윈도우 시작
    session, window_end = find_current_window(entries, now)

    total_all_tokens = sum(e["tokens"] for e in session)
    total_cost = sum(e["cost"] for e in session)
    total_out = sum(e["out"] for e in session)
    msg_count = len(session)

    # 모델별 output token 비중 + 현재 모델
    model_out = {}
    for e in session:
        m = e.get("model", "") or ""
        # 모델명 정규화: "claude-sonnet-4-..." → "Sonnet"
        if "opus" in m.lower():
            name = "Opus"
        elif "haiku" in m.lower():
            name = "Haiku"
        else:
            name = "Sonnet"
        model_out[name] = model_out.get(name, 0) + e["out"]

    model_breakdown = {}
    if total_out > 0:
        for name, out in model_out.items():
            model_breakdown[name] = round(out / total_out * 100, 1)

    current_model = ""
    if session:
        last_model = session[-1].get("model", "")
        if "opus" in last_model.lower():
            current_model = "Opus"
        elif "haiku" in last_model.lower():
            current_model = "Haiku"
        else:
            current_model = "Sonnet"

    # Burn rate — output tokens 기준 (limit과 비교 가능하도록)
    burn_rate = 0.0
    cost_rate = 0.0
    duration_min = 0.0
    if session:
        first_ts = session[0]["ts"]
        duration_min = max((now - first_ts).total_seconds() / 60, 1.0)
        burn_rate = total_out / duration_min
        cost_rate = total_cost / duration_min

    # 예측 — output tokens 기준
    tokens_exhaust_min = -1.0
    cost_exhaust_min = -1.0
    if burn_rate > 0:
        remaining_tok = max(limits["tokens"] - total_out, 0)
        tokens_exhaust_min = remaining_tok / burn_rate
    if cost_rate > 0:
        remaining_cost = max(limits["cost"] - total_cost, 0)
        cost_exhaust_min = remaining_cost / cost_rate

    # 예상 소진 시각 (로컬 시간)
    tokens_exhaust_at = ""
    cost_exhaust_at = ""
    if tokens_exhaust_min > 0:
        exhaust_dt = now + timedelta(minutes=tokens_exhaust_min)
        tokens_exhaust_at = exhaust_dt.astimezone(local_tz).strftime("%H:%M")
    if cost_exhaust_min > 0:
        exhaust_dt = now + timedelta(minutes=cost_exhaust_min)
        cost_exhaust_at = exhaust_dt.astimezone(local_tz).strftime("%H:%M")

    # 세션 리셋 시각 (로컬 시간) — 고정 윈도우 종료 시점
    reset_time = ""
    if window_end:
        reset_time = window_end.astimezone(local_tz).strftime("%H:%M")

    # Limit 초과 여부 — output tokens 기준 (실제 차단 기준)
    exceeded_tokens = total_out >= limits["tokens"]

    # 활성 에이전트 감지 (사전 수집된 파일 리스트 재사용)
    active_agents = detect_active_agents(
        subagent_files, session_files, Path(claude_dir), now
    )

    # 팀 정보 수집
    teams = collect_teams(Path(claude_dir))

    # MCP 서버 수 (mcp.json에서 mcpServers 키 수 카운트)
    mcps_count = 0
    mcp_json_path = Path(claude_dir) / "mcp.json"
    if mcp_json_path.exists():
        try:
            with open(mcp_json_path) as f:
                mcp_data = json.load(f)
            mcps_count = len(mcp_data.get("mcpServers", {}))
        except Exception:
            pass

    result = {
        "burn_rate": round(burn_rate, 1),
        "cost_rate": round(cost_rate, 4),
        "total_tokens": total_out,
        "total_all_tokens": total_all_tokens,
        "total_cost": round(total_cost, 4),
        "output_tokens": total_out,
        "messages": msg_count,
        "tokens_exhaust_min": round(tokens_exhaust_min, 0),
        "cost_exhaust_min": round(cost_exhaust_min, 0),
        "tokens_exhaust_at": tokens_exhaust_at,
        "cost_exhaust_at": cost_exhaust_at,
        "reset_time": reset_time,
        "plan": plan,
        "token_limit": limits["tokens"],
        "cost_limit": limits["cost"],
        "exceeded": exceeded_tokens,
        "active": len(session) > 0,
        "active_agents": active_agents,
        "teams": teams,
        "current_model": current_model,
        "model_breakdown": model_breakdown,
        "active_sessions": active_session_count,
        "mcps_count": mcps_count,
    }

    print(json.dumps(result))


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        import traceback
        err_file = Path.home() / ".config" / "zellij" / "plugins" / "monitor-error.log"
        with open(err_file, "w") as f:
            traceback.print_exc(file=f)
        # 에러 시에도 최소 JSON 출력 (WASM 파싱 실패 방지)
        print(json.dumps({
            "burn_rate": 0, "cost_rate": 0, "total_tokens": 0,
            "total_all_tokens": 0, "total_cost": 0, "output_tokens": 0,
            "messages": 0, "tokens_exhaust_min": 0, "cost_exhaust_min": 0,
            "tokens_exhaust_at": "", "cost_exhaust_at": "",
            "reset_time": "", "plan": "error", "token_limit": 0,
            "cost_limit": 0, "exceeded": False, "active": False,
            "active_agents": [f"ERR:{e}"], "teams": [],
            "current_model": "", "model_breakdown": {},
            "active_sessions": 0, "mcps_count": 0,
        }))
