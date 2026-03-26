use crate::state::DashboardState;
use zellij_tile::prelude::*;

fn fmt_num(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn fmt_cost(c: f64) -> String {
    if c >= 100.0 {
        format!("${:.0}", c)
    } else if c >= 1.0 {
        format!("${:.2}", c)
    } else {
        format!("${:.4}", c)
    }
}

fn fmt_rate(r: f64) -> String {
    if r >= 1000.0 {
        format!("{:.0}", r)
    } else if r >= 1.0 {
        format!("{:.1}", r)
    } else {
        format!("{:.4}", r)
    }
}

/// 분을 시:분 형태로 변환
fn fmt_minutes(min: f64) -> String {
    if min < 0.0 {
        return "--:--".to_string();
    }
    let h = min as u64 / 60;
    let m = min as u64 % 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
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
        self.colors.push((color, start, end));
        self
    }

    fn color_all(mut self, color: usize) -> Self {
        let len = self.text.len();
        self.colors.push((color, 0, len));
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
    let status = if mon.active { "● Active" } else { "○ Idle" };
    let plan_upper = state.plan.to_uppercase();
    let header = format!(" {}  Claude Monitor [{}]  {}", status, plan_upper, state.today_date);
    let status_end = 1 + status.len();
    let header_color = if mon.active { 2 } else { 3 };
    lines.push(Line::new(&header).color(header_color, 1, status_end));

    // 구분선
    lines.push(Line::new(&sep).color_all(0));

    // ── 모델 정보 ──
    if !mon.current_model.is_empty() {
        let mut parts: Vec<(String, f64)> = mon.model_breakdown.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        parts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let breakdown: String = parts.iter()
            .map(|(name, pct)| format!("{} {:.1}%", name, pct))
            .collect::<Vec<_>>()
            .join(" | ");

        let model_line = format!(" {}  {}", mon.current_model, breakdown);
        let model_name_end = 1 + mon.current_model.len();
        let breakdown_start = model_name_end + 2;
        lines.push(
            Line::new(&model_line)
                .color(2, 1, model_name_end)
                .color(4, breakdown_start, model_line.len())
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

        let sess_line = format!(" {} | {}", sess.model.display_name, sess_name);
        let model_end = 1 + sess.model.display_name.len();
        lines.push(Line::new(&sess_line).color(1, 1, model_end));

        // Context usage progress bar
        let pct = sess.context_window.used_percentage;
        let bar_width = w.saturating_sub(16).min(30);
        let bar = draw_progress_bar(pct, bar_width);
        let ctx_line = format!(" Context {} {:3.0}%", bar, pct);
        let color = if pct >= 80.0 { 3 } else if pct >= 50.0 { 4 } else { 2 };
        let bar_start = 9;
        lines.push(
            Line::new(&ctx_line)
                .color(0, 1, 9)
                .color(color, bar_start, ctx_line.len())
        );

        // 구분선
        lines.push(Line::new(&sep).color_all(0));
    }

    // ── Monitor 섹션 ──
    if mon.exceeded {
        let warn = " !! LIMIT EXCEEDED !!";
        lines.push(Line::new(warn).color_all(3));
    }

    // Burn Rate + Cost Rate
    let rate_line = format!(
        " Burn: {} tok/min   Cost: {} /min",
        fmt_rate(mon.burn_rate),
        fmt_cost(mon.cost_rate),
    );
    let burn_pos = 1;
    let burn_end = burn_pos + 4;
    let burn_val_start = burn_end + 2; // after ": "
    let burn_val_end = rate_line.find("tok/min").unwrap_or(burn_val_start);
    let cost_pos = rate_line.find("Cost").unwrap_or(0);
    let cost_val_start = cost_pos + 6; // after "Cost: "
    let cost_val_end = rate_line.find("/min").unwrap_or(cost_val_start);
    lines.push(
        Line::new(&rate_line)
            .color(6, burn_pos, burn_end)
            .color(6, burn_val_start, burn_val_end)
            .color(5, cost_pos, cost_pos + 4)
            .color(5, cost_val_start, cost_val_end)
    );

    // Token / Cost usage — output tokens 기준
    let tok_exceeded = mon.total_tokens >= mon.token_limit && mon.token_limit > 0;
    let cost_exceeded = mon.total_cost >= mon.cost_limit && mon.cost_limit > 0.0;
    let usage_line = format!(
        " Tokens: {} / {}   Cost: {} / {}",
        fmt_num(mon.total_tokens),
        fmt_num(mon.token_limit),
        fmt_cost(mon.total_cost),
        fmt_cost(mon.cost_limit),
    );
    let tok_pos = usage_line.find("Tokens").unwrap_or(0);
    // "Tokens: " 뒤의 값 영역
    let tok_val_start = tok_pos + 8;
    let cost2_pos = usage_line.rfind("Cost").unwrap_or(0);
    let cost_val_start = cost2_pos + 6;
    let tok_color = if tok_exceeded { 3 } else { 4 };
    let cost_color = if cost_exceeded { 3 } else { 5 };
    lines.push(
        Line::new(&usage_line)
            .color(tok_color, tok_pos, tok_pos + 6)
            .color(tok_color, tok_val_start, cost2_pos.saturating_sub(3))
            .color(cost_color, cost2_pos, cost2_pos + 4)
            .color(cost_color, cost_val_start, usage_line.len())
    );

    // Predictions
    let tok_exhaust = if mon.exceeded {
        "exceeded".to_string()
    } else if !mon.tokens_exhaust_at.is_empty() {
        mon.tokens_exhaust_at.clone()
    } else if mon.tokens_exhaust_min >= 0.0 {
        fmt_minutes(mon.tokens_exhaust_min)
    } else {
        "--:--".to_string()
    };
    let cost_exhaust = if !mon.cost_exhaust_at.is_empty() {
        mon.cost_exhaust_at.clone()
    } else if mon.cost_exhaust_min >= 0.0 {
        fmt_minutes(mon.cost_exhaust_min)
    } else {
        "--:--".to_string()
    };
    let reset = if mon.reset_time.is_empty() { "--:--".to_string() } else { mon.reset_time.clone() };
    let pred_line = format!(" Exhaust: {} / {}  Reset: {}", tok_exhaust, cost_exhaust, reset);
    let exhaust_pos = 1;
    let exhaust_label_end = exhaust_pos + 7;
    let exhaust_val_start = exhaust_label_end + 2;
    let reset_pos = pred_line.find("Reset").unwrap_or(0);
    let reset_val_start = reset_pos + 7;
    let exhaust_color = if mon.exceeded { 3 } else { 4 };
    lines.push(
        Line::new(&pred_line)
            .color(exhaust_color, exhaust_pos, exhaust_label_end)
            .color(exhaust_color, exhaust_val_start, reset_pos.saturating_sub(2))
            .color(2, reset_pos, reset_pos + 5)
            .color(2, reset_val_start, pred_line.len())
    );

    // 구분선
    lines.push(Line::new(&sep).color_all(0));

    // ── Today / Total 통계 ──
    let today_line = format!(
        " Today  Sess: {}  Msg: {}  Tools: {}  Tok: {}",
        state.stats.sessions,
        fmt_num(state.stats.messages),
        fmt_num(state.stats.tool_calls),
        fmt_num(state.stats.tokens),
    );
    let today_sess_pos = today_line.find("Sess").unwrap_or(0);
    let today_msg_pos = today_line.find("Msg").unwrap_or(0);
    let today_tools_pos = today_line.find("Tools").unwrap_or(0);
    let today_tok_pos = today_line.rfind("Tok").unwrap_or(0);
    lines.push(
        Line::new(&today_line)
            .color(2, 1, 6)
            .color(0, today_sess_pos, today_sess_pos + 4)
            .color(0, today_msg_pos, today_msg_pos + 3)
            .color(0, today_tools_pos, today_tools_pos + 5)
            .color(0, today_tok_pos, today_tok_pos + 3)
    );

    let total_line = format!(
        " Total  Sess: {}  Msg: {}",
        fmt_num(state.stats.total_sessions),
        fmt_num(state.stats.total_messages),
    );
    lines.push(Line::new(&total_line).color(0, 1, 6));

    // 구분선
    lines.push(Line::new(&sep).color_all(0));

    // ── 카운트 요약 ──
    let counts = format!(
        " Agents: {}  Sessions: {}  MCPs: {}  Skills: {}",
        state.agents.len(),
        state.active_sessions,
        state.mcps_count,
        state.skills.len(),
    );
    let agents_val_end = counts.find("  Sessions").unwrap_or(8);
    let sess_pos = counts.find("Sessions").unwrap_or(0);
    let sess_val_end = counts.find("  MCPs").unwrap_or(sess_pos + 8);
    let mcps_pos = counts.find("MCPs").unwrap_or(0);
    let mcps_val_end = counts.find("  Skills").unwrap_or(mcps_pos + 4);
    let skills_pos = counts.find("Skills").unwrap_or(0);
    lines.push(
        Line::new(&counts)
            .color(6, 1, agents_val_end)
            .color(if state.active_sessions > 0 { 2 } else { 4 }, sess_pos, sess_val_end)
            .color(5, mcps_pos, mcps_val_end)
            .color(4, skills_pos, counts.len())
    );

    // ── 팀 섹션 ──
    if !mon.teams.is_empty() {
        lines.push(Line::new(""));

        for team in &mon.teams {
            let total_tasks = team.tasks_pending + team.tasks_in_progress + team.tasks_completed;
            let team_header = format!(
                " Team: {}  [{}/{}/{}]",
                team.name,
                team.tasks_completed,
                team.tasks_in_progress,
                team.tasks_pending,
            );
            let team_pos = 1;
            let team_name_start = 7; // after "Team: "
            let bracket_pos = team_header.find('[').unwrap_or(0);
            lines.push(
                Line::new(&team_header)
                    .color(6, team_pos, team_pos + 4)
                    .color(0, team_name_start, bracket_pos.saturating_sub(2))
                    .color(if total_tasks > 0 { 2 } else { 0 }, bracket_pos, team_header.len())
            );

            for member in &team.members {
                let prefix = if member.busy { "● " } else { "  " };
                let task_info = if member.task.is_empty() {
                    String::new()
                } else {
                    let max_task_len = w.saturating_sub(member.name.len() + member.agent_type.len() + 10);
                    let truncated = if member.task.len() > max_task_len {
                        format!("{}…", &member.task[..max_task_len.saturating_sub(1)])
                    } else {
                        member.task.clone()
                    };
                    format!(" → {}", truncated)
                };
                let member_line = format!(
                    " {}{} ({}){}", prefix, member.name, member.agent_type, task_info
                );
                if member.busy {
                    lines.push(Line::new(&member_line).color_all(2));
                } else {
                    lines.push(Line::new(&member_line));
                }
            }
        }
    }

    // ── 에이전트 리스트 ──
    if !state.agents.is_empty() {
        lines.push(Line::new(&sep).color_all(0));

        let active_set: Vec<&str> = mon.active_agents.iter().map(|s| s.as_str()).collect();

        // 2열 레이아웃
        if w >= 36 {
            let col_w = w / 2;
            let mut i = 0;
            while i < state.agents.len() {
                let left_name = &state.agents[i].name;
                let left_active = active_set.contains(&left_name.as_str());
                let left_prefix = if left_active { "● " } else { "○ " };
                let left = format!("{}{}", left_prefix, left_name);

                if i + 1 < state.agents.len() {
                    let right_name = &state.agents[i + 1].name;
                    let right_active = active_set.contains(&right_name.as_str());
                    let right_prefix = if right_active { "● " } else { "○ " };
                    let right = format!("{}{}", right_prefix, right_name);
                    let line_str = format!("{:<width$}{}", left, right, width = col_w);
                    let mut line = Line::new(&line_str);
                    if left_active {
                        line = line.color(2, 0, col_w.min(line_str.len()));
                    }
                    if right_active {
                        line = line.color(2, col_w, line_str.len());
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
            for agent in &state.agents {
                let is_active = active_set.contains(&agent.name.as_str());
                let prefix = if is_active { "● " } else { "○ " };
                let line_str = format!("{}{}", prefix, agent.name);
                if is_active {
                    lines.push(Line::new(&line_str).color_all(2));
                } else {
                    lines.push(Line::new(&line_str));
                }
            }
        }
    }

    // ── 스크롤 적용 및 출력 ──
    let total_lines = lines.len();
    state.content_height = total_lines;

    // 스크롤 오프셋 클램핑 (콘텐츠가 화면보다 작으면 스크롤 불가)
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

    // 스크롤 인디케이터 (콘텐츠가 화면보다 클 때)
    if total_lines > rows {
        let indicator = format!(
            " [{}/{}] ↑k ↓j",
            offset + 1,
            total_lines.saturating_sub(rows) + 1,
        );
        // 마지막 줄에 스크롤 힌트 오버레이
        let hint_y = rows.saturating_sub(1);
        let hint_x = w.saturating_sub(indicator.len());
        print_text_with_coordinates(
            Text::new(&indicator).color_range(0, ..),
            hint_x, hint_y, None, None,
        );
    }
}
