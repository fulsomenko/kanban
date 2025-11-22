use crate::app::{App, AppMode, Focus};
use crate::layout_strategy::VirtualUnifiedLayout;
use crate::view_strategy::UnifiedViewStrategy;
use kanban_domain::TaskListView;

impl App {
    fn get_effective_viewport_height(&self) -> usize {
        // Check if using grouped view (which has column headers)
        if let Some(unified) = self
            .view_strategy
            .as_any()
            .downcast_ref::<UnifiedViewStrategy>()
        {
            if let Some(layout) = unified
                .get_layout_strategy()
                .as_any()
                .downcast_ref::<VirtualUnifiedLayout>()
            {
                // Estimate headers that will appear in the viewport
                let column_boundaries = layout.get_column_boundaries();
                if let Some(list) = self.view_strategy.get_active_task_list() {
                    let estimated_headers = column_boundaries
                        .iter()
                        .filter(|b| {
                            let boundary_end = b.start_index + b.card_count;
                            let viewport_end = list.get_scroll_offset() + self.viewport_height;
                            b.start_index < viewport_end && boundary_end > list.get_scroll_offset()
                        })
                        .count();

                    return self.viewport_height.saturating_sub(estimated_headers);
                }
            }
        }

        self.viewport_height
    }

    fn get_column_boundaries(&self) -> Vec<crate::layout_strategy::ColumnBoundary> {
        if let Some(unified) = self
            .view_strategy
            .as_any()
            .downcast_ref::<UnifiedViewStrategy>()
        {
            if let Some(layout) = unified
                .get_layout_strategy()
                .as_any()
                .downcast_ref::<VirtualUnifiedLayout>()
            {
                return layout.get_column_boundaries().to_vec();
            }
        }
        Vec::new()
    }

    pub fn handle_focus_switch(&mut self, focus_target: Focus) {
        match focus_target {
            Focus::Boards => {
                self.focus = Focus::Boards;
            }
            Focus::Cards => {
                if self.active_board_index.is_some() {
                    self.focus = Focus::Cards;
                }
            }
        }
    }

    pub fn handle_navigation_down(&mut self) {
        match self.focus {
            Focus::Boards => {
                self.board_selection.next(self.boards.len());
                self.switch_view_strategy(TaskListView::GroupedByColumn);
            }
            Focus::Cards => {
                let effective_viewport = self.get_effective_viewport_height();
                let hit_bottom = if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    list.navigate_down(effective_viewport)
                } else {
                    false
                };

                if hit_bottom {
                    self.view_strategy.navigate_right(false);
                }
            }
        }
    }

    pub fn handle_navigation_up(&mut self) {
        match self.focus {
            Focus::Boards => {
                self.board_selection.prev();
                self.switch_view_strategy(TaskListView::GroupedByColumn);
            }
            Focus::Cards => {
                let effective_viewport = self.get_effective_viewport_height();
                let hit_top = if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    list.navigate_up(effective_viewport)
                } else {
                    false
                };

                if hit_top {
                    self.view_strategy.navigate_left(true);
                }
            }
        }
    }

    pub fn handle_selection_activate(&mut self) {
        match self.focus {
            Focus::Boards => {
                if self.board_selection.get().is_some() {
                    self.active_board_index = self.board_selection.get();

                    if let Some(board_idx) = self.active_board_index {
                        let (task_list_view, task_sort_field, task_sort_order) = {
                            if let Some(board) = self.boards.get(board_idx) {
                                (
                                    board.task_list_view,
                                    board.task_sort_field,
                                    board.task_sort_order,
                                )
                            } else {
                                (
                                    kanban_domain::TaskListView::Flat,
                                    kanban_domain::SortField::Default,
                                    kanban_domain::SortOrder::Ascending,
                                )
                            }
                        };

                        self.current_sort_field = Some(task_sort_field);
                        self.current_sort_order = Some(task_sort_order);
                        self.switch_view_strategy(task_list_view);

                        if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                            if !list.is_empty() {
                                list.set_selected_index(Some(0));
                                list.ensure_selected_visible(self.viewport_height);
                            }
                        }
                    }

                    self.focus = Focus::Cards;
                }
            }
            Focus::Cards => {
                if let Some(selected_card) = self.get_selected_card_in_context() {
                    let card_id = selected_card.id;
                    let actual_idx = self.cards.iter().position(|c| c.id == card_id);
                    self.active_card_index = actual_idx;
                    self.mode = AppMode::CardDetail;
                }
            }
        }
    }

    pub fn handle_escape_key(&mut self) {
        if self.active_board_index.is_some() {
            self.active_board_index = None;
            self.focus = Focus::Boards;

            self.switch_view_strategy(TaskListView::GroupedByColumn);
        }
    }

    pub fn is_kanban_view(&self) -> bool {
        if let Some(board_idx) = self.active_board_index.or(self.board_selection.get()) {
            if let Some(board) = self.boards.get(board_idx) {
                return board.task_list_view == TaskListView::ColumnView;
            }
        }
        false
    }

    pub fn handle_kanban_column_left(&mut self) {
        if !self.is_kanban_view() || self.focus != Focus::Cards {
            return;
        }

        if self.view_strategy.navigate_left(false) {
            tracing::info!("Moved to previous column");
        }
    }

    pub fn handle_kanban_column_right(&mut self) {
        if !self.is_kanban_view() || self.focus != Focus::Cards {
            return;
        }

        if self.view_strategy.navigate_right(false) {
            tracing::info!("Moved to next column");
        }
    }

    pub fn handle_column_or_focus_switch(&mut self, index: usize) {
        if self.is_kanban_view() && self.focus == Focus::Cards {
            let column_count = self.view_strategy.get_all_task_lists().len();

            if index < column_count {
                self.view_strategy
                    .as_any_mut()
                    .downcast_mut::<UnifiedViewStrategy>()
                    .map(|unified| unified.try_set_active_column_index(index));

                if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    if list.is_empty() {
                        list.clear();
                    } else if list.get_selected_index().is_none() {
                        list.set_selected_index(Some(0));
                    }
                    list.ensure_selected_visible(self.viewport_height);
                }
                tracing::info!("Switched to column {}", index);
            }
        } else {
            match index {
                0 => self.handle_focus_switch(Focus::Boards),
                1 => self.handle_focus_switch(Focus::Cards),
                _ => {}
            }
        }
    }

    pub fn handle_jump_to_top(&mut self) {
        match self.focus {
            Focus::Boards => {
                self.board_selection.jump_to_first();
                self.switch_view_strategy(TaskListView::GroupedByColumn);
            }
            Focus::Cards => {
                if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    list.jump_to_top();
                }
            }
        }
    }

    pub fn handle_jump_to_bottom(&mut self) {
        match self.focus {
            Focus::Boards => {
                self.board_selection.jump_to_last(self.boards.len());
                self.switch_view_strategy(TaskListView::GroupedByColumn);
            }
            Focus::Cards => {
                let effective_viewport = self.get_effective_viewport_height();
                if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    list.jump_to_bottom(effective_viewport);
                }
            }
        }
    }

    pub fn handle_jump_half_viewport_up(&mut self) {
        if self.focus == Focus::Cards {
            // Pass RAW viewport height, not adjusted - Page will handle header calculations
            let raw_viewport = self.viewport_height;
            let column_boundaries = self.get_column_boundaries();

            if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                list.set_column_boundaries(column_boundaries);
                list.jump_half_viewport_up(raw_viewport);
            }
        }
    }

    pub fn handle_jump_half_viewport_down(&mut self) {
        if self.focus == Focus::Cards {
            // Pass RAW viewport height, not adjusted - Page will handle header calculations
            let raw_viewport = self.viewport_height;
            let column_boundaries = self.get_column_boundaries();

            if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                list.set_column_boundaries(column_boundaries);
                list.jump_half_viewport_down(raw_viewport);
            }
        }
    }
}
