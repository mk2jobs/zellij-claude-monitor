# Development Log

> Built from scratch with zero Rust knowledge, entirely through conversation with Claude Code.
> Below are the key prompts and progression from the actual development process.

## Phase 1: Problem Recognition

I had a bash script called `claude-agents-monitor` using `while true; sleep 5` + ANSI cursor control.
Running it inside Zellij caused **pane duplication and flickering** every 5 seconds.

```
"The claude-agents-monitor script flickers and duplicates panes every 5 seconds
 in Zellij. Can we make this a native Zellij plugin instead?"
```

## Phase 2: Architecture — Zellij WASM Plugin

Claude Code investigated the Zellij WASM plugin system and proposed a plan.
It identified the WASM sandbox constraint (`/host` maps only CWD, no direct `~/.claude/` access)
and designed an architecture using `run_command` to shell out to the host.

```
"Design this in plan mode"
-> Rust + wasm32-wasip1 target, zellij-tile SDK
-> Async data collection via run_command, 5-second Timer refresh
-> 5-module split (main/state/data/collector/render)
```

## Phase 3: Scaffolding

```
"Start implementing per the plan"
-> cargo init, wasm32-wasip1 target setup
-> ZellijPlugin trait implementation, register_plugin! macro
-> First build + install.sh
```

## Phase 4: Basic Dashboard

```
"Build and check"
-> Permission denied (RunCommands) -> added permission request logic
-> Agent/skill listing working
```

## Phase 5: Token Monitoring

Ported functionality from a bash `claude-monitor` tool into a Python helper script.

```
"Show token usage, cost, and burn rate like the existing claude-monitor"
-> Created monitor-data.py (JSONL parsing, price calculation)
-> Added MonitorData struct, 7th command in collector
-> Added Monitor section to render.rs
```

## Phase 6: Bug Hunting — Comparison with Existing Monitor

Comparing against the original `claude-monitor` revealed mismatches. This was the longest phase.

```
"Burn rate shows 304,180 vs 72.9 — completely different"
-> Cause: calculating with all tokens (input+output+cache) vs output only
-> Fix: burn_rate and total_tokens now based on output tokens only

"Reset shows 06:00 but it should be 3:00 PM"
-> Cause: UTC time not converted to local
-> Fix: applied astimezone(local_tz)

"LIMIT EXCEEDED but Claude Code still works?"
-> Cause: exceeded based on cost ($126 > $35), but actual blocking is token-based
-> Fix: exceeded now based on output tokens only

"Reset: 16:00 but it's 16:08 now. Why hasn't it changed?"
-> Cause: sliding window + hour floor = always past time
-> Fix: switched to fixed window model (first use floor -> +5h -> new window after expiry)

"Existing monitor shows 8:00 PM but ours shows 20:54"
-> Cause: window start not floored to hour (15:54 -> 20:54)
-> Fix: applied replace(minute=0, second=0) -> 15:00 -> 20:00
```

## Phase 7: Active Agent Detection

Agent list appeared but there was no way to tell which ones were currently running.

```
"Mark currently active agents with bullet"
-> Attempt 1: subagent file mtime check -> failed (2min cutoff too short)
-> Attempt 2: grep agent filename in parent JSONL -> failed (different ID systems)
-> Attempt 3: Task tool_use vs tool_result matching -> success
   (pending tool_use = that subagent_type is active)
```

## Phase 8: Team Monitoring + Polish

```
"Show team info in the dashboard"
-> Parsed teams/ config.json + tasks/ status
-> Member busy state, task count display

"Tasks: 14 doesn't mean anything useful. Replace with better data"
-> .lock file count (meaningless) -> Sessions (active JSONL) + MCPs (mcp.json)
```

## Phase 9: Incident — CPU 99.9% Process Storm (2026-04-07)

### Symptom

사용 중 macOS 전체가 느려지면서 CPU 점유율 99.9%. `ps aux`로 확인하니:
- `monitor-data.py` 인스턴스 **~15개** 동시 실행
- `python3 -c "import json;f=open('mcp.json')..."` 인스턴스 **~10개** 동시 실행

### Root Cause

**Timer → spawn 경쟁 조건 (process accumulation)**

```
Timer fires (every 5s)
  └─ collect_data() spawns 7 commands (ls, find, python3, cat, date, ...)
       └─ monitor-data.py takes >5s (reads ALL JSONL files line-by-line)
            └─ Timer fires again → collect_data() spawns 7 MORE commands
                 └─ Previous processes STILL running
                      └─ ∞ accumulation → CPU saturation
```

핵심: `collect_data()`에 **이전 명령 완료 확인 없이** 무조건 새 프로세스를 spawn하는 구조.
`pending_commands` 카운터가 존재했지만, 가드로 사용되지 않고 있었다.

### Why It Got Worse Over Time

`monitor-data.py`는 `~/.claude/projects/` 하위의 **모든 JSONL 파일을 전체 읽기(line-by-line)**한다.
세션이 쌓일수록 파일 크기 증가 → 실행 시간 증가 → 5초 타이머 내 완료 실패 확률 증가 → 누적 가속화.

### Fix

**1. 동시성 가드 (`collector.rs`)**

```rust
pub fn collect_data(state: &mut DashboardState) {
    // 이전 명령이 아직 실행 중이면 새 명령 spawn 차단
    if state.pending_commands > 0 {
        return;
    }
    state.pending_commands = TOTAL_COMMANDS;
    // ...spawn commands...
}
```

**2. JSONL tail-only 읽기 (`monitor-data.py`)**

```python
# Before: 파일 전체를 한 줄씩 읽기 (수십 MB 가능)
with open(jsonl_path, encoding="utf-8") as f:
    for line in f:  # ← 전체 순회

# After: 마지막 512KB만 seek+read
with open(jsonl_path, "rb") as f:
    offset = max(0, file_size - 512 * 1024)
    if offset > 0:
        f.seek(offset)
    raw = f.read().decode("utf-8", errors="ignore")
```

### Impact Assessment

| 항목 | 상태 |
|------|------|
| 데이터 손실 | 없음 (모니터링 전용, 쓰기 없음) |
| 서비스 영향 | CPU 99.9% → macOS 전체 응답 불가 |
| 복구 방법 | `pkill -f monitor-data.py` 후 플러그인 재배포 |
| 발생 조건 | JSONL 누적 → monitor-data.py 실행 >5초 |

### Lessons (Production Perspective)

이건 개인 도구여서 `pkill`로 끝났지만, 실제 서비스였다면:

1. **Unbounded process spawn = 시한폭탄**
   - Timer/cron이 외부 프로세스를 spawn할 때, **반드시 이전 실행 완료 확인 또는 mutex/lock 필요**
   - "5초면 충분하겠지"는 가정이지 보장이 아니다

2. **O(n) 전체 읽기는 시간이 지나면 반드시 터진다**
   - 로그/세션 파일은 단조 증가. 오늘 1MB가 다음 달엔 100MB
   - tail/seek 패턴이나 인덱스 없이 전체 순회하면 SLA 위반 확정

3. **모니터링 시스템이 장애 원인이 되면 안 된다**
   - Observability 도구가 호스트 리소스를 고갈시키는 건 최악의 시나리오
   - 모니터링은 **자원 상한(cgroup, nice, timeout)** 필수

4. **"되니까 괜찮다"의 함정**
   - 초기엔 JSONL이 작아서 1초 안에 끝남 → 문제 안 보임
   - 부하가 점진적으로 증가하는 버그는 테스트에서 못 잡고 운영에서 터짐
   - **부하 테스트 또는 worst-case 시나리오 검증** 필요

### Timeline

```
[개발 시점] collect_data()에 가드 없이 출시 — 당시 JSONL 파일 작아서 무증상
[사용 누적] JSONL 파일 크기 증가, 세션 수 증가
[2026-04-07] CPU 99.9% 발견 → 원인 분석 → 가드 추가 + tail 최적화 → 배포
```

---

## Key Lessons

1. **You don't need to know Rust** — Claude Code handled Rust syntax, WASM targets, and Zellij SDK
2. **Validate against real data** — bugs only surface when comparing with existing tools
3. **Understand ID systems first** — agent filename != tool_use_id != session ID
4. **Rate limits use fixed windows** — not sliding; first use floor + 5 hours
5. **Guard your spawns** — timer-driven process creation without completion checks is a ticking bomb
6. **O(n) on growing data = guaranteed incident** — always bound your reads
