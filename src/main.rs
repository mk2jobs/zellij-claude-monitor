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
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());

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
                format!("{}/.config/zellij/plugins/monitor-data.py", home)
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
