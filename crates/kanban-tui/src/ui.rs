use crate::app::{App, AppMode, BoardFocus, CardFocus, Focus};
use crate::components::*;
use crate::theme::*;
use crate::view_strategy::UnifiedViewStrategy;
use kanban_domain::{Sprint, SprintStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, ListItem, Paragraph},
    Frame,
};

pub fn render(app: &mut App, frame: &mut Frame) {
    // Check if we're in Help mode and render underlying view
    let is_help_mode = matches!(app.mode, AppMode::Help(_));

    if !is_help_mode {
        match app.mode {
            AppMode::CardDetail
            | AppMode::AssignCardToSprint
            | AppMode::AssignMultipleCardsToSprint => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(3)])
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
                    .constraints([Constraint::Min(0), Constraint::Length(3)])
                    .split(frame.area());

                render_board_detail_view(app, frame, chunks[0]);
                render_footer(app, frame, chunks[1]);
            }
            AppMode::SprintDetail => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(3)])
                    .split(frame.area());

                render_sprint_detail_view(app, frame, chunks[0]);
                render_footer(app, frame, chunks[1]);
            }
            _ => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(3)])
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
                    AppMode::SetSprintPrefix => render_set_sprint_prefix_popup(app, frame),
                    AppMode::SetSprintCardPrefix => render_set_sprint_card_prefix_popup(app, frame),
                    AppMode::OrderCards => render_order_cards_popup(app, frame),
                    AppMode::CreateColumn => render_create_column_popup(app, frame),
                    AppMode::RenameColumn => render_rename_column_popup(app, frame),
                    AppMode::DeleteColumnConfirm => render_delete_column_confirm_popup(app, frame),
                    AppMode::SelectTaskListView => render_select_task_list_view_popup(app, frame),
                    AppMode::FilterOptions => render_filter_options_popup(app, frame),
                    _ => {}
                }
            }
        }
    } else {
        // Render underlying view without footer
        match app.mode {
            AppMode::CardDetail
            | AppMode::AssignCardToSprint
            | AppMode::AssignMultipleCardsToSprint => {
                render_card_detail_view(app, frame, frame.area());
            }
            AppMode::BoardDetail => {
                render_board_detail_view(app, frame, frame.area());
            }
            AppMode::SprintDetail => {
                render_sprint_detail_view(app, frame, frame.area());
            }
            _ => {
                render_main(app, frame, frame.area());
            }
        }

        // Render help popup on top
        render_help_popup(app, frame);
    }
}

fn render_main(app: &mut App, frame: &mut Frame, area: Rect) {
    let is_kanban_view = if let Some(idx) = app.active_board_index {
        if let Some(board) = app.boards.get(idx) {
            board.task_list_view == kanban_domain::TaskListView::ColumnView
        } else {
            false
        }
    } else {
        false
    };

    if is_kanban_view {
        app.viewport_height = area.height.saturating_sub(2) as usize;
        render_tasks_panel(app, frame, area);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        app.viewport_height = chunks[1].height.saturating_sub(2) as usize;
        render_projects_panel(app, frame, chunks[0]);
        render_tasks_panel(app, frame, chunks[1]);
    }
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

pub fn build_filter_title_suffix(app: &App) -> Option<String> {
    let mut filters = vec![];

    if app.hide_assigned_cards {
        filters.push("Unassigned Cards".to_string());
    }

    if !app.active_sprint_filters.is_empty() {
        if let Some(board_idx) = app.active_board_index.or(app.board_selection.get()) {
            if let Some(board) = app.boards.get(board_idx) {
                let mut sprint_names: Vec<String> = app
                    .sprints
                    .iter()
                    .filter(|s| app.active_sprint_filters.contains(&s.id))
                    .map(|s| s.formatted_name(board, "sprint"))
                    .collect();
                sprint_names.sort();
                filters.extend(sprint_names);
            }
        }
    }

    if filters.is_empty() {
        None
    } else {
        Some(format!(" - {}", filters.join(" + ")))
    }
}

pub fn build_tasks_panel_title(app: &App, with_filter_suffix: bool) -> String {
    let mut title = if app.mode == AppMode::ArchivedCardsView {
        "Archive".to_string()
    } else if app.focus == Focus::Cards {
        "Tasks [2]".to_string()
    } else {
        "Tasks".to_string()
    };

    if with_filter_suffix && app.mode != AppMode::ArchivedCardsView {
        if let Some(suffix) = build_filter_title_suffix(app) {
            title.push_str(&suffix);
        }
    }

    title
}

fn render_tasks_panel(app: &App, frame: &mut Frame, area: Rect) {
    render_tasks(app, frame, area);
}

fn render_tasks(app: &App, frame: &mut Frame, area: Rect) {
    if let Some(unified_strategy) = app
        .view_strategy
        .as_any()
        .downcast_ref::<UnifiedViewStrategy>()
    {
        unified_strategy
            .get_render_strategy()
            .render(app, frame, area);
    }
}

fn render_sprint_detail_view(app: &App, frame: &mut Frame, area: Rect) {
    if let Some(sprint_idx) = app.active_sprint_index {
        if let Some(sprint) = app.sprints.get(sprint_idx) {
            if let Some(board_idx) = app.active_board_index {
                if let Some(board) = app.boards.get(board_idx) {
                    let is_completed = sprint.status == SprintStatus::Completed;

                    if is_completed {
                        render_sprint_detail_with_tasks(app, frame, area, sprint, board);
                    } else {
                        render_sprint_detail_metadata(app, frame, area, sprint, board);
                    }
                }
            }
        }
    }
}

fn render_sprint_detail_metadata(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    sprint: &Sprint,
    board: &kanban_domain::Board,
) {
    let sprint_name = sprint.formatted_name(board, "sprint");

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
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));

    if board.active_sprint_id == Some(sprint.id) {
        lines.push(metadata_line_styled("Active Sprint", "Yes", active_item()));
    }

    lines.push(Line::from(""));

    if let Some(prefix) = &sprint.prefix {
        lines.push(metadata_line_styled("Sprint Prefix", prefix, active_item()));
    }

    if let Some(prefix) = &sprint.card_prefix {
        lines.push(metadata_line_styled(
            "Card Prefix Override",
            prefix,
            active_item(),
        ));
    }

    if sprint.prefix.is_some() || sprint.card_prefix.is_some() {
        lines.push(Line::from(""));
    }

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

fn render_sprint_detail_with_tasks(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    sprint: &Sprint,
    board: &kanban_domain::Board,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_sprint_task_panel_with_selection(
        app,
        frame,
        chunks[0],
        sprint,
        board,
        &app.sprint_uncompleted_cards,
        "Uncompleted",
        app.sprint_task_panel == crate::app::SprintTaskPanel::Uncompleted,
    );

    render_sprint_task_panel_with_selection(
        app,
        frame,
        chunks[1],
        sprint,
        board,
        &app.sprint_completed_cards,
        "Completed",
        app.sprint_task_panel == crate::app::SprintTaskPanel::Completed,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_sprint_task_panel_with_selection(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    _sprint: &Sprint,
    board: &kanban_domain::Board,
    task_list: &crate::card_list::CardList,
    title_suffix: &str,
    is_focused: bool,
) {
    let mut lines = vec![];
    let selected_idx = task_list.get_selected_index();

    if task_list.is_empty() {
        lines.push(Line::from(Span::styled("  (no tasks)", label_text())));
    } else {
        let viewport_height = area.height.saturating_sub(2) as usize;
        let render_info = task_list.get_render_info(viewport_height);

        if render_info.show_above_indicator {
            let count = render_info.cards_above_count;
            let plural = if count == 1 { "" } else { "s" };
            lines.push(Line::from(Span::styled(
                format!("  {} Task{} above", count, plural),
                Style::default().fg(Color::DarkGray),
            )));
        }

        for card_idx in &render_info.visible_card_indices {
            if let Some(card_id) = task_list.cards.get(*card_idx) {
                if let Some(card) = app.cards.iter().find(|c| c.id == *card_id) {
                    let is_selected = selected_idx == Some(*card_idx) && is_focused;
                    let animation_type =
                        app.animating_cards.get(&card.id).map(|a| a.animation_type);
                    let line = render_card_list_item(CardListItemConfig {
                        card,
                        board,
                        sprints: &app.sprints,
                        is_selected,
                        is_focused,
                        is_multi_selected: false,
                        show_sprint_name: false,
                        animation_type,
                    });
                    lines.push(line);
                }
            }
        }

        if render_info.show_below_indicator {
            let count = render_info.cards_below_count;
            let plural = if count == 1 { "" } else { "s" };
            lines.push(Line::from(Span::styled(
                format!("  {} Task{} below", count, plural),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Calculate points from cards in this panel
    let cards: Vec<&kanban_domain::Card> = task_list
        .cards
        .iter()
        .filter_map(|card_id| app.cards.iter().find(|c| c.id == *card_id))
        .collect();
    let points = App::calculate_points(&cards);

    lines.push(Line::from(Span::styled(
        format!("Points: {}", points),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    let border_style = if is_focused {
        focused_border()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let content = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!("{} ({})", title_suffix, task_list.len())),
    );
    frame.render_widget(content, area);
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let _is_kanban_view =
        if let Some(board_idx) = app.active_board_index.or(app.board_selection.get()) {
            if let Some(board) = app.boards.get(board_idx) {
                board.task_list_view == kanban_domain::TaskListView::ColumnView
            } else {
                false
            }
        } else {
            false
        };

    if app.mode == AppMode::Search {
        let search_text = format!("/{}", app.search.query());
        let help_text = "ESC/ENTER: exit search";

        let available_width = area.width.saturating_sub(4);
        let help_len = help_text.len() as u16;
        let search_len = search_text.len() as u16;

        let padding = if available_width > search_len + help_len + 1 {
            available_width
                .saturating_sub(search_len)
                .saturating_sub(help_len)
        } else {
            1
        };

        let footer_line = Line::from(vec![
            Span::styled(search_text, Style::default().fg(Color::White)),
            Span::styled(
                format!("{:width$}", "", width = padding as usize),
                label_text(),
            ),
            Span::styled(help_text, label_text()),
        ]);

        let help = Paragraph::new(footer_line).block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, area);
        return;
    }

    use crate::keybindings::KeybindingRegistry;

    let help_text: String = if let AppMode::SprintDetail = app.mode {
        let component = match app.sprint_task_panel {
            crate::app::SprintTaskPanel::Uncompleted => &app.sprint_uncompleted_component,
            crate::app::SprintTaskPanel::Completed => &app.sprint_completed_component,
        };
        let provider = KeybindingRegistry::get_provider(app);
        let context = provider.get_context();
        let keybindings = context
            .bindings
            .iter()
            .map(|b| format!("{}: {}", b.key, b.short_description))
            .collect::<Vec<_>>()
            .join(" | ");
        let component_help = component.help_text();
        format!("{} | {}", keybindings, component_help)
    } else {
        let provider = KeybindingRegistry::get_provider(app);
        let context = provider.get_context();
        context
            .bindings
            .iter()
            .map(|b| format!("{}: {}", b.key, b.short_description))
            .collect::<Vec<_>>()
            .join(" | ")
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
    use crate::components::{PriorityDialog, SelectionDialog};
    let dialog = PriorityDialog;
    dialog.render(app, frame);
}

fn render_card_detail_view(app: &App, frame: &mut Frame, area: Rect) {
    if let Some(card_idx) = app.active_card_index {
        if let Some(card) = app.cards.get(card_idx) {
            if let Some(board_idx) = app.active_board_index {
                if let Some(board) = app.boards.get(board_idx) {
                    let has_sprint_logs = card.sprint_logs.len() > 1;

                    let constraints = vec![
                        Constraint::Length(5),
                        Constraint::Length(6),
                        Constraint::Min(0),
                    ];

                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(constraints)
                        .split(area);

                    let title_config = FieldSectionConfig::new("Task Title")
                        .with_focus_indicator("Task Title [1]")
                        .focused(app.card_focus == CardFocus::Title);
                    let title = Paragraph::new(card.title.clone())
                        .style(bold_highlight())
                        .block(title_config.block());
                    frame.render_widget(title, chunks[0]);

                    if has_sprint_logs {
                        let meta_chunks = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                            .split(chunks[1]);

                        let meta_config = FieldSectionConfig::new("Metadata")
                            .with_focus_indicator("Metadata [2]")
                            .focused(app.card_focus == CardFocus::Metadata);

                        let meta_lines = vec![
                            metadata_line_multi(vec![
                                ("Priority", format!("{:?}", card.priority), normal_text()),
                                ("Status", format!("{:?}", card.status), normal_text()),
                                (
                                    "Points",
                                    card.points
                                        .map(|p| p.to_string())
                                        .unwrap_or_else(|| "-".to_string()),
                                    normal_text(),
                                ),
                            ]),
                            if let Some(due_date) = card.due_date {
                                metadata_line_styled(
                                    "Due",
                                    due_date.format("%Y-%m-%d %H:%M").to_string(),
                                    Style::default().fg(Color::Red),
                                )
                            } else {
                                Line::from(Span::styled("No due date", label_text()))
                            },
                            metadata_line_styled(
                                "Branch",
                                card.branch_name(
                                    board,
                                    &app.sprints,
                                    app.app_config.effective_default_card_prefix(),
                                ),
                                active_item(),
                            ),
                        ];

                        let meta = Paragraph::new(meta_lines).block(meta_config.block());
                        frame.render_widget(meta, meta_chunks[0]);

                        let sprint_logs_config = FieldSectionConfig::new("Sprint History");
                        let mut sprint_log_lines = vec![];

                        let max_visible = 3;
                        let total_logs = card.sprint_logs.len();
                        let start_idx = total_logs.saturating_sub(max_visible);

                        for (display_idx, log) in card.sprint_logs[start_idx..].iter().enumerate() {
                            let absolute_idx = start_idx + display_idx;
                            let sprint_name_str = log
                                .sprint_name
                                .as_deref()
                                .unwrap_or(&log.sprint_number.to_string())
                                .to_string();
                            let started = log.started_at.format("%m-%d %H:%M").to_string();
                            let status_str = if let Some(ended) = log.ended_at {
                                let ended_fmt = ended.format("%m-%d %H:%M").to_string();
                                format!("{} → {}", started, ended_fmt)
                            } else {
                                format!("{} → (current)", started)
                            };

                            sprint_log_lines.push(Line::from(vec![
                                Span::styled(format!("{}. ", absolute_idx + 1), label_text()),
                                Span::styled(
                                    format!("{} ", sprint_name_str),
                                    Style::default().fg(Color::Cyan),
                                ),
                                Span::styled(status_str, label_text()),
                            ]));
                        }

                        if start_idx > 0 {
                            sprint_log_lines.insert(
                                0,
                                Line::from(Span::styled(
                                    format!("... ({} earlier entries)", start_idx),
                                    label_text(),
                                )),
                            );
                        }

                        let sprint_logs =
                            Paragraph::new(sprint_log_lines).block(sprint_logs_config.block());
                        frame.render_widget(sprint_logs, meta_chunks[1]);

                        let desc_config = FieldSectionConfig::new("Description")
                            .with_focus_indicator("Description [3]")
                            .focused(app.card_focus == CardFocus::Description);
                        let desc_lines = if let Some(desc_text) = &card.description {
                            crate::markdown_renderer::render_markdown(desc_text)
                        } else {
                            vec![Line::from(Span::styled("No description", label_text()))]
                        };
                        let desc = Paragraph::new(desc_lines).block(desc_config.block());
                        frame.render_widget(desc, chunks[2]);
                    } else {
                        let meta_config = FieldSectionConfig::new("Metadata")
                            .with_focus_indicator("Metadata [2]")
                            .focused(app.card_focus == CardFocus::Metadata);

                        let meta_lines = vec![
                            metadata_line_multi(vec![
                                ("Priority", format!("{:?}", card.priority), normal_text()),
                                ("Status", format!("{:?}", card.status), normal_text()),
                                (
                                    "Points",
                                    card.points
                                        .map(|p| p.to_string())
                                        .unwrap_or_else(|| "-".to_string()),
                                    normal_text(),
                                ),
                            ]),
                            if let Some(due_date) = card.due_date {
                                metadata_line_styled(
                                    "Due",
                                    due_date.format("%Y-%m-%d %H:%M").to_string(),
                                    Style::default().fg(Color::Red),
                                )
                            } else {
                                Line::from(Span::styled("No due date", label_text()))
                            },
                            metadata_line_styled(
                                "Branch",
                                card.branch_name(
                                    board,
                                    &app.sprints,
                                    app.app_config.effective_default_card_prefix(),
                                ),
                                active_item(),
                            ),
                        ];

                        let meta = Paragraph::new(meta_lines).block(meta_config.block());
                        frame.render_widget(meta, chunks[1]);

                        let desc_config = FieldSectionConfig::new("Description")
                            .with_focus_indicator("Description [3]")
                            .focused(app.card_focus == CardFocus::Description);
                        let desc_lines = if let Some(desc_text) = &card.description {
                            crate::markdown_renderer::render_markdown(desc_text)
                        } else {
                            vec![Line::from(Span::styled("No description", label_text()))]
                        };
                        let desc = Paragraph::new(desc_lines).block(desc_config.block());
                        frame.render_widget(desc, chunks[2]);
                    }
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
        let empty_msg =
            Paragraph::new("No JSON files found in current directory").style(label_text());
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
                    Constraint::Percentage(30),
                    Constraint::Percentage(35),
                    Constraint::Percentage(35),
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
            let desc_lines = if let Some(desc_text) = &board.description {
                crate::markdown_renderer::render_markdown(desc_text)
            } else {
                vec![Line::from(Span::styled("No description", label_text()))]
            };
            let desc = Paragraph::new(desc_lines).block(desc_config.block());
            frame.render_widget(desc, chunks[1]);

            let settings_config = FieldSectionConfig::new("Settings")
                .with_focus_indicator("Settings [3]")
                .focused(app.board_focus == BoardFocus::Settings);

            let mut settings_lines = vec![
                if let Some(prefix) = &board.sprint_prefix {
                    metadata_line_styled("Sprint Prefix", prefix, active_item())
                } else {
                    Line::from(vec![
                        Span::styled("Sprint Prefix: ", label_text()),
                        Span::styled(
                            app.app_config.effective_default_sprint_prefix().to_string(),
                            normal_text(),
                        ),
                        Span::styled(" (default)", label_text()),
                    ])
                },
                if let Some(prefix) = &board.card_prefix {
                    metadata_line_styled("Card Prefix", prefix, active_item())
                } else {
                    Line::from(vec![
                        Span::styled("Card Prefix: ", label_text()),
                        Span::styled(
                            app.app_config.effective_default_card_prefix().to_string(),
                            normal_text(),
                        ),
                        Span::styled(" (default)", label_text()),
                    ])
                },
            ];

            // Show active sprint's card prefix override if it exists
            if let Some(sprint_prefix) =
                crate::board_context::get_active_sprint_card_prefix_override(board, &app.sprints)
            {
                settings_lines.push(metadata_line_styled(
                    "Active Sprint Card Prefix",
                    sprint_prefix,
                    Style::default().fg(Color::Cyan),
                ));
            }

            settings_lines.push(Line::from(""));
            settings_lines.push(metadata_line(
                "Sprint Duration",
                board
                    .sprint_duration_days
                    .map(|d| format!("{} days", d))
                    .unwrap_or_else(|| "(not set)".to_string()),
            ));

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

                    let sprint_name = sprint.formatted_name(board, "sprint");

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

            let columns_config = FieldSectionConfig::new("Columns")
                .with_focus_indicator("Columns [5]")
                .focused(app.board_focus == BoardFocus::Columns);

            let mut board_columns: Vec<_> = app
                .columns
                .iter()
                .filter(|col| col.board_id == board.id)
                .collect();
            board_columns.sort_by_key(|col| col.position);

            let mut column_lines = vec![];

            if board_columns.is_empty() {
                column_lines.push(Line::from(Span::styled(
                    "  No columns yet. Press 'n' to create one!",
                    label_text(),
                )));
            } else {
                for (column_idx, column) in board_columns.iter().enumerate() {
                    let is_selected = app.column_selection.get() == Some(column_idx);
                    let is_focused = app.board_focus == BoardFocus::Columns;

                    let card_count = app
                        .cards
                        .iter()
                        .filter(|c| c.column_id == column.id)
                        .count();

                    let mut base_style = normal_text();
                    if is_selected && is_focused {
                        base_style = base_style.bg(SELECTED_BG);
                    }

                    let spans = vec![
                        Span::styled(format!("{}. ", column.position + 1), label_text()),
                        Span::styled(&column.name, base_style),
                        Span::styled(format!(" ({})", card_count), label_text()),
                    ];

                    column_lines.push(Line::from(spans));
                }
            }

            let columns = Paragraph::new(column_lines).block(columns_config.block());
            frame.render_widget(columns, chunks[4]);
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

fn render_set_sprint_prefix_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Set Sprint Prefix",
        "Sprint Prefix:",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_set_sprint_card_prefix_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Set Card Prefix Override",
        "Card Prefix:",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_order_cards_popup(app: &App, frame: &mut Frame) {
    use crate::components::{SelectionDialog, SortFieldDialog};
    let dialog = SortFieldDialog;
    dialog.render(app, frame);
}

fn render_assign_sprint_popup(app: &App, frame: &mut Frame) {
    use crate::components::{SelectionDialog, SprintAssignDialog};
    let dialog = SprintAssignDialog;
    dialog.render(app, frame);
}

fn render_assign_multiple_cards_popup(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 50, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(
            "Assign {} Cards to Sprint",
            app.selected_cards.len()
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

fn render_create_column_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Create New Column",
        "Column Name:",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_rename_column_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Rename Column",
        "New Column Name:",
        app.input.as_str(),
        app.input.cursor_pos(),
    );
}

fn render_delete_column_confirm_popup(_app: &App, frame: &mut Frame) {
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

fn render_select_task_list_view_popup(app: &App, frame: &mut Frame) {
    use kanban_domain::TaskListView;

    let views = [
        TaskListView::Flat,
        TaskListView::GroupedByColumn,
        TaskListView::ColumnView,
    ];

    let selected = app.task_list_view_selection.get();

    let current_view = app
        .active_board_index
        .and_then(|idx| app.boards.get(idx))
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

fn render_filter_options_popup(app: &App, frame: &mut Frame) {
    let area = centered_rect(70, 75, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Filter Options")
        .borders(Borders::ALL)
        .border_style(focused_border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(inner);

    if let Some(ref dialog_state) = app.filter_dialog_state {
        let section_index = dialog_state.section_index;

        let mut sprint_lines = vec![Line::from(Span::styled(
            "Sprints",
            if section_index == 0 {
                bold_highlight()
            } else {
                normal_text()
            },
        ))];

        let unassigned_cursor = if section_index == 0 && dialog_state.item_selection == 0 {
            "> "
        } else {
            "  "
        };

        sprint_lines.push(Line::from(vec![
            Span::raw(unassigned_cursor),
            Span::styled(
                if dialog_state.filters.show_unassigned_sprints {
                    "[✓]"
                } else {
                    "[ ]"
                },
                normal_text(),
            ),
            Span::raw(" "),
            Span::styled("Show cards with unassigned sprints", normal_text()),
        ]));

        sprint_lines.push(Line::from(Span::styled(
            "─────────────────────────",
            label_text(),
        )));

        if let Some(board_idx) = app.active_board_index {
            if let Some(board) = app.boards.get(board_idx) {
                let board_sprints: Vec<_> = app
                    .sprints
                    .iter()
                    .filter(|s| s.board_id == board.id)
                    .collect();

                if board_sprints.is_empty() {
                    sprint_lines.push(Line::from(Span::styled(
                        "  (no sprints available)",
                        label_text(),
                    )));
                } else {
                    for (idx, sprint) in board_sprints.iter().enumerate() {
                        let is_selected = dialog_state
                            .filters
                            .selected_sprint_ids
                            .contains(&sprint.id);
                        let cursor = if section_index == 0 && dialog_state.item_selection == idx + 1
                        {
                            "> "
                        } else {
                            "  "
                        };

                        sprint_lines.push(Line::from(vec![
                            Span::raw(cursor),
                            Span::styled(if is_selected { "[✓]" } else { "[ ]" }, normal_text()),
                            Span::raw(" "),
                            Span::styled(sprint.formatted_name(board, "sprint"), normal_text()),
                        ]));
                    }
                }
            }
        }

        let section1 = Paragraph::new(sprint_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if section_index == 0 {
                    focused_border()
                } else {
                    Style::default()
                }),
        );
        frame.render_widget(section1, chunks[0]);

        let date_lines = vec![
            Line::from(Span::styled(
                "Date Range (Future)",
                if section_index == 1 {
                    bold_highlight()
                } else {
                    label_text()
                },
            )),
            Line::from(Span::styled(
                "  Filter by last updated or created date",
                label_text(),
            )),
        ];

        let section2 =
            Paragraph::new(date_lines).block(Block::default().borders(Borders::ALL).border_style(
                if section_index == 1 {
                    focused_border()
                } else {
                    Style::default()
                },
            ));
        frame.render_widget(section2, chunks[1]);

        let tag_lines = vec![
            Line::from(Span::styled(
                "Tags (Future)",
                if section_index == 2 {
                    bold_highlight()
                } else {
                    label_text()
                },
            )),
            Line::from(Span::styled("  Filter cards by tags", label_text())),
        ];

        let section3 =
            Paragraph::new(tag_lines).block(Block::default().borders(Borders::ALL).border_style(
                if section_index == 2 {
                    focused_border()
                } else {
                    Style::default()
                },
            ));
        frame.render_widget(section3, chunks[2]);
    }
}

fn render_help_popup(app: &App, frame: &mut Frame) {
    use crate::keybindings::KeybindingRegistry;

    let area = centered_rect(80, 80, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Help - Keybindings for Current Context")
        .borders(Borders::ALL)
        .border_style(focused_border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(0)])
        .split(inner);

    // Get keybindings for the underlying mode
    let provider = KeybindingRegistry::get_provider(app);
    let context = provider.get_context();

    // Build lines for each keybinding
    let mut lines = vec![
        Line::from(Span::styled(
            context.name.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for binding in &context.bindings {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}", binding.key),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(" "),
            Span::styled(binding.description.clone(), normal_text()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press ESC or ? to close help",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    )));

    let content = Paragraph::new(lines);
    frame.render_widget(content, chunks[0]);
}
