//! Generic pagination utilities.
//!
//! Provides a reusable virtual list/pagination implementation that can be used
//! by any UI framework. Handles scroll position, viewport calculation, and
//! scroll indicators.

/// Information about the visible portion of a paginated list.
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// Indices of items visible in the current viewport.
    pub visible_indices: Vec<usize>,
    /// Index of the first visible item.
    pub first_visible: usize,
    /// Index of the last visible item.
    pub last_visible: usize,
    /// Number of items that fit in one page.
    pub items_per_page: usize,
    /// Whether there are items above the viewport.
    pub show_above_indicator: bool,
    /// Count of items above the viewport.
    pub items_above: usize,
    /// Whether there are items below the viewport.
    pub show_below_indicator: bool,
    /// Count of items below the viewport.
    pub items_below: usize,
    /// Current page number (0-indexed).
    pub current_page: usize,
    /// Total number of pages.
    pub total_pages: usize,
}

impl PageInfo {
    /// Create an empty page info (no items).
    pub fn empty() -> Self {
        Self {
            visible_indices: vec![],
            first_visible: 0,
            last_visible: 0,
            items_per_page: 0,
            show_above_indicator: false,
            items_above: 0,
            show_below_indicator: false,
            items_below: 0,
            current_page: 0,
            total_pages: 0,
        }
    }
}

/// Manages pagination state for a virtual list.
///
/// This is a pure data component that knows nothing about rendering.
/// It only tracks which items should be visible based on scroll position.
#[derive(Clone)]
pub struct Page {
    /// Total number of items in the list.
    pub total_items: usize,
    /// Current scroll offset (index of first visible item).
    pub scroll_offset: usize,
}

impl Page {
    /// Create a new page with the given item count.
    pub fn new(total_items: usize) -> Self {
        Self {
            total_items,
            scroll_offset: 0,
        }
    }

    /// Update the total item count, clamping scroll offset if necessary.
    pub fn set_total_items(&mut self, total_items: usize) {
        self.total_items = total_items;
        if self.scroll_offset >= total_items && total_items > 0 {
            self.scroll_offset = total_items.saturating_sub(1);
        }
    }

    /// Get information about what items to render.
    ///
    /// # Arguments
    /// * `viewport_height` - Number of items that fit in the viewport
    pub fn get_page_info(&self, viewport_height: usize) -> PageInfo {
        if self.total_items == 0 || viewport_height == 0 {
            return PageInfo::empty();
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

        // Calculate pages
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
            visible_indices,
            first_visible,
            last_visible,
            items_per_page: viewport_height,
            show_above_indicator: has_items_above,
            items_above,
            show_below_indicator: has_items_below,
            items_below,
            current_page,
            total_pages,
        }
    }

    /// Set scroll offset, clamping to valid range.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset.min(self.total_items.saturating_sub(1));
    }

    /// Scroll to make an item visible in the viewport.
    pub fn scroll_to_visible(&mut self, item_idx: usize, viewport_height: usize) {
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

    /// Navigate up by one item, returning the new index.
    pub fn navigate_up(&self, current_idx: usize) -> usize {
        current_idx.saturating_sub(1)
    }

    /// Navigate down by one item, returning the new index.
    pub fn navigate_down(&self, current_idx: usize) -> usize {
        if current_idx >= self.total_items.saturating_sub(1) {
            current_idx
        } else {
            current_idx + 1
        }
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.total_items == 0
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_info_empty() {
        let page = Page::new(0);
        let info = page.get_page_info(10);

        assert!(info.visible_indices.is_empty());
        assert!(!info.show_above_indicator);
        assert!(!info.show_below_indicator);
    }

    #[test]
    fn test_single_page_fits_all() {
        let page = Page::new(5);
        let info = page.get_page_info(10);

        assert_eq!(info.visible_indices, vec![0, 1, 2, 3, 4]);
        assert!(!info.show_above_indicator);
        assert!(!info.show_below_indicator);
    }

    #[test]
    fn test_first_page_multi_page() {
        let page = Page::new(20);
        let info = page.get_page_info(5);

        assert_eq!(info.visible_indices, vec![0, 1, 2, 3, 4]);
        assert!(!info.show_above_indicator);
        assert!(info.show_below_indicator);
        assert_eq!(info.items_below, 15);
    }

    #[test]
    fn test_middle_page() {
        let mut page = Page::new(20);
        page.set_scroll_offset(5);
        let info = page.get_page_info(5);

        assert_eq!(info.visible_indices, vec![5, 6, 7, 8, 9]);
        assert!(info.show_above_indicator);
        assert!(info.show_below_indicator);
        assert_eq!(info.items_above, 5);
        assert_eq!(info.items_below, 10);
    }

    #[test]
    fn test_last_page() {
        let mut page = Page::new(20);
        page.set_scroll_offset(15);
        let info = page.get_page_info(5);

        assert_eq!(info.visible_indices, vec![15, 16, 17, 18, 19]);
        assert!(info.show_above_indicator);
        assert!(!info.show_below_indicator);
    }

    #[test]
    fn test_scroll_to_visible() {
        let mut page = Page::new(20);
        page.scroll_to_visible(15, 5);

        assert_eq!(page.scroll_offset, 11); // 15 - (5 - 1)
        let info = page.get_page_info(5);
        assert!(info.visible_indices.contains(&15));
    }

    #[test]
    fn test_navigate_up_down() {
        let page = Page::new(10);

        let idx = page.navigate_down(0);
        assert_eq!(idx, 1);

        let idx = page.navigate_up(idx);
        assert_eq!(idx, 0);

        // At boundary
        let idx = page.navigate_up(0);
        assert_eq!(idx, 0);

        let idx = page.navigate_down(9);
        assert_eq!(idx, 9);
    }

    #[test]
    fn test_set_total_items_clamps_scroll() {
        let mut page = Page::new(20);
        page.set_scroll_offset(15);

        page.set_total_items(10);
        assert_eq!(page.scroll_offset, 9);
    }
}
