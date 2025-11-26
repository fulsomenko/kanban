use crate::app::{App, AppMode, Focus};
use crate::view_strategy::UnifiedViewStrategy;
use kanban_domain::TaskListView;

/// Page boundary with start index and end index (exclusive)
#[derive(Debug, Clone, Copy)]
struct PageBoundary {
    start: usize,
    end: usize,
}

impl PageBoundary {
    fn contains(&self, index: usize) -> bool {
        index >= self.start && index < self.end
    }

    fn size(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

/// Compute variable-sized page boundaries for jumping navigation.
///
/// Pages are sized based on position:
/// - First page: viewport_height - 1 (only "below" indicator)
/// - Middle pages: viewport_height - 2 (both indicators)
/// - Last page: remaining items (only "above" indicator)
fn compute_page_boundaries(total_items: usize, viewport_height: usize) -> Vec<PageBoundary> {
    if total_items == 0 || viewport_height == 0 {
        return vec![];
    }

    let mut pages = Vec::new();

    // First page: viewport_height - 1 items
    let first_page_size = viewport_height.saturating_sub(1).max(1);
    let first_page_end = first_page_size.min(total_items);
    pages.push(PageBoundary {
        start: 0,
        end: first_page_end,
    });
    let mut current_idx = first_page_end;

    // Middle pages: viewport_height - 2 items each
    let middle_page_size = viewport_height.saturating_sub(2).max(1);
    while current_idx < total_items {
        let page_end = (current_idx + middle_page_size).min(total_items);
        pages.push(PageBoundary {
            start: current_idx,
            end: page_end,
        });
        current_idx = page_end;
    }

    pages
}

/// Find which page contains the given index
fn find_current_page(pages: &[PageBoundary], index: usize) -> Option<usize> {
    pages.iter().position(|page| page.contains(index))
}

/// Calculate jump target for page-down navigation (Ctrl+D style)
/// Implements three-level navigation: top → middle → bottom → next page
fn calculate_jump_target_down(
    pages: &[PageBoundary],
    current_index: usize,
    current_page_idx: usize,
) -> usize {
    let current_page = pages[current_page_idx];

    // Determine position within current page
    let position_in_page = current_index - current_page.start;
    let page_size = current_page.size();

    if page_size == 0 {
        return current_index;
    }

    // Three-level jumping: top (0) → middle (size/2) → bottom (size-1) → next page
    let next_target = if position_in_page < page_size / 2 {
        // Currently at top, jump to middle
        current_page.start + page_size / 2
    } else if position_in_page < page_size - 1 {
        // Currently in middle/near-bottom, jump to bottom
        current_page.end - 1
    } else if current_page_idx < pages.len() - 1 {
        // Currently at bottom, jump to top of next page
        pages[current_page_idx + 1].start
    } else {
        // At bottom of last page, stay there
        current_index
    };

    next_target.min(pages.iter().map(|p| p.end).max().unwrap_or(0).saturating_sub(1))
}

/// Calculate jump target for page-up navigation (Ctrl+U style)
/// Implements three-level navigation: bottom → middle → top → previous page
fn calculate_jump_target_up(
    pages: &[PageBoundary],
    current_index: usize,
    current_page_idx: usize,
) -> usize {
    let current_page = pages[current_page_idx];

    // Determine position within current page
    let position_in_page = current_index - current_page.start;
    let page_size = current_page.size();

    if page_size == 0 {
        return current_index;
    }

    // Three-level jumping: bottom (size-1) → middle (size/2) → top (0) → previous page
    let next_target = if position_in_page > page_size / 2 {
        // Currently at bottom, jump to middle
        current_page.start + page_size / 2
    } else if position_in_page > 0 {
        // Currently at top/near-top, jump to top
        current_page.start
    } else if current_page_idx > 0 {
        // Currently at top of page, jump to bottom of previous page
        pages[current_page_idx - 1].end - 1
    } else {
        // At top of first page, stay there
        current_index
    };

    next_target
}

impl App {

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
                if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    if !list.is_empty() {
                        let last_idx = list.len() - 1;
                        list.jump_to(last_idx);
                        list.ensure_selected_visible(self.viewport_height);
                    }
                }
            }
        }
    }

    pub fn handle_jump_half_viewport_up(&mut self) {
        if self.focus == Focus::Cards {
            if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                let total_items = list.len();
                if total_items == 0 {
                    return;
                }

                // Compute variable-sized pages
                let pages = compute_page_boundaries(total_items, self.viewport_height);
                if pages.is_empty() {
                    return;
                }

                // Find current page and position
                if let Some(current_idx) = list.get_selected_index() {
                    if let Some(page_idx) = find_current_page(&pages, current_idx) {
                        let target = calculate_jump_target_up(&pages, current_idx, page_idx);
                        list.jump_to(target);
                    }
                }
            }
        }
    }

    pub fn handle_jump_half_viewport_down(&mut self) {
        if self.focus == Focus::Cards {
            if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                let total_items = list.len();
                if total_items == 0 {
                    return;
                }

                // Compute variable-sized pages
                let pages = compute_page_boundaries(total_items, self.viewport_height);
                if pages.is_empty() {
                    return;
                }

                // Find current page and position
                if let Some(current_idx) = list.get_selected_index() {
                    if let Some(page_idx) = find_current_page(&pages, current_idx) {
                        let target = calculate_jump_target_down(&pages, current_idx, page_idx);
                        list.jump_to(target);
                    }
                }
            }
        }
    }
}
