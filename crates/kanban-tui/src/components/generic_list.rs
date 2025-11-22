use crate::selection::SelectionState;
use crate::components::{Page, PageInfo};
use std::collections::HashSet;

pub type ListRenderInfo = PageInfo;

#[derive(Clone)]
pub struct ListComponent {
    pub selection: SelectionState,
    page: Page,
    pub multi_selected: HashSet<usize>,
    pub allow_multi_select: bool,
}

impl ListComponent {
    pub fn new(allow_multi_select: bool) -> Self {
        Self {
            selection: SelectionState::new(),
            page: Page::new(0, 0),
            multi_selected: HashSet::new(),
            allow_multi_select,
        }
    }

    pub fn update_item_count(&mut self, count: usize) {
        let current_selection = self.selection.get();
        self.page.set_total_items(count);

        if let Some(idx) = current_selection {
            if idx >= count && count > 0 {
                self.selection.set(Some(count - 1));
            } else if count == 0 {
                self.selection.clear();
            }
        } else if count > 0 {
            self.selection.auto_select_first_if_empty(true);
        }
    }

    pub fn navigate_up(&mut self) -> bool {
        if self.page.total_items == 0 {
            return true;
        }

        let was_at_top = self.selection.get() == Some(0) || self.selection.get().is_none();

        if !was_at_top {
            let current_idx = self.selection.get().unwrap_or(0);
            let page_info = self.page.get_page_info();
            let first_visible_idx = page_info.first_visible_idx;

            if current_idx == first_visible_idx && self.page.scroll_offset > 0 {
                let target_selection = current_idx.saturating_sub(1);
                self.page.scroll_offset = self.page.scroll_offset.saturating_sub(1);

                loop {
                    let mut temp_page = self.page.clone();
                    let current_render_start = temp_page.get_page_info().first_visible_idx;
                    if target_selection >= current_render_start || self.page.scroll_offset == 0 {
                        break;
                    }
                    self.page.scroll_offset = self.page.scroll_offset.saturating_sub(1);
                }

                self.selection.set(Some(target_selection));
            } else if current_idx > first_visible_idx {
                self.selection.prev();
            }
        }

        self.clamp_selection_to_visible();
        was_at_top
    }

    pub fn navigate_down(&mut self) -> bool {
        if self.page.total_items == 0 {
            return true;
        }

        let was_at_bottom = self.selection.get() == Some(self.page.total_items - 1);

        if !was_at_bottom {
            let current_idx = self.selection.get().unwrap_or(0);
            let page_info = self.page.get_page_info();
            let last_visible_idx = page_info.last_visible_idx;

            if current_idx == last_visible_idx && current_idx < self.page.total_items - 1 {
                let target_selection = (current_idx + 1).min(self.page.total_items - 1);
                self.page.scroll_offset = self.page.scroll_offset.saturating_add(1);

                loop {
                    let mut temp_page = self.page.clone();
                    let temp_page_info = temp_page.get_page_info();
                    let new_last_visible_idx = temp_page_info.last_visible_idx;

                    if target_selection <= new_last_visible_idx || target_selection >= self.page.total_items {
                        break;
                    }
                    self.page.scroll_offset = self.page.scroll_offset.saturating_add(1);
                }

                self.selection.set(Some(target_selection));
            } else if current_idx < last_visible_idx {
                self.selection.next(self.page.total_items);
            }
        }

        self.clamp_selection_to_visible();
        was_at_bottom
    }

    pub fn get_selected_index(&self) -> Option<usize> {
        self.selection.get()
    }

    pub fn set_selected_index(&mut self, index: Option<usize>) {
        if let Some(idx) = index {
            if idx < self.page.total_items {
                self.selection.set(Some(idx));
            } else {
                self.selection.clear();
            }
        } else {
            self.selection.clear();
        }
    }

    pub fn toggle_multi_select(&mut self, index: usize) {
        if self.allow_multi_select && index < self.page.total_items {
            if self.multi_selected.contains(&index) {
                self.multi_selected.remove(&index);
            } else {
                self.multi_selected.insert(index);
            }
        }
    }

    pub fn clear_multi_select(&mut self) {
        self.multi_selected.clear();
    }

    pub fn select_all(&mut self) {
        if self.allow_multi_select {
            self.multi_selected = (0..self.page.total_items).collect();
        }
    }

    pub fn get_multi_selected_indices(&self) -> Vec<usize> {
        let mut indices: Vec<_> = self.multi_selected.iter().copied().collect();
        indices.sort_unstable();
        indices
    }

    pub fn get_scroll_offset(&self) -> usize {
        self.page.scroll_offset
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.page.scroll_offset = offset.min(self.page.total_items.saturating_sub(1));
    }

    pub fn ensure_selected_visible(&mut self) {
        if let Some(selected_idx) = self.selection.get() {
            self.page.scroll_to_make_visible(selected_idx);
        }
    }

    pub fn jump_half_page_up(&mut self) {
        if self.page.total_items == 0 {
            return;
        }
        let current_idx = self.selection.get().unwrap_or(0);
        let new_idx = self.page.jump_half_page_up(current_idx);
        self.selection.set(Some(new_idx));
    }

    pub fn jump_half_page_down(&mut self) {
        if self.page.total_items == 0 {
            return;
        }
        let current_idx = self.selection.get().unwrap_or(0);
        let new_idx = self.page.jump_half_page_down(current_idx);
        self.selection.set(Some(new_idx));
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.page.set_viewport_height(height);
    }

    pub fn get_render_info(&self) -> ListRenderInfo {
        self.page.get_page_info()
    }

    fn clamp_selection_to_visible(&mut self) {
        if self.page.total_items == 0 {
            self.selection.clear();
            return;
        }

        if let Some(selected_idx) = self.selection.get() {
            let render_start = if self.page.scroll_offset == 1 {
                0
            } else {
                self.page.scroll_offset
            };

            let page_info = self.page.get_page_info();
            let render_end = render_start + page_info.items_per_page;

            if selected_idx < render_start {
                self.page.scroll_offset = selected_idx;
            } else if selected_idx >= render_end {
                self.page.scroll_offset = selected_idx.saturating_sub(page_info.items_per_page - 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_list(item_count: usize, viewport_height: usize) -> ListComponent {
        let mut list = ListComponent::new(false);
        list.update_item_count(item_count);
        list.set_viewport_height(viewport_height);
        list
    }

    #[test]
    fn test_navigate_down() {
        let mut list = create_test_list(5, 3);
        assert_eq!(list.get_selected_index(), Some(0));

        list.navigate_down();
        assert_eq!(list.get_selected_index(), Some(1));

        list.navigate_down();
        assert_eq!(list.get_selected_index(), Some(2));
    }

    #[test]
    fn test_navigate_up() {
        let mut list = create_test_list(5, 3);
        list.set_selected_index(Some(2));

        list.navigate_up();
        assert_eq!(list.get_selected_index(), Some(1));

        list.navigate_up();
        assert_eq!(list.get_selected_index(), Some(0));
    }

    #[test]
    fn test_navigate_down_at_boundary() {
        let mut list = create_test_list(5, 3);
        list.set_selected_index(Some(4));

        let was_at_bottom = list.navigate_down();
        assert!(was_at_bottom);
        assert_eq!(list.get_selected_index(), Some(4));
    }

    #[test]
    fn test_navigate_up_at_boundary() {
        let mut list = create_test_list(5, 3);
        list.set_selected_index(Some(0));

        let was_at_top = list.navigate_up();
        assert!(was_at_top);
        assert_eq!(list.get_selected_index(), Some(0));
    }

    #[test]
    fn test_viewport_scrolling() {
        let mut list = create_test_list(20, 3);
        assert_eq!(list.get_scroll_offset(), 0);

        for _ in 0..10 {
            list.navigate_down();
        }

        assert!(list.get_scroll_offset() > 0);
    }

    #[test]
    fn test_multi_select() {
        let mut list = ListComponent::new(true);
        list.update_item_count(5);

        list.toggle_multi_select(1);
        list.toggle_multi_select(3);

        assert!(list.multi_selected.contains(&1));
        assert!(list.multi_selected.contains(&3));
        assert_eq!(list.get_multi_selected_indices(), vec![1, 3]);
    }

    #[test]
    fn test_clear_multi_select() {
        let mut list = ListComponent::new(true);
        list.update_item_count(5);
        list.toggle_multi_select(1);
        list.toggle_multi_select(3);

        list.clear_multi_select();
        assert_eq!(list.get_multi_selected_indices().len(), 0);
    }

    #[test]
    fn test_select_all() {
        let mut list = ListComponent::new(true);
        list.update_item_count(5);

        list.select_all();
        assert_eq!(list.get_multi_selected_indices().len(), 5);
    }

    #[test]
    fn test_empty_list() {
        let mut list = create_test_list(0, 3);
        assert_eq!(list.get_selected_index(), None);

        list.navigate_down();
        assert_eq!(list.get_selected_index(), None);

        list.navigate_up();
        assert_eq!(list.get_selected_index(), None);
    }

    #[test]
    fn test_update_item_count_keeps_selection() {
        let mut list = create_test_list(5, 3);
        list.set_selected_index(Some(2));

        list.update_item_count(10);
        assert_eq!(list.get_selected_index(), Some(2));
    }

    #[test]
    fn test_update_item_count_clamps_selection() {
        let mut list = create_test_list(10, 3);
        list.set_selected_index(Some(8));

        list.update_item_count(5);
        assert_eq!(list.get_selected_index(), Some(4));
    }

    #[test]
    fn test_render_info_basic() {
        let list = create_test_list(5, 3);
        let info = list.get_render_info();

        assert!(!info.show_above_indicator);
        // With 5 items and viewport 3, below indicator takes space, so only 2 items shown
        assert_eq!(info.visible_item_indices, vec![0, 1]);
        assert!(info.show_below_indicator);
    }

    #[test]
    fn test_render_info_with_scroll() {
        let mut list = create_test_list(10, 3);
        list.set_scroll_offset(3);

        let info = list.get_render_info();
        assert!(info.show_above_indicator);
        assert_eq!(info.items_above_count, 3);
    }

    #[test]
    fn test_multi_select_disabled() {
        let mut list = ListComponent::new(false);
        list.update_item_count(5);

        list.toggle_multi_select(1);
        assert!(list.multi_selected.is_empty());
    }
}
