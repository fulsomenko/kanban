use crate::app::App;
use crate::components::*;
use crate::theme::*;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, ListItem, Paragraph},
    Frame,
};

pub(crate) fn render_create_column_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Create New Column",
        "Column Name:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(crate) fn render_rename_column_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Rename Column",
        "New Column Name:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(crate) fn render_delete_column_confirm_popup(_app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 30, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Delete Column")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner);

    let message = Paragraph::new("Are you sure you want to delete this column?\nAll cards will be moved to the first column.")
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(message, chunks[0]);

    let confirm_text =
        Paragraph::new("Press ENTER/y to delete, n/ESC to cancel").style(label_text());
    frame.render_widget(confirm_text, chunks[1]);
}

pub(crate) fn render_select_task_list_view_popup(app: &App, frame: &mut Frame) {
    use kanban_domain::TaskListView;

    let views = [
        TaskListView::Flat,
        TaskListView::GroupedByColumn,
        TaskListView::ColumnView,
    ];

    let selected = app.dialog_input.task_list_view_selection.get();

    let current_view = app
        .selection
        .active_board_index
        .and_then(|idx| app.view.boards.get(idx).map(|board| board.task_list_view));

    let items: Vec<ListItem> = views
        .iter()
        .enumerate()
        .map(|(idx, view)| {
            let style = if Some(idx) == selected {
                bold_highlight()
            } else {
                normal_text()
            };
            let is_current = current_view == Some(*view);
            let view_name = match view {
                TaskListView::Flat => {
                    if is_current {
                        "Flat (current)"
                    } else {
                        "Flat"
                    }
                }
                TaskListView::GroupedByColumn => {
                    if is_current {
                        "Grouped by Column (current)"
                    } else {
                        "Grouped by Column"
                    }
                }
                TaskListView::ColumnView => {
                    if is_current {
                        "Column View (kanban board) (current)"
                    } else {
                        "Column View (kanban board)"
                    }
                }
            };
            ListItem::new(view_name).style(style)
        })
        .collect();

    render_selection_popup_with_list_items(frame, "Select Task List View", items, 50, 40);
}
