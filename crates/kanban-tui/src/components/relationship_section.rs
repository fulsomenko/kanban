use crate::components::ListItemConfig;
use crate::selection::SelectionState;
use crate::theme::*;
use kanban_domain::Card;
use ratatui::text::{Line, Span};
use uuid::Uuid;

pub fn render_relationship_section(
    card_ids: &[Uuid],
    all_cards: &[Card],
    title: &str,
    is_focused: bool,
    selection: &SelectionState,
) -> (Vec<Line<'static>>, bool) {
    let mut lines: Vec<Line> = Vec::new();
    let has_items = !card_ids.is_empty();

    if has_items {
        // Build lines for up to 3 visible cards
        for (idx, card_id) in card_ids.iter().take(3).enumerate() {
            if let Some(card) = all_cards.iter().find(|c| c.id == *card_id) {
                let is_selected = selection.get() == Some(idx);

                let config = ListItemConfig::new()
                    .selected(is_selected)
                    .focused(is_selected && is_focused);

                let style = config.item_style();

                let line = Line::from(vec![
                    Span::styled("→ ", label_text()),
                    Span::styled(card.title.clone(), style),
                ]);
                lines.push(line);
            }
        }

        // Pad with blank lines to always show 3 lines
        while lines.len() < 3 {
            lines.push(Line::from(""));
        }

        // Add pagination indicator if there are more than 3 items
        if card_ids.len() > 3 {
            lines.push(Line::from(Span::styled(
                format!("... ({} more)", card_ids.len() - 3),
                label_text(),
            )));
        }
    } else {
        // Empty list case
        let empty_text = match title {
            "Parents" => "No parents",
            "Children" => "No children",
            _ => "No items",
        };
        lines.push(Line::from(Span::styled(empty_text, label_text())));

        // Pad with 2 blank lines
        lines.push(Line::from(""));
        lines.push(Line::from(""));
    }

    (lines, has_items)
}
