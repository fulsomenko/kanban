use crate::theme::*;
use kanban_domain::{Board, Card, CardStatus, Sprint};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

pub struct CardListItemConfig<'a> {
    pub card: &'a Card,
    pub board: &'a Board,
    pub sprints: &'a [Sprint],
    pub is_selected: bool,
    pub is_focused: bool,
    pub is_multi_selected: bool,
    pub show_sprint_name: bool,
}

pub fn render_card_list_item(config: CardListItemConfig) -> Line<'static> {
    let is_done = config.card.status == CardStatus::Done;

    let (checkbox, text_color) = if is_done {
        ("[x]", DONE_TEXT)
    } else {
        ("[ ]", NORMAL_TEXT)
    };

    let mut base_style = Style::default().fg(text_color);
    let mut title_style = Style::default().fg(text_color);

    if is_done {
        title_style = title_style.add_modifier(Modifier::CROSSED_OUT);
    }

    if config.is_selected && config.is_focused {
        base_style = base_style.bg(SELECTED_BG);
        title_style = title_style.bg(SELECTED_BG);
    }

    let suffix_text = if config.show_sprint_name {
        if let Some(sprint_id) = config.card.sprint_id {
            config
                .sprints
                .iter()
                .find(|s| s.id == sprint_id)
                .map(|s| format!(" ({})", s.formatted_name(config.board, "sprint")))
                .unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        let prefix = if let Some(sprint_id) = config.card.sprint_id {
            config
                .sprints
                .iter()
                .find(|s| s.id == sprint_id)
                .map(|sprint| sprint.effective_prefix(config.board, "task"))
                .unwrap_or("task")
        } else {
            "task"
        };
        format!(" ({}-{})", prefix, config.card.card_number)
    };

    let select_indicator = if config.is_multi_selected {
        "► "
    } else {
        "  "
    };

    let mut points_style = if let Some(points) = config.card.points {
        points_style(points)
    } else {
        normal_text()
    };

    if config.is_selected && config.is_focused {
        points_style = points_style.bg(SELECTED_BG);
    }

    let mut priority_style_val = priority_style(config.card.priority);
    if config.is_selected && config.is_focused {
        priority_style_val = priority_style_val.bg(SELECTED_BG);
    }

    let points_text = config
        .card
        .points
        .map(|p| p.to_string())
        .unwrap_or_else(|| " ".to_string());

    let mut spans = vec![
        Span::styled("● ", priority_style_val),
        Span::styled(points_text, points_style),
        Span::raw(" "),
        Span::styled(format!("{}{} ", select_indicator, checkbox), base_style),
        Span::styled(config.card.title.clone(), title_style),
    ];

    if !suffix_text.is_empty() {
        let mut suffix_style = label_text();
        if config.is_selected && config.is_focused {
            suffix_style = suffix_style.bg(SELECTED_BG);
        }
        spans.push(Span::styled(suffix_text, suffix_style));
    }

    Line::from(spans)
}
