use crate::app::App;
use crate::components::centered_rect;
use crate::error_log::LogLevel;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_error_log_popup(app: &App, frame: &mut Frame) {
    let area = centered_rect(85, 75, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Error Log [F12] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .horizontal_margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(inner);

    let log = app.error_log.lock().unwrap();
    let total = log.entries.len();
    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("{total} entries"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ]);
    frame.render_widget(header, chunks[0]);

    let viewport_height = chunks[1].height as usize;
    let total_entries = log.entries.len();

    let scroll_offset = app.ui_state.error_log_list.get_scroll_offset();
    let visible: Vec<Line> = log
        .entries
        .iter()
        .rev()
        .skip(scroll_offset)
        .take(viewport_height)
        .map(|entry| {
            let (label, color) = match entry.level {
                LogLevel::Error => ("[ERROR]", Color::Red),
                LogLevel::Warn => (" [WARN]", Color::Yellow),
            };
            let ts = entry.timestamp.format("%H:%M:%S").to_string();
            Line::from(vec![
                Span::styled(
                    label,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" {} {} ", ts, entry.target)),
                Span::raw(entry.message.clone()),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(visible), chunks[1]);

    let items_above = scroll_offset.min(total_entries);
    let items_below = total_entries.saturating_sub(scroll_offset + viewport_height);
    let mut footer_lines: Vec<Line> = Vec::new();
    if items_above > 0 || items_below > 0 {
        footer_lines.push(Line::from(Span::styled(
            format!("↑ {items_above} above  ↓ {items_below} below"),
            Style::default().fg(Color::DarkGray),
        )));
    }
    footer_lines.push(Line::from(Span::styled(
        "ESC/q: close | j/k: scroll",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    )));
    frame.render_widget(Paragraph::new(footer_lines), chunks[2]);
}
