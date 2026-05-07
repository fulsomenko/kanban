use crate::app::App;
use crate::components::sprint_assign_list::{build_entries, SprintAssignEntry};
use kanban_domain::SprintStatus;
use ratatui::Frame;

pub trait SelectionDialog {
    fn title(&self) -> &str;
    fn get_current_selection(&self, app: &App) -> usize;
    fn options_count(&self, app: &App) -> usize;
    fn render(&self, app: &App, frame: &mut Frame);
}

pub struct PriorityDialog;

impl SelectionDialog for PriorityDialog {
    fn title(&self) -> &str {
        "Set Priority"
    }

    fn get_current_selection(&self, app: &App) -> usize {
        app.get_current_priority_selection_index()
    }

    fn options_count(&self, _app: &App) -> usize {
        4 // Low, Medium, High, Critical
    }

    fn render(&self, app: &App, frame: &mut Frame) {
        use crate::components::render_selection_popup_with_list_items;
        use crate::theme::*;
        use kanban_domain::CardPriority;
        use ratatui::widgets::ListItem;

        let priorities = [
            CardPriority::Low,
            CardPriority::Medium,
            CardPriority::High,
            CardPriority::Critical,
        ];

        let selected = app.dialog_input.priority_selection.get();

        let items: Vec<ListItem> = priorities
            .iter()
            .enumerate()
            .map(|(idx, priority)| {
                let style = if Some(idx) == selected {
                    bold_highlight()
                } else {
                    normal_text()
                };
                ListItem::new(format!("{:?}", priority)).style(style)
            })
            .collect();

        render_selection_popup_with_list_items(frame, "Set Priority", items, 30, 40);
    }
}

pub struct BulkPriorityDialog {
    pub count: usize,
}

impl SelectionDialog for BulkPriorityDialog {
    fn title(&self) -> &str {
        "Set Priority (Bulk)"
    }

    fn get_current_selection(&self, _app: &App) -> usize {
        0
    }

    fn options_count(&self, _app: &App) -> usize {
        4 // Low, Medium, High, Critical
    }

    fn render(&self, app: &App, frame: &mut Frame) {
        use crate::components::render_selection_popup_with_list_items;
        use crate::theme::*;
        use kanban_domain::CardPriority;
        use ratatui::widgets::ListItem;

        let priorities = [
            CardPriority::Low,
            CardPriority::Medium,
            CardPriority::High,
            CardPriority::Critical,
        ];

        let selected = app.dialog_input.priority_selection.get();

        let items: Vec<ListItem> = priorities
            .iter()
            .enumerate()
            .map(|(idx, priority)| {
                let style = if Some(idx) == selected {
                    bold_highlight()
                } else {
                    normal_text()
                };
                ListItem::new(format!("{:?}", priority)).style(style)
            })
            .collect();

        let title = format!("Set Priority ({} cards)", self.count);
        render_selection_popup_with_list_items(frame, &title, items, 35, 40);
    }
}

pub struct SortFieldDialog;

impl SelectionDialog for SortFieldDialog {
    fn title(&self) -> &str {
        "Order Tasks By"
    }

    fn get_current_selection(&self, app: &App) -> usize {
        app.get_current_sort_field_selection_index()
    }

    fn options_count(&self, _app: &App) -> usize {
        7 // Points, Priority, CreatedAt, UpdatedAt, Status, Position, Default
    }

    fn render(&self, app: &App, frame: &mut Frame) {
        use crate::components::render_selection_popup_with_lines;
        use kanban_domain::{SortField, SortOrder};

        let sort_fields = [
            SortField::Points,
            SortField::Priority,
            SortField::CreatedAt,
            SortField::UpdatedAt,
            SortField::Status,
            SortField::Position,
            SortField::Default,
        ];

        let active_idx = sort_fields
            .iter()
            .position(|f| Some(*f) == app.filter.current_sort_field);

        render_selection_popup_with_lines(
            frame,
            "Order Tasks By",
            Some("Select sort field:"),
            sort_fields.iter().enumerate(),
            |_idx, (_, field), _is_selected, is_active| {
                let field_name = match field {
                    SortField::Priority => "Priority",
                    SortField::Points => "Points",
                    SortField::CreatedAt => "Date Created",
                    SortField::UpdatedAt => "Date Updated",
                    SortField::Default => "Task Number",
                    SortField::Status => "Status",
                    SortField::Position => "Position",
                };

                let order_indicator = if is_active {
                    match app.filter.current_sort_order {
                        Some(SortOrder::Ascending) => Some(" (↑)".to_string()),
                        Some(SortOrder::Descending) => Some(" (↓)".to_string()),
                        None => None,
                    }
                } else {
                    None
                };

                (field_name.to_string(), order_indicator)
            },
            app.filter.sort_field_selection.get(),
            active_idx,
            60,
            50,
        );
    }
}

pub struct CarryOverSprintDialog {
    pub card_count: usize,
}

impl SelectionDialog for CarryOverSprintDialog {
    fn title(&self) -> &str {
        "Carry Over to Sprint"
    }

    fn get_current_selection(&self, app: &App) -> usize {
        app.dialog_input
            .carry_over_sprint_selection
            .get()
            .unwrap_or(0)
    }

    fn options_count(&self, app: &App) -> usize {
        if let Some(board_idx) = app.selection.active_board_index {
            if let Some(board) = app.model.boards().get(board_idx) {
                app.model
                    .sprints()
                    .iter()
                    .filter(|s| s.board_id == board.id && s.status == SprintStatus::Planning)
                    .count()
            } else {
                0
            }
        } else {
            0
        }
    }

    fn render(&self, app: &App, frame: &mut Frame) {
        use crate::components::centered_rect;
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::{Color, Style},
            text::{Line, Span},
            widgets::{Block, Borders, Clear, Paragraph},
        };

        let area = centered_rect(60, 50, frame.area());
        frame.render_widget(Clear, area);

        let title = format!("Carry Over to Sprint ({} cards)", self.card_count);
        let block = Block::default()
            .title(title.as_str())
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);

        let label =
            Paragraph::new("Select target sprint:").style(Style::default().fg(Color::Yellow));
        frame.render_widget(label, chunks[0]);

        let mut lines = vec![];

        if let Some(board_idx) = app.selection.active_board_index {
            let boards = app.model.boards();
            if let Some(board) = boards.get(board_idx) {
                let sprints = app.model.sprints();
                let planning_sprints: Vec<_> = sprints
                    .iter()
                    .filter(|s| s.board_id == board.id && s.status == SprintStatus::Planning)
                    .collect();

                for (idx, sprint) in planning_sprints.iter().enumerate() {
                    let is_selected =
                        app.dialog_input.carry_over_sprint_selection.get() == Some(idx);

                    let style = if is_selected {
                        Style::default().fg(Color::White).bg(Color::Blue)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let prefix = if is_selected { "> " } else { "  " };
                    let sprint_name = sprint.formatted_name(board, "sprint");

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
}

pub struct SprintAssignDialog;

impl SelectionDialog for SprintAssignDialog {
    fn title(&self) -> &str {
        "Assign to Sprint"
    }

    fn get_current_selection(&self, app: &App) -> usize {
        app.get_current_sprint_selection_index()
    }

    fn options_count(&self, app: &App) -> usize {
        if let Some(board_idx) = app.selection.active_board_index {
            let boards = app.model.boards();
            if let Some(board) = boards.get(board_idx) {
                let sprints = app.model.sprints();
                return build_entries(sprints, board.id, chrono::Utc::now()).len();
            }
        }
        1
    }

    fn render(&self, app: &App, frame: &mut Frame) {
        use crate::components::centered_rect;
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::{Color, Style},
            widgets::{Block, Borders, Clear, Paragraph},
        };

        let area = centered_rect(60, 50, frame.area());
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title("Assign to Sprint")
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
            let boards = app.model.boards();
            if let Some(board) = boards.get(board_idx) {
                let sprints = app.model.sprints();
                let entries = build_entries(sprints, board.id, chrono::Utc::now());

                let cards = app.model.cards();
                let current_sprint_id = if let Some(card_idx) = app.selection.active_card_index {
                    cards.get(card_idx).and_then(|c| c.sprint_id)
                } else {
                    None
                };

                for (idx, entry) in entries.iter().enumerate() {
                    let is_selected = app.dialog_input.sprint_assign_selection.get() == Some(idx);
                    lines.push(render_entry_line(
                        entry,
                        is_selected,
                        current_sprint_id,
                        board,
                    ));
                }
            }
        }

        let list = Paragraph::new(lines);
        frame.render_widget(list, chunks[1]);
    }
}

fn render_entry_line<'a>(
    entry: &SprintAssignEntry<'_>,
    is_selected: bool,
    current_sprint_id: Option<uuid::Uuid>,
    board: &kanban_domain::Board,
) -> ratatui::text::Line<'a> {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};

    match entry {
        SprintAssignEntry::Header(label) => Line::from(Span::styled(
            (*label).to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        SprintAssignEntry::None => {
            let is_current = current_sprint_id.is_none();
            let prefix = if is_selected { "> " } else { "  " };
            let suffix = if is_current { " (current)" } else { "" };
            let style = if is_selected {
                Style::default().fg(Color::White).bg(Color::Blue)
            } else if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(
                format!("{}(None){}", prefix, suffix),
                style,
            ))
        }
        SprintAssignEntry::ActiveOrPlanned(s) => {
            let is_current = current_sprint_id == Some(s.id);
            let prefix = if is_selected { "> " } else { "  " };
            let suffix = if is_current { " (current)" } else { "" };
            let style = if is_selected {
                Style::default().fg(Color::White).bg(Color::Blue)
            } else if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(
                format!("{}{}{}", prefix, s.formatted_name(board, "sprint"), suffix),
                style,
            ))
        }
        SprintAssignEntry::Completed(s) | SprintAssignEntry::Ended(s) => {
            let is_current = current_sprint_id == Some(s.id);
            let prefix = if is_selected { "> " } else { "  " };
            let suffix = if is_current { " (current)" } else { "" };
            let status_color = if matches!(entry, SprintAssignEntry::Completed(_)) {
                Color::Green
            } else {
                Color::Red
            };
            let style = if is_selected {
                Style::default().fg(Color::White).bg(Color::Blue)
            } else {
                // Status colour wins; no bold override even when is_current.
                Style::default().fg(status_color)
            };
            Line::from(Span::styled(
                format!("{}{}{}", prefix, s.formatted_name(board, "sprint"), suffix),
                style,
            ))
        }
    }
}
