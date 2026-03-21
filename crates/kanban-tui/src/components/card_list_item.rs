use crate::theme::*;
use kanban_domain::AnimationType;
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
    pub animation_type: Option<AnimationType>,
    pub search_query: Option<&'a str>,
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

    // Apply animation flash effect if card is animating
    if let Some(animation_type) = config.animation_type {
        let flash_bg = match animation_type {
            AnimationType::Archiving | AnimationType::Deleting => FLASH_DELETE,
            AnimationType::Restoring => FLASH_RESTORE,
        };
        base_style = base_style.bg(flash_bg);
        title_style = title_style.bg(flash_bg);
    } else if config.is_selected && config.is_focused {
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

    let title_spans = build_title_spans(&config.card.title, title_style, config.search_query);

    let mut spans = vec![
        Span::styled("● ", priority_style_val),
        Span::styled(points_text, points_style),
        Span::raw(" "),
        Span::styled(format!("{}{} ", select_indicator, checkbox), base_style),
    ];
    spans.extend(title_spans);

    if !suffix_text.is_empty() {
        let mut suffix_style = label_text();
        if config.is_selected && config.is_focused {
            suffix_style = suffix_style.bg(SELECTED_BG);
        }
        spans.push(Span::styled(suffix_text, suffix_style));
    }

    Line::from(spans)
}

fn build_title_spans(title: &str, base_style: Style, query: Option<&str>) -> Vec<Span<'static>> {
    let Some(q) = query.filter(|q| !q.is_empty()) else {
        return vec![Span::styled(title.to_owned(), base_style)];
    };

    let title_lower = title.to_lowercase();
    let query_lower = q.to_lowercase();
    let highlight_style = base_style.fg(HIGHLIGHT_TEXT).add_modifier(Modifier::BOLD);

    let mut spans = Vec::new();
    let mut pos = 0;
    while let Some(idx) = title_lower[pos..].find(&query_lower) {
        let abs = pos + idx;
        if abs > pos {
            spans.push(Span::styled(title[pos..abs].to_owned(), base_style));
        }
        let end = abs + q.len();
        spans.push(Span::styled(title[abs..end].to_owned(), highlight_style));
        pos = end;
    }
    if pos < title.len() {
        spans.push(Span::styled(title[pos..].to_owned(), base_style));
    }
    spans
}
