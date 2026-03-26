use std::collections::BTreeMap;

use crate::data::{AgentInfo, MonitorData, SessionInfo, SkillInfo, StatsCacheJson, TodayStats};
use crate::state::DashboardState;
use zellij_tile::prelude::*;

const CMD_KEY: &str = "cmd";
const CMD_AGENTS: &str = "agents";
const CMD_STATS: &str = "stats";
const CMD_SESSIONS: &str = "sessions";
const CMD_MCPS: &str = "mcps";
const CMD_SKILLS: &str = "skills";
const CMD_DATE: &str = "date";
const CMD_MONITOR: &str = "monitor";
const CMD_SESSION: &str = "session";

const TOTAL_COMMANDS: usize = 8;

fn make_context(cmd: &str) -> BTreeMap<String, String> {
    let mut ctx = BTreeMap::new();
    ctx.insert(CMD_KEY.to_string(), cmd.to_string());
    ctx
}

pub fn collect_data(state: &mut DashboardState) {
    let dir = &state.claude_dir;
    if dir.is_empty() {
        return;
    }

    state.pending_commands = TOTAL_COMMANDS;

    run_command(
        &["ls", "-1", &format!("{}/agents/", dir)],
        make_context(CMD_AGENTS),
    );

    run_command(
        &["cat", &format!("{}/stats-cache.json", dir)],
        make_context(CMD_STATS),
    );

    // 활성 세션 수 (5분 이내 수정된 JSONL)
    run_command(
        &[
            "find",
            &format!("{}/projects/", dir),
            "-name",
            "*.jsonl",
            "-not",
            "-path",
            "*/subagents/*",
            "-mmin",
            "-5",
            "-type",
            "f",
        ],
        make_context(CMD_SESSIONS),
    );

    // MCP 서버 수 (mcp.json에서 mcpServers 키 수 카운트)
    run_command(
        &[
            "python3", "-c",
            &format!(
                "import json;f=open('{}/mcp.json');d=json.load(f);print(len(d.get('mcpServers',{{}})))",
                dir
            ),
        ],
        make_context(CMD_MCPS),
    );

    run_command(
        &["ls", "-1", &format!("{}/skills/", dir)],
        make_context(CMD_SKILLS),
    );

    run_command(&["date", "+%Y-%m-%d"], make_context(CMD_DATE));

    // Session 데이터 (statusline.json)
    run_command(
        &["cat", &format!("{}/statusline.json", dir)],
        make_context(CMD_SESSION),
    );

    // Monitor 데이터 수집 (Python 헬퍼 스크립트)
    run_command(
        &[
            "python3",
            &state.monitor_script,
            &state.claude_dir,
            &state.plan,
        ],
        make_context(CMD_MONITOR),
    );
}

pub fn handle_command_result(
    state: &mut DashboardState,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    _stderr: Vec<u8>,
    context: BTreeMap<String, String>,
) -> bool {
    let cmd = match context.get(CMD_KEY) {
        Some(c) => c.as_str(),
        None => {
            if state.pending_commands > 0 {
                state.pending_commands -= 1;
            }
            return state.pending_commands == 0;
        }
    };

    let success = exit_code == Some(0);
    let output = String::from_utf8_lossy(&stdout);

    match cmd {
        CMD_AGENTS => {
            if success {
                state.agents = output
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| AgentInfo {
                        name: l.trim_end_matches(".md").to_string(),
                    })
                    .collect();
            } else {
                state.agents.clear();
            }
        }
        CMD_STATS => {
            if success {
                parse_stats(state, &output);
            } else {
                state.stats = TodayStats::default();
            }
        }
        CMD_SESSIONS => {
            if success {
                state.active_sessions = output.lines().filter(|l| !l.is_empty()).count();
            } else {
                state.active_sessions = 0;
            }
        }
        CMD_MCPS => {
            if success {
                state.mcps_count = output.trim().parse::<usize>().unwrap_or(0);
            } else {
                state.mcps_count = 0;
            }
        }
        CMD_SKILLS => {
            if success {
                state.skills = output
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| SkillInfo {
                        name: l.to_string(),
                    })
                    .collect();
            } else {
                state.skills.clear();
            }
        }
        CMD_DATE => {
            if success {
                state.today_date = output.trim().to_string();
            }
        }
        CMD_SESSION => {
            if success {
                if let Ok(info) = serde_json::from_str::<SessionInfo>(&output) {
                    state.session = info;
                }
            }
        }
        CMD_MONITOR => {
            if success {
                if let Ok(data) = serde_json::from_str::<MonitorData>(&output) {
                    state.monitor = data;
                }
            }
        }
        _ => {}
    }

    if state.pending_commands > 0 {
        state.pending_commands -= 1;
    }

    if state.pending_commands == 0 {
        state.loaded = true;
    }

    state.pending_commands == 0
}

fn parse_stats(state: &mut DashboardState, json_str: &str) {
    if let Ok(cache) = serde_json::from_str::<StatsCacheJson>(json_str) {
        state.stats.total_sessions = cache.total_sessions;
        state.stats.total_messages = cache.total_messages;

        if !state.today_date.is_empty() {
            if let Some(activity) = cache
                .daily_activity
                .iter()
                .find(|a| a.date == state.today_date)
            {
                state.stats.sessions = activity.session_count;
                state.stats.messages = activity.message_count;
                state.stats.tool_calls = activity.tool_call_count;
            }

            if let Some(tokens) = cache
                .daily_model_tokens
                .iter()
                .find(|t| t.date == state.today_date)
            {
                state.stats.tokens = tokens.tokens_by_model.values().sum();
            }
        }
    }
}
