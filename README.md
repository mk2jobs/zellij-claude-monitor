# zellij-claude-monitor

A [Zellij](https://zellij.dev) WASM plugin that monitors your [Claude Code](https://docs.anthropic.com/en/docs/claude-code) usage in real time.

Track token consumption, costs, burn rate, rate limit resets, active agents, and team activity — all inside your terminal.

![Zellij Plugin](https://img.shields.io/badge/Zellij-WASM_Plugin-orange)
![Rust](https://img.shields.io/badge/Rust-wasm32--wasip1-blue)
![License](https://img.shields.io/badge/License-MIT-green)

## Features

- **Token monitoring** — output token usage, burn rate (tok/min), estimated exhaustion time
- **Cost tracking** — real-time cost calculation based on model pricing (Opus/Sonnet/Haiku)
- **Rate limit** — 5-hour fixed window reset time, limit exceeded warnings
- **Agent status** — lists `~/.claude/agents/` with live active/idle detection
- **Team monitoring** — team members, task progress (pending/in_progress/completed)
- **Session context** — Claude Code statusLine integration, model/project/context usage progress bar
- **Session stats** — today/total sessions, messages, MCP server count
- **Auto-refresh** — 5-second update cycle with flicker-free native Zellij rendering

## Screenshot

```
┌─ Monitor ────────────────────────────────────┐
│ ● Active  Claude Monitor [MAX5]  2026-03-04  │
│──────────────────────────────────────────────│
│ Claude Opus 4.6  Opus 85.2% | Sonnet 14.8%  │
│ Claude Opus 4.6 | my-project                 │
│ Context [████████████░░░░░░░░] 62%           │
│──────────────────────────────────────────────│
│ Burn: 361.5 tok/min   Cost: $0.7476 /min     │
│ Tokens: 10.7K / 88.0K  Cost: $22.15 / $35.00│
│ Exhaust: 19:57 / 16:41  Reset: 20:00         │
│──────────────────────────────────────────────│
│ Today  Sess: 0  Msg: 0  Tools: 0  Tok: 0    │
│ Total  Sess: 166  Msg: 43.5K                 │
│──────────────────────────────────────────────│
│ Agents: 16  Sessions: 1  MCPs: 1  Skills: 41│
│                                              │
│ ○ api-dev          ○ code-reviewer           │
│ ○ config-optimizer ○ db-architect            │
│ ● fastify-api-dev  ○ frontend-dev            │
│ ○ react-next-dev   ○ sbs-publisher           │
└──────────────────────────────────────────────┘
```

## Requirements

- [Zellij](https://zellij.dev/) 0.41+ (this is a Zellij WASM plugin — does not work in tmux or plain terminals)
- [Rust](https://rustup.rs/) (stable) + `wasm32-wasip1` target
- Python 3.6+ (standard library only, no pip packages needed)

## Installation

### 1. Add the wasm32-wasip1 target

```bash
rustup target add wasm32-wasip1
```

### 2. Build & install

```bash
git clone https://github.com/miki-saarna/zellij-claude-monitor.git
cd zellij-claude-monitor
./install.sh
```

`install.sh` will:
- Build the WASM binary via `cargo build --release`
- Copy `zellij-claude-monitor.wasm` + helper scripts to `~/.config/zellij/plugins/`

### 3. Add to your Zellij layout

Create or edit `~/.config/zellij/layouts/claude.kdl`:

```kdl
layout {
    pane size=1 borderless=true {
        plugin location="zellij:tab-bar"
    }

    pane split_direction="vertical" {
        pane size="65%" command="claude" name="Claude Code"

        pane {
            pane size="60%" name="Monitor" {
                plugin location="file:~/.config/zellij/plugins/zellij-claude-monitor.wasm" {
                    // claude_dir "$HOME/.claude"
                    plan "max5"
                }
            }
            pane size="40%" name="Files" {
                plugin location="zellij:strider"
            }
        }
    }

    pane size=1 borderless=true {
        plugin location="zellij:status-bar"
    }
}
```

### 4. Configure statusLine (optional)

To display session context (model, project, context window usage), add a statusLine to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "python3 ~/.config/zellij/plugins/statusline.py"
  }
}
```

The statusLine script saves session data to `~/.claude/statusline.json`, which the plugin reads every 5 seconds.

### 5. Launch

```bash
zellij -l claude
```

## Configuration

Set options in the `plugin` block of your KDL layout:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `claude_dir` | `~/.claude` | Path to Claude Code config directory |
| `plan` | `max5` | Subscription plan: `pro`, `max5`, `max20` |
| `monitor_script` | `~/.config/zellij/plugins/monitor-data.py` | Path to Python helper script |

### Plan Limits

| Plan | Output Token Limit | Cost Limit | Window |
|------|-------------------|-----------|--------|
| Pro | 19,000 | $18.00 | 5 hours |
| Max (5x) | 88,000 | $35.00 | 5 hours |
| Max (20x) | 220,000 | $140.00 | 5 hours |

## Dashboard Elements

### Header

| Element | Description |
|---------|-------------|
| `● Active` / `○ Idle` | Whether Claude Code has been active in the last 5 minutes |
| `[MAX5]` | Current subscription plan |

### Monitoring

| Element | Description |
|---------|-------------|
| `Burn` | Output token consumption rate (tokens/minute) |
| `Cost` | Cost consumption rate ($/minute) |
| `Tokens` | Output token usage / plan limit |
| `Cost` | Cumulative cost / plan cost limit |
| `Exhaust` (left) | Estimated time when token limit will be reached |
| `Exhaust` (right) | Estimated time when cost limit will be reached |
| `Reset` | Rate limit window reset time (5-hour fixed window) |

> **Why are token and cost exhaustion times different?**
>
> Token limits count output tokens regardless of model. Cost limits apply per-model pricing (Opus >> Sonnet >> Haiku).
> - Opus-heavy usage: cost exhausts before tokens
> - Sonnet/Haiku-heavy usage: tokens exhaust before cost
>
> **The earlier of the two is when you'll actually hit the rate limit.**

### Agent List

| Symbol | Meaning |
|--------|---------|
| `●` (green) | Active agent — has a pending `Task` tool_use without a matching `tool_result` |
| `○` | Idle agent |

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `Space` / `PageDown` | Scroll down 10 lines |
| `PageUp` | Scroll up 10 lines |

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Zellij Runtime                      │
│                                                      │
│  ┌──────────────┐     Event::Timer (5s)              │
│  │  main.rs     │─────────────────┐                  │
│  │  ZellijPlugin│                 ▼                  │
│  │              │     ┌────────────────────┐         │
│  │  load()      │     │  collector.rs      │         │
│  │  update()    │     │  8x run_command()  │         │
│  │  render()    │     └────────┬───────────┘         │
│  └──────┬───────┘              │                     │
│         │              Event::RunCommandResult        │
│         ▼                      │                     │
│  ┌──────────────┐     ┌───────▼────────────┐        │
│  │  render.rs   │     │  state.rs          │        │
│  │  draw_       │     │  DashboardState    │        │
│  │  dashboard() │◄────│  agents, stats,    │        │
│  │  Text + Color│     │  monitor, teams    │        │
│  └──────────────┘     └────────────────────┘        │
└─────────────────────────────────────────────────────┘
         │                       ▲
         │ print_text_with_      │ stdout (JSON)
         │ coordinates()         │
         ▼                       │
    ┌─────────┐          ┌───────────────┐
    │ Terminal │          │ monitor-data  │
    │ Display  │          │ .py           │
    └─────────┘          │               │
                         │ reads:        │
                         │ ~/.claude/    │
                         │  projects/    │
                         │  teams/       │
                         │  tasks/       │
                         └───────────────┘
```

### Data Collection (8 async commands)

| # | Command | Data Source |
|---|---------|------------|
| 1 | `ls ~/.claude/agents/` | Agent list |
| 2 | `cat ~/.claude/stats-cache.json` | Today/total session stats |
| 3 | `find ~/.claude/projects/ -name "*.jsonl" -mmin -5` | Active session count |
| 4 | `python3 -c "..."` (parse mcp.json) | MCP server count |
| 5 | `ls ~/.claude/skills/` | Skill list |
| 6 | `date +%Y-%m-%d` | Today's date |
| 7 | `cat ~/.claude/statusline.json` | Session context (model/project/context%) |
| 8 | `python3 monitor-data.py` | Tokens/cost/agents/teams |

## Project Structure

```
zellij-claude-monitor/
├── .cargo/
│   └── config.toml          # target = "wasm32-wasip1"
├── src/
│   ├── main.rs              # ZellijPlugin impl, event loop
│   ├── state.rs             # DashboardState struct
│   ├── data.rs              # Data models (AgentInfo, MonitorData, TeamInfo, etc.)
│   ├── collector.rs         # 8x run_command dispatch + result parsing
│   └── render.rs            # Text UI rendering (colors, progress bars)
├── scripts/
│   ├── monitor-data.py      # Token/cost calculation, agent detection, team collection
│   └── statusline.py        # Claude Code statusLine -> statusline.json bridge
├── Cargo.toml
└── install.sh               # Build + copy to plugin directory
```

## Development

```bash
# Build
cargo build --release

# Test Python helper standalone
python3 scripts/monitor-data.py ~/.claude max5 | python3 -m json.tool

# Build + install + run
./install.sh && zellij -l claude
```

## Limitations

- `stats-cache.json` is only updated when a Claude Code session ends, so Today stats may show 0 during active sessions
- WASM sandbox cannot access `~/.claude/` directly — uses `run_command` to shell out to the host
- Requires `RunCommands` permission in Zellij (prompted on first run)

## Dev Story

See [DEVLOG.md](DEVLOG.md) for the full development story — built from zero Rust knowledge entirely through Claude Code conversations.

## License

[MIT](LICENSE)
