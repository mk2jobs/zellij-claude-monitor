use crate::data::{AgentInfo, SkillInfo, TodayStats};

#[derive(Default)]
pub struct DashboardState {
    /// ~/.claude 경로 (플러그인 설정에서 주입)
    pub claude_dir: String,
    /// 에이전트 목록
    pub agents: Vec<AgentInfo>,
    /// 활성 태스크 수 (.lock 파일 기준)
    pub tasks_count: usize,
    /// 스킬 목록
    pub skills: Vec<SkillInfo>,
    /// 오늘의 통계
    pub stats: TodayStats,
    /// 대기 중인 커맨드 수 (0이면 렌더 가능)
    pub pending_commands: usize,
    /// 오늘 날짜 (YYYY-MM-DD)
    pub today_date: String,
    /// 권한 오류 여부
    pub permission_error: bool,
    /// 초기 로드 완료 여부
    pub loaded: bool,
}
