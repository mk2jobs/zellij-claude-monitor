#!/usr/bin/env python3
"""detect_active_agents / _parse_active_agents 단위 테스트.

실행: python3 scripts/test_detect_agents.py
"""
import json
import os
import sys
import tempfile
from datetime import datetime, timezone, timedelta
from pathlib import Path

# monitor-data.py에서 함수 import
sys.path.insert(0, os.path.dirname(__file__))
from importlib import import_module

mod = import_module("monitor-data")
_parse_active_agents = mod._parse_active_agents
_identify_agent_from_meta = mod._identify_agent_from_meta
detect_active_agents = mod.detect_active_agents


def write_jsonl(path: Path, records: list):
    with open(path, "w") as f:
        for rec in records:
            f.write(json.dumps(rec) + "\n")


def make_agent_start(tool_id: str, subagent_type: str, description: str = "") -> dict:
    return {
        "type": "assistant",
        "message": {
            "content": [
                {
                    "type": "tool_use",
                    "id": tool_id,
                    "name": "Agent",
                    "input": {
                        "subagent_type": subagent_type,
                        "description": description or f"Run {subagent_type}",
                    },
                }
            ]
        },
    }


def make_tool_result(tool_id: str, content: str = "Done") -> dict:
    return {
        "type": "user",
        "message": {
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": tool_id,
                    "content": content,
                }
            ]
        },
    }


def make_other_tool(tool_id: str, name: str = "Bash") -> dict:
    return {
        "type": "assistant",
        "message": {
            "content": [
                {
                    "type": "tool_use",
                    "id": tool_id,
                    "name": name,
                    "input": {"command": "echo hello"},
                }
            ]
        },
    }


passed = 0
failed = 0


def assert_eq(test_name, actual, expected):
    global passed, failed
    if actual == expected:
        passed += 1
        print(f"  ✓ {test_name}")
    else:
        failed += 1
        print(f"  ✗ {test_name}")
        print(f"    expected: {expected}")
        print(f"    actual:   {actual}")


# ── Test 1: 활성 에이전트 1개 (시작만, 완료 없음) ──
print("Test 1: 활성 에이전트 1개")
with tempfile.NamedTemporaryFile(suffix=".jsonl", mode="w", delete=False) as f:
    records = [
        make_agent_start("toolu_001", "bruno-manager", "Bruno API 조사"),
    ]
    for rec in records:
        f.write(json.dumps(rec) + "\n")
    f.flush()
    path = Path(f.name)
    result = _parse_active_agents(path, path.stat().st_size, 32768)
    assert_eq("bruno-manager 활성", result, {"bruno-manager"})
    os.unlink(f.name)


# ── Test 2: 완료된 에이전트 (시작 + 완료) ──
print("Test 2: 완료된 에이전트")
with tempfile.NamedTemporaryFile(suffix=".jsonl", mode="w", delete=False) as f:
    records = [
        make_agent_start("toolu_002", "code-reviewer"),
        make_tool_result("toolu_002", "Review complete"),
    ]
    for rec in records:
        f.write(json.dumps(rec) + "\n")
    f.flush()
    path = Path(f.name)
    result = _parse_active_agents(path, path.stat().st_size, 32768)
    assert_eq("완료된 에이전트 = 빈 셋", result, set())
    os.unlink(f.name)


# ── Test 3: 복수 에이전트 — 1개 활성, 1개 완료 ──
print("Test 3: 복수 에이전트 혼합")
with tempfile.NamedTemporaryFile(suffix=".jsonl", mode="w", delete=False) as f:
    records = [
        make_agent_start("toolu_003", "bruno-manager"),
        make_agent_start("toolu_004", "code-reviewer"),
        make_tool_result("toolu_003", "Done"),
        # toolu_004 (code-reviewer) 는 아직 완료 안 됨
    ]
    for rec in records:
        f.write(json.dumps(rec) + "\n")
    f.flush()
    path = Path(f.name)
    result = _parse_active_agents(path, path.stat().st_size, 32768)
    assert_eq("code-reviewer만 활성", result, {"code-reviewer"})
    os.unlink(f.name)


# ── Test 4: Agent가 아닌 다른 tool_use는 무시 ──
print("Test 4: 비-Agent tool_use 무시")
with tempfile.NamedTemporaryFile(suffix=".jsonl", mode="w", delete=False) as f:
    records = [
        make_other_tool("toolu_005", "Bash"),
        make_other_tool("toolu_006", "Read"),
    ]
    for rec in records:
        f.write(json.dumps(rec) + "\n")
    f.flush()
    path = Path(f.name)
    result = _parse_active_agents(path, path.stat().st_size, 32768)
    assert_eq("Agent 아닌 것은 무시", result, set())
    os.unlink(f.name)


# ── Test 5: 빈 파일 ──
print("Test 5: 빈 파일")
with tempfile.NamedTemporaryFile(suffix=".jsonl", mode="w", delete=False) as f:
    f.flush()
    path = Path(f.name)
    result = _parse_active_agents(path, 0, 32768)
    assert_eq("빈 파일 = 빈 셋", result, set())
    os.unlink(f.name)


# ── Test 6: tail 읽기 — 파일이 tail_bytes보다 큰 경우 ──
print("Test 6: tail 읽기 (큰 파일)")
with tempfile.NamedTemporaryFile(suffix=".jsonl", mode="w", delete=False) as f:
    # 앞부분을 패딩으로 채우기 (Agent 시작이 tail 밖에 있는 경우)
    padding_record = {"type": "system", "message": {"content": "x" * 500}}
    for _ in range(100):
        f.write(json.dumps(padding_record) + "\n")
    # 뒤쪽에 활성 Agent
    f.write(json.dumps(make_agent_start("toolu_007", "deployer")) + "\n")
    f.flush()
    path = Path(f.name)
    size = path.stat().st_size
    # tail_bytes를 작게 설정하여 뒤쪽만 읽기
    result = _parse_active_agents(path, size, 2048)
    assert_eq("tail에 있는 deployer 감지", result, {"deployer"})
    os.unlink(f.name)


# ── Test 7: tail 밖에 있는 Agent 시작 + tail 안에 완료 ──
print("Test 7: Agent 시작이 tail 밖, 완료가 tail 안")
with tempfile.NamedTemporaryFile(suffix=".jsonl", mode="w", delete=False) as f:
    # Agent 시작 (앞부분)
    f.write(json.dumps(make_agent_start("toolu_008", "db-architect")) + "\n")
    # 패딩
    padding_record = {"type": "system", "message": {"content": "x" * 500}}
    for _ in range(100):
        f.write(json.dumps(padding_record) + "\n")
    # 완료 (뒤쪽)
    f.write(json.dumps(make_tool_result("toolu_008")) + "\n")
    f.flush()
    path = Path(f.name)
    size = path.stat().st_size
    # tail_bytes를 작게 — Agent 시작은 tail 밖, 완료만 tail 안
    result = _parse_active_agents(path, size, 2048)
    # 시작을 못 봤으므로 활성으로 안 뜸 — 올바른 동작
    assert_eq("시작 못 봤으면 무시 (정상)", result, set())
    os.unlink(f.name)


# ── Test 8: subagent_type 없이 description만 있는 경우 ──
print("Test 8: subagent_type 없이 description fallback")
with tempfile.NamedTemporaryFile(suffix=".jsonl", mode="w", delete=False) as f:
    rec = {
        "type": "assistant",
        "message": {
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_009",
                    "name": "Agent",
                    "input": {
                        "description": "Search codebase for patterns",
                    },
                }
            ]
        },
    }
    f.write(json.dumps(rec) + "\n")
    f.flush()
    path = Path(f.name)
    result = _parse_active_agents(path, path.stat().st_size, 32768)
    assert_eq("description fallback", result, {"Search codebase for patterns"})
    os.unlink(f.name)


# ── Test 9: 동시에 같은 타입 에이전트 2개 ──
print("Test 9: 동일 타입 2개 동시 실행")
with tempfile.NamedTemporaryFile(suffix=".jsonl", mode="w", delete=False) as f:
    records = [
        make_agent_start("toolu_010", "code-reviewer", "Review auth"),
        make_agent_start("toolu_011", "code-reviewer", "Review api"),
        make_tool_result("toolu_010"),
        # toolu_011 아직 활성
    ]
    for rec in records:
        f.write(json.dumps(rec) + "\n")
    f.flush()
    path = Path(f.name)
    result = _parse_active_agents(path, path.stat().st_size, 32768)
    assert_eq("동일 타입이라도 1개 활성이면 셋에 포함", result, {"code-reviewer"})
    os.unlink(f.name)


# ── Test 10: detect_active_agents 통합 (JSONL 방식) ──
print("Test 10: detect_active_agents 통합 (JSONL)")
with tempfile.TemporaryDirectory() as tmpdir:
    projects_dir = Path(tmpdir) / "projects"
    proj = projects_dir / "-test-project"
    proj.mkdir(parents=True)
    claude_dir = Path(tmpdir)

    # 활성 세션 JSONL 생성
    session_jsonl = proj / "session-001.jsonl"
    write_jsonl(session_jsonl, [
        make_agent_start("toolu_020", "bruno-manager"),
        make_agent_start("toolu_021", "deployer"),
        make_tool_result("toolu_020"),
        # deployer는 활성
    ])

    # 오래된 세션 (감지 안 되어야 함)
    old_session = proj / "session-old.jsonl"
    write_jsonl(old_session, [
        make_agent_start("toolu_030", "log-analyst"),
    ])
    # mtime을 10분 전으로
    old_time = datetime.now().timestamp() - 600
    os.utime(old_session, (old_time, old_time))

    now = datetime.now(timezone.utc)
    result = detect_active_agents(projects_dir, claude_dir, now)
    assert_eq("deployer만 활성 (bruno 완료, old 무시)", result, ["deployer"])


# ── Test 12: subagents/ 디렉토리 + meta.json 감지 ──
print("Test 12: subagents/ 디렉토리 + meta.json 감지")
with tempfile.TemporaryDirectory() as tmpdir:
    projects_dir = Path(tmpdir) / "projects"
    proj = projects_dir / "-test-project"
    proj.mkdir(parents=True)
    claude_dir = Path(tmpdir)

    # 세션 JSONL (Agent 시작+완료 — JSONL로는 비활성)
    session_jsonl = proj / "session-002.jsonl"
    write_jsonl(session_jsonl, [
        make_agent_start("toolu_040", "bruno-manager"),
        make_tool_result("toolu_040"),
    ])

    # subagents/ 디렉토리 (최근 수정 — 아직 실행 중)
    sa_dir = proj / "session-002" / "subagents"
    sa_dir.mkdir(parents=True)
    sa_jsonl = sa_dir / "agent-a123456789.jsonl"
    sa_jsonl.write_text('{"type":"user","message":{"content":"test"}}\n')
    meta = sa_dir / "agent-a123456789.meta.json"
    meta.write_text('{"agentType":"bruno-manager","description":"Find APIs"}')

    now = datetime.now(timezone.utc)
    result = detect_active_agents(projects_dir, claude_dir, now)
    assert_eq("subagents/ meta.json으로 bruno-manager 감지", result, ["bruno-manager"])


# ── Test 13: subagents/ 오래된 파일은 무시 ──
print("Test 13: subagents/ 오래된 파일 무시")
with tempfile.TemporaryDirectory() as tmpdir:
    projects_dir = Path(tmpdir) / "projects"
    proj = projects_dir / "-test-project"
    proj.mkdir(parents=True)
    claude_dir = Path(tmpdir)

    session_jsonl = proj / "session-003.jsonl"
    write_jsonl(session_jsonl, [{"type": "system", "message": {"content": "init"}}])

    sa_dir = proj / "session-003" / "subagents"
    sa_dir.mkdir(parents=True)
    sa_jsonl = sa_dir / "agent-aold.jsonl"
    sa_jsonl.write_text('{"type":"user","message":{"content":"test"}}\n')
    meta = sa_dir / "agent-aold.meta.json"
    meta.write_text('{"agentType":"deployer","description":"Deploy"}')
    # mtime을 5분 전으로
    old_time = datetime.now().timestamp() - 300
    os.utime(sa_jsonl, (old_time, old_time))

    now = datetime.now(timezone.utc)
    result = detect_active_agents(projects_dir, claude_dir, now)
    assert_eq("오래된 subagent 파일 무시", result, [])


# ── Test 14: _identify_agent_from_meta 직접 테스트 ──
print("Test 14: _identify_agent_from_meta")
with tempfile.TemporaryDirectory() as tmpdir:
    sa_jsonl = Path(tmpdir) / "agent-a999.jsonl"
    sa_jsonl.write_text("{}\n")
    meta = Path(tmpdir) / "agent-a999.meta.json"
    meta.write_text('{"agentType":"figma-reader","description":"Read design"}')
    result = _identify_agent_from_meta(sa_jsonl)
    assert_eq("meta.json에서 figma-reader 읽기", result, "figma-reader")

    # meta.json 없는 경우
    sa_jsonl2 = Path(tmpdir) / "agent-a888.jsonl"
    sa_jsonl2.write_text("{}\n")
    result2 = _identify_agent_from_meta(sa_jsonl2)
    assert_eq("meta.json 없으면 빈 문자열", result2, "")


# ── Test 11: 실제 JSONL로 테스트 (있으면) ──
print("Test 11: 실제 JSONL 검증")
real_jsonl = Path(os.path.expanduser(
    "~/.claude/projects/-Users-miki-Developer-zellij-claude-monitor/"
    "fe3713dc-45b6-421c-a196-5b36ab032299.jsonl"
))
if real_jsonl.exists():
    size = real_jsonl.stat().st_size
    result = _parse_active_agents(real_jsonl, size, 32768)
    # 이 세션의 bruno-manager 2개는 모두 완료됨
    assert_eq("실제 JSONL — 완료된 에이전트는 비활성", result, set())
else:
    passed += 1
    print("  ✓ (실제 JSONL 없음, 스킵)")


# ── 결과 ──
print(f"\n{'=' * 40}")
print(f"Results: {passed} passed, {failed} failed")
if failed > 0:
    sys.exit(1)
else:
    print("All tests passed!")
