use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Row, Table, Tabs};
use ratatui::Frame;

use crate::app::{App, Tab};
use crate::launchd::Service;
use crate::schedule;
use chrono::Local;

pub fn draw(frame: &mut Frame, app: &App) {
    let selected = app.selected_service();
    let show_error_panel = app.tab != Tab::Timeline
        && selected.is_some_and(|s| s.exit_code.is_some_and(|c| c != 0) && s.pid.is_none());

    let chunks = if show_error_panel {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(12),
        ])
        .areas::<3>(frame.area())
    } else {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(0),
        ])
        .areas::<3>(frame.area())
    };

    draw_tabs(frame, app, chunks[0]);
    draw_service_list(frame, app, chunks[1]);

    if show_error_panel {
        if let Some(svc) = selected {
            draw_error_panel(frame, svc, chunks[2]);
        }
    }

    if app.show_action_menu {
        draw_action_menu(frame, app);
    }
}

fn draw_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<&str> = Tab::all().iter().map(|t| t.label()).collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("launched"))
        .select(app.tab.index())
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    frame.render_widget(tabs, area);
}

fn status_icon(svc: &Service) -> &'static str {
    if svc.pid.is_some() {
        "🔄"
    } else {
        match svc.exit_code {
            Some(0) => "✅",
            Some(_) => "❌",
            None => "⏸️",
        }
    }
}

fn draw_service_list(frame: &mut Frame, app: &App, area: Rect) {
    let now = Local::now();
    let grouped = app.grouped_services();

    let mut rows: Vec<Row> = Vec::new();
    let mut flat_idx: usize = 0;

    let is_timeline = app.tab == Tab::Timeline;

    for (category, svcs) in &grouped {
        if !category.is_empty() {
            if is_timeline {
                rows.push(
                    Row::new(vec![
                        "".to_string(),
                        format!("── {} ──", category),
                        "".to_string(),
                    ])
                    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                );
            } else {
                rows.push(
                    Row::new(vec![
                        "".to_string(),
                        format!("── {} ──", category),
                        "".to_string(),
                        "".to_string(),
                    ])
                    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                );
            }
        }

        for svc in svcs {
            let next_run = schedule::next_run_time(
                &svc.start_calendar_interval,
                &svc.start_interval,
                now,
            );
            let next_str = schedule::format_next_run(next_run, now);

            let is_selected = flat_idx == app.cursor;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            let prefix = if is_selected { "> " } else { "  " };

            if is_timeline {
                rows.push(
                    Row::new(vec![
                        prefix.to_string(),
                        svc.label.clone(),
                        next_str,
                    ])
                    .style(style),
                );
            } else {
                let icon = status_icon(svc);
                let exit_str = match svc.exit_code {
                    Some(c) => c.to_string(),
                    None => "-".to_string(),
                };
                rows.push(
                    Row::new(vec![
                        format!("{}{}", prefix, icon),
                        svc.label.clone(),
                        exit_str,
                        next_str,
                    ])
                    .style(style),
                );
            }
            flat_idx += 1;
        }
    }

    let (widths, header) = if is_timeline {
        (
            vec![
                Constraint::Length(2),
                Constraint::Fill(1),
                Constraint::Length(14),
            ],
            Row::new(vec!["", "Label", "Next Run"])
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        )
    } else {
        (
            vec![
                Constraint::Length(5),
                Constraint::Fill(1),
                Constraint::Length(6),
                Constraint::Length(14),
            ],
            Row::new(vec!["", "Label", "Exit", "Next Run"])
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        )
    };

    let block = if is_timeline {
        Block::default().borders(Borders::ALL)
    } else {
        Block::default()
            .borders(Borders::ALL)
            .title_bottom(Line::from(vec![
                Span::raw(" ✅ Success  "),
                Span::raw("❌ Failed  "),
                Span::raw("🔄 Running  "),
                Span::raw("⏸️  Unloaded "),
            ]).style(Style::default().fg(Color::DarkGray)).right_aligned())
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(block);

    frame.render_widget(table, area);
}

fn draw_error_panel(frame: &mut Frame, svc: &Service, area: Rect) {
    let title = format!(" [Error] {} (exit: {}) ", svc.label, svc.exit_code.unwrap_or(-1));
    let log_lines = read_log_tail(&svc.standard_error_path, 10);
    let text: Vec<Line> = log_lines
        .iter()
        .map(|l| Line::from(l.as_str()))
        .collect();

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(Style::default().fg(Color::Red)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

fn read_log_tail(path: &Option<String>, lines: usize) -> Vec<String> {
    let Some(path) = path else {
        return vec!["(no log path configured)".to_string()];
    };

    match std::fs::read_to_string(path) {
        Ok(content) => content
            .lines()
            .rev()
            .take(lines)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|s| s.to_string())
            .collect(),
        Err(e) => vec![format!("(cannot read log: {})", e)],
    }
}

fn draw_action_menu(frame: &mut Frame, app: &App) {
    let Some(svc) = app.selected_service() else {
        return;
    };

    let area = frame.area();
    let popup_width = 40u16;
    let popup_height = 6u16;
    let popup_area = Rect {
        x: area.width.saturating_sub(popup_width) / 2,
        y: area.height.saturating_sub(popup_height) / 2,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    frame.render_widget(Clear, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  [l] ", Style::default().fg(Color::Yellow)),
            Span::raw("Load"),
        ]),
        Line::from(vec![
            Span::styled("  [u] ", Style::default().fg(Color::Yellow)),
            Span::raw("Unload"),
        ]),
        Line::from(vec![
            Span::styled("  [Esc] ", Style::default().fg(Color::DarkGray)),
            Span::raw("Cancel"),
        ]),
    ];

    let title = format!(" {} ", svc.label);
    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::White).bg(Color::Black)),
    );

    frame.render_widget(paragraph, popup_area);
}
