use std::collections::BTreeMap;

use crate::data::{AgentInfo, MonitorData, SessionInfo, SkillInfo};
use crate::state::DashboardState;
use zellij_tile::prelude::*;

const CMD_KEY: &str = "cmd";
const GEN_KEY: &str = "gen";
const CMD_AGENTS: &str = "agents";
const CMD_SKILLS: &str = "skills";
const CMD_DATE: &str = "date";
const CMD_MONITOR: &str = "monitor";
const CMD_SESSION: &str = "session";

/// 정적 데이터(agents, skills) 갱신 간격 (사이클 수, 5초 × 12 = 60초)
const STATIC_REFRESH_INTERVAL: usize = 12;

fn make_context(cmd: &str, generation: usize) -> BTreeMap<String, String> {
    let mut ctx = BTreeMap::new();
    ctx.insert(CMD_KEY.to_string(), cmd.to_string());
    ctx.insert(GEN_KEY.to_string(), generation.to_string());
    ctx
}

pub fn collect_data(state: &mut DashboardState) {
    let dir = &state.claude_dir;
    if dir.is_empty() {
        return;
    }

    // 새 세대 시작 — 이전 사이클의 in-flight 응답은 generation 불일치로 무시됨
    state.generation = state.generation.wrapping_add(1);
    let gen = state.generation;

    // 이전 사이클이 미완료였으면 loaded 처리
    if state.pending_commands > 0 && !state.loaded {
        state.loaded = true;
    }

    // 정적 데이터(agents, skills)는 STATIC_REFRESH_INTERVAL 사이클마다만 갱신
    let refresh_static = state.static_refresh_counter == 0;
    state.static_refresh_counter = (state.static_refresh_counter + 1) % STATIC_REFRESH_INTERVAL;

    // 동적 커맨드: date, session, monitor (항상 실행)
    let mut cmd_count = 3;
    if refresh_static {
        cmd_count += 2; // agents, skills
    }
    state.pending_commands = cmd_count;

    if refresh_static {
        run_command(
            &["ls", "-1", &format!("{}/agents/", dir)],
            make_context(CMD_AGENTS, gen),
        );

        run_command(
            &["ls", "-1", &format!("{}/skills/", dir)],
            make_context(CMD_SKILLS, gen),
        );
    }

    run_command(&["date", "+%Y-%m-%d"], make_context(CMD_DATE, gen));

    // Session 데이터 (statusline.json)
    run_command(
        &["cat", &format!("{}/statusline.json", dir)],
        make_context(CMD_SESSION, gen),
    );

    // Monitor 데이터 수집 (Python 헬퍼 스크립트)
    // active_sessions, mcps_count도 여기서 함께 반환
    run_command(
        &[
            "python3",
            &state.monitor_script,
            &state.claude_dir,
            &state.plan,
        ],
        make_context(CMD_MONITOR, gen),
    );
}

pub fn handle_command_result(
    state: &mut DashboardState,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    _stderr: Vec<u8>,
    context: BTreeMap<String, String>,
) -> bool {
    // 이전 사이클의 in-flight 응답은 무시
    let ctx_gen: usize = context
        .get(GEN_KEY)
        .and_then(|g| g.parse().ok())
        .unwrap_or(0);
    if ctx_gen != state.generation {
        return false;
    }

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
                    state.active_sessions = data.active_sessions;
                    state.mcps_count = data.mcps_count;
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
