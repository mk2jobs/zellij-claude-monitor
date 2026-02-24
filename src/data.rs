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
