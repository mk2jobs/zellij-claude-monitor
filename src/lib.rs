mod collector;
mod data;
mod render;
mod state;

use std::collections::BTreeMap;

use state::DashboardState;
use zellij_tile::prelude::*;

register_plugin!(DashboardState);

impl ZellijPlugin for DashboardState {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        // 플러그인 설정에서 claude_dir 읽기
        self.claude_dir = configuration
            .get("claude_dir")
            .cloned()
            .unwrap_or_else(|| {
                // 기본값: 환경변수 HOME 기반
                format!(
                    "{}/.claude",
                    std::env::var("HOME").unwrap_or_else(|_| "/Users/miki".to_string())
                )
            });

        // 권한 요청
        request_permission(&[PermissionType::RunCommands]);

        // 이벤트 구독
        subscribe(&[
            EventType::Timer,
            EventType::RunCommandResult,
            EventType::PermissionRequestResult,
        ]);

        // 선택 불가 (정보 표시 전용)
        set_selectable(false);

        // 1초 후 첫 데이터 수집
        set_timeout(1.0);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Timer(_) => {
                collector::collect_data(self);
                // 5초 후 다음 갱신
                set_timeout(5.0);
                false
            }
            Event::RunCommandResult(exit_code, stdout, stderr, context) => {
                collector::handle_command_result(self, exit_code, stdout, stderr, context)
            }
            Event::PermissionRequestResult(permission) => {
                if permission == PermissionStatus::Granted {
                    self.permission_error = false;
                } else {
                    self.permission_error = true;
                }
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        render::draw_dashboard(self, rows, cols);
    }
}
