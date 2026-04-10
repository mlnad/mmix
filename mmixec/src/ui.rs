use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::app::{App, InputMode};

pub fn draw(f: &mut Frame, app: &App) {
    // Main layout: left code (60%) | right panel (40%)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(f.area());

    // Left: code panel + status bar
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(main_chunks[0]);

    // Right: registers (top 50%) | output (bottom 50%)
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    draw_code_panel(f, app, left_chunks[0]);
    draw_status_bar(f, app, left_chunks[1]);
    draw_register_panel(f, app, right_chunks[0]);
    draw_output_panel(f, app, right_chunks[1]);
}

fn draw_code_panel(f: &mut Frame, app: &App, area: Rect) {
    let current_line = app.current_line();
    let visible_height = area.height.saturating_sub(2) as usize; // minus border

    // Auto-scroll to follow current line
    let scroll = if let Some(cur) = current_line {
        if cur < app.code_scroll {
            cur
        } else if cur >= app.code_scroll + visible_height {
            cur.saturating_sub(visible_height / 2)
        } else {
            app.code_scroll
        }
    } else {
        app.code_scroll
    };

    let lines: Vec<Line> = app
        .source_lines
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(idx, src)| {
            let is_current = current_line == Some(idx);
            let is_bp = app.breakpoints.contains(&idx);

            let line_num = format!("{:>4} ", idx + 1);
            let marker = if is_bp { "● " } else { "  " };

            let num_style = if is_bp {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let marker_style = Style::default().fg(Color::Red);

            let src_style = if is_current {
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            Line::from(vec![
                Span::styled(line_num, num_style),
                Span::styled(marker, marker_style),
                Span::styled(src.to_string(), src_style),
            ])
        })
        .collect();

    let title = if app.halted {
        " Code [HALTED] "
    } else if app.running {
        " Code [RUNNING] "
    } else {
        " Code "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_register_panel(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // Special registers (only non-zero ones, plus PC shown separately)
    lines.push(Line::from(Span::styled(
        "── Special ──",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )));

    // Always show PC and timing
    lines.push(Line::from(vec![
        Span::styled(format!("{:<4}", "@"), Style::default().fg(Color::Green)),
        Span::styled(format!(" = {:#018x}", app.machine.pc), Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:<11}", "cycles (υ)"), Style::default().fg(Color::Green)),
        Span::styled(format!(" = {}", app.machine.oops), Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(format!("{:<9}", "mems (μ)"), Style::default().fg(Color::Green)),
        Span::styled(format!(" = {}", app.machine.mems), Style::default().fg(Color::White)),
    ]));

    for (name, val) in app.all_special_regs() {
        if val != 0 {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<4}", name), Style::default().fg(Color::Green)),
                Span::styled(format!(" = {:#018x}", val), Style::default().fg(Color::White)),
            ]));
        }
    }

    // General registers (non-zero)
    let gen_regs = app.nonzero_general_regs();
    if !gen_regs.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "── General ──",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        for (idx, val) in &gen_regs {
            lines.push(Line::from(vec![
                Span::styled(format!("${:<3}", idx), Style::default().fg(Color::Green)),
                Span::styled(format!(" = {:#018x}", val), Style::default().fg(Color::White)),
            ]));
        }
    }

    let block = Block::default()
        .title(" Registers ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll = app.reg_scroll.min(lines.len().saturating_sub(visible_height));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((scroll as u16, 0));
    f.render_widget(paragraph, area);
}

fn draw_output_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Output ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let text = if app.output.is_empty() {
        "(no output yet)".to_string()
    } else {
        app.output.clone()
    };

    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));
    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let content = match app.input_mode {
        InputMode::BreakpointInput => {
            format!("Breakpoint line: {}█  (Enter=confirm, Esc=cancel)", app.bp_input)
        }
        InputMode::Normal => {
            app.status_msg.clone()
        }
    };

    let style = match app.input_mode {
        InputMode::BreakpointInput => Style::default().bg(Color::DarkGray).fg(Color::White),
        InputMode::Normal => {
            if app.error_msg.is_some() {
                Style::default().bg(Color::Red).fg(Color::White)
            } else {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            }
        }
    };

    let paragraph = Paragraph::new(content).style(style);
    f.render_widget(paragraph, area);
}
