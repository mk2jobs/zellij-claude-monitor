use crate::state::DashboardState;
use zellij_tile::prelude::*;

/// 토큰 수를 읽기 쉬운 형태로 포맷
fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// 숫자를 천 단위 구분자로 포맷
fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

pub fn draw_dashboard(state: &DashboardState, rows: usize, cols: usize) {
    if !state.loaded {
        let loading = Text::new("Loading...");
        print_text_with_coordinates(loading, 1, 1, None, None);
        return;
    }

    if state.permission_error {
        let err = Text::new("Permission error - grant RunCommands permission").color_range(3, ..);
        print_text_with_coordinates(err, 1, 1, None, None);
        return;
    }

    let compact = rows < 12;
    let mut y = 0;

    // ── 헤더 ──
    let header = format!(" Claude Dashboard  {}", state.today_date);
    let header_text = Text::new(&header).color_range(2, 0..18);
    print_text_with_coordinates(header_text, 0, y, Some(cols), None);
    y += 1;

    // 구분선
    let separator = "─".repeat(cols.min(80));
    print_text_with_coordinates(Text::new(&separator).color_range(0, ..), 0, y, Some(cols), None);
    y += 1;

    // ── 통계 요약 ──
    if compact {
        // 한 줄 컴팩트 모드
        let stats_line = format!(
            " Sessions: {}  Messages: {}  Tools: {}  Tokens: {}",
            state.stats.sessions,
            format_number(state.stats.messages),
            format_number(state.stats.tool_calls),
            format_tokens(state.stats.tokens),
        );
        let text = Text::new(&stats_line)
            .color_range(1, 1..10)
            .color_range(1, stats_line.find("Messages").unwrap_or(0)..stats_line.find("Messages").unwrap_or(0) + 8)
            .color_range(1, stats_line.find("Tools").unwrap_or(0)..stats_line.find("Tools").unwrap_or(0) + 5)
            .color_range(1, stats_line.find("Tokens").unwrap_or(0)..stats_line.find("Tokens").unwrap_or(0) + 6);
        print_text_with_coordinates(text, 0, y, Some(cols), None);
        y += 1;
    } else {
        // 2행 표시
        let line1 = format!(
            " Today  Sessions: {}  Messages: {}",
            state.stats.sessions,
            format_number(state.stats.messages),
        );
        let text1 = Text::new(&line1)
            .color_range(2, 1..6)
            .color_range(1, 8..16);
        print_text_with_coordinates(text1, 0, y, Some(cols), None);
        y += 1;

        let line2 = format!(
            "        Tools: {}  Tokens: {}",
            format_number(state.stats.tool_calls),
            format_tokens(state.stats.tokens),
        );
        let text2 = Text::new(&line2).color_range(1, 8..13).color_range(1, line2.find("Tokens").unwrap_or(0)..line2.find("Tokens").unwrap_or(0) + 6);
        print_text_with_coordinates(text2, 0, y, Some(cols), None);
        y += 1;

        // 누적 통계
        let total_line = format!(
            " Total  Sessions: {}  Messages: {}",
            format_number(state.stats.total_sessions),
            format_number(state.stats.total_messages),
        );
        let total_text = Text::new(&total_line)
            .color_range(0, 1..6)
            .color_range(1, 8..16);
        print_text_with_coordinates(total_text, 0, y, Some(cols), None);
        y += 1;
    }

    // 구분선
    let separator2 = "─".repeat(cols.min(80));
    print_text_with_coordinates(Text::new(&separator2).color_range(0, ..), 0, y, Some(cols), None);
    y += 1;

    // ── 카운트 요약 ──
    let counts_line = format!(
        " Agents: {}  Tasks: {}  Skills: {}",
        state.agents.len(),
        state.tasks_count,
        state.skills.len(),
    );
    let counts_text = Text::new(&counts_line)
        .color_range(1, 1..8)
        .color_range(1, counts_line.find("Tasks").unwrap_or(0)..counts_line.find("Tasks").unwrap_or(0) + 5)
        .color_range(1, counts_line.find("Skills").unwrap_or(0)..counts_line.find("Skills").unwrap_or(0) + 6);
    print_text_with_coordinates(counts_text, 0, y, Some(cols), None);
    y += 1;

    // 구분선
    let separator3 = "─".repeat(cols.min(80));
    print_text_with_coordinates(Text::new(&separator3).color_range(0, ..), 0, y, Some(cols), None);
    y += 1;

    // ── 에이전트 리스트 ──
    if !state.agents.is_empty() {
        let agents_header = " Agents";
        print_text_with_coordinates(
            Text::new(agents_header).color_range(2, 1..),
            0,
            y,
            Some(cols),
            None,
        );
        y += 1;

        let available_rows = if rows > y + 1 { rows - y - 1 } else { 0 };
        let agents_to_show = state.agents.len().min(available_rows);

        for agent in state.agents.iter().take(agents_to_show) {
            let line = format!("   {}", agent.name);
            print_text_with_coordinates(
                Text::new(&line).color_range(0, 3..),
                0,
                y,
                Some(cols),
                None,
            );
            y += 1;
        }

        if state.agents.len() > agents_to_show {
            let more = format!("   +{} more", state.agents.len() - agents_to_show);
            print_text_with_coordinates(Text::new(&more), 0, y, Some(cols), None);
        }
    }
}
