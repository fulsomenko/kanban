use crate::components::{Page, PageInfo};
use crate::selection::SelectionState;
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
            page: Page::new(0),
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
            let new_idx = self.page.navigate_up(current_idx);
            self.selection.set(Some(new_idx));

            // Scroll up if selection moved before scroll window
            if new_idx < self.page.scroll_offset {
                self.page.set_scroll_offset(new_idx);
            }
        }

        was_at_top
    }

    pub fn navigate_down(&mut self) -> bool {
        if self.page.total_items == 0 {
            return true;
        }

        let was_at_bottom = self.selection.get() == Some(self.page.total_items - 1);

        if !was_at_bottom {
            let current_idx = self.selection.get().unwrap_or(0);
            let new_idx = self.page.navigate_down(current_idx);
            self.selection.set(Some(new_idx));
        }

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
        self.page.set_scroll_offset(offset);
    }

    /// Ensure selected item is visible by scrolling if needed
    pub fn ensure_selected_visible(&mut self, viewport_height: usize) {
        if let Some(selected_idx) = self.selection.get() {
            self.page
                .scroll_to_make_visible(selected_idx, viewport_height);
        }
    }

    /// Get rendering information given a viewport height (raw lines, all overhead pre-accounted)
    pub fn get_render_info(&self, viewport_height: usize) -> ListRenderInfo {
        self.page.get_page_info(viewport_height)
    }

    /// Direct navigation - jump to a specific index
    pub fn jump_to(&mut self, index: usize) {
        if index < self.page.total_items {
            self.selection.set(Some(index));
        }
    }

    /// Get total item count
    pub fn len(&self) -> usize {
        self.page.total_items
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.page.total_items == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_list(item_count: usize) -> ListComponent {
        let mut list = ListComponent::new(false);
        list.update_item_count(item_count);
        list
    }

    #[test]
    fn test_navigate_down() {
        let mut list = create_test_list(5);
        assert_eq!(list.get_selected_index(), Some(0));

        list.navigate_down();
        assert_eq!(list.get_selected_index(), Some(1));

        list.navigate_down();
        assert_eq!(list.get_selected_index(), Some(2));
    }

    #[test]
    fn test_navigate_up() {
        let mut list = create_test_list(5);
        list.set_selected_index(Some(2));

        list.navigate_up();
        assert_eq!(list.get_selected_index(), Some(1));

        list.navigate_up();
        assert_eq!(list.get_selected_index(), Some(0));
    }

    #[test]
    fn test_navigate_down_at_boundary() {
        let mut list = create_test_list(5);
        list.set_selected_index(Some(4));

        let was_at_bottom = list.navigate_down();
        assert!(was_at_bottom);
        assert_eq!(list.get_selected_index(), Some(4));
    }

    #[test]
    fn test_navigate_up_at_boundary() {
        let mut list = create_test_list(5);
        list.set_selected_index(Some(0));

        let was_at_top = list.navigate_up();
        assert!(was_at_top);
        assert_eq!(list.get_selected_index(), Some(0));
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
        let mut list = create_test_list(0);
        assert_eq!(list.get_selected_index(), None);

        list.navigate_down();
        assert_eq!(list.get_selected_index(), None);

        list.navigate_up();
        assert_eq!(list.get_selected_index(), None);
    }

    #[test]
    fn test_update_item_count_keeps_selection() {
        let mut list = create_test_list(5);
        list.set_selected_index(Some(2));

        list.update_item_count(10);
        assert_eq!(list.get_selected_index(), Some(2));
    }

    #[test]
    fn test_update_item_count_clamps_selection() {
        let mut list = create_test_list(10);
        list.set_selected_index(Some(8));

        list.update_item_count(5);
        assert_eq!(list.get_selected_index(), Some(4));
    }

    #[test]
    fn test_render_info_basic() {
        let list = create_test_list(5);
        let info = list.get_render_info(3);

        assert!(!info.show_above_indicator);
        assert_eq!(info.visible_item_indices, vec![0, 1, 2]);
        assert!(info.show_below_indicator);
    }

    #[test]
    fn test_render_info_with_scroll() {
        let mut list = create_test_list(10);
        list.set_scroll_offset(3);

        let info = list.get_render_info(3);
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

    #[test]
    fn test_jump_to() {
        let mut list = create_test_list(10);
        list.jump_to(5);
        assert_eq!(list.get_selected_index(), Some(5));
    }

    #[test]
    fn test_ensure_selected_visible() {
        let mut list = create_test_list(20);
        list.jump_to(15);
        list.ensure_selected_visible(5);

        let info = list.get_render_info(5);
        assert!(info.visible_item_indices.contains(&15));
    }
}
