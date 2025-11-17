use crate::selection::SelectionState;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ListRenderInfo {
    pub visible_indices: Vec<usize>,
    pub show_above_indicator: bool,
    pub items_above_count: usize,
    pub show_below_indicator: bool,
    pub items_below_count: usize,
}

pub struct ListComponent {
    pub item_count: usize,
    pub selection: SelectionState,
    pub scroll_offset: usize,
    pub viewport_height: usize,
    pub multi_selected: HashSet<usize>,
    pub allow_multi_select: bool,
}

impl ListComponent {
    pub fn new(allow_multi_select: bool) -> Self {
        Self {
            item_count: 0,
            selection: SelectionState::new(),
            scroll_offset: 0,
            viewport_height: 0,
            multi_selected: HashSet::new(),
            allow_multi_select,
        }
    }

    pub fn update_item_count(&mut self, count: usize) {
        let current_selection = self.selection.get();
        self.item_count = count;

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
        if self.item_count == 0 {
            return true;
        }

        let was_at_top = self.selection.get() == Some(0) || self.selection.get().is_none();

        if !was_at_top {
            let current_idx = self.selection.get().unwrap_or(0);
            let render_start = if self.scroll_offset == 1 {
                0
            } else {
                self.scroll_offset
            };
            let first_visible_idx = render_start;

            if current_idx == first_visible_idx && self.scroll_offset > 0 {
                let target_selection = current_idx.saturating_sub(1);

                self.scroll_offset = self.scroll_offset.saturating_sub(1);

                loop {
                    let current_render_start = if self.scroll_offset == 1 {
                        0
                    } else {
                        self.scroll_offset
                    };
                    if target_selection >= current_render_start || self.scroll_offset == 0 {
                        break;
                    }
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
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
        if self.item_count == 0 {
            return true;
        }

        let was_at_bottom = self.selection.get() == Some(self.item_count - 1);

        if !was_at_bottom {
            let current_idx = self.selection.get().unwrap_or(0);

            let info = self.calculate_viewport_info();
            let render_start = if self.scroll_offset == 1 {
                0
            } else {
                self.scroll_offset
            };
            let actual_items_to_show = (render_start..self.item_count)
                .take(info.items_to_show)
                .count();
            let last_visible_idx = render_start + actual_items_to_show.saturating_sub(1);

            if current_idx == last_visible_idx && current_idx < self.item_count - 1 {
                let target_selection = (current_idx + 1).min(self.item_count - 1);

                self.scroll_offset = self.scroll_offset.saturating_add(1);

                let mut new_info = self.calculate_viewport_info();
                let mut new_render_start = if self.scroll_offset == 1 {
                    0
                } else {
                    self.scroll_offset
                };
                let mut new_actual_items = (new_render_start..self.item_count)
                    .take(new_info.items_to_show)
                    .count();
                let mut new_last_visible_idx =
                    new_render_start + new_actual_items.saturating_sub(1);

                while target_selection > new_last_visible_idx && target_selection < self.item_count
                {
                    self.scroll_offset = self.scroll_offset.saturating_add(1);
                    new_info = self.calculate_viewport_info();
                    new_render_start = if self.scroll_offset == 1 {
                        0
                    } else {
                        self.scroll_offset
                    };
                    new_actual_items = (new_render_start..self.item_count)
                        .take(new_info.items_to_show)
                        .count();
                    new_last_visible_idx = new_render_start + new_actual_items.saturating_sub(1);
                }

                self.selection.set(Some(target_selection));
            } else if current_idx < last_visible_idx {
                self.selection.next(self.item_count);
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
            if idx < self.item_count {
                self.selection.set(Some(idx));
            } else {
                self.selection.clear();
            }
        } else {
            self.selection.clear();
        }
    }

    pub fn toggle_multi_select(&mut self, index: usize) {
        if self.allow_multi_select && index < self.item_count {
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
            self.multi_selected = (0..self.item_count).collect();
        }
    }

    pub fn get_multi_selected_indices(&self) -> Vec<usize> {
        let mut indices: Vec<_> = self.multi_selected.iter().copied().collect();
        indices.sort_unstable();
        indices
    }

    pub fn get_scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset.min(self.item_count.saturating_sub(1));
    }

    pub fn ensure_selected_visible(&mut self) {
        if self.viewport_height == 0 {
            return;
        }

        if let Some(selected_idx) = self.selection.get() {
            let scroll_start = self.scroll_offset;
            let scroll_end = scroll_start + self.viewport_height;

            if selected_idx < scroll_start {
                self.scroll_offset = selected_idx;
            } else if selected_idx >= scroll_end {
                self.scroll_offset = selected_idx.saturating_sub(self.viewport_height - 1);
            }
        }
    }

    pub fn get_render_info(&self) -> ListRenderInfo {
        if self.item_count == 0 {
            return ListRenderInfo {
                visible_indices: Vec::new(),
                show_above_indicator: false,
                items_above_count: 0,
                show_below_indicator: false,
                items_below_count: 0,
            };
        }

        let info = self.calculate_viewport_info();

        let render_start = if self.scroll_offset == 1 {
            0
        } else {
            self.scroll_offset
        };

        let visible_indices: Vec<usize> = (0..info.items_to_show)
            .map(|i| render_start + i)
            .filter(|&idx| idx < self.item_count)
            .collect();

        let items_below_count = if visible_indices.is_empty() {
            self.item_count.saturating_sub(render_start)
        } else {
            let last_visible_idx = *visible_indices.last().unwrap();
            self.item_count.saturating_sub(last_visible_idx + 1)
        };

        ListRenderInfo {
            visible_indices,
            show_above_indicator: info.has_items_above,
            items_above_count: self.scroll_offset,
            show_below_indicator: info.has_items_below,
            items_below_count,
        }
    }

    fn calculate_viewport_info(&self) -> ViewportInfo {
        let has_items_above = self.scroll_offset > 1;
        let available_space = self
            .viewport_height
            .saturating_sub(has_items_above as usize);
        let has_items_below = self.scroll_offset + available_space < self.item_count;

        let num_indicator_lines = (has_items_above as usize) + (has_items_below as usize);
        let items_to_show = self.viewport_height.saturating_sub(num_indicator_lines);

        ViewportInfo {
            has_items_above,
            has_items_below,
            items_to_show,
        }
    }

    fn clamp_selection_to_visible(&mut self) {
        if self.item_count == 0 {
            self.selection.clear();
            return;
        }

        if let Some(selected_idx) = self.selection.get() {
            let render_start = if self.scroll_offset == 1 {
                0
            } else {
                self.scroll_offset
            };

            let info = self.calculate_viewport_info();
            let render_end = render_start + info.items_to_show;

            if selected_idx < render_start {
                self.scroll_offset = selected_idx;
            } else if selected_idx >= render_end {
                self.scroll_offset = selected_idx.saturating_sub(info.items_to_show - 1);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ViewportInfo {
    has_items_above: bool,
    has_items_below: bool,
    items_to_show: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_list(item_count: usize, viewport_height: usize) -> ListComponent {
        let mut list = ListComponent::new(false);
        list.update_item_count(item_count);
        list.viewport_height = viewport_height;
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
        assert_eq!(info.visible_indices, vec![0, 1]);
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
