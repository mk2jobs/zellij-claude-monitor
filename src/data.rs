use serde::Deserialize;

#[derive(Default, Clone, Debug)]
pub struct AgentInfo {
    pub name: String,
}

#[derive(Default, Clone, Debug)]
pub struct SkillInfo {
    pub name: String,
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
pub struct RateLimitWindow {
    #[serde(default)]
    pub used_percentage: f64,
    #[serde(default)]
    pub resets_at: i64,
}

#[derive(Deserialize, Default, Clone, Debug)]
pub struct RateLimits {
    #[serde(default)]
    pub five_hour: RateLimitWindow,
    #[serde(default)]
    pub seven_day: RateLimitWindow,
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
    pub rate_limits: RateLimits,
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
    #[serde(default)]
    pub active_sessions: usize,
    #[serde(default)]
    pub mcps_count: usize,
}
