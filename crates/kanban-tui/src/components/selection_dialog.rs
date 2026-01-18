use crate::app::App;
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

        let selected = app.priority_selection.get();

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
            .position(|f| Some(*f) == app.current_sort_field);

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
                    match app.current_sort_order {
                        Some(SortOrder::Ascending) => Some(" (↑)".to_string()),
                        Some(SortOrder::Descending) => Some(" (↓)".to_string()),
                        None => None,
                    }
                } else {
                    None
                };

                (field_name.to_string(), order_indicator)
            },
            app.sort_field_selection.get(),
            active_idx,
            60,
            50,
        );
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
        use kanban_domain::SprintStatus;
        if let Some(board_idx) = app.active_board_index {
            if let Some(board) = app.ctx.boards.get(board_idx) {
                let sprint_count = app
                    .ctx
                    .sprints
                    .iter()
                    .filter(|s| s.board_id == board.id)
                    .filter(|s| {
                        s.status != SprintStatus::Completed && s.status != SprintStatus::Cancelled
                    })
                    .count();
                sprint_count + 1 // +1 for None option
            } else {
                1
            }
        } else {
            1
        }
    }

    fn render(&self, app: &App, frame: &mut Frame) {
        use crate::components::centered_rect;
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            style::{Color, Modifier, Style},
            text::{Line, Span},
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

        if let Some(board_idx) = app.active_board_index {
            if let Some(board) = app.ctx.boards.get(board_idx) {
                use kanban_domain::SprintStatus;
                let board_sprints: Vec<_> = app
                    .ctx
                    .sprints
                    .iter()
                    .filter(|s| s.board_id == board.id)
                    .filter(|s| {
                        s.status != SprintStatus::Completed && s.status != SprintStatus::Cancelled
                    })
                    .collect();

                let current_sprint_id = if let Some(card_idx) = app.active_card_index {
                    app.ctx.cards.get(card_idx).and_then(|c| c.sprint_id)
                } else {
                    None
                };

                for (idx, sprint_option) in std::iter::once(None)
                    .chain(board_sprints.iter().map(|s| Some(*s)))
                    .enumerate()
                {
                    let is_selected = app.sprint_assign_selection.get() == Some(idx);
                    let is_current = match (sprint_option, current_sprint_id) {
                        (None, None) => true,
                        (Some(s), Some(id)) => s.id == id,
                        _ => false,
                    };

                    let style = if is_selected {
                        Style::default().fg(Color::White).bg(Color::Blue)
                    } else if is_current {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let prefix = if is_selected { "> " } else { "  " };
                    let current_indicator = if is_current { " (current)" } else { "" };

                    let sprint_name = if let Some(sprint) = sprint_option {
                        sprint.formatted_name(board, "sprint")
                    } else {
                        "(None)".to_string()
                    };

                    lines.push(Line::from(Span::styled(
                        format!("{}{}{}", prefix, sprint_name, current_indicator),
                        style,
                    )));
                }
            }
        }

        let list = Paragraph::new(lines);
        frame.render_widget(list, chunks[1]);
    }
}
