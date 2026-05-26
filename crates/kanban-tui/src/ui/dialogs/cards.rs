use crate::app::App;
use crate::components::*;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub(crate) fn render_create_card_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Create New Task",
        "Task Title:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(crate) fn render_set_card_points_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Set Points",
        "Points (1-5 or empty):",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(crate) fn render_set_card_priority_popup(app: &App, frame: &mut Frame) {
    use crate::components::{PriorityDialog, SelectionDialog};
    let dialog = PriorityDialog;
    dialog.render(app, frame);
}

pub(crate) fn render_set_multiple_cards_priority_popup(app: &App, frame: &mut Frame) {
    use crate::components::{BulkPriorityDialog, SelectionDialog};
    let dialog = BulkPriorityDialog {
        count: app.multi_select.selected_cards.len(),
    };
    dialog.render(app, frame);
}

pub(crate) fn render_order_cards_popup(app: &App, frame: &mut Frame) {
    use crate::components::{SelectionDialog, SortFieldDialog};
    let dialog = SortFieldDialog;
    dialog.render(app, frame);
}

pub(crate) fn render_assign_sprint_popup(app: &App, frame: &mut Frame) {
    use crate::components::{SelectionDialog, SprintAssignDialog};
    let dialog = SprintAssignDialog;
    dialog.render(app, frame);
}

pub(crate) fn render_assign_multiple_cards_popup(app: &App, frame: &mut Frame) {
    use crate::components::radio_list::scroll_offset_to_show;
    use crate::components::sprint_assign_list::{
        build_entries, render_entry_line, section_header_for,
    };
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    let area = centered_rect(60, 50, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(
            "Assign {} Cards to Sprint",
            app.multi_select.selected_cards.len()
        ))
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let label = Paragraph::new("Select sprint:").style(Style::default().fg(Color::Yellow));
    frame.render_widget(label, chunks[0]);

    let mut lines = vec![];
    let mut entries_for_header = Vec::new();

    if let Some(board_idx) = app.selection.active_board_index {
        if let Some(board) = app.model.boards().get(board_idx) {
            let sprints = app.model.sprints();
            let entries = build_entries(sprints, board.id, chrono::Utc::now());
            for (idx, entry) in entries.iter().enumerate() {
                let is_selected = app.dialog_input.sprint_assign_selection.get() == Some(idx);
                lines.push(render_entry_line(entry, is_selected, None, board));
            }
            entries_for_header = entries;
        }
    }

    let selected = app.dialog_input.sprint_assign_selection.get().unwrap_or(0);
    let scroll = scroll_offset_to_show(selected, lines.len(), chunks[1].height as usize);
    let list = Paragraph::new(lines).scroll((scroll as u16, 0));
    frame.render_widget(list, chunks[1]);

    if let Some((header_idx, label)) = section_header_for(&entries_for_header, selected) {
        if header_idx < scroll && chunks[1].height > 0 {
            let overlay = Paragraph::new(Line::from(Span::styled(
                label.to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            let top_row = ratatui::layout::Rect {
                x: chunks[1].x,
                y: chunks[1].y,
                width: chunks[1].width,
                height: 1,
            };
            frame.render_widget(overlay, top_row);
        }
    }
}
