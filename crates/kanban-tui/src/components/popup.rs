use crate::theme::{focused_border, popup_bg};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn render_input_popup(
    frame: &mut Frame,
    title: &str,
    label: &str,
    input_text: &str,
    cursor_pos: usize,
) {
    let area = centered_rect(60, 30, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(popup_bg());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(inner);

    let label_widget = Paragraph::new(label).style(crate::theme::highlight_text());
    frame.render_widget(label_widget, chunks[0]);

    let input = Paragraph::new(input_text)
        .style(crate::theme::normal_text())
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(input, chunks[1]);

    let cursor_x = chunks[1].x + cursor_pos as u16 + 1;
    let cursor_y = chunks[1].y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

pub fn render_popup_with_block(
    frame: &mut Frame,
    title: &str,
    width_percent: u16,
    height_percent: u16,
) -> Rect {
    let area = centered_rect(width_percent, height_percent, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(focused_border())
        .style(popup_bg());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    inner
}
