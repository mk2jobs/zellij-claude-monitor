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
        // claude_dir: 설정에 명시되어 있으면 사용, 아니면 /Users/miki 하드코딩
        self.claude_dir = configuration
            .get("claude_dir")
            .cloned()
            .unwrap_or_else(|| "/Users/miki/.claude".to_string());

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
        request_permission(&[
            PermissionType::RunCommands,
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);

        // 이벤트 구독 (Key로 스크롤 지원, SessionUpdate로 세션 목록)
        subscribe(&[
            EventType::Timer,
            EventType::RunCommandResult,
            EventType::PermissionRequestResult,
            EventType::Key,
            EventType::Mouse,
            EventType::SessionUpdate,
        ]);

        // 1초 후 첫 데이터 수집
        set_timeout(1.0);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Timer(_) => {
                collector::collect_data(self);
                set_timeout(30.0);
                false
            }
            Event::SessionUpdate(sessions, dead_sessions) => {
                self.zellij_sessions = sessions;
                self.dead_sessions = dead_sessions;
                // 커서가 범위를 벗어나면 보정
                let total = self.zellij_sessions.len() + self.dead_sessions.len();
                if total > 0 && self.selected_session >= total {
                    self.selected_session = total - 1;
                }
                // session_mode일 때만 즉시 렌더링 (일반 모드에서는 Timer 주기에 맡김)
                self.session_mode
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
                let bare = key_with_mod.bare_key;

                // 세션 모드일 때: 세션 탐색/선택/종료
                if self.session_mode {
                    let total = self.zellij_sessions.len() + self.dead_sessions.len();
                    match bare {
                        BareKey::Down | BareKey::Char('j') => {
                            if total > 0 && self.selected_session < total - 1 {
                                self.selected_session += 1;
                            }
                            true
                        }
                        BareKey::Up | BareKey::Char('k') => {
                            if self.selected_session > 0 {
                                self.selected_session -= 1;
                            }
                            true
                        }
                        BareKey::Enter => {
                            // 활성 세션으로 전환
                            if self.selected_session < self.zellij_sessions.len() {
                                let name = &self.zellij_sessions[self.selected_session].name;
                                if !self.zellij_sessions[self.selected_session].is_current_session {
                                    switch_session(Some(name));
                                }
                            }
                            true
                        }
                        BareKey::Char('d') | BareKey::Char('x') => {
                            // 세션 종료 (현재 세션 제외)
                            if self.selected_session < self.zellij_sessions.len() {
                                let session = &self.zellij_sessions[self.selected_session];
                                if !session.is_current_session {
                                    kill_sessions(&[&session.name]);
                                }
                            } else {
                                // dead session 삭제
                                let dead_idx = self.selected_session - self.zellij_sessions.len();
                                if dead_idx < self.dead_sessions.len() {
                                    let name = &self.dead_sessions[dead_idx].0;
                                    delete_dead_session(name);
                                }
                            }
                            true
                        }
                        BareKey::Esc | BareKey::Char('s') | BareKey::Char('q') => {
                            self.session_mode = false;
                            true
                        }
                        _ => false,
                    }
                } else {
                    // 일반 모드: 스크롤
                    let max_scroll = self.content_height.saturating_sub(1);
                    match bare {
                        BareKey::Char('s') => {
                            // 세션 모드 진입
                            self.session_mode = true;
                            true
                        }
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
