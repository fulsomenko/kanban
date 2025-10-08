use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Paragraph, Clear}};
use crate::app::{App, AppMode, Focus, TaskFocus, BoardFocus};

pub fn render(app: &App, frame: &mut Frame) {
    match app.mode {
        AppMode::TaskDetail => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            render_header(frame, chunks[0]);
            render_task_detail_view(app, frame, chunks[1]);
            render_footer(app, frame, chunks[2]);
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
                AppMode::CreateProject => render_create_project_popup(app, frame),
                AppMode::CreateTask => render_create_task_popup(app, frame),
                AppMode::RenameProject => render_rename_project_popup(app, frame),
                _ => {}
            }
        }
    }
}

fn render_header(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new("Kanban Board")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
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
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
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
            let is_focused = app.focus == Focus::Projects;

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

    let is_focused = app.focus == Focus::Projects;
    let border_color = if is_focused { Color::Cyan } else { Color::White };
    let title = if is_focused { "Projects [1]" } else { "Projects" };

    let content = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(title));
    frame.render_widget(content, area);
}

fn render_tasks_panel(app: &App, frame: &mut Frame, area: Rect) {
    let project_name = if let Some(board_idx) = app.active_board_index {
        app.boards.get(board_idx).map(|b| b.name.as_str()).unwrap_or("Unknown")
    } else {
        "No Project"
    };

    let mut lines = vec![
        Line::from(Span::styled(
            format!("{}", project_name),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("")),
    ];

    if let Some(idx) = app.active_board_index {
        if let Some(board) = app.boards.get(idx) {
            let board_tasks: Vec<_> = app.cards.iter()
                .filter(|card| {
                    app.columns.iter()
                        .any(|col| col.id == card.column_id && col.board_id == board.id)
                })
                .collect();

            if board_tasks.is_empty() {
                lines.push(Line::from(Span::styled(
                    "No tasks yet. Press 'n' to create one!",
                    Style::default().fg(Color::Gray),
                )));
            } else {
                for (task_idx, task) in board_tasks.iter().enumerate() {
                    let is_selected = app.task_selection.get() == Some(task_idx);
                    let is_focused = app.focus == Focus::Tasks;
                    let style = if is_selected && is_focused {
                        Style::default().fg(Color::White).bg(Color::Blue)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("  ☐ {}", task.title),
                        style,
                    )));
                }
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Activate a project (Enter/Space) to view tasks",
            Style::default().fg(Color::Gray),
        )));
    }

    let is_focused = app.focus == Focus::Tasks;
    let border_color = if is_focused { Color::Cyan } else { Color::White };
    let title = if is_focused { "Tasks [2]" } else { "Tasks" };

    let content = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(title));
    frame.render_widget(content, area);
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let help_text = match app.mode {
        AppMode::Normal => "q: quit | n: new | r: rename | e: edit board | 1/2: switch panel | j/k: navigate | Enter/Space: activate",
        AppMode::CreateProject => "ESC: cancel | ENTER: confirm",
        AppMode::CreateTask => "ESC: cancel | ENTER: confirm",
        AppMode::RenameProject => "ESC: cancel | ENTER: confirm",
        AppMode::TaskDetail => match app.task_focus {
            TaskFocus::Title => "q: quit | ESC: back | 1/2/3: select panel | e: edit title",
            TaskFocus::Description => "q: quit | ESC: back | 1/2/3: select panel | e: edit description",
            _ => "q: quit | ESC: back | 1/2/3: select panel",
        },
        AppMode::BoardDetail => match app.board_focus {
            BoardFocus::Name => "q: quit | ESC: back | 1/2: select panel | e: edit name",
            BoardFocus::Description => "q: quit | ESC: back | 1/2: select panel | e: edit description",
        },
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, area);
}

fn render_create_project_popup(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 30, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Create New Project")
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

    let label = Paragraph::new("Project Name:")
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(label, chunks[0]);

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(input, chunks[1]);

    let cursor_x = chunks[1].x + app.input_buffer.len() as u16 + 1;
    let cursor_y = chunks[1].y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn render_create_task_popup(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 30, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Create New Task")
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

    let label = Paragraph::new("Task Title:")
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(label, chunks[0]);

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(input, chunks[1]);

    let cursor_x = chunks[1].x + app.input_buffer.len() as u16 + 1;
    let cursor_y = chunks[1].y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn render_task_detail_view(app: &App, frame: &mut Frame, area: Rect) {
    if let Some(task_idx) = app.active_task_index {
        if let Some(board_idx) = app.active_board_index {
            if let Some(board) = app.boards.get(board_idx) {
                let board_tasks: Vec<_> = app.cards.iter()
                    .filter(|card| {
                        app.columns.iter()
                            .any(|col| col.id == card.column_id && col.board_id == board.id)
                    })
                    .collect();

                if let Some(task) = board_tasks.get(task_idx) {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(5),
                            Constraint::Length(5),
                            Constraint::Min(0),
                        ])
                        .split(area);

                    let title_focused = app.task_focus == TaskFocus::Title;
                    let title_border_color = if title_focused { Color::Cyan } else { Color::White };
                    let title_block = Block::default()
                        .title(if title_focused { "Task Title [1]" } else { "Task Title" })
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(title_border_color));
                    let title = Paragraph::new(task.title.clone())
                        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                        .block(title_block);
                    frame.render_widget(title, chunks[0]);

                    let meta_focused = app.task_focus == TaskFocus::Metadata;
                    let meta_border_color = if meta_focused { Color::Cyan } else { Color::White };
                    let meta_block = Block::default()
                        .title(if meta_focused { "Metadata [2]" } else { "Metadata" })
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(meta_border_color));
                    let meta_lines = vec![
                        Line::from(vec![
                            Span::styled("Priority: ", Style::default().fg(Color::Gray)),
                            Span::styled(format!("{:?}", task.priority), Style::default().fg(Color::White)),
                            Span::raw("  "),
                            Span::styled("Status: ", Style::default().fg(Color::Gray)),
                            Span::styled(format!("{:?}", task.status), Style::default().fg(Color::White)),
                        ]),
                        Line::from(if let Some(due_date) = task.due_date {
                            vec![
                                Span::styled("Due: ", Style::default().fg(Color::Gray)),
                                Span::styled(due_date.format("%Y-%m-%d %H:%M").to_string(), Style::default().fg(Color::Red)),
                            ]
                        } else {
                            vec![Span::styled("No due date", Style::default().fg(Color::Gray))]
                        }),
                    ];
                    let meta = Paragraph::new(meta_lines).block(meta_block);
                    frame.render_widget(meta, chunks[1]);

                    let desc_focused = app.task_focus == TaskFocus::Description;
                    let desc_border_color = if desc_focused { Color::Cyan } else { Color::White };
                    let desc_block = Block::default()
                        .title(if desc_focused { "Description [3]" } else { "Description" })
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(desc_border_color));
                    let desc_text = if let Some(desc) = &task.description {
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

fn render_rename_project_popup(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 30, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Rename Project")
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

    let label = Paragraph::new("Project Name:")
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(label, chunks[0]);

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(input, chunks[1]);

    let cursor_x = chunks[1].x + app.input_buffer.len() as u16 + 1;
    let cursor_y = chunks[1].y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn render_board_detail_view(app: &App, frame: &mut Frame, area: Rect) {
    if let Some(board_idx) = app.board_selection.get() {
        if let Some(board) = app.boards.get(board_idx) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5),
                    Constraint::Min(0),
                ])
                .split(area);

            let name_focused = app.board_focus == BoardFocus::Name;
            let name_border_color = if name_focused { Color::Cyan } else { Color::White };
            let name_block = Block::default()
                .title(if name_focused { "Board Name [1]" } else { "Board Name" })
                .borders(Borders::ALL)
                .border_style(Style::default().fg(name_border_color));
            let name = Paragraph::new(board.name.clone())
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .block(name_block);
            frame.render_widget(name, chunks[0]);

            let desc_focused = app.board_focus == BoardFocus::Description;
            let desc_border_color = if desc_focused { Color::Cyan } else { Color::White };
            let desc_block = Block::default()
                .title(if desc_focused { "Description [2]" } else { "Description" })
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
        }
    }
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
