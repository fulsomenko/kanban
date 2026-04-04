use crate::app::{App, AppMode, Focus};
use crate::components::*;
use crate::theme::*;
use crate::view_strategy::UnifiedViewStrategy;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub(super) fn render_main(app: &mut App, frame: &mut Frame, area: Rect) {
    let is_kanban_view = if let Some(idx) = app.selection.active_board_index {
        if let Some(board) = app.ctx.boards.get(idx) {
            board.task_list_view == kanban_domain::TaskListView::ColumnView
        } else {
            false
        }
    } else {
        false
    };

    if is_kanban_view {
        app.view.viewport_height = area.height.saturating_sub(2) as usize;
        render_tasks_panel(app, frame, area);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        app.view.viewport_height = chunks[1].height.saturating_sub(2) as usize;
        render_projects_panel(app, frame, chunks[0]);
        render_tasks_panel(app, frame, chunks[1]);
    }
}

pub(super) fn render_projects_panel(app: &App, frame: &mut Frame, area: Rect) {
    let mut lines = vec![];

    if app.ctx.boards.is_empty() {
        lines.push(Line::from(Span::styled(
            "No projects yet. Press 'n' to create one!",
            label_text(),
        )));
    } else {
        for (idx, board) in app.ctx.boards.iter().enumerate() {
            let config = ListItemConfig::new()
                .selected(app.selection.board.get() == Some(idx))
                .focused(app.focus.active == Focus::Boards)
                .active(app.selection.active_board_index == Some(idx));

            lines.push(styled_list_item(&board.name, &config));
        }
    }

    let panel_config = PanelConfig::new("Projects")
        .with_focus_indicator("Projects [1]")
        .focused(app.focus.active == Focus::Boards);

    let content = Paragraph::new(lines);
    render_panel(frame, area, &panel_config, content);
}

pub fn build_filter_title_suffix(app: &App) -> Option<String> {
    let mut filters = vec![];

    if app.filter.hide_assigned_cards {
        filters.push("Unassigned Cards".to_string());
    }

    if !app.filter.active_sprint_filters.is_empty() {
        if let Some(board_idx) = app
            .selection
            .active_board_index
            .or(app.selection.board.get())
        {
            if let Some(board) = app.ctx.boards.get(board_idx) {
                let mut sprint_names: Vec<String> = app
                    .ctx
                    .sprints
                    .iter()
                    .filter(|s| app.filter.active_sprint_filters.contains(&s.id))
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
    } else if app.focus.active == Focus::Cards {
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

pub(super) fn render_tasks_panel(app: &App, frame: &mut Frame, area: Rect) {
    render_tasks(app, frame, area);
}

pub(super) fn render_tasks(app: &App, frame: &mut Frame, area: Rect) {
    if let Some(unified_strategy) = app
        .view
        .strategy
        .as_any()
        .downcast_ref::<UnifiedViewStrategy>()
    {
        unified_strategy
            .get_render_strategy()
            .render(app, frame, area);
    }
}
