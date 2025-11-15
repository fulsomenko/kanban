use crate::app::App;
use crate::components::{
    card_list_item::{render_card_list_item, CardListItemConfig},
    PanelConfig,
};
use crate::theme::label_text;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub trait RenderStrategy {
    fn render(&self, app: &App, frame: &mut Frame, area: Rect);
}

pub struct SinglePanelRenderer {
    show_column_headers: bool,
}

impl SinglePanelRenderer {
    pub fn new(show_column_headers: bool) -> Self {
        Self {
            show_column_headers,
        }
    }

    pub fn flat() -> Self {
        Self::new(false)
    }

    pub fn grouped() -> Self {
        Self::new(true)
    }
}

impl RenderStrategy for SinglePanelRenderer {
    fn render(&self, app: &App, frame: &mut Frame, area: Rect) {
        let board_idx = app.active_board_index.or(app.board_selection.get());

        let mut lines = vec![];

        if let Some(idx) = board_idx {
            if let Some(board) = app.boards.get(idx) {
                let task_lists = app.view_strategy.get_all_task_lists();
                let active_task_list = app.view_strategy.get_active_task_list();

                if self.show_column_headers {
                    let mut board_columns: Vec<_> = app
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .collect();
                    board_columns.sort_by_key(|col| col.position);

                    if board_columns.is_empty() {
                        lines.push(Line::from(Span::styled(
                            "  No columns yet. Add columns in board settings.",
                            label_text(),
                        )));
                    } else if task_lists.is_empty() {
                        let message = if app.active_board_index.is_some() {
                            "  No tasks yet. Press 'n' to create one!"
                        } else {
                            "  (Enter/Space) to add tasks"
                        };
                        lines.push(Line::from(Span::styled(message, label_text())));
                    } else {
                        for (col_idx, task_list) in task_lists.iter().enumerate() {
                            if let Some(column) = board_columns.get(col_idx) {
                                let card_count = task_list.len();
                                let is_active_column = active_task_list
                                    .map(|active| std::ptr::eq(*task_list, active))
                                    .unwrap_or(false);

                                lines.push(Line::from(Span::styled(
                                    format!("── {} ({}) ──", column.name, card_count),
                                    ratatui::style::Style::default()
                                        .fg(ratatui::style::Color::Cyan)
                                        .add_modifier(ratatui::style::Modifier::BOLD),
                                )));

                                if task_list.is_empty() {
                                    lines.push(Line::from(Span::styled(
                                        "  (no tasks)",
                                        label_text(),
                                    )));
                                } else {
                                    let viewport_height = (area.height as usize).saturating_sub(2);
                                    let render_info = task_list.get_render_info(viewport_height);

                                    if render_info.show_above_indicator {
                                        let count = render_info.cards_above_count;
                                        let plural = if count == 1 { "" } else { "s" };
                                        lines.push(Line::from(Span::styled(
                                            format!("  {} Task{} above", count, plural),
                                            ratatui::style::Style::default()
                                                .fg(ratatui::style::Color::DarkGray),
                                        )));
                                    }

                                    for card_idx in &render_info.visible_card_indices {
                                        if let Some(card_id) = task_list.cards.get(*card_idx) {
                                            if let Some(card) =
                                                app.cards.iter().find(|c| c.id == *card_id)
                                            {
                                                let is_selected = if is_active_column {
                                                    task_list.get_selected_index()
                                                        == Some(*card_idx)
                                                } else {
                                                    false
                                                };

                                                let line =
                                                    render_card_list_item(CardListItemConfig {
                                                        card,
                                                        board,
                                                        sprints: &app.sprints,
                                                        is_selected,
                                                        is_focused: app.focus
                                                            == crate::app::Focus::Cards
                                                            && is_active_column,
                                                        is_multi_selected: app
                                                            .selected_cards
                                                            .contains(&card.id),
                                                        show_sprint_name: app
                                                            .active_sprint_filters
                                                            .is_empty(),
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
                                            ratatui::style::Style::default()
                                                .fg(ratatui::style::Color::DarkGray),
                                        )));
                                    }
                                }

                                if col_idx < task_lists.len() - 1 {
                                    lines.push(Line::from(""));
                                }
                            }
                        }
                    }
                } else if let Some(task_list) = app.view_strategy.get_active_task_list() {
                    if task_list.is_empty() {
                        let message = if app.active_board_index.is_some() {
                            "  No tasks yet. Press 'n' to create one!"
                        } else {
                            "  (Enter/Space) to add tasks"
                        };
                        lines.push(Line::from(Span::styled(message, label_text())));
                    } else {
                        let viewport_height = area.height.saturating_sub(2) as usize;
                        let render_info = task_list.get_render_info(viewport_height);

                        if render_info.show_above_indicator {
                            let count = render_info.cards_above_count;
                            let plural = if count == 1 { "" } else { "s" };
                            lines.push(Line::from(Span::styled(
                                format!("  {} Task{} above", count, plural),
                                ratatui::style::Style::default()
                                    .fg(ratatui::style::Color::DarkGray),
                            )));
                        }

                        for card_idx in &render_info.visible_card_indices {
                            if let Some(card_id) = task_list.cards.get(*card_idx) {
                                if let Some(card) = app.cards.iter().find(|c| c.id == *card_id) {
                                    let line = render_card_list_item(CardListItemConfig {
                                        card,
                                        board,
                                        sprints: &app.sprints,
                                        is_selected: task_list.get_selected_index()
                                            == Some(*card_idx),
                                        is_focused: app.focus == crate::app::Focus::Cards,
                                        is_multi_selected: app.selected_cards.contains(&card.id),
                                        show_sprint_name: app.active_sprint_filters.is_empty(),
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
                                ratatui::style::Style::default()
                                    .fg(ratatui::style::Color::DarkGray),
                            )));
                        }
                    }
                }
            }
        } else {
            lines.push(Line::from(Span::styled(
                "  Select a project to preview tasks",
                label_text(),
            )));
        }

        let title = crate::ui::build_tasks_panel_title(app, true);

        let panel_config = PanelConfig::new(&title)
            .with_focus_indicator(&title)
            .focused(app.focus == crate::app::Focus::Cards);

        let content = Paragraph::new(lines).block(panel_config.block());
        frame.render_widget(content, area);
    }
}

pub struct MultiPanelRenderer;

impl RenderStrategy for MultiPanelRenderer {
    fn render(&self, app: &App, frame: &mut Frame, area: Rect) {
        let board_idx = app.active_board_index.or(app.board_selection.get());

        if let Some(idx) = board_idx {
            if let Some(board) = app.boards.get(idx) {
                let task_lists = app.view_strategy.get_all_task_lists();

                if task_lists.is_empty() {
                    let lines = vec![Line::from(Span::styled(
                        "  No columns yet. Add columns in board settings.",
                        label_text(),
                    ))];

                    let panel_config = PanelConfig::new("Tasks")
                        .with_focus_indicator("Tasks [2]")
                        .focused(app.focus == crate::app::Focus::Cards);

                    let content = Paragraph::new(lines).block(panel_config.block());
                    frame.render_widget(content, area);
                    return;
                }

                let sprint_filter_suffix = crate::ui::build_filter_title_suffix(app);

                let column_count = task_lists.len();
                let column_width = 100 / column_count as u16;

                let mut constraints = vec![];
                for _ in 0..column_count {
                    constraints.push(Constraint::Percentage(column_width));
                }

                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(area);

                let active_task_list = app.view_strategy.get_active_task_list();

                for (col_idx, task_list) in task_lists.iter().enumerate() {
                    let mut lines = vec![];

                    let card_count = task_list.len();
                    let is_focused_column = active_task_list
                        .map(|active| std::ptr::eq(*task_list, active))
                        .unwrap_or(false);

                    if task_list.is_empty() {
                        lines.push(Line::from(Span::styled("  (no tasks)", label_text())));
                    } else {
                        let viewport_height = chunks[col_idx].height.saturating_sub(2) as usize;
                        let render_info = task_list.get_render_info(viewport_height);

                        if render_info.show_above_indicator {
                            let count = render_info.cards_above_count;
                            let plural = if count == 1 { "" } else { "s" };
                            lines.push(Line::from(Span::styled(
                                format!("  {} Task{} above", count, plural),
                                ratatui::style::Style::default()
                                    .fg(ratatui::style::Color::DarkGray),
                            )));
                        }

                        for card_idx in &render_info.visible_card_indices {
                            if let Some(card_id) = task_list.cards.get(*card_idx) {
                                if let Some(card) = app.cards.iter().find(|c| c.id == *card_id) {
                                    let is_selected = if is_focused_column {
                                        task_list.get_selected_index() == Some(*card_idx)
                                    } else {
                                        false
                                    };

                                    let line = render_card_list_item(CardListItemConfig {
                                        card,
                                        board,
                                        sprints: &app.sprints,
                                        is_selected,
                                        is_focused: app.focus == crate::app::Focus::Cards
                                            && is_focused_column,
                                        is_multi_selected: app.selected_cards.contains(&card.id),
                                        show_sprint_name: app.active_sprint_filters.is_empty(),
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
                                ratatui::style::Style::default()
                                    .fg(ratatui::style::Color::DarkGray),
                            )));
                        }
                    }

                    let column_name =
                        if let crate::card_list::CardListId::Column(column_id) = task_list.id {
                            app.columns
                                .iter()
                                .find(|c| c.id == column_id)
                                .map(|c| c.name.clone())
                                .unwrap_or_else(|| "Unknown".to_string())
                        } else {
                            "All".to_string()
                        };

                    let mut title = if col_idx < 9 {
                        format!("{} ({}) [{}]", column_name, card_count, col_idx + 1)
                    } else {
                        format!("{} ({})", column_name, card_count)
                    };

                    if col_idx == 0 {
                        if let Some(ref suffix) = sprint_filter_suffix {
                            title.push_str(suffix);
                        }
                    }

                    let panel_config = PanelConfig::new(&title)
                        .with_focus_indicator(&title)
                        .focused(app.focus == crate::app::Focus::Cards && is_focused_column);

                    let content = Paragraph::new(lines).block(panel_config.block());
                    frame.render_widget(content, chunks[col_idx]);
                }
            }
        } else {
            let lines = vec![Line::from(Span::styled(
                "  Select a project to preview tasks",
                label_text(),
            ))];

            let panel_config = PanelConfig::new("Tasks")
                .with_focus_indicator("Tasks [2]")
                .focused(app.focus == crate::app::Focus::Cards);

            let content = Paragraph::new(lines).block(panel_config.block());
            frame.render_widget(content, area);
        }
    }
}
