use crate::app::{App, AppMode, BoardFocus, CardFocus, Focus};
use kanban_domain::{CardStatus, Sprint, SprintStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(app: &App, frame: &mut Frame) {
    match app.mode {
        AppMode::CardDetail | AppMode::AssignCardToSprint => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            render_header(frame, chunks[0]);
            render_card_detail_view(app, frame, chunks[1]);
            render_footer(app, frame, chunks[2]);

            if app.mode == AppMode::AssignCardToSprint {
                render_assign_sprint_popup(app, frame);
            }
        }
        AppMode::BoardDetail => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            render_header(frame, chunks[0]);
            render_board_detail_view(app, frame, chunks[1]);
            render_footer(app, frame, chunks[2]);
        }
        AppMode::SprintDetail => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            render_header(frame, chunks[0]);
            render_sprint_detail_view(app, frame, chunks[1]);
            render_footer(app, frame, chunks[2]);
        }
        _ => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            render_header(frame, chunks[0]);
            render_main(app, frame, chunks[1]);
            render_footer(app, frame, chunks[2]);

            match app.mode {
                AppMode::CreateBoard => render_create_board_popup(app, frame),
                AppMode::CreateCard => render_create_card_popup(app, frame),
                AppMode::CreateSprint => render_create_sprint_popup(app, frame),
                AppMode::RenameBoard => render_rename_board_popup(app, frame),
                AppMode::ExportBoard => render_export_board_popup(app, frame),
                AppMode::ExportAll => render_export_all_popup(app, frame),
                AppMode::ImportBoard => render_import_board_popup(app, frame),
                AppMode::SetCardPoints => render_set_card_points_popup(app, frame),
                AppMode::SetBranchPrefix => render_set_branch_prefix_popup(app, frame),
                AppMode::OrderCards => render_order_cards_popup(app, frame),
                _ => {}
            }
        }
    }
}

fn render_header(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new("Kanban Board")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, area);
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
    let mut lines = vec![
        Line::from(Span::styled(
            "Projects",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("")),
    ];

    if app.boards.is_empty() {
        lines.push(Line::from(Span::styled(
            "No projects yet. Press 'n' to create one!",
            Style::default().fg(Color::Gray),
        )));
    } else {
        for (idx, board) in app.boards.iter().enumerate() {
            let is_selected = app.board_selection.get() == Some(idx);
            let is_active = app.active_board_index == Some(idx);
            let is_focused = app.focus == Focus::Boards;

            let mut style = Style::default();
            let prefix;

            if is_active {
                style = style.fg(Color::Green).add_modifier(Modifier::BOLD);
                prefix = "● ";
            } else {
                style = style.fg(Color::White);
                prefix = "  ";
            }

            if is_selected && is_focused {
                style = style.bg(Color::Blue);
            }

            lines.push(Line::from(Span::styled(
                format!("{}{}", prefix, board.name),
                style,
            )));
        }
    }

    let is_focused = app.focus == Focus::Boards;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::White
    };
    let title = if is_focused {
        "Projects [1]"
    } else {
        "Projects"
    };

    let content = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(title),
    );
    frame.render_widget(content, area);
}

fn render_tasks_panel(app: &App, frame: &mut Frame, area: Rect) {
    let board_idx = app.active_board_index.or(app.board_selection.get());

    let project_name = if let Some(idx) = board_idx {
        app.boards
            .get(idx)
            .map(|b| b.name.as_str())
            .unwrap_or("Unknown")
    } else {
        "No Project"
    };

    let mut lines = vec![
        Line::from(Span::styled(
            project_name.to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("")),
    ];

    if let Some(idx) = board_idx {
        if let Some(board) = app.boards.get(idx) {
            let board_cards = app.get_sorted_board_cards(board.id);

            if board_cards.is_empty() {
                let message = if app.active_board_index.is_some() {
                    "  No tasks yet. Press 'n' to create one!"
                } else {
                    "  (Enter/Space) to add tasks"
                };
                lines.push(Line::from(Span::styled(
                    message,
                    Style::default().fg(Color::Gray),
                )));
            } else {
                for (card_idx, card) in board_cards.iter().enumerate() {
                    let is_selected = app.card_selection.get() == Some(card_idx);
                    let is_focused = app.focus == Focus::Cards;
                    let is_done = card.status == CardStatus::Done;

                    let (checkbox, text_color, text_modifier) = if is_done {
                        ("☑", Color::DarkGray, Modifier::CROSSED_OUT)
                    } else {
                        ("☐", Color::White, Modifier::empty())
                    };

                    let mut style = Style::default().fg(text_color).add_modifier(text_modifier);

                    if is_selected && is_focused {
                        style = style.bg(Color::Blue);
                    }

                    let points_badge = if let Some(points) = card.points {
                        format!(" [{}]", points)
                    } else {
                        String::new()
                    };

                    let sprint_name = if let Some(sprint_id) = card.sprint_id {
                        app.sprints
                            .iter()
                            .find(|s| s.id == sprint_id)
                            .map(|s| {
                                format!(
                                    " ({})",
                                    s.formatted_name(
                                        board,
                                        board.sprint_prefix.as_deref().unwrap_or("sprint")
                                    )
                                )
                            })
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };

                    let mut spans = vec![
                        Span::styled(format!("  {} {}", checkbox, card.title), style)
                    ];

                    if !points_badge.is_empty() {
                        let points_color = card
                            .points
                            .map(|p| match p {
                                1 => Color::Cyan,
                                2 => Color::Green,
                                3 => Color::Yellow,
                                4 => Color::LightMagenta,
                                5 => Color::Red,
                                _ => Color::White,
                            })
                            .unwrap_or(Color::White);

                        let mut points_style = Style::default().fg(points_color);
                        if is_selected && is_focused {
                            points_style = points_style.bg(Color::Blue);
                        }
                        spans.push(Span::styled(points_badge, points_style));
                    }

                    if !sprint_name.is_empty() {
                        let mut sprint_style = Style::default().fg(Color::DarkGray);
                        if is_selected && is_focused {
                            sprint_style = sprint_style.bg(Color::Blue);
                        }
                        spans.push(Span::styled(sprint_name, sprint_style));
                    }

                    lines.push(Line::from(spans));
                }
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  Select a project to preview tasks",
            Style::default().fg(Color::Gray),
        )));
    }

    let is_focused = app.focus == Focus::Cards;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::White
    };

    let mut title = if is_focused {
        "Tasks [2]".to_string()
    } else {
        "Tasks".to_string()
    };

    if let Some(sprint_id) = app.active_sprint_filter {
        if let Some(sprint) = app.sprints.iter().find(|s| s.id == sprint_id) {
            if let Some(board_idx) = app.active_board_index.or(app.board_selection.get()) {
                if let Some(board) = app.boards.get(board_idx) {
                    let sprint_name = sprint.formatted_name(board, board.sprint_prefix.as_deref().unwrap_or("sprint"));
                    title.push_str(&format!(" - {}", sprint_name));
                }
            }
        }
    }

    let content = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(title),
    );
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

                    let status_text = format!("{:?}", sprint.status);
                    let status_color = match sprint.status {
                        SprintStatus::Planning => Color::Yellow,
                        SprintStatus::Active => Color::Green,
                        SprintStatus::Completed => Color::Blue,
                        SprintStatus::Cancelled => Color::Red,
                    };

                    let mut lines = vec![
                        Line::from(vec![
                            Span::styled("Sprint: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                sprint_name,
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("Status: ", Style::default().fg(Color::Gray)),
                            Span::styled(status_text, Style::default().fg(status_color)),
                        ]),
                        Line::from(vec![
                            Span::styled("Sprint Number: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                sprint.sprint_number.to_string(),
                                Style::default().fg(Color::White),
                            ),
                        ]),
                    ];

                    if let Some(name) = sprint.get_name(board) {
                        lines.push(Line::from(vec![
                            Span::styled("Name: ", Style::default().fg(Color::Gray)),
                            Span::styled(name.to_string(), Style::default().fg(Color::White)),
                        ]));
                    }

                    if let Some(start) = sprint.start_date {
                        lines.push(Line::from(vec![
                            Span::styled("Start Date: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                start.format("%Y-%m-%d %H:%M UTC").to_string(),
                                Style::default().fg(Color::White),
                            ),
                        ]));
                    }

                    if let Some(end) = sprint.end_date {
                        let end_color = if sprint.is_ended() {
                            Color::Red
                        } else {
                            Color::White
                        };
                        lines.push(Line::from(vec![
                            Span::styled("End Date: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                end.format("%Y-%m-%d %H:%M UTC").to_string(),
                                Style::default().fg(end_color),
                            ),
                        ]));
                    }

                    lines.push(Line::from(""));

                    let card_count = app.cards.iter().filter(|c| c.sprint_id == Some(sprint.id)).count();
                    lines.push(Line::from(vec![
                        Span::styled("Cards Assigned: ", Style::default().fg(Color::Gray)),
                        Span::styled(
                            card_count.to_string(),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                    ]));

                    let is_active = board.active_sprint_id == Some(sprint.id);
                    if is_active {
                        lines.push(Line::from(vec![
                            Span::styled("Active Sprint: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                "Yes (used for filtering)",
                                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                            ),
                        ]));
                    }

                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("Created: ", Style::default().fg(Color::Gray)),
                        Span::styled(
                            sprint.created_at.format("%Y-%m-%d %H:%M UTC").to_string(),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Updated: ", Style::default().fg(Color::Gray)),
                        Span::styled(
                            sprint.updated_at.format("%Y-%m-%d %H:%M UTC").to_string(),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));

                    let content = Paragraph::new(lines).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Cyan))
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
        AppMode::Normal => "q: quit | n: new | r: rename | e: edit project | x: export | X: export all | i: import | c: toggle complete | t: toggle sprint filter | 1/2: switch panel | j/k: navigate | Enter/Space: activate",
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
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, area);
}

fn render_create_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(app, frame, "Create New Project", "Project Name:");
}

fn render_create_card_popup(app: &App, frame: &mut Frame) {
    render_input_popup(app, frame, "Create New Task", "Task Title:");
}

fn render_create_sprint_popup(app: &App, frame: &mut Frame) {
    render_input_popup(app, frame, "Create New Sprint", "Sprint Name (optional):");
}

fn render_set_card_points_popup(app: &App, frame: &mut Frame) {
    render_input_popup(app, frame, "Set Points", "Points (1-5 or empty):");
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

                    let title_focused = app.card_focus == CardFocus::Title;
                    let title_border_color = if title_focused {
                        Color::Cyan
                    } else {
                        Color::White
                    };
                    let title_block = Block::default()
                        .title(if title_focused {
                            "Task Title [1]"
                        } else {
                            "Task Title"
                        })
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(title_border_color));
                    let title = Paragraph::new(card.title.clone())
                        .style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )
                        .block(title_block);
                    frame.render_widget(title, chunks[0]);

                    let meta_focused = app.card_focus == CardFocus::Metadata;
                    let meta_border_color = if meta_focused {
                        Color::Cyan
                    } else {
                        Color::White
                    };
                    let meta_block = Block::default()
                        .title(if meta_focused {
                            "Metadata [2]"
                        } else {
                            "Metadata"
                        })
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(meta_border_color));
                    let mut meta_lines = vec![
                        Line::from(vec![
                            Span::styled("Priority: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                format!("{:?}", card.priority),
                                Style::default().fg(Color::White),
                            ),
                            Span::raw("  "),
                            Span::styled("Status: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                format!("{:?}", card.status),
                                Style::default().fg(Color::White),
                            ),
                            Span::raw("  "),
                            Span::styled("Points: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                card.points
                                    .map(|p| p.to_string())
                                    .unwrap_or_else(|| "-".to_string()),
                                Style::default().fg(Color::White),
                            ),
                        ]),
                        Line::from(if let Some(due_date) = card.due_date {
                            vec![
                                Span::styled("Due: ", Style::default().fg(Color::Gray)),
                                Span::styled(
                                    due_date.format("%Y-%m-%d %H:%M").to_string(),
                                    Style::default().fg(Color::Red),
                                ),
                            ]
                        } else {
                            vec![Span::styled(
                                "No due date",
                                Style::default().fg(Color::Gray),
                            )]
                        }),
                    ];

                    let branch_name =
                        card.branch_name(board, &app.sprints, app.app_config.effective_default_prefix());
                    meta_lines.push(Line::from(vec![
                        Span::styled("Branch: ", Style::default().fg(Color::Gray)),
                        Span::styled(
                            branch_name,
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    let meta = Paragraph::new(meta_lines).block(meta_block);
                    frame.render_widget(meta, chunks[1]);

                    let desc_focused = app.card_focus == CardFocus::Description;
                    let desc_border_color = if desc_focused {
                        Color::Cyan
                    } else {
                        Color::White
                    };
                    let desc_block = Block::default()
                        .title(if desc_focused {
                            "Description [3]"
                        } else {
                            "Description"
                        })
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(desc_border_color));
                    let desc_text = if let Some(desc) = &card.description {
                        desc.clone()
                    } else {
                        "No description".to_string()
                    };
                    let desc = Paragraph::new(desc_text)
                        .style(Style::default().fg(Color::White))
                        .block(desc_block);
                    frame.render_widget(desc, chunks[2]);
                }
            }
        }
    }
}

fn render_rename_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(app, frame, "Rename Project", "Project Name:");
}

fn render_export_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(app, frame, "Export Board", "Filename:");
}

fn render_export_all_popup(app: &App, frame: &mut Frame) {
    render_input_popup(app, frame, "Export All Boards", "Filename:");
}

fn render_import_board_popup(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 50, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Import Board")
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
        Paragraph::new("Select a JSON file to import:").style(Style::default().fg(Color::Yellow));
    frame.render_widget(label, chunks[0]);

    if app.import_files.is_empty() {
        let empty_msg = Paragraph::new("No JSON files found in current directory")
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(empty_msg, chunks[1]);
    } else {
        let mut lines = vec![];
        for (idx, filename) in app.import_files.iter().enumerate() {
            let is_selected = app.import_selection.get() == Some(idx);
            let style = if is_selected {
                Style::default().fg(Color::White).bg(Color::Blue)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if is_selected { "> " } else { "  " };
            lines.push(Line::from(Span::styled(
                format!("{}{}", prefix, filename),
                style,
            )));
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

            let name_focused = app.board_focus == BoardFocus::Name;
            let name_border_color = if name_focused {
                Color::Cyan
            } else {
                Color::White
            };
            let name_block = Block::default()
                .title(if name_focused {
                    "Project Name [1]"
                } else {
                    "Project Name"
                })
                .borders(Borders::ALL)
                .border_style(Style::default().fg(name_border_color));
            let name = Paragraph::new(board.name.clone())
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .block(name_block);
            frame.render_widget(name, chunks[0]);

            let desc_focused = app.board_focus == BoardFocus::Description;
            let desc_border_color = if desc_focused {
                Color::Cyan
            } else {
                Color::White
            };
            let desc_block = Block::default()
                .title(if desc_focused {
                    "Description [2]"
                } else {
                    "Description"
                })
                .borders(Borders::ALL)
                .border_style(Style::default().fg(desc_border_color));
            let desc_text = if let Some(desc) = &board.description {
                desc.clone()
            } else {
                "No description".to_string()
            };
            let desc = Paragraph::new(desc_text)
                .style(Style::default().fg(Color::White))
                .block(desc_block);
            frame.render_widget(desc, chunks[1]);

            let settings_focused = app.board_focus == BoardFocus::Settings;
            let settings_border_color = if settings_focused {
                Color::Cyan
            } else {
                Color::White
            };
            let settings_block = Block::default()
                .title(if settings_focused {
                    "Settings [3]"
                } else {
                    "Settings"
                })
                .borders(Borders::ALL)
                .border_style(Style::default().fg(settings_border_color));

            let mut settings_lines = vec![
                Line::from(if let Some(prefix) = &board.branch_prefix {
                    vec![
                        Span::styled("Branch Prefix: ", Style::default().fg(Color::Gray)),
                        Span::styled(prefix.clone(), Style::default().fg(Color::Green)),
                    ]
                } else {
                    vec![
                        Span::styled("Branch Prefix: ", Style::default().fg(Color::Gray)),
                        Span::styled(
                            app.app_config.effective_default_prefix().to_string(),
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(" (default)", Style::default().fg(Color::DarkGray)),
                    ]
                }),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Sprint Duration: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        board
                            .sprint_duration_days
                            .map(|d| format!("{} days", d))
                            .unwrap_or_else(|| "(not set)".to_string()),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Sprint Prefix: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        board.sprint_prefix.as_deref().unwrap_or("(not set)"),
                        Style::default().fg(Color::White),
                    ),
                ]),
            ];

            let available_names: Vec<&str> = board.sprint_names
                .iter()
                .skip(board.sprint_name_used_count)
                .map(|s| s.as_str())
                .collect();

            if !available_names.is_empty() {
                settings_lines.push(Line::from(vec![
                    Span::styled("Sprint Names: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        available_names.join(", "),
                        Style::default().fg(Color::White),
                    ),
                ]));
            }

            let settings = Paragraph::new(settings_lines).block(settings_block);
            frame.render_widget(settings, chunks[2]);

            let sprints_focused = app.board_focus == BoardFocus::Sprints;
            let sprints_border_color = if sprints_focused {
                Color::Cyan
            } else {
                Color::White
            };
            let sprints_title = if sprints_focused {
                "Sprints [4]"
            } else {
                "Sprints"
            };

            let board_sprints: Vec<&Sprint> = app
                .sprints
                .iter()
                .filter(|s| s.board_id == board.id)
                .collect();

            let mut sprint_lines = vec![];

            if board_sprints.is_empty() {
                sprint_lines.push(Line::from(Span::styled(
                    "  No sprints yet. Press 'n' to create one!",
                    Style::default().fg(Color::Gray),
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

                    let status_color = match sprint.status {
                        SprintStatus::Planning => Color::Yellow,
                        SprintStatus::Active => Color::Green,
                        SprintStatus::Completed => Color::Blue,
                        SprintStatus::Cancelled => Color::Red,
                    };

                    let sprint_name = sprint.formatted_name(
                        board,
                        board.sprint_prefix.as_deref().unwrap_or("sprint"),
                    );

                    let card_count = app.cards.iter().filter(|c| c.sprint_id == Some(sprint.id)).count();

                    let is_active_sprint = board.active_sprint_id == Some(sprint.id);
                    let is_ended = sprint.is_ended();

                    let mut style = Style::default().fg(Color::White);

                    if is_selected && is_focused {
                        style = style.bg(Color::Blue);
                    }

                    let mut spans = vec![
                        Span::styled(format!("{} ", status_symbol), Style::default().fg(status_color)),
                        Span::styled(sprint_name, style),
                        Span::styled(format!(" ({})", card_count), Style::default().fg(Color::DarkGray)),
                    ];

                    if is_active_sprint {
                        let mut active_style = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
                        if is_selected && is_focused {
                            active_style = active_style.bg(Color::Blue);
                        }
                        spans.push(Span::styled(" Active", active_style));
                    }

                    if is_ended {
                        let mut ended_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
                        if is_selected && is_focused {
                            ended_style = ended_style.bg(Color::Blue);
                        }
                        spans.push(Span::styled(" Ended", ended_style));
                    }

                    sprint_lines.push(Line::from(spans));
                }
            }

            let sprints_block = Block::default()
                .title(sprints_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(sprints_border_color));
            let sprints = Paragraph::new(sprint_lines).block(sprints_block);
            frame.render_widget(sprints, chunks[3]);
        }
    }
}


fn render_set_branch_prefix_popup(app: &App, frame: &mut Frame) {
    render_input_popup(app, frame, "Set Branch Prefix", "Prefix (empty to clear):");
}

fn render_order_cards_popup(app: &App, frame: &mut Frame) {
    use kanban_domain::{SortField, SortOrder};

    let area = centered_rect(60, 50, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Order Tasks By")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let label = Paragraph::new("Select sort field:").style(Style::default().fg(Color::Yellow));
    frame.render_widget(label, chunks[0]);

    let sort_fields = [
        SortField::Points,
        SortField::Priority,
        SortField::CreatedAt,
        SortField::UpdatedAt,
        SortField::Status,
        SortField::Default,
    ];

    let mut lines = vec![];
    for (idx, field) in sort_fields.iter().enumerate() {
        let is_selected = app.sort_field_selection.get() == Some(idx);
        let is_active = app.current_sort_field == Some(*field);

        let style = if is_selected {
            Style::default().fg(Color::White).bg(Color::Blue)
        } else if is_active {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let prefix = if is_selected { "> " } else { "  " };
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
                Some(SortOrder::Ascending) => " (↑)",
                Some(SortOrder::Descending) => " (↓)",
                None => "",
            }
        } else {
            ""
        };

        lines.push(Line::from(Span::styled(
            format!("{}{}{}", prefix, field_name, order_indicator),
            style,
        )));
    }
    let list = Paragraph::new(lines);
    frame.render_widget(list, chunks[1]);
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

fn render_input_popup(app: &App, frame: &mut Frame, title: &str, label: &str) {
    let area = centered_rect(60, 30, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(inner);

    let label_widget = Paragraph::new(label).style(Style::default().fg(Color::Yellow));
    frame.render_widget(label_widget, chunks[0]);

    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(input, chunks[1]);

    let cursor_x = chunks[1].x + app.input.cursor_pos() as u16 + 1;
    let cursor_y = chunks[1].y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
