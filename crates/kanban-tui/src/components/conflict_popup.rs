use crate::app::App;
use crate::components::centered_rect;
use crate::theme::*;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_conflict_resolution_popup(_app: &App, frame: &mut Frame) {
    let area = centered_rect(70, 40, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("File Conflict Detected")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Min(0),
        ])
        .split(inner);

    let message = Paragraph::new(
        "The file was modified by another instance.\nChoose how to resolve this conflict:",
    )
    .style(Style::default().fg(Color::Yellow));
    frame.render_widget(message, chunks[0]);

    let options = vec![
        Line::from(Span::styled(
            "(O)verwrite",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "  Keep your changes and overwrite the file",
            label_text(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "(T)ake theirs",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "  Discard your changes and reload the file",
            label_text(),
        )),
    ];
    let options_para = Paragraph::new(options);
    frame.render_widget(options_para, chunks[1]);

    let instructions =
        Paragraph::new("Press O or T to choose, ESC to retry later").style(label_text());
    frame.render_widget(instructions, chunks[2]);
}

pub fn render_external_change_detected_popup(_app: &App, frame: &mut Frame) {
    let area = centered_rect(70, 40, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("External File Change Detected")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Min(0),
        ])
        .split(inner);

    let message = Paragraph::new(
        "The file was modified by another instance.\nYou have unsaved changes. Choose an action:",
    )
    .style(Style::default().fg(Color::Yellow));
    frame.render_widget(message, chunks[0]);

    let options = vec![
        Line::from(Span::styled("(R)eload", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled(
            "  Discard your changes and reload the file",
            label_text(),
        )),
        Line::from(""),
        Line::from(Span::styled("(K)eep", Style::default().fg(Color::Cyan))),
        Line::from(Span::styled(
            "  Continue with your changes (save will overwrite)",
            label_text(),
        )),
    ];
    let options_para = Paragraph::new(options);
    frame.render_widget(options_para, chunks[1]);

    let instructions =
        Paragraph::new("Press R or K to choose, ESC to continue").style(label_text());
    frame.render_widget(instructions, chunks[2]);
}
