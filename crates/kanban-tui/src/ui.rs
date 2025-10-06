use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Paragraph, Clear}};
use crate::app::{App, AppMode, Focus};

pub fn render(app: &App, frame: &mut Frame) {
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

    if app.mode == AppMode::CreateProject {
        render_create_project_popup(app, frame);
    } else if app.mode == AppMode::CreateTask {
        render_create_task_popup(app, frame);
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

    if app.projects.is_empty() {
        lines.push(Line::from(Span::styled(
            "No projects yet. Press 'n' to create one!",
            Style::default().fg(Color::Gray),
        )));
    } else {
        for (idx, project) in app.projects.iter().enumerate() {
            let is_selected = app.project_selection.get() == Some(idx);
            let is_active = app.active_project_index == Some(idx);

            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if is_selected {
                "▶ "
            } else if is_active {
                "● "
            } else {
                "  "
            };

            lines.push(Line::from(Span::styled(
                format!("{}{}", prefix, project.name),
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
    let mut lines = vec![
        Line::from(Span::styled(
            "Tasks",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("")),
    ];

    if let Some(idx) = app.active_project_index {
        if let Some(project) = app.projects.get(idx) {
            let project_tasks: Vec<_> = app.cards.iter()
                .filter(|card| {
                    app.columns.iter()
                        .any(|col| col.id == card.column_id && col.board_id == project.id)
                })
                .collect();

            if project_tasks.is_empty() {
                lines.push(Line::from(Span::styled(
                    "No tasks yet. Press 'n' to create one!",
                    Style::default().fg(Color::Gray),
                )));
            } else {
                for (task_idx, task) in project_tasks.iter().enumerate() {
                    let is_selected = app.task_selection.get() == Some(task_idx);
                    let style = if is_selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let prefix = if is_selected { "▶ " } else { "  " };
                    lines.push(Line::from(Span::styled(
                        format!("{}☐ {}", prefix, task.title),
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
        AppMode::Normal => "q: quit | n: new | 1/2: switch panel | j/k: navigate | Enter/Space: activate project",
        AppMode::CreateProject => "ESC: cancel | ENTER: confirm",
        AppMode::CreateTask => "ESC: cancel | ENTER: confirm",
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
