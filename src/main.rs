mod collector;
mod data;
mod render;
mod state;

use std::collections::BTreeMap;

use state::DashboardState;
use zellij_tile::prelude::*;

register_plugin!(DashboardState);

/// WASI에서 HOME=/root 문제를 우회하여 실제 유저 홈 디렉토리를 찾는다.
/// 1순위: 환경변수 HOME (/root이 아닌 경우)
/// 2순위: /etc/passwd에서 현재 UID의 홈 디렉토리 파싱
/// 3순위: /root (fallback)
fn resolve_home() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() && home != "/root" {
        return home;
    }
    // /etc/passwd에서 UID 501 (macOS 기본 유저) 또는 첫 번째 일반 유저의 홈 찾기
    if let Ok(contents) = std::fs::read_to_string("/etc/passwd") {
        for line in contents.lines() {
            let fields: Vec<&str> = line.split(':').collect();
            if fields.len() >= 6 {
                if let Ok(uid) = fields[2].parse::<u32>() {
                    // macOS: UID >= 500, Linux: UID >= 1000
                    if uid >= 500 && uid < 65534 {
                        return fields[5].to_string();
                    }
                }
            }
        }
    }
    home
}

impl ZellijPlugin for DashboardState {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        // WASI 환경에서 HOME=/root이므로 /etc/passwd에서 실제 홈 디렉토리 추론
        let home = resolve_home();

        self.claude_dir = configuration
            .get("claude_dir")
            .cloned()
            .unwrap_or_else(|| format!("{}/.claude", home));

        self.plan = configuration
            .get("plan")
            .cloned()
            .unwrap_or_else(|| "max5".to_string());

        self.monitor_script = configuration
            .get("monitor_script")
            .cloned()
            .unwrap_or_else(|| {
                let user_home = self.claude_dir.trim_end_matches("/.claude");
                format!("{}/.config/zellij/plugins/monitor-data.py", user_home)
            });

        // 권한 요청
        request_permission(&[PermissionType::RunCommands]);

        // 이벤트 구독 (Key로 스크롤 지원)
        subscribe(&[
            EventType::Timer,
            EventType::RunCommandResult,
            EventType::PermissionRequestResult,
            EventType::Key,
            EventType::Mouse,
        ]);

        // 1초 후 첫 데이터 수집
        set_timeout(1.0);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Timer(_) => {
                if !self.too_narrow {
                    collector::collect_data(self);
                }
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
            Event::Key(key_with_mod) => {
                let max_scroll = self.content_height.saturating_sub(1);
                let bare = key_with_mod.bare_key;
                match bare {
                    BareKey::Down | BareKey::Char('j') => {
                        if self.scroll_offset < max_scroll {
                            self.scroll_offset += 1;
                        }
                        true
                    }
                    BareKey::Up | BareKey::Char('k') => {
                        if self.scroll_offset > 0 {
                            self.scroll_offset -= 1;
                        }
                        true
                    }
                    BareKey::PageDown | BareKey::Char(' ') => {
                        self.scroll_offset = (self.scroll_offset + 10).min(max_scroll);
                        true
                    }
                    BareKey::PageUp => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(10);
                        true
                    }
                    BareKey::Home | BareKey::Char('g') => {
                        self.scroll_offset = 0;
                        true
                    }
                    BareKey::End => {
                        self.scroll_offset = max_scroll;
                        true
                    }
                    BareKey::Char('G') => {
                        // Shift+g (대문자 G)
                        if key_with_mod.key_modifiers.contains(&KeyModifier::Shift) {
                            self.scroll_offset = max_scroll;
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            Event::Mouse(mouse) => {
                match mouse {
                    Mouse::ScrollDown(_) => {
                        let max_scroll = self.content_height.saturating_sub(1);
                        self.scroll_offset = (self.scroll_offset + 3).min(max_scroll);
                        true
                    }
                    Mouse::ScrollUp(_) => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(3);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        render::draw_dashboard(self, rows, cols);
    }
}
