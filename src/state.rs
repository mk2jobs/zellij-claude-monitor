use crate::data::{AgentInfo, MonitorData, SessionInfo, SkillInfo};
use std::time::Duration;

#[derive(Default)]
pub struct DashboardState {
    /// ~/.claude 경로 (플러그인 설정에서 주입)
    pub claude_dir: String,
    /// 구독 플랜 (pro, max5, max20)
    pub plan: String,
    /// monitor-data.py 스크립트 경로
    pub monitor_script: String,
    /// 에이전트 목록
    pub agents: Vec<AgentInfo>,
    /// 활성 세션 수 (최근 5분 이내)
    pub active_sessions: usize,
    /// MCP 서버 수
    pub mcps_count: usize,
    /// 스킬 목록
    pub skills: Vec<SkillInfo>,
    /// 모니터 데이터 (burn rate, cost 등)
    pub monitor: MonitorData,
    /// 대기 중인 커맨드 수 (0이면 렌더 가능)
    pub pending_commands: usize,
    /// 오늘 날짜 (YYYY-MM-DD)
    pub today_date: String,
    /// 현재 세션 정보 (statusline)
    pub session: SessionInfo,
    /// 권한 오류 여부
    pub permission_error: bool,
    /// 초기 로드 완료 여부
    pub loaded: bool,
    /// 스크롤 오프셋 (전체 대시보드)
    pub scroll_offset: usize,
    /// 전체 콘텐츠 높이 (마지막 렌더에서 계산)
    pub content_height: usize,
    /// Zellij 세션 목록 (네이티브 API)
    pub zellij_sessions: Vec<zellij_tile::prelude::SessionInfo>,
    /// 종료 후 복원 가능한 세션 목록
    pub dead_sessions: Vec<(String, Duration)>,
    /// 세션 리스트 내 커서 위치
    pub selected_session: usize,
    /// 세션 리스트 포커스 모드
    pub session_mode: bool,
    /// 정적 데이터(agents, skills) 갱신 사이클 카운터
    pub static_refresh_counter: usize,
    /// 커맨드 세대 번호 (이전 사이클의 응답을 무시하기 위함)
    pub generation: usize,
}
