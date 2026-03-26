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

## Key Lessons

1. **You don't need to know Rust** — Claude Code handled Rust syntax, WASM targets, and Zellij SDK
2. **Validate against real data** — bugs only surface when comparing with existing tools
3. **Understand ID systems first** — agent filename != tool_use_id != session ID
4. **Rate limits use fixed windows** — not sliding; first use floor + 5 hours
