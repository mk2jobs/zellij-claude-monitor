use std::collections::BTreeMap;

use crate::data::{AgentInfo, MonitorData, SessionInfo, SkillInfo};
use crate::state::DashboardState;
use zellij_tile::prelude::*;

const CMD_KEY: &str = "cmd";
const CMD_AGENTS: &str = "agents";
// CMD_STATS removed — Today/Total 통계 표시 제거
const CMD_SESSIONS: &str = "sessions";
const CMD_MCPS: &str = "mcps";
const CMD_SKILLS: &str = "skills";
const CMD_DATE: &str = "date";
const CMD_MONITOR: &str = "monitor";
const CMD_SESSION: &str = "session";

const TOTAL_COMMANDS: usize = 7;

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

    // 이전 명령이 아직 실행 중이면 새 명령 spawn 차단 (프로세스 누적 방지)
    if state.pending_commands > 0 {
        return;
    }

    state.pending_commands = TOTAL_COMMANDS;

    run_command(
        &["ls", "-1", &format!("{}/agents/", dir)],
        make_context(CMD_AGENTS),
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
        // CMD_STATS removed
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

// parse_stats removed — Today/Total 통계 표시 제거
