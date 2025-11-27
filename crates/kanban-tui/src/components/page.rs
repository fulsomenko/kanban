#[derive(Debug, Clone)]
pub struct PageInfo {
    pub visible_item_indices: Vec<usize>,
    pub first_visible_idx: usize,
    pub last_visible_idx: usize,
    pub items_per_page: usize,
    pub show_above_indicator: bool,
    pub items_above_count: usize,
    pub show_below_indicator: bool,
    pub items_below_count: usize,
    pub current_page: usize,
    pub total_pages: usize,
}

/// Pure data component - knows nothing about viewport, indicators, or rendering
/// Only concerned with which items to show based on scroll position
#[derive(Clone)]
pub struct Page {
    pub total_items: usize,
    pub scroll_offset: usize,
}

impl Page {
    /// Create a new page with no items
    pub fn new(total_items: usize) -> Self {
        Page {
            total_items,
            scroll_offset: 0,
        }
    }

    /// Update total item count
    pub fn set_total_items(&mut self, total_items: usize) {
        self.total_items = total_items;
        // Clamp scroll_offset to valid range
        if self.scroll_offset >= total_items && total_items > 0 {
            self.scroll_offset = total_items.saturating_sub(1);
        }
    }

    /// Get information about what items to render given a viewport height
    /// viewport_height: The raw number of lines available (all overhead pre-accounted by caller)
    pub fn get_page_info(&self, viewport_height: usize) -> PageInfo {
        if self.total_items == 0 || viewport_height == 0 {
            return PageInfo {
                visible_item_indices: Vec::new(),
                first_visible_idx: 0,
                last_visible_idx: 0,
                items_per_page: 0,
                show_above_indicator: false,
                items_above_count: 0,
                show_below_indicator: false,
                items_below_count: 0,
                current_page: 0,
                total_pages: 0,
            };
        }

        let render_start = self.scroll_offset;

        // Determine which items are visible
        let visible_indices: Vec<usize> = (0..viewport_height)
            .map(|i| render_start + i)
            .filter(|&idx| idx < self.total_items)
            .collect();

        let first_visible = visible_indices.first().copied().unwrap_or(0);
        let last_visible = visible_indices.last().copied().unwrap_or(0);

        // Count items above and below
        let items_above = render_start;
        let items_below = if visible_indices.is_empty() {
            self.total_items.saturating_sub(render_start)
        } else {
            self.total_items.saturating_sub(last_visible + 1)
        };

        // Determine if we should show indicators
        let has_items_above = render_start > 0;
        let has_items_below = (render_start + viewport_height) < self.total_items;

        // Calculate pages (for reference - not used for data)
        let total_pages = if viewport_height > 0 {
            self.total_items.div_ceil(viewport_height)
        } else {
            0
        };
        let current_page = if viewport_height > 0 {
            self.scroll_offset / viewport_height
        } else {
            0
        };

        PageInfo {
            visible_item_indices: visible_indices,
            first_visible_idx: first_visible,
            last_visible_idx: last_visible,
            items_per_page: viewport_height,
            show_above_indicator: has_items_above,
            items_above_count: items_above,
            show_below_indicator: has_items_below,
            items_below_count: items_below,
            current_page,
            total_pages,
        }
    }

    /// Set scroll offset (typically updated by rendering/navigation)
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset.min(self.total_items.saturating_sub(1));
    }

    /// Scroll to make an item visible
    pub fn scroll_to_make_visible(&mut self, item_idx: usize, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }

        let scroll_start = self.scroll_offset;
        let scroll_end = scroll_start + viewport_height;

        if item_idx < scroll_start {
            self.scroll_offset = item_idx;
        } else if item_idx >= scroll_end {
            self.scroll_offset = item_idx.saturating_sub(viewport_height - 1);
        }
    }

    /// Navigate up by one item
    pub fn navigate_up(&mut self, current_idx: usize) -> usize {
        if current_idx == 0 {
            return current_idx;
        }
        current_idx.saturating_sub(1)
    }

    /// Navigate down by one item
    pub fn navigate_down(&mut self, current_idx: usize) -> usize {
        if current_idx >= self.total_items.saturating_sub(1) {
            return current_idx;
        }
        current_idx + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_info_empty() {
        let page = Page::new(0);
        let info = page.get_page_info(10);

        assert_eq!(info.visible_item_indices, vec![] as Vec<usize>);
        assert!(!info.show_above_indicator);
        assert!(!info.show_below_indicator);
    }

    #[test]
    fn test_single_page_fits_all() {
        let page = Page::new(5);
        let info = page.get_page_info(10);

        assert_eq!(info.visible_item_indices, vec![0, 1, 2, 3, 4]);
        assert!(!info.show_above_indicator);
        assert!(!info.show_below_indicator);
    }

    #[test]
    fn test_first_page_multi_page() {
        let page = Page::new(20);
        let info = page.get_page_info(5);

        assert_eq!(info.visible_item_indices, vec![0, 1, 2, 3, 4]);
        assert!(!info.show_above_indicator);
        assert!(info.show_below_indicator);
        assert_eq!(info.items_below_count, 15);
    }

    #[test]
    fn test_middle_page() {
        let mut page = Page::new(20);
        page.set_scroll_offset(5);
        let info = page.get_page_info(5);

        assert_eq!(info.visible_item_indices, vec![5, 6, 7, 8, 9]);
        assert!(info.show_above_indicator);
        assert!(info.show_below_indicator);
        assert_eq!(info.items_above_count, 5);
        assert_eq!(info.items_below_count, 10);
    }

    #[test]
    fn test_last_page() {
        let mut page = Page::new(20);
        page.set_scroll_offset(15);
        let info = page.get_page_info(5);

        assert_eq!(info.visible_item_indices, vec![15, 16, 17, 18, 19]);
        assert!(info.show_above_indicator);
        assert!(!info.show_below_indicator);
        assert_eq!(info.items_above_count, 15);
        assert_eq!(info.items_below_count, 0);
    }

    #[test]
    fn test_scroll_to_make_visible() {
        let mut page = Page::new(20);
        page.scroll_to_make_visible(15, 5);

        assert_eq!(page.scroll_offset, 11); // 15 - (5 - 1)
        let info = page.get_page_info(5);
        assert!(info.visible_item_indices.contains(&15));
    }

    #[test]
    fn test_navigate_up_down() {
        let mut page = Page::new(10);

        let idx = page.navigate_down(0);
        assert_eq!(idx, 1);

        let idx = page.navigate_up(idx);
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_set_total_items_clamps_scroll() {
        let mut page = Page::new(20);
        page.set_scroll_offset(15);

        page.set_total_items(10);
        assert_eq!(page.scroll_offset, 9);
    }
}
