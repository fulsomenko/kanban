use crate::app::{App, AppMode, Focus};
use crate::layout_strategy::VirtualUnifiedLayout;
use crate::view_strategy::UnifiedViewStrategy;
use kanban_domain::TaskListView;

impl App {
    /// Calculate stable page size that won't change during navigation
    /// Uses worst-case estimates to ensure pages don't shift as you scroll
    fn get_stable_page_size(&self) -> usize {
        const INDICATOR_OVERHEAD: usize = 2; // Worst case: both indicators present

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
                // Use worst-case header count: all columns could appear in viewport
                let column_boundaries = layout.get_column_boundaries();
                let header_overhead = column_boundaries.len(); // All columns visible in worst case

                // For stability, always assume both indicators are present (worst case)
                // This prevents page size from changing based on scroll position
                return self.viewport_height
                    .saturating_sub(header_overhead)
                    .saturating_sub(INDICATOR_OVERHEAD);
            }
        }

        // For flat view, always assume both indicators (worst case)
        self.viewport_height.saturating_sub(INDICATOR_OVERHEAD)
    }

    /// Calculate conservative page size accounting for position
    /// At boundaries (top/bottom), only 1 indicator appears; in middle, both appear
    fn get_conservative_page_size(&self) -> usize {
        let mut indicator_overhead = 2; // Assume both by default

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
                // Count headers that will appear in viewport at current scroll position
                let column_boundaries = layout.get_column_boundaries();
                let mut header_overhead = 0;
                if let Some(list) = self.view_strategy.get_active_task_list() {
                    let scroll_offset = list.get_scroll_offset();
                    let viewport_end = scroll_offset + self.viewport_height;

                    // Count column headers that overlap with viewport
                    for boundary in column_boundaries.iter() {
                        let boundary_end = boundary.start_index + boundary.card_count;
                        if boundary.start_index < viewport_end && boundary_end > scroll_offset {
                            header_overhead += 1;
                        }
                    }

                    // Check if at boundaries (where only 1 indicator appears)
                    if scroll_offset == 0 {
                        // At top: only "below" indicator
                        indicator_overhead = 1;
                    } else if scroll_offset + self.viewport_height >= list.len() {
                        // At bottom: only "above" indicator
                        indicator_overhead = 1;
                    }
                } else {
                    // Fallback: use a conservative estimate
                    header_overhead = column_boundaries.len().min(3);
                }

                return self.viewport_height
                    .saturating_sub(header_overhead)
                    .saturating_sub(indicator_overhead);
            }
        } else if let Some(list) = self.view_strategy.get_active_task_list() {
            // For flat view, check if at boundaries
            if list.get_scroll_offset() == 0 {
                // At top: only "below" indicator
                indicator_overhead = 1;
            } else if list.get_scroll_offset() + self.viewport_height >= list.len() {
                // At bottom: only "above" indicator
                indicator_overhead = 1;
            }
        }

        // For flat view, use indicator overhead based on position
        self.viewport_height.saturating_sub(indicator_overhead)
    }

    /// Calculate effective page size based on current scroll position
    /// This accounts for headers and indicators that actually appear now
    fn get_effective_page_size(&self) -> usize {
        let mut overhead = 0;

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

                    overhead += estimated_headers;

                    // Estimate scroll indicator lines based on current position
                    let space_after_headers =
                        self.viewport_height.saturating_sub(estimated_headers);
                    let has_items_above = list.get_scroll_offset() > 0;
                    let has_items_below = list.get_scroll_offset() + space_after_headers < list.len();
                    overhead += (has_items_above as usize) + (has_items_below as usize);

                    return self.viewport_height.saturating_sub(overhead);
                }
            }
        } else if let Some(list) = self.view_strategy.get_active_task_list() {
            // For flat view, estimate scroll indicator lines
            let has_items_above = list.get_scroll_offset() > 0;
            let has_items_below = list.get_scroll_offset() + self.viewport_height < list.len();
            overhead = (has_items_above as usize) + (has_items_below as usize);
        }

        self.viewport_height.saturating_sub(overhead)
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
                let hit_bottom = if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    list.navigate_down()
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
                let hit_top = if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    list.navigate_up()
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

                        let page_size = self.get_conservative_page_size();
                        if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                            if !list.is_empty() {
                                list.set_selected_index(Some(0));
                                list.ensure_selected_visible(page_size);
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

                let page_size = self.get_conservative_page_size();
                if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    if list.is_empty() {
                        list.clear();
                    } else if list.get_selected_index().is_none() {
                        list.set_selected_index(Some(0));
                    }
                    list.ensure_selected_visible(page_size);
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
                let page_size = self.get_stable_page_size();
                if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    list.jump_to_bottom(page_size);
                }
            }
        }
    }

    pub fn handle_jump_half_viewport_up(&mut self) {
        if self.focus == Focus::Cards {
            // Use conservative page size accounting for actual visible headers
            // This correctly estimates the number of card lines visible in a page
            let page_size = self.get_conservative_page_size();

            if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                list.jump_half_viewport_up(page_size);
            }
        }
    }

    pub fn handle_jump_half_viewport_down(&mut self) {
        if self.focus == Focus::Cards {
            // Use conservative page size accounting for actual visible headers
            // This correctly estimates the number of card lines visible in a page
            let page_size = self.get_conservative_page_size();

            if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                list.jump_half_viewport_down(page_size);
            }
        }
    }
}
