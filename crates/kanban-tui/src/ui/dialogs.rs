use crate::app::App;
use crate::components::*;
use crate::theme::*;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, ListItem, Paragraph},
    Frame,
};

pub(super) fn render_export_boards_popup(app: &App, frame: &mut Frame) {
    use crate::app::{ExportFormat, ExportStep};

    let Some(ref dialog) = app.export_dialog else {
        return;
    };

    match dialog.step {
        ExportStep::SelectBoards => {
            let inner = render_popup_with_block(frame, "Select Boards to Export", 60, 60);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(inner);

            let items: Vec<Line> = app
                .ctx
                .boards
                .iter()
                .enumerate()
                .map(|(i, board)| {
                    let checkbox = if dialog.board_selections.get(i).copied().unwrap_or(false) {
                        "[x] "
                    } else {
                        "[ ] "
                    };
                    let style = if i == dialog.cursor {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Line::from(Span::styled(format!("{}{}", checkbox, board.name), style))
                })
                .collect();

            let list = Paragraph::new(items);
            frame.render_widget(list, chunks[0]);

            let hint = Paragraph::new(Line::from(vec![Span::styled(
                "Space: toggle | a: all | Enter: next | Esc: cancel",
                Style::default().fg(Color::DarkGray),
            )]));
            frame.render_widget(hint, chunks[1]);
        }
        ExportStep::ExportOptions => {
            let inner = render_popup_with_block(frame, "Export Options", 60, 30);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(inner);

            let filename_label = Paragraph::new(Line::from(vec![
                Span::styled("Filename: ", Style::default().fg(Color::Cyan)),
                Span::styled(&dialog.filename, Style::default().fg(Color::White)),
                Span::styled("_", Style::default().fg(Color::Yellow)),
            ]));
            frame.render_widget(filename_label, chunks[0]);

            frame.render_widget(Paragraph::new(""), chunks[1]);

            let json_style = if dialog.format == ExportFormat::Json {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let sqlite_style = if dialog.format == ExportFormat::Sqlite {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let json_radio = if dialog.format == ExportFormat::Json {
                "(*)"
            } else {
                "( )"
            };
            let sqlite_radio = if dialog.format == ExportFormat::Sqlite {
                "(*)"
            } else {
                "( )"
            };

            let format_line = Paragraph::new(Line::from(vec![
                Span::styled("Format: ", Style::default().fg(Color::Cyan)),
                Span::styled(format!("{} JSON  ", json_radio), json_style),
                Span::styled(format!("{} SQLite", sqlite_radio), sqlite_style),
            ]));
            frame.render_widget(format_line, chunks[2]);

            let hint = Paragraph::new(Line::from(vec![Span::styled(
                "Tab: format | Enter: export | Esc: back",
                Style::default().fg(Color::DarkGray),
            )]));
            frame.render_widget(hint, chunks[3]);
        }
    }
}

pub(super) fn render_create_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Create New Project",
        "Project Name:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_create_card_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Create New Task",
        "Task Title:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_create_sprint_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Create New Sprint",
        "Sprint Name (optional):",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_set_card_points_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Set Points",
        "Points (1-5 or empty):",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_set_card_priority_popup(app: &App, frame: &mut Frame) {
    use crate::components::{PriorityDialog, SelectionDialog};
    let dialog = PriorityDialog;
    dialog.render(app, frame);
}

pub(super) fn render_set_multiple_cards_priority_popup(app: &App, frame: &mut Frame) {
    use crate::components::{BulkPriorityDialog, SelectionDialog};
    let dialog = BulkPriorityDialog {
        count: app.multi_select.selected_cards.len(),
    };
    dialog.render(app, frame);
}

pub(super) fn render_rename_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Rename Project",
        "New Project Name:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_export_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Export Project",
        "Filename:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_export_all_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Export All Projects",
        "Filename:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_import_board_popup(app: &App, frame: &mut Frame) {
    let inner = render_popup_with_block(frame, "Import Projects", 60, 50);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let label = Paragraph::new("Select a JSON file to import:").style(highlight_text());
    frame.render_widget(label, chunks[0]);

    if app.dialog_input.import_files.is_empty() {
        let empty_msg =
            Paragraph::new("No JSON files found in current directory").style(label_text());
        frame.render_widget(empty_msg, chunks[1]);
    } else {
        let mut lines = vec![];
        for (idx, filename) in app.dialog_input.import_files.iter().enumerate() {
            let config = ListItemConfig::new()
                .selected(app.dialog_input.import_selection.get() == Some(idx))
                .focused(true);
            lines.push(styled_list_item(filename, &config));
        }
        let list = Paragraph::new(lines);
        frame.render_widget(list, chunks[1]);
    }
}

pub(super) fn render_set_branch_prefix_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Set Branch Prefix",
        "Branch Prefix:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_set_sprint_prefix_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Set Sprint Prefix",
        "Sprint Prefix:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_set_sprint_card_prefix_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Set Card Prefix Override",
        "Card Prefix:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_order_cards_popup(app: &App, frame: &mut Frame) {
    use crate::components::{SelectionDialog, SortFieldDialog};
    let dialog = SortFieldDialog;
    dialog.render(app, frame);
}

pub(super) fn render_assign_sprint_popup(app: &App, frame: &mut Frame) {
    use crate::components::{SelectionDialog, SprintAssignDialog};
    let dialog = SprintAssignDialog;
    dialog.render(app, frame);
}

pub(super) fn render_assign_multiple_cards_popup(app: &App, frame: &mut Frame) {
    use kanban_domain::Sprint;

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

    if let Some(board_idx) = app.selection.active_board_index {
        if let Some(board) = app.ctx.boards.get(board_idx) {
            let board_sprints = Sprint::assignable(&app.ctx.sprints, board.id);

            for (idx, sprint_option) in std::iter::once(None)
                .chain(board_sprints.iter().map(|s| Some(*s)))
                .enumerate()
            {
                let is_selected = app.dialog_input.sprint_assign_selection.get() == Some(idx);

                let style = if is_selected {
                    Style::default().fg(Color::White).bg(Color::Blue)
                } else {
                    Style::default().fg(Color::White)
                };

                let prefix = if is_selected { "> " } else { "  " };

                let sprint_name = if let Some(sprint) = sprint_option {
                    sprint.formatted_name(board, "sprint")
                } else {
                    "(None)".to_string()
                };

                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, sprint_name),
                    style,
                )));
            }
        }
    }

    let list = Paragraph::new(lines);
    frame.render_widget(list, chunks[1]);
}

pub(super) fn render_create_column_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Create New Column",
        "Column Name:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_rename_column_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Rename Column",
        "New Column Name:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(super) fn render_delete_column_confirm_popup(_app: &App, frame: &mut Frame) {
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

pub(super) fn render_select_task_list_view_popup(app: &App, frame: &mut Frame) {
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
        .and_then(|idx| app.ctx.boards.get(idx))
        .map(|board| board.task_list_view);

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

pub(super) fn render_carry_over_sprint_popup(app: &App, frame: &mut Frame) {
    use crate::components::selection_dialog::CarryOverSprintDialog;
    use crate::components::SelectionDialog;
    let card_count = app
        .dialog_input
        .carry_over_source_sprint_id
        .map(|id| {
            use kanban_domain::query::sprint::get_sprint_uncompleted_cards;
            get_sprint_uncompleted_cards(id, &app.ctx.cards).len()
        })
        .unwrap_or(0);
    CarryOverSprintDialog { card_count }.render(app, frame);
}
