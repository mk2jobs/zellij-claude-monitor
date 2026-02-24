use std::collections::BTreeMap;

use crate::data::{AgentInfo, SkillInfo, StatsCacheJson, TodayStats};
use crate::state::DashboardState;
use zellij_tile::prelude::*;

/// 커맨드 식별용 컨텍스트 키
const CMD_KEY: &str = "cmd";

/// 커맨드 종류
const CMD_AGENTS: &str = "agents";
const CMD_STATS: &str = "stats";
const CMD_TASKS: &str = "tasks";
const CMD_SKILLS: &str = "skills";
const CMD_DATE: &str = "date";

fn make_context(cmd: &str) -> BTreeMap<String, String> {
    let mut ctx = BTreeMap::new();
    ctx.insert(CMD_KEY.to_string(), cmd.to_string());
    ctx
}

/// 5개 run_command를 디스패치하고 pending_commands를 설정
pub fn collect_data(state: &mut DashboardState) {
    let dir = &state.claude_dir;
    if dir.is_empty() {
        return;
    }

    state.pending_commands = 5;

    // 1. 에이전트 목록
    run_command(
        &["ls", "-1", &format!("{}/agents/", dir)],
        make_context(CMD_AGENTS),
    );

    // 2. 통계 캐시
    run_command(
        &["cat", &format!("{}/stats-cache.json", dir)],
        make_context(CMD_STATS),
    );

    // 3. 활성 태스크 (.lock 파일 수)
    run_command(
        &[
            "find",
            &format!("{}/tasks/", dir),
            "-name",
            "*.lock",
            "-type",
            "f",
        ],
        make_context(CMD_TASKS),
    );

    // 4. 스킬 목록
    run_command(
        &["ls", "-1", &format!("{}/skills/", dir)],
        make_context(CMD_SKILLS),
    );

    // 5. 오늘 날짜
    run_command(&["date", "+%Y-%m-%d"], make_context(CMD_DATE));
}

/// RunCommandResult 이벤트 처리. 모든 커맨드가 완료되면 true 반환
pub fn handle_command_result(
    state: &mut DashboardState,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
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
                    .map(|l| {
                        let name = l.trim_end_matches(".md").to_string();
                        AgentInfo { name }
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
        CMD_TASKS => {
            if success {
                state.tasks_count = output.lines().filter(|l| !l.is_empty()).count();
            } else {
                state.tasks_count = 0;
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
                // 날짜 갱신 후 stats가 이미 로드되어 있으면 다시 매칭
                if state.stats.sessions == 0 && !state.today_date.is_empty() {
                    // stats 재파싱은 다음 사이클에서 수행
                }
            }
        }
        _ => {}
    }

    // stderr에 permission 관련 에러가 있는지 확인
    let err_output = String::from_utf8_lossy(&stderr);
    if err_output.contains("Permission denied") || err_output.contains("permission") {
        state.permission_error = true;
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

        // 오늘 날짜와 매칭되는 활동 찾기
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

            // 오늘의 토큰 합산
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
