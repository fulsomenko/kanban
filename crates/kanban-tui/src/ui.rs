use crate::app::{App, AppMode, BoardFocus, CardFocus, Focus};
use crate::components::*;
use crate::theme::*;
use kanban_domain::{Sprint, SprintStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, ListItem, Paragraph},
    Frame,
};

pub fn render(app: &App, frame: &mut Frame) {
    match app.mode {
        AppMode::CardDetail | AppMode::AssignCardToSprint | AppMode::AssignMultipleCardsToSprint => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            render_card_detail_view(app, frame, chunks[0]);
            render_footer(app, frame, chunks[1]);

            if app.mode == AppMode::AssignCardToSprint {
                render_assign_sprint_popup(app, frame);
            }

            if app.mode == AppMode::AssignMultipleCardsToSprint {
                render_assign_multiple_cards_popup(app, frame);
            }
        }
        AppMode::BoardDetail => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            render_board_detail_view(app, frame, chunks[0]);
            render_footer(app, frame, chunks[1]);
        }
        AppMode::SprintDetail => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            render_sprint_detail_view(app, frame, chunks[0]);
            render_footer(app, frame, chunks[1]);
        }
        _ => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            render_main(app, frame, chunks[0]);
            render_footer(app, frame, chunks[1]);

            match app.mode {
                AppMode::CreateBoard => render_create_board_popup(app, frame),
                AppMode::CreateCard => render_create_card_popup(app, frame),
                AppMode::CreateSprint => render_create_sprint_popup(app, frame),
                AppMode::RenameBoard => render_rename_board_popup(app, frame),
                AppMode::ExportBoard => render_export_board_popup(app, frame),
                AppMode::ExportAll => render_export_all_popup(app, frame),
                AppMode::ImportBoard => render_import_board_popup(app, frame),
                AppMode::SetCardPoints => render_set_card_points_popup(app, frame),
                AppMode::SetCardPriority => render_set_card_priority_popup(app, frame),
                AppMode::SetBranchPrefix => render_set_branch_prefix_popup(app, frame),
                AppMode::OrderCards => render_order_cards_popup(app, frame),
                _ => {}
            }
        }
    }
}

fn render_main(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    render_projects_panel(app, frame, chunks[0]);
    render_tasks_panel(app, frame, chunks[1]);
}

fn render_projects_panel(app: &App, frame: &mut Frame, area: Rect) {
    let mut lines = vec![];

    if app.boards.is_empty() {
        lines.push(Line::from(Span::styled(
            "No projects yet. Press 'n' to create one!",
            label_text(),
        )));
    } else {
        for (idx, board) in app.boards.iter().enumerate() {
            let config = ListItemConfig::new()
                .selected(app.board_selection.get() == Some(idx))
                .focused(app.focus == Focus::Boards)
                .active(app.active_board_index == Some(idx));

            lines.push(styled_list_item(&board.name, &config));
        }
    }

    let panel_config = PanelConfig::new("Projects")
        .with_focus_indicator("Projects [1]")
        .focused(app.focus == Focus::Boards);

    let content = Paragraph::new(lines);
    render_panel(frame, area, &panel_config, content);
}

fn render_tasks_panel(app: &App, frame: &mut Frame, area: Rect) {
    let board_idx = app.active_board_index.or(app.board_selection.get());

    let mut lines = vec![];

    if let Some(idx) = board_idx {
        if let Some(board) = app.boards.get(idx) {
            let board_cards = app.get_sorted_board_cards(board.id);

            if board_cards.is_empty() {
                let message = if app.active_board_index.is_some() {
                    "  No tasks yet. Press 'n' to create one!"
                } else {
                    "  (Enter/Space) to add tasks"
                };
                lines.push(Line::from(Span::styled(message, label_text())));
            } else {
                for (card_idx, card) in board_cards.iter().enumerate() {
                    let line = render_card_list_item(CardListItemConfig {
                        card,
                        board,
                        sprints: &app.sprints,
                        is_selected: app.card_selection.get() == Some(card_idx),
                        is_focused: app.focus == Focus::Cards,
                        is_multi_selected: app.selected_cards.contains(&card.id),
                    });
                    lines.push(line);
                }
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  Select a project to preview tasks",
            label_text(),
        )));
    }

    let mut title = if app.focus == Focus::Cards {
        "Tasks [2]".to_string()
    } else {
        "Tasks".to_string()
    };

    if let Some(sprint_id) = app.active_sprint_filter {
        if let Some(sprint) = app.sprints.iter().find(|s| s.id == sprint_id) {
            if let Some(board_idx) = app.active_board_index.or(app.board_selection.get()) {
                if let Some(board) = app.boards.get(board_idx) {
                    let sprint_name = sprint.formatted_name(
                        board,
                        board.sprint_prefix.as_deref().unwrap_or("sprint"),
                    );
                    title.push_str(&format!(" - {}", sprint_name));
                }
            }
        }
    }

    let panel_config = PanelConfig::new("Tasks")
        .with_focus_indicator("Tasks [2]")
        .focused(app.focus == Focus::Cards);

    let content = Paragraph::new(lines).block(panel_config.block().title(title));
    frame.render_widget(content, area);
}


fn render_sprint_detail_view(app: &App, frame: &mut Frame, area: Rect) {
    if let Some(sprint_idx) = app.active_sprint_index {
        if let Some(sprint) = app.sprints.get(sprint_idx) {
            if let Some(board_idx) = app.active_board_index {
                if let Some(board) = app.boards.get(board_idx) {
                    let sprint_name = sprint.formatted_name(
                        board,
                        board.sprint_prefix.as_deref().unwrap_or("sprint"),
                    );

                    let mut lines = vec![
                        metadata_line_styled("Sprint", sprint_name, bold_highlight()),
                        Line::from(""),
                        metadata_line_styled(
                            "Status",
                            format!("{:?}", sprint.status),
                            sprint_status_style(sprint.status),
                        ),
                        metadata_line("Sprint Number", sprint.sprint_number.to_string()),
                    ];

                    if let Some(name) = sprint.get_name(board) {
                        lines.push(metadata_line("Name", name));
                    }

                    if let Some(start) = sprint.start_date {
                        lines.push(metadata_line(
                            "Start Date",
                            start.format("%Y-%m-%d %H:%M UTC").to_string(),
                        ));
                    }

                    if let Some(end) = sprint.end_date {
                        let end_style = if sprint.is_ended() {
                            Style::default().fg(Color::Red)
                        } else {
                            normal_text()
                        };
                        lines.push(metadata_line_styled(
                            "End Date",
                            end.format("%Y-%m-%d %H:%M UTC").to_string(),
                            end_style,
                        ));
                    }

                    lines.push(Line::from(""));

                    let card_count = app
                        .cards
                        .iter()
                        .filter(|c| c.sprint_id == Some(sprint.id))
                        .count();
                    lines.push(metadata_line_styled(
                        "Cards Assigned",
                        card_count.to_string(),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ));

                    if board.active_sprint_id == Some(sprint.id) {
                        lines.push(metadata_line_styled(
                            "Active Sprint",
                            "Yes (used for filtering)",
                            active_item(),
                        ));
                    }

                    lines.push(Line::from(""));
                    lines.push(metadata_line_styled(
                        "Created",
                        sprint.created_at.format("%Y-%m-%d %H:%M UTC").to_string(),
                        label_text(),
                    ));
                    lines.push(metadata_line_styled(
                        "Updated",
                        sprint.updated_at.format("%Y-%m-%d %H:%M UTC").to_string(),
                        label_text(),
                    ));

                    let content = Paragraph::new(lines).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(focused_border())
                            .title("Sprint Details"),
                    );
                    frame.render_widget(content, area);
                }
            }
        }
    }
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let help_text = match app.mode {
        AppMode::Normal => "q: quit | n: new | r: rename | e: edit project | x: export | X: export all | i: import | c: toggle complete | t: toggle sprint filter | v: select card | a: assign selected | 1/2: switch panel | j/k: navigate | Enter/Space: activate",
        AppMode::CreateBoard => "ESC: cancel | ENTER: confirm",
        AppMode::CreateCard => "ESC: cancel | ENTER: confirm",
        AppMode::CreateSprint => "ESC: cancel | ENTER: confirm",
        AppMode::RenameBoard => "ESC: cancel | ENTER: confirm",
        AppMode::ExportBoard => "ESC: cancel | ENTER: export",
        AppMode::ExportAll => "ESC: cancel | ENTER: export all",
        AppMode::ImportBoard => "ESC: cancel | j/k: navigate | ENTER/Space: import selected",
        AppMode::CardDetail => match app.card_focus {
            CardFocus::Title => "q: quit | ESC: back | 1/2/3: select panel | y: copy branch | Y: copy git cmd | e: edit title | s: assign sprint",
            CardFocus::Description => "q: quit | ESC: back | 1/2/3: select panel | y: copy branch | Y: copy git cmd | e: edit description | s: assign sprint",
            CardFocus::Metadata => "q: quit | ESC: back | 1/2/3: select panel | y: copy branch | Y: copy git cmd | e: edit points | s: assign sprint",
        },
        AppMode::SetCardPoints => "ESC: cancel | ENTER: confirm",
        AppMode::SetCardPriority => "ESC: cancel | j/k: navigate | ENTER: confirm",
        AppMode::BoardDetail => match app.board_focus {
            BoardFocus::Name => "q: quit | ESC: back | 1/2/3/4: select panel | e: edit name",
            BoardFocus::Description => "q: quit | ESC: back | 1/2/3/4: select panel | e: edit description",
            BoardFocus::Settings => "q: quit | ESC: back | 1/2/3/4: select panel | e: edit settings JSON | p: set branch prefix",
            BoardFocus::Sprints => "q: quit | ESC: back | 1/2/3/4: select panel | n: new sprint | j/k: navigate | Enter/Space: open sprint",
        },
        AppMode::SetBranchPrefix => "ESC: cancel | ENTER: confirm (empty to clear)",
        AppMode::OrderCards => "ESC: cancel | j/k: navigate | ENTER/Space/a: ascending | d: descending",
        AppMode::SprintDetail => "q: quit | ESC: back | a: activate sprint | c: complete sprint",
        AppMode::AssignCardToSprint => "ESC: cancel | j/k: navigate | ENTER/Space: assign",
        AppMode::AssignMultipleCardsToSprint => "ESC: cancel | j/k: navigate | ENTER/Space: assign",
    };
    let help = Paragraph::new(help_text)
        .style(label_text())
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, area);
}

fn render_create_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Create New Project",
        "Project Name:",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_create_card_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Create New Task",
        "Task Title:",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_create_sprint_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Create New Sprint",
        "Sprint Name (optional):",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_set_card_points_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Set Points",
        "Points (1-5 or empty):",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_set_card_priority_popup(app: &App, frame: &mut Frame) {
    use kanban_domain::CardPriority;

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

fn render_card_detail_view(app: &App, frame: &mut Frame, area: Rect) {
    if let Some(card_idx) = app.active_card_index {
        if let Some(card) = app.cards.get(card_idx) {
            if let Some(board_idx) = app.active_board_index {
                if let Some(board) = app.boards.get(board_idx) {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(5),
                            Constraint::Length(5),
                            Constraint::Min(0),
                        ])
                        .split(area);

                    let title_config = FieldSectionConfig::new("Task Title")
                        .with_focus_indicator("Task Title [1]")
                        .focused(app.card_focus == CardFocus::Title);
                    let title = Paragraph::new(card.title.clone())
                        .style(bold_highlight())
                        .block(title_config.block());
                    frame.render_widget(title, chunks[0]);

                    let meta_config = FieldSectionConfig::new("Metadata")
                        .with_focus_indicator("Metadata [2]")
                        .focused(app.card_focus == CardFocus::Metadata);

                    let meta_lines = vec![
                        metadata_line_multi(vec![
                            ("Priority", format!("{:?}", card.priority), normal_text()),
                            ("Status", format!("{:?}", card.status), normal_text()),
                            ("Points", card.points.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string()), normal_text()),
                        ]),
                        if let Some(due_date) = card.due_date {
                            metadata_line_styled("Due", due_date.format("%Y-%m-%d %H:%M").to_string(), Style::default().fg(Color::Red))
                        } else {
                            Line::from(Span::styled("No due date", label_text()))
                        },
                        metadata_line_styled(
                            "Branch",
                            card.branch_name(board, &app.sprints, app.app_config.effective_default_prefix()),
                            active_item()
                        ),
                    ];

                    let meta = Paragraph::new(meta_lines).block(meta_config.block());
                    frame.render_widget(meta, chunks[1]);

                    let desc_config = FieldSectionConfig::new("Description")
                        .with_focus_indicator("Description [3]")
                        .focused(app.card_focus == CardFocus::Description);
                    let desc_text = card.description.as_deref().unwrap_or("No description");
                    let desc = Paragraph::new(desc_text)
                        .style(normal_text())
                        .block(desc_config.block());
                    frame.render_widget(desc, chunks[2]);
                }
            }
        }
    }
}

fn render_rename_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Rename Project",
        "New Project Name:",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_export_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Export Project",
        "Filename:",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_export_all_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Export All Projects",
        "Filename:",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_import_board_popup(app: &App, frame: &mut Frame) {
    let inner = render_popup_with_block(frame, "Import Projects", 60, 50);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let label = Paragraph::new("Select a JSON file to import:").style(highlight_text());
    frame.render_widget(label, chunks[0]);

    if app.import_files.is_empty() {
        let empty_msg = Paragraph::new("No JSON files found in current directory")
            .style(label_text());
        frame.render_widget(empty_msg, chunks[1]);
    } else {
        let mut lines = vec![];
        for (idx, filename) in app.import_files.iter().enumerate() {
            let config = ListItemConfig::new()
                .selected(app.import_selection.get() == Some(idx))
                .focused(true);
            lines.push(styled_list_item(filename, &config));
        }
        let list = Paragraph::new(lines);
        frame.render_widget(list, chunks[1]);
    }
}

fn render_board_detail_view(app: &App, frame: &mut Frame, area: Rect) {
    if let Some(board_idx) = app.board_selection.get() {
        if let Some(board) = app.boards.get(board_idx) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5),
                    Constraint::Length(8),
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(area);

            let name_config = FieldSectionConfig::new("Project Name")
                .with_focus_indicator("Project Name [1]")
                .focused(app.board_focus == BoardFocus::Name);
            let name = Paragraph::new(board.name.clone())
                .style(bold_highlight())
                .block(name_config.block());
            frame.render_widget(name, chunks[0]);

            let desc_config = FieldSectionConfig::new("Description")
                .with_focus_indicator("Description [2]")
                .focused(app.board_focus == BoardFocus::Description);
            let desc_text = board.description.as_deref().unwrap_or("No description");
            let desc = Paragraph::new(desc_text)
                .style(normal_text())
                .block(desc_config.block());
            frame.render_widget(desc, chunks[1]);

            let settings_config = FieldSectionConfig::new("Settings")
                .with_focus_indicator("Settings [3]")
                .focused(app.board_focus == BoardFocus::Settings);

            let mut settings_lines = vec![
                if let Some(prefix) = &board.branch_prefix {
                    metadata_line_styled("Branch Prefix", prefix, active_item())
                } else {
                    Line::from(vec![
                        Span::styled("Branch Prefix: ", label_text()),
                        Span::styled(
                            app.app_config.effective_default_prefix().to_string(),
                            normal_text(),
                        ),
                        Span::styled(" (default)", label_text()),
                    ])
                },
                Line::from(""),
                metadata_line(
                    "Sprint Duration",
                    board
                        .sprint_duration_days
                        .map(|d| format!("{} days", d))
                        .unwrap_or_else(|| "(not set)".to_string()),
                ),
                metadata_line(
                    "Sprint Prefix",
                    board.sprint_prefix.as_deref().unwrap_or("(not set)"),
                ),
            ];

            let available_names: Vec<&str> = board
                .sprint_names
                .iter()
                .skip(board.sprint_name_used_count)
                .map(|s| s.as_str())
                .collect();

            if !available_names.is_empty() {
                settings_lines.push(metadata_line("Sprint Names", available_names.join(", ")));
            }

            let settings = Paragraph::new(settings_lines).block(settings_config.block());
            frame.render_widget(settings, chunks[2]);

            let sprints_config = FieldSectionConfig::new("Sprints")
                .with_focus_indicator("Sprints [4]")
                .focused(app.board_focus == BoardFocus::Sprints);

            let board_sprints: Vec<&Sprint> = app
                .sprints
                .iter()
                .filter(|s| s.board_id == board.id)
                .collect();

            let mut sprint_lines = vec![];

            if board_sprints.is_empty() {
                sprint_lines.push(Line::from(Span::styled(
                    "  No sprints yet. Press 'n' to create one!",
                    label_text(),
                )));
            } else {
                for (sprint_idx, sprint) in board_sprints.iter().enumerate() {
                    let is_selected = app.sprint_selection.get() == Some(sprint_idx);
                    let is_focused = app.board_focus == BoardFocus::Sprints;

                    let status_symbol = match sprint.status {
                        SprintStatus::Planning => "○",
                        SprintStatus::Active => "●",
                        SprintStatus::Completed => "✓",
                        SprintStatus::Cancelled => "✗",
                    };

                    let sprint_name = sprint.formatted_name(
                        board,
                        board.sprint_prefix.as_deref().unwrap_or("sprint"),
                    );

                    let card_count = app
                        .cards
                        .iter()
                        .filter(|c| c.sprint_id == Some(sprint.id))
                        .count();

                    let is_active_sprint = board.active_sprint_id == Some(sprint.id);
                    let is_ended = sprint.is_ended();

                    let mut base_style = normal_text();
                    if is_selected && is_focused {
                        base_style = base_style.bg(SELECTED_BG);
                    }

                    let mut spans = vec![
                        Span::styled(
                            format!("{} ", status_symbol),
                            sprint_status_style(sprint.status),
                        ),
                        Span::styled(sprint_name, base_style),
                        Span::styled(format!(" ({})", card_count), label_text()),
                    ];

                    if is_active_sprint {
                        let mut active_style = active_item();
                        if is_selected && is_focused {
                            active_style = active_style.bg(SELECTED_BG);
                        }
                        spans.push(Span::styled(" Active", active_style));
                    }

                    if is_ended {
                        let mut ended_style =
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
                        if is_selected && is_focused {
                            ended_style = ended_style.bg(SELECTED_BG);
                        }
                        spans.push(Span::styled(" Ended", ended_style));
                    }

                    sprint_lines.push(Line::from(spans));
                }
            }

            let sprints = Paragraph::new(sprint_lines).block(sprints_config.block());
            frame.render_widget(sprints, chunks[3]);
        }
    }
}


fn render_set_branch_prefix_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Set Branch Prefix",
        "Branch Prefix:",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_order_cards_popup(app: &App, frame: &mut Frame) {
    use kanban_domain::{SortField, SortOrder};

    let sort_fields = [
        SortField::Points,
        SortField::Priority,
        SortField::CreatedAt,
        SortField::UpdatedAt,
        SortField::Status,
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

fn render_assign_sprint_popup(app: &App, frame: &mut Frame) {
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
        if let Some(board) = app.boards.get(board_idx) {
            let board_sprints: Vec<_> = app
                .sprints
                .iter()
                .filter(|s| s.board_id == board.id)
                .collect();

            let current_sprint_id = if let Some(card_idx) = app.active_card_index {
                app.cards.get(card_idx).and_then(|c| c.sprint_id)
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
                    sprint.formatted_name(board, board.sprint_prefix.as_deref().unwrap_or("sprint"))
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

fn render_assign_multiple_cards_popup(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 50, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("Assign {} Cards to Sprint", app.selected_cards.len()))
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
        if let Some(board) = app.boards.get(board_idx) {
            let board_sprints: Vec<_> = app
                .sprints
                .iter()
                .filter(|s| s.board_id == board.id)
                .collect();

            for (idx, sprint_option) in std::iter::once(None)
                .chain(board_sprints.iter().map(|s| Some(*s)))
                .enumerate()
            {
                let is_selected = app.sprint_assign_selection.get() == Some(idx);

                let style = if is_selected {
                    Style::default().fg(Color::White).bg(Color::Blue)
                } else {
                    Style::default().fg(Color::White)
                };

                let prefix = if is_selected { "> " } else { "  " };

                let sprint_name = if let Some(sprint) = sprint_option {
                    sprint.formatted_name(board, board.sprint_prefix.as_deref().unwrap_or("sprint"))
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

