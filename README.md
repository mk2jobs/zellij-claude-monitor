# zellij-claude-monitor

A [Zellij](https://zellij.dev) WASM plugin that monitors your [Claude Code](https://docs.anthropic.com/en/docs/claude-code) usage in real time.

Track active agents, team activity, session context, and Zellij session management — all inside your terminal.

![Zellij Plugin](https://img.shields.io/badge/Zellij-WASM_Plugin-orange)
![Rust](https://img.shields.io/badge/Rust-wasm32--wasip1-blue)
![License](https://img.shields.io/badge/License-MIT-green)

## Features

- **Agent status** — lists `~/.claude/agents/` with live active/idle detection
- **Team monitoring** — team members, task progress (pending/in_progress/completed)
- **Session context** — Claude Code statusLine integration, model/project/context usage progress bar
- **Session stats** — active sessions, MCP server count, agent/skill counts
- **Zellij session manager** — switch, kill, delete sessions with keyboard navigation
- **Auto-refresh** — 30-second update cycle with flicker-free native Zellij rendering

## Screenshot

```
┌─ Monitor ────────────────────────────────────┐
│ [MAX5] 2026-04-14                            │
│──────────────────────────────────────────────│
│ Opus | Opus 85% | Sonnet 15%                 │
│ Claude Opus 4.6 | my-project                 │
│ Context [████████████░░░░░░░░] 62%           │
│ Ag:32 Sess:1 MCP:5 Sk:79                    │
│──────────────────────────────────────────────│
│ ● fastify-api-dev  ○ code-reviewer           │
│ ○ config-optimizer ○ db-architect            │
│ ○ react-next-dev   ○ sbs-publisher           │
│──────────────────────────────────────────────│
│ Sess:3 dead:1 (s:open)                       │
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
git clone https://github.com/mk2jobs/zellij-claude-monitor.git
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

The statusLine script saves session data to `~/.claude/statusline.json`, which the plugin reads every 30 seconds.

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

## Dashboard Elements

### Header

| Element | Description |
|---------|-------------|
| `[MAX5]` | Current subscription plan |
| Date | Today's date |

### Session Context

| Element | Description |
|---------|-------------|
| Model + breakdown | Current model and output token distribution by model |
| Session name | Active Claude Code session (from statusLine) |
| Context bar | Context window usage percentage |

### Counts

| Element | Description |
|---------|-------------|
| `Ag` | Number of registered agents |
| `Sess` | Active Claude Code sessions (JSONL modified in last 5 min) |
| `MCP` | Connected MCP server count |
| `Sk` | Number of registered skills |

### Agent List

| Symbol | Meaning |
|--------|---------|
| `●` (green) | Active agent — running subagent or in-progress team task |
| `○` | Idle agent |

### Zellij Session Manager

Press `s` to enter session mode.

| Key | Action |
|-----|--------|
| `s` | Toggle session mode |
| `j` / `↓` | Navigate down |
| `k` / `↑` | Navigate up |
| `Enter` | Attach to selected session |
| `d` / `x` | Kill selected session (or delete dead session) |
| `Esc` / `q` | Exit session mode |

### Scrolling

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `Space` / `PageDown` | Scroll down 10 lines |
| `PageUp` | Scroll up 10 lines |
| `g` / `Home` | Scroll to top |
| `G` / `End` | Scroll to bottom |

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Zellij Runtime                      │
│                                                      │
│  ┌──────────────┐     Event::Timer (30s)             │
│  │  main.rs     │─────────────────┐                  │
│  │  ZellijPlugin│                 ▼                  │
│  │              │     ┌────────────────────┐         │
│  │  load()      │     │  collector.rs      │         │
│  │  update()    │     │  3~5x run_command()│         │
│  │  render()    │     └────────┬───────────┘         │
│  └──────┬───────┘              │                     │
│         │              Event::RunCommandResult        │
│         ▼                      │                     │
│  ┌──────────────┐     ┌───────▼────────────┐        │
│  │  render.rs   │     │  state.rs          │        │
│  │  draw_       │     │  DashboardState    │        │
│  │  dashboard() │◄────│  agents, monitor,  │        │
│  │  Text + Color│     │  sessions, teams   │        │
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

### Data Collection

Dynamic commands run every 30 seconds. Static commands (agents, skills) run every 60 seconds.

| Command | Data |
|---------|------|
| `date +%Y-%m-%d` | Today's date |
| `cat ~/.claude/statusline.json` | Session context (model/project/context%) |
| `python3 monitor-data.py` | Active agents, teams, session/MCP counts |
| `ls ~/.claude/agents/` | Agent list (every 60s) |
| `ls ~/.claude/skills/` | Skill list (every 60s) |

## Project Structure

```
zellij-claude-monitor/
├── .cargo/
│   └── config.toml          # target = "wasm32-wasip1"
├── src/
│   ├── main.rs              # ZellijPlugin impl, event loop
│   ├── state.rs             # DashboardState struct
│   ├── data.rs              # Data models (AgentInfo, MonitorData, TeamInfo, etc.)
│   ├── collector.rs         # run_command dispatch + result parsing
│   └── render.rs            # Text UI rendering (colors, progress bars)
├── scripts/
│   ├── monitor-data.py      # Agent detection, team collection
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

- WASM sandbox cannot access `~/.claude/` directly — uses `run_command` to shell out to the host
- Requires `RunCommands` permission in Zellij (prompted on first run)
- `monitor-data.py` uses `find -mmin` for file discovery — initial scan of large `~/.claude/projects/` directories may take ~1 second

## Dev Story

See [DEVLOG.md](DEVLOG.md) for the full development story — built from zero Rust knowledge entirely through Claude Code conversations.

## License

[MIT](LICENSE)
