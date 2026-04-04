use crate::app::App;
use crate::components::centered_rect;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_manage_parents_popup(app: &App, frame: &mut Frame) {
    render_relationship_popup(app, frame, "Set Parents");
}

pub fn render_manage_children_popup(app: &App, frame: &mut Frame) {
    render_relationship_popup(app, frame, "Set Children");
}

fn render_relationship_popup(app: &App, frame: &mut Frame, title: &str) {
    let area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    let search_border_style = if app.relationship.search_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };
    let search_block = Block::default()
        .title("Search")
        .borders(Borders::ALL)
        .border_style(search_border_style);

    let search_text: Line = if app.relationship.search_active {
        Line::from(vec![
            Span::styled(&app.relationship.search, Style::default().fg(Color::White)),
            Span::styled("_", Style::default().fg(Color::Yellow)),
        ])
    } else if app.relationship.search.is_empty() {
        Line::from(Span::styled(
            "/ to search",
            Style::default().fg(Color::DarkGray),
        ))
    } else {
        Line::from(Span::styled(
            &app.relationship.search,
            Style::default().fg(Color::White),
        ))
    };

    let search = Paragraph::new(search_text).block(search_block);
    frame.render_widget(search, chunks[0]);

    let filtered_cards: Vec<_> = if app.relationship.search.is_empty() {
        app.relationship.card_ids.clone()
    } else {
        let search_lower = app.relationship.search.to_lowercase();
        app.relationship
            .card_ids
            .iter()
            .filter(|card_id| {
                app.ctx
                    .cards
                    .iter()
                    .find(|c| c.id == **card_id)
                    .map(|c| c.title.to_lowercase().contains(&search_lower))
                    .unwrap_or(false)
            })
            .copied()
            .collect()
    };

    let mut lines = vec![];
    for (idx, card_id) in filtered_cards.iter().enumerate() {
        if let Some(card) = app.ctx.cards.iter().find(|c| c.id == *card_id) {
            let is_selected = app.relationship.selection.get() == Some(idx);
            let is_checked = app.relationship.selected.contains(card_id);

            let checkbox = if is_checked { "[✓]" } else { "[ ]" };

            let style = if is_selected {
                Style::default().fg(Color::White).bg(Color::Blue)
            } else if is_checked {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            lines.push(Line::from(Span::styled(
                format!("{} {}", checkbox, card.title),
                style,
            )));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No eligible cards found",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let list = Paragraph::new(lines);
    frame.render_widget(list, chunks[1]);

    let instructions_text = if app.relationship.search_active {
        "Type to search | Enter/Esc: exit search"
    } else {
        "j/k: navigate | Space: toggle | /: search | Esc: close"
    };
    let instructions =
        Paragraph::new(instructions_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(instructions, chunks[2]);
}
