use crate::state::DashboardState;
use zellij_tile::prelude::*;

/// 문자열의 문자 수 (Unicode 안전, color_range 위치 계산용)
fn clen(s: &str) -> usize {
    s.chars().count()
}

/// 문자열에서 부분 문자열의 문자 위치를 반환
fn cfind(s: &str, substr: &str) -> usize {
    match s.find(substr) {
        Some(byte_pos) => s[..byte_pos].chars().count(),
        None => 0,
    }
}

/// 문자열을 주어진 너비로 잘라서 말줄임 처리
fn truncate(s: &str, max_w: usize) -> String {
    if max_w == 0 {
        return String::new();
    }
    let char_len: usize = s.chars().count();
    if char_len <= max_w {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_w.saturating_sub(1)).collect();
    format!("{}…", truncated)
}

/// Unix timestamp → 남은 시간 표시
fn format_reset_time(ts: i64) -> String {
    if ts <= 0 {
        return "--:--".to_string();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let remaining_secs = ts - now;
    if remaining_secs <= 0 {
        return "reset".to_string();
    }
    let total_hours = remaining_secs / 3600;
    let mins = (remaining_secs % 3600) / 60;
    let days = total_hours / 24;
    let hours = total_hours % 24;
    if days > 0 {
        format!("{}d{}h", days, hours)
    } else if hours > 0 {
        format!("{}h{}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn draw_progress_bar(percentage: f64, width: usize) -> String {
    let clamped = percentage.clamp(0.0, 100.0);
    let bar_w = width.saturating_sub(2);
    let filled = ((clamped / 100.0) * bar_w as f64).round() as usize;
    let empty = bar_w.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// 렌더링할 한 줄의 정보
struct Line {
    text: String,
    /// (color_index, start, end) 튜플 목록
    colors: Vec<(usize, usize, usize)>,
}

impl Line {
    fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            colors: Vec::new(),
        }
    }

    fn color(mut self, color: usize, start: usize, end: usize) -> Self {
        if start < end {
            self.colors.push((color, start, end));
        }
        self
    }

    fn color_all(mut self, color: usize) -> Self {
        let len = self.text.len();
        if len > 0 {
            self.colors.push((color, 0, len));
        }
        self
    }

    fn to_text(&self) -> Text {
        let mut t = Text::new(&self.text);
        for &(color, start, end) in &self.colors {
            t = t.color_range(color, start..end);
        }
        t
    }
}

pub fn draw_dashboard(state: &mut DashboardState, rows: usize, cols: usize) {
    if !state.loaded {
        print_text_with_coordinates(Text::new(" Loading..."), 0, 0, None, None);
        return;
    }

    if state.permission_error {
        print_text_with_coordinates(
            Text::new(" Grant RunCommands permission").color_range(3, ..),
            0, 0, None, None,
        );
        return;
    }

    let w = cols.min(80);
    let mon = &state.monitor;
    let sep = "─".repeat(w);

    // 모든 라인을 버퍼에 쌓기
    let mut lines: Vec<Line> = Vec::new();

    // ── 헤더 ──
    let plan_upper = state.plan.to_uppercase();
    let header_raw = format!("[{}] {}", plan_upper, state.today_date);
    let header = format!(" {}", truncate(&header_raw, w.saturating_sub(1)));
    lines.push(Line::new(&header).color_all(0));

    // 구분선
    lines.push(Line::new(&sep).color_all(0));

    // ── 모델 정보 ──
    if !mon.current_model.is_empty() {
        let mut parts: Vec<(String, f64)> = mon.model_breakdown.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        parts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let breakdown: String = parts.iter()
            .map(|(name, pct)| format!("{} {:.0}%", name, pct))
            .collect::<Vec<_>>()
            .join(" | ");

        let model_raw = format!("{} | {}", mon.current_model, breakdown);
        let model_line = format!(" {}", truncate(&model_raw, w.saturating_sub(1)));
        let model_name_end = 1 + clen(&mon.current_model);
        let model_line_cl = clen(&model_line);
        lines.push(
            Line::new(&model_line)
                .color(2, 1, model_name_end.min(model_line_cl))
                .color(4, model_name_end.min(model_line_cl), model_line_cl)
        );
    }

    // ── Session 섹션 (statusline) ──
    let sess = &state.session;
    if !sess.session_id.is_empty() {
        let sess_name = if !sess.session_name.is_empty() {
            sess.session_name.clone()
        } else if !sess.workspace.project_dir.is_empty() {
            sess.workspace.project_dir
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_string()
        } else {
            "session".to_string()
        };

        let sess_raw = format!("{} | {}", sess.model.display_name, sess_name);
        let sess_line = format!(" {}", truncate(&sess_raw, w.saturating_sub(1)));
        let model_end = (1 + clen(&sess.model.display_name)).min(clen(&sess_line));
        lines.push(Line::new(&sess_line).color(1, 1, model_end));

        // Context usage progress bar
        let pct = sess.context_window.used_percentage;
        let bar_width = w.saturating_sub(16).min(30);
        let bar = draw_progress_bar(pct, bar_width);
        let ctx_line = format!(" Context {} {:3.0}%", bar, pct);
        let color = if pct >= 80.0 { 3 } else if pct >= 50.0 { 4 } else { 2 };
        lines.push(
            Line::new(&ctx_line)
                .color(0, 1, 9)
                .color(color, 9, clen(&ctx_line))
        );
    }

    // 구분선
    lines.push(Line::new(&sep).color_all(0));

    // ── 사용량 한도 (Sess / Week 각각 한 줄) ──
    let rl = &state.session.rate_limits;

    let sess_pct = rl.five_hour.used_percentage;
    let sess_reset = format_reset_time(rl.five_hour.resets_at);
    let sess_bar_color = if sess_pct >= 90.0 { 3 } else if sess_pct >= 70.0 { 4 } else { 2 };
    let sess_bar_w = w.saturating_sub(22).min(20);
    let sess_bar = draw_progress_bar(sess_pct, sess_bar_w);
    let sess_line = format!(" Sess  {} {:3.0}%  {}", sess_bar, sess_pct, sess_reset);
    let sess_line_t = truncate(&sess_line, w);
    let sess_bar_start = 7;
    let sess_bar_end = (7 + clen(&sess_bar) + 5).min(clen(&sess_line_t));
    lines.push(
        Line::new(&sess_line_t)
            .color(6, 1, 5)
            .color(sess_bar_color, sess_bar_start, sess_bar_end)
    );

    let week_pct = rl.seven_day.used_percentage;
    let week_reset = format_reset_time(rl.seven_day.resets_at);
    let week_bar_color = if week_pct >= 90.0 { 3 } else if week_pct >= 70.0 { 4 } else { 2 };
    let week_bar = draw_progress_bar(week_pct, sess_bar_w);
    let week_line = format!(" Week  {} {:3.0}%  {}", week_bar, week_pct, week_reset);
    let week_line_t = truncate(&week_line, w);
    let week_bar_start = 7;
    let week_bar_end = (7 + clen(&week_bar) + 5).min(clen(&week_line_t));
    lines.push(
        Line::new(&week_line_t)
            .color(5, 1, 5)
            .color(week_bar_color, week_bar_start, week_bar_end)
    );

    if sess_pct >= 100.0 || week_pct >= 100.0 {
        let warn = " !! LIMIT EXCEEDED !!";
        lines.push(Line::new(warn).color_all(3));
    }

    // 구분선
    lines.push(Line::new(&sep).color_all(0));

    // ── 카운트 요약 (줄임 처리) ──
    let counts_raw = format!(
        "Ag:{} Sess:{} MCP:{} Sk:{}",
        state.agents.len(),
        state.active_sessions,
        state.mcps_count,
        state.skills.len(),
    );
    let counts = format!(" {}", truncate(&counts_raw, w.saturating_sub(1)));

    // 동적으로 색상 위치 계산
    let ag_pos = cfind(&counts, "Ag:");
    let sess_c_pos = cfind(&counts, "Sess:");
    let mcp_pos = cfind(&counts, "MCP:");
    let sk_pos = cfind(&counts, "Sk:");
    lines.push(
        Line::new(&counts)
            .color(6, ag_pos, sess_c_pos.max(ag_pos))
            .color(if state.active_sessions > 0 { 2 } else { 4 }, sess_c_pos, mcp_pos.max(sess_c_pos))
            .color(5, mcp_pos, sk_pos.max(mcp_pos))
            .color(4, sk_pos, clen(&counts))
    );

    // ── 팀 섹션 ──
    if !mon.teams.is_empty() {
        lines.push(Line::new(""));

        for team in &mon.teams {
            let total_tasks = team.tasks_pending + team.tasks_in_progress + team.tasks_completed;
            let team_header_raw = format!(
                "Team: {} [{}/{}/{}]",
                team.name,
                team.tasks_completed,
                team.tasks_in_progress,
                team.tasks_pending,
            );
            let team_header = format!(" {}", truncate(&team_header_raw, w.saturating_sub(1)));
            let team_header_cl = clen(&team_header);
            let bracket_pos = cfind(&team_header, "[");
            let bracket_pos = if bracket_pos == 0 && !team_header.contains('[') { team_header_cl } else { bracket_pos };
            lines.push(
                Line::new(&team_header)
                    .color(6, 1, 5.min(team_header_cl))
                    .color(if total_tasks > 0 { 2 } else { 0 }, bracket_pos, team_header_cl)
            );

            for member in &team.members {
                let prefix = if member.busy { "● " } else { "  " };
                let task_info = if member.task.is_empty() {
                    String::new()
                } else {
                    let max_task_len = w.saturating_sub(clen(&member.name) + clen(&member.agent_type) + 10);
                    let truncated = truncate(&member.task, max_task_len);
                    format!(" → {}", truncated)
                };
                let member_raw = format!(
                    "{}{} ({}){}", prefix, member.name, member.agent_type, task_info
                );
                let member_line = format!(" {}", truncate(&member_raw, w.saturating_sub(1)));
                if member.busy {
                    lines.push(Line::new(&member_line).color_all(2));
                } else {
                    lines.push(Line::new(&member_line));
                }
            }
        }
    }

    // ── 에이전트 리스트 (●/○ 토글, 활성 상단 정렬) ──
    if !state.agents.is_empty() {
        lines.push(Line::new(&sep).color_all(0));

        let active_set: Vec<&str> = mon.active_agents.iter().map(|s| s.as_str()).collect();

        // 활성 에이전트를 상단으로 정렬
        let mut sorted_agents: Vec<&crate::data::AgentInfo> = state.agents.iter().collect();
        sorted_agents.sort_by_key(|a| !active_set.contains(&a.name.as_str()));

        // 2열 레이아웃
        if w >= 36 {
            let col_w = w / 2;
            let name_max = col_w.saturating_sub(3);
            let mut i = 0;
            while i < sorted_agents.len() {
                let left_name = &sorted_agents[i].name;
                let left_active = active_set.contains(&left_name.as_str());
                let left_prefix = if left_active { "● " } else { "○ " };
                let left_display = truncate(left_name, name_max);
                let left = format!("{}{}", left_prefix, left_display);

                if i + 1 < sorted_agents.len() {
                    let right_name = &sorted_agents[i + 1].name;
                    let right_active = active_set.contains(&right_name.as_str());
                    let right_prefix = if right_active { "● " } else { "○ " };
                    let right_display = truncate(right_name, name_max);
                    let right = format!("{}{}", right_prefix, right_display);
                    let line_str = format!("{:<width$}{}", left, right, width = col_w);
                    let line_str_cl = clen(&line_str);
                    let mut line = Line::new(&line_str);
                    if left_active {
                        line = line.color(2, 0, col_w.min(line_str_cl));
                    }
                    if right_active {
                        line = line.color(2, col_w, line_str_cl);
                    }
                    lines.push(line);
                } else {
                    let mut line = Line::new(&left);
                    if left_active {
                        line = line.color_all(2);
                    }
                    lines.push(line);
                }
                i += 2;
            }
        } else {
            let name_max = w.saturating_sub(3);
            for agent in &sorted_agents {
                let is_active = active_set.contains(&agent.name.as_str());
                let prefix = if is_active { "● " } else { "○ " };
                let display = truncate(&agent.name, name_max);
                let line_str = format!("{}{}", prefix, display);
                if is_active {
                    lines.push(Line::new(&line_str).color_all(2));
                } else {
                    lines.push(Line::new(&line_str));
                }
            }
        }
    }

    // ── Zellij Sessions 섹션 ──
    let total_sessions = state.zellij_sessions.len();
    let dead_count = state.dead_sessions.len();
    if total_sessions > 0 || dead_count > 0 {
        if state.session_mode {
            // 세션 모드: 전체 목록 표시
            lines.push(Line::new(&sep).color_all(0));

            let mode_hint = " [SESSION] s:exit ↑↓:nav ⏎:attach d:kill";
            let hint_line = truncate(mode_hint, w);
            lines.push(Line::new(&hint_line).color_all(4));

            for (i, session) in state.zellij_sessions.iter().enumerate() {
                let is_current = session.is_current_session;
                let is_selected = i == state.selected_session;
                let marker = if is_current { "●" } else { "○" };
                let tabs = session.tabs.len();
                let clients = session.connected_clients;

                if is_selected {
                    let sess_raw = format!(">> {} {} [{}t {}c] <<", marker, session.name, tabs, clients);
                    let sess_line = truncate(&sess_raw, w);
                    lines.push(Line::new(&sess_line).color_all(3));
                } else {
                    let sess_raw = format!("   {} {} [{}t {}c]", marker, session.name, tabs, clients);
                    let sess_line = truncate(&sess_raw, w);
                    if is_current {
                        lines.push(Line::new(&sess_line).color_all(2));
                    } else {
                        lines.push(Line::new(&sess_line));
                    }
                }
            }

            // Dead (resurrectable) sessions
            if dead_count > 0 {
                let dead_header = format!(" dead --- ({})", dead_count);
                lines.push(Line::new(&truncate(&dead_header, w)).color_all(0));

                for (i, (name, duration)) in state.dead_sessions.iter().enumerate() {
                    let ago_secs = duration.as_secs();
                    let ago_str = if ago_secs >= 86400 {
                        format!("{}d ago", ago_secs / 86400)
                    } else if ago_secs >= 3600 {
                        format!("{}h ago", ago_secs / 3600)
                    } else {
                        format!("{}m ago", ago_secs / 60)
                    };
                    let dead_idx = state.zellij_sessions.len() + i;
                    let is_selected = dead_idx == state.selected_session;
                    if is_selected {
                        let dead_raw = format!(">> {} ({}) <<", name, ago_str);
                        let dead_line = truncate(&dead_raw, w);
                        lines.push(Line::new(&dead_line).color_all(3));
                    } else {
                        let dead_raw = format!("   {} ({})", name, ago_str);
                        let dead_line = truncate(&dead_raw, w);
                        lines.push(Line::new(&dead_line).color_all(0));
                    }
                }
            }
        } else {
            // 일반 모드: 요약 한 줄만 표시
            let sess_summary = if dead_count > 0 {
                format!(" Sess:{} dead:{} (s:open)", total_sessions, dead_count)
            } else {
                format!(" Sess:{} (s:open)", total_sessions)
            };
            let summary_line = truncate(&sess_summary, w);
            lines.push(Line::new(&summary_line).color_all(0));
        }
    }

    // ── 스크롤 적용 및 출력 ──
    let total_lines = lines.len();
    state.content_height = total_lines;

    let max_scroll = if total_lines > rows { total_lines - rows } else { 0 };
    if state.scroll_offset > max_scroll {
        state.scroll_offset = max_scroll;
    }

    let offset = state.scroll_offset;
    for (i, line) in lines.iter().enumerate().skip(offset) {
        let screen_y = i - offset;
        if screen_y >= rows {
            break;
        }
        print_text_with_coordinates(line.to_text(), 0, screen_y, Some(w), None);
    }

    // 스크롤 인디케이터
    if total_lines > rows {
        let indicator = format!(
            " [{}/{}] ↑k ↓j",
            offset + 1,
            total_lines.saturating_sub(rows) + 1,
        );
        let hint_y = rows.saturating_sub(1);
        let hint_x = w.saturating_sub(indicator.len());
        print_text_with_coordinates(
            Text::new(&indicator).color_range(0, ..),
            hint_x, hint_y, None, None,
        );
    }
}
