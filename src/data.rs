use serde::Deserialize;
use std::collections::HashMap;

#[derive(Default, Clone, Debug)]
pub struct AgentInfo {
    pub name: String,
}

#[derive(Default, Clone, Debug)]
pub struct SkillInfo {
    pub name: String,
}

/// stats-cache.json의 dailyActivity 항목
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DailyActivity {
    pub date: String,
    pub message_count: u64,
    pub session_count: u64,
    pub tool_call_count: u64,
}

/// stats-cache.json의 dailyModelTokens 항목
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DailyModelTokens {
    pub date: String,
    pub tokens_by_model: HashMap<String, u64>,
}

/// stats-cache.json 전체 구조
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatsCacheJson {
    pub daily_activity: Vec<DailyActivity>,
    pub daily_model_tokens: Vec<DailyModelTokens>,
    pub total_sessions: u64,
    pub total_messages: u64,
}

/// 오늘의 통계 요약
#[derive(Default, Clone, Debug)]
pub struct TodayStats {
    pub sessions: u64,
    pub messages: u64,
    pub tool_calls: u64,
    pub tokens: u64,
    pub total_sessions: u64,
    pub total_messages: u64,
}

/// 팀 멤버 정보
#[derive(Deserialize, Default, Clone, Debug)]
pub struct TeamMember {
    pub name: String,
    pub agent_type: String,
    #[serde(default)]
    pub task: String,
    #[serde(default)]
    pub busy: bool,
}

/// 팀 정보
#[derive(Deserialize, Default, Clone, Debug)]
pub struct TeamInfo {
    pub name: String,
    #[serde(default)]
    pub members: Vec<TeamMember>,
    #[serde(default)]
    pub tasks_pending: u64,
    #[serde(default)]
    pub tasks_in_progress: u64,
    #[serde(default)]
    pub tasks_completed: u64,
}

/// Claude Code statusline 세션 정보
#[derive(Deserialize, Default, Clone, Debug)]
pub struct SessionModel {
    #[serde(default)]
    pub display_name: String,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct SessionContextWindow {
    #[serde(default)]
    pub used_percentage: f64,
    #[serde(default)]
    pub remaining_percentage: f64,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct SessionWorkspace {
    #[serde(default)]
    pub current_dir: String,
    #[serde(default)]
    pub project_dir: String,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct SessionInfo {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub session_name: String,
    #[serde(default)]
    pub model: SessionModel,
    #[serde(default)]
    pub workspace: SessionWorkspace,
    #[serde(default)]
    pub context_window: SessionContextWindow,
    #[serde(default)]
    pub version: String,
}

/// monitor-data.py 출력 JSON 구조
#[derive(Deserialize, Default, Clone, Debug)]
pub struct MonitorData {
    pub burn_rate: f64,
    pub cost_rate: f64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub output_tokens: u64,
    pub messages: u64,
    pub tokens_exhaust_min: f64,
    pub cost_exhaust_min: f64,
    #[serde(default)]
    pub tokens_exhaust_at: String,
    #[serde(default)]
    pub cost_exhaust_at: String,
    pub reset_time: String,
    pub plan: String,
    pub token_limit: u64,
    pub cost_limit: f64,
    pub exceeded: bool,
    pub active: bool,
    #[serde(default)]
    pub active_agents: Vec<String>,
    #[serde(default)]
    pub teams: Vec<TeamInfo>,
    #[serde(default)]
    pub current_model: String,
    #[serde(default)]
    pub model_breakdown: std::collections::HashMap<String, f64>,
}
