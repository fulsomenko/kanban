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

#[derive(Debug, Clone)]
struct ComputedPage {
    start_index: usize,
    end_index: usize,
    scroll_offset: usize,  // Where to scroll to display this page
}

#[derive(Clone)]
pub struct Page {
    pub total_items: usize,
    pub page_size: usize,  // Number of card items per page
    pub scroll_offset: usize,
    current_page: usize, // explicit page tracking
    computed_pages: Vec<ComputedPage>, // Pre-computed page boundaries
}

#[derive(Debug, Clone)]
struct ViewportInfo {
    has_items_above: bool,
    has_items_below: bool,
    items_to_show: usize,
}

impl Page {
    pub fn new(total_items: usize, page_size: usize) -> Self {
        let mut page = Self {
            total_items,
            page_size,
            scroll_offset: 0,
            current_page: 0,
            computed_pages: Vec::new(),
        };
        page.recompute_pages();
        page
    }

    /// Pre-compute all page boundaries
    fn recompute_pages(&mut self) {
        self.computed_pages = self.compute_pages();
    }

    /// Compute page boundaries based purely on card items and page size
    /// Page size is the number of card items per page (all UI overhead already accounted for by caller)
    fn compute_pages(&self) -> Vec<ComputedPage> {
        if self.total_items == 0 {
            return Vec::new();
        }

        let mut pages = Vec::new();
        let mut current_scroll = 0;
        let items_per_page = self.page_size;

        if items_per_page == 0 {
            return Vec::new();
        }

        while current_scroll < self.total_items {
            let start_index = current_scroll;
            let end_index = (current_scroll + items_per_page - 1).min(self.total_items - 1);

            pages.push(ComputedPage {
                start_index,
                end_index,
                scroll_offset: current_scroll,
            });

            current_scroll = end_index + 1;
        }

        pages
    }

    pub fn set_total_items(&mut self, total_items: usize) {
        self.total_items = total_items;
        if self.scroll_offset > total_items.saturating_sub(1) {
            self.scroll_offset = total_items.saturating_sub(1);
        }
        self.sync_page_from_scroll();
        self.recompute_pages();
    }

    pub fn set_page_size(&mut self, size: usize) {
        self.page_size = size;
        self.sync_page_from_scroll();
        self.recompute_pages();
    }

    /// Sync current_page from scroll_offset
    fn sync_page_from_scroll(&mut self) {
        if self.page_size == 0 {
            self.current_page = 0;
        } else {
            self.current_page = self.scroll_offset / self.page_size;
        }
    }

    pub fn get_current_page(&self) -> usize {
        self.current_page
    }

    pub fn get_total_pages(&self) -> usize {
        if self.total_items == 0 || self.page_size == 0 {
            return 0;
        }
        self.total_items.div_ceil(self.page_size)
    }

    pub fn get_page_info(&self) -> PageInfo {
        let total_pages = self.get_total_pages();

        if self.total_items == 0 {
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

        let viewport_info = self.calculate_viewport_info();

        let render_start = if self.scroll_offset == 1 {
            0
        } else {
            self.scroll_offset
        };

        let visible_indices: Vec<usize> = (0..viewport_info.items_to_show)
            .map(|i| render_start + i)
            .filter(|&idx| idx < self.total_items)
            .collect();

        let first_visible = visible_indices.first().copied().unwrap_or(0);
        let last_visible = visible_indices.last().copied().unwrap_or(0);

        let items_below_count = if visible_indices.is_empty() {
            self.total_items.saturating_sub(render_start)
        } else {
            self.total_items.saturating_sub(last_visible + 1)
        };

        PageInfo {
            visible_item_indices: visible_indices,
            first_visible_idx: first_visible,
            last_visible_idx: last_visible,
            items_per_page: viewport_info.items_to_show,
            show_above_indicator: viewport_info.has_items_above,
            items_above_count: self.scroll_offset,
            show_below_indicator: viewport_info.has_items_below,
            items_below_count,
            current_page: self.current_page + 1, // 1-indexed for display
            total_pages,
        }
    }

    pub fn jump_half_page_down(&mut self, current_idx: usize) -> usize {
        if self.computed_pages.is_empty() {
            return current_idx;
        }

        // Find which page contains the current index
        if let Some(page_idx) = self
            .computed_pages
            .iter()
            .position(|p| current_idx >= p.start_index && current_idx <= p.end_index)
        {
            let page = &self.computed_pages[page_idx];
            let middle_idx = (page.start_index + page.end_index) / 2;

            // Three-level navigation: Top → Middle → Bottom → Next Page
            if current_idx < middle_idx {
                // In top half: jump to middle
                return middle_idx;
            } else if current_idx < page.end_index {
                // In bottom half (but not at very bottom): jump to end
                return page.end_index;
            } else {
                // At or past bottom: jump to first card of next page
                if page_idx < self.computed_pages.len() - 1 {
                    let next_page = &self.computed_pages[page_idx + 1];
                    self.scroll_offset = next_page.scroll_offset;
                    return next_page.start_index;
                } else {
                    // On last page: try to reach the very last item
                    let last_idx = self.total_items.saturating_sub(1);
                    if current_idx < last_idx {
                        return last_idx;
                    }
                }
            }
        }

        // Already at the end, stay where we are
        current_idx
    }

    pub fn jump_half_page_up(&mut self, current_idx: usize) -> usize {
        if self.computed_pages.is_empty() {
            return current_idx;
        }

        // Find which page contains the current index
        if let Some(page_idx) = self
            .computed_pages
            .iter()
            .position(|p| current_idx >= p.start_index && current_idx <= p.end_index)
        {
            let page = &self.computed_pages[page_idx];
            let middle_idx = (page.start_index + page.end_index) / 2;

            // Three-level navigation: Bottom → Middle → Top → Previous Page
            if current_idx > middle_idx {
                // In bottom half: jump to middle
                return middle_idx;
            } else if current_idx > page.start_index {
                // In top half (but not at very top): jump to start
                return page.start_index;
            } else {
                // At or before top: jump to last card of previous page
                if page_idx > 0 {
                    let prev_page = &self.computed_pages[page_idx - 1];
                    self.scroll_offset = prev_page.scroll_offset;
                    return prev_page.end_index;
                }
            }
        }

        // Already on first page or not on any page, stay where we are
        current_idx
    }

    pub fn jump_full_page_down(&mut self, current_idx: usize) -> usize {
        if self.total_items == 0 {
            return 0;
        }

        let jump_distance = self.page_size;
        let new_idx = (current_idx + jump_distance).min(self.total_items - 1);

        // Check if jumping to a new page
        let current_page = current_idx / self.page_size;
        let new_page = new_idx / self.page_size;

        if new_page > current_page {
            // Jumping to next page: scroll to show it
            self.scroll_offset = new_page * self.page_size;
        }

        new_idx
    }

    pub fn jump_full_page_up(&mut self, current_idx: usize) -> usize {
        if self.total_items == 0 {
            return 0;
        }

        let jump_distance = self.page_size;
        let new_idx = current_idx.saturating_sub(jump_distance);

        // Check if jumping to a previous page
        let current_page = current_idx / self.page_size;
        let new_page = new_idx / self.page_size;

        if new_page < current_page {
            // Jumping to previous page: scroll to show it
            self.scroll_offset = new_page * self.page_size;
        }

        new_idx
    }

    pub fn scroll_to_make_visible(&mut self, item_idx: usize) {
        if self.page_size == 0 {
            return;
        }

        let scroll_start = self.scroll_offset;
        let scroll_end = scroll_start + self.page_size;

        if item_idx < scroll_start {
            self.scroll_offset = item_idx;
        } else if item_idx >= scroll_end {
            self.scroll_offset = item_idx.saturating_sub(self.page_size - 1);
        }

        self.sync_page_from_scroll();
    }

    pub fn navigate_up(&mut self, current_idx: usize) -> usize {
        if self.total_items == 0 {
            return current_idx;
        }

        if current_idx == 0 {
            return current_idx;
        }

        let page_info = self.get_page_info();
        let first_visible_idx = page_info.first_visible_idx;

        if current_idx == first_visible_idx && self.scroll_offset > 0 {
            let target_selection = current_idx.saturating_sub(1);
            self.scroll_offset = self.scroll_offset.saturating_sub(1);

            loop {
                let temp_page_info = self.get_page_info();
                let current_render_start = temp_page_info.first_visible_idx;
                if target_selection >= current_render_start || self.scroll_offset == 0 {
                    break;
                }
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }

            target_selection
        } else if current_idx > first_visible_idx {
            current_idx.saturating_sub(1)
        } else {
            current_idx
        }
    }

    pub fn navigate_down(&mut self, current_idx: usize) -> usize {
        if self.total_items == 0 {
            return current_idx;
        }

        if current_idx == self.total_items - 1 {
            return current_idx;
        }

        let page_info = self.get_page_info();
        let last_visible_idx = page_info.last_visible_idx;

        if current_idx == last_visible_idx && current_idx < self.total_items - 1 {
            let target_selection = (current_idx + 1).min(self.total_items - 1);
            self.scroll_offset = self.scroll_offset.saturating_add(1);

            loop {
                let temp_page_info = self.get_page_info();
                let new_last_visible_idx = temp_page_info.last_visible_idx;

                if target_selection <= new_last_visible_idx || target_selection >= self.total_items {
                    break;
                }
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }

            target_selection
        } else if current_idx < last_visible_idx {
            (current_idx + 1).min(self.total_items - 1)
        } else {
            current_idx
        }
    }

    pub fn clamp_selection_to_visible(&mut self, selected_idx: usize) {
        if self.total_items == 0 {
            return;
        }

        let render_start = if self.scroll_offset == 1 {
            0
        } else {
            self.scroll_offset
        };

        let page_info = self.get_page_info();
        let render_end = render_start + page_info.items_per_page;

        if selected_idx < render_start {
            self.scroll_offset = selected_idx;
        } else if selected_idx >= render_end {
            self.scroll_offset = selected_idx.saturating_sub(page_info.items_per_page - 1);
        }
    }

    fn calculate_viewport_info(&self) -> ViewportInfo {
        // Pure card positioning - page_size is the number of card items per page
        // All UI overhead (headers, indicators) is pre-accounted for by the caller

        // Special case: when scroll_offset == 1, we render from index 0 (no items above)
        let render_start = if self.scroll_offset == 1 {
            0
        } else {
            self.scroll_offset
        };

        let has_items_above = render_start > 0;
        let has_items_below = self.scroll_offset + self.page_size < self.total_items;

        ViewportInfo {
            has_items_above,
            has_items_below,
            items_to_show: self.page_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_info_empty() {
        let page = Page::new(0, 10);
        let info = page.get_page_info();

        assert!(info.visible_item_indices.is_empty());
        assert!(!info.show_above_indicator);
        assert!(!info.show_below_indicator);
    }

    #[test]
    fn test_page_info_fits_in_viewport() {
        let page = Page::new(5, 10);
        let info = page.get_page_info();

        assert_eq!(info.visible_item_indices, vec![0, 1, 2, 3, 4]);
        assert!(!info.show_above_indicator);
        assert!(!info.show_below_indicator);
    }

    #[test]
    fn test_page_info_with_below_indicator() {
        let page = Page::new(20, 10);
        let info = page.get_page_info();

        // viewport_height=10 is the true card space (no overhead)
        // so items_per_page should be 10
        assert_eq!(info.items_per_page, 10);
        assert!(!info.show_above_indicator);
        assert!(info.show_below_indicator);
        assert_eq!(info.items_below_count, 10);
    }

    #[test]
    fn test_page_info_with_above_indicator() {
        let mut page = Page::new(20, 10);
        page.scroll_offset = 6;
        let info = page.get_page_info();

        assert!(info.show_above_indicator);
        assert_eq!(info.items_above_count, 6);
        assert!(info.show_below_indicator);
    }

    #[test]
    fn test_jump_half_page_down_from_top_of_page() {
        let mut page = Page::new(100, 10);
        // Page 0 has items 0-8, middle = (0+8)/2 = 4
        // From item 0 (top half): jump to middle (4)
        let new_idx = page.jump_half_page_down(0);
        assert_eq!(new_idx, 4);
    }

    #[test]
    fn test_jump_half_page_down_from_middle_of_page() {
        let mut page = Page::new(100, 10);
        // viewport=10 means 10 items per page (pure cards, no overhead)
        // Page 0: items 0-9, middle = (0+9)/2 = 4
        // From item 4 (at middle): jump to bottom (9)
        let new_idx = page.jump_half_page_down(4);
        assert_eq!(new_idx, 9);
    }

    #[test]
    fn test_jump_half_page_down_from_bottom_of_page() {
        let mut page = Page::new(100, 10);
        // Page 0: items 0-9, middle = 4
        // From item 9 (at bottom): jump to first item of next page (10)
        let new_idx = page.jump_half_page_down(9);
        assert_eq!(new_idx, 10);
    }

    #[test]
    fn test_jump_half_page_down_three_step_navigation() {
        let mut page = Page::new(100, 10);
        // Page 0: items 0-9, middle = 4
        // Step 1: From top (0) → middle (4)
        let idx1 = page.jump_half_page_down(0);
        assert_eq!(idx1, 4);

        // Step 2: From middle (4) → bottom (9)
        let idx2 = page.jump_half_page_down(idx1);
        assert_eq!(idx2, 9);

        // Step 3: From bottom (9) → next page start (10)
        let idx3 = page.jump_half_page_down(idx2);
        assert_eq!(idx3, 10);
    }

    #[test]
    fn test_jump_half_page_up_from_bottom_of_page() {
        let mut page = Page::new(100, 10);
        // Page 0: items 0-9, middle = 4
        // From item 9 (bottom half): jump to middle (4)
        let new_idx = page.jump_half_page_up(9);
        assert_eq!(new_idx, 4);
    }

    #[test]
    fn test_jump_half_page_up_from_middle_of_page() {
        let mut page = Page::new(100, 10);
        // Page 0: items 0-9, middle = 4
        // From item 4 (at middle): jump to top (0)
        let new_idx = page.jump_half_page_up(4);
        assert_eq!(new_idx, 0);
    }

    #[test]
    fn test_jump_half_page_up_from_top_of_page() {
        let mut page = Page::new(100, 10);
        // Page 0: items 0-9, middle = 4
        // From item 0 (at top): jump to last item of previous page
        // But we're on first page, so stay at 0
        let new_idx = page.jump_half_page_up(0);
        assert_eq!(new_idx, 0);
    }

    #[test]
    fn test_jump_half_page_up_three_step_navigation() {
        let mut page = Page::new(100, 10);
        // Page 1: items 10-19, middle = (10+19)/2 = 14
        // Starting from item 19 (bottom of page 1)
        let idx1 = page.jump_half_page_up(19);
        assert_eq!(idx1, 14); // Jump to middle

        // From middle (14)
        let idx2 = page.jump_half_page_up(idx1);
        assert_eq!(idx2, 10); // Jump to top

        // From top (10)
        let idx3 = page.jump_half_page_up(idx2);
        assert_eq!(idx3, 9); // Jump to end of previous page
    }

    #[test]
    fn test_jump_multiple_pages_down() {
        let mut page = Page::new(100, 10);
        // With viewport 10:
        // Page 0: items 0-9, middle = 4
        // Page 1: items 10-19, middle = 14
        // Page 2: items 20-29, middle = 24

        // From top of page 0 -> middle -> bottom -> next page
        let idx1 = page.jump_half_page_down(0);   // 0 -> 4 (middle of page 0)
        assert_eq!(idx1, 4);
        let idx2 = page.jump_half_page_down(idx1); // 4 -> 9 (bottom of page 0)
        assert_eq!(idx2, 9);
        let idx3 = page.jump_half_page_down(idx2); // 9 -> 10 (start of page 1)
        assert_eq!(idx3, 10);
        let idx4 = page.jump_half_page_down(idx3); // 10 -> 14 (middle of page 1)
        assert_eq!(idx4, 14);
    }

    #[test]
    fn test_jump_multiple_pages_up() {
        let mut page = Page::new(100, 10);
        // With viewport 10:
        // Page 0: items 0-9, middle = 4
        // Page 1: items 10-19, middle = 14

        // From bottom of page 1 -> middle -> top -> previous page
        let idx1 = page.jump_half_page_up(19);    // 19 -> 14 (middle of page 1)
        assert_eq!(idx1, 14);
        let idx2 = page.jump_half_page_up(idx1);  // 14 -> 10 (top of page 1)
        assert_eq!(idx2, 10);
        let idx3 = page.jump_half_page_up(idx2);  // 10 -> 9 (end of page 0)
        assert_eq!(idx3, 9);
        let idx4 = page.jump_half_page_up(idx3);  // 9 -> 4 (middle of page 0)
        assert_eq!(idx4, 4);
    }

    #[test]
    fn test_scroll_to_make_visible_above() {
        let mut page = Page::new(100, 10);
        page.scroll_offset = 50;
        page.scroll_to_make_visible(40);

        assert_eq!(page.scroll_offset, 40);
    }

    #[test]
    fn test_scroll_to_make_visible_below() {
        let mut page = Page::new(100, 10);
        page.scroll_offset = 0;
        page.scroll_to_make_visible(15);

        assert!(page.scroll_offset > 0);
    }

    #[test]
    fn test_items_per_page_with_indicators() {
        let mut page = Page::new(100, 10);
        page.scroll_offset = 5;
        let info = page.get_page_info();

        // viewport_height=10 is the true card space (no overhead accounting in Page)
        assert!(info.show_above_indicator);
        assert!(info.show_below_indicator);
        assert_eq!(info.items_per_page, 10);
    }

    #[test]
    fn test_scroll_offset_stays_in_bounds() {
        let mut page = Page::new(10, 5);
        page.scroll_offset = 100;
        page.set_total_items(10);

        assert!(page.scroll_offset < 10);
    }

    #[test]
    fn test_navigate_up_from_middle() {
        let mut page = Page::new(20, 10);
        page.scroll_offset = 5;
        let new_idx = page.navigate_up(10);

        assert_eq!(new_idx, 9);
        assert_eq!(page.scroll_offset, 5);
    }

    #[test]
    fn test_navigate_up_at_top() {
        let mut page = Page::new(20, 10);
        let new_idx = page.navigate_up(0);

        assert_eq!(new_idx, 0);
        assert_eq!(page.scroll_offset, 0);
    }

    #[test]
    fn test_navigate_up_with_scroll_adjustment() {
        let mut page = Page::new(20, 10);
        page.scroll_offset = 5;
        let new_idx = page.navigate_up(5);

        assert_eq!(new_idx, 4);
        assert!(page.scroll_offset < 5);
    }

    #[test]
    fn test_navigate_down_from_middle() {
        let mut page = Page::new(20, 10);
        page.scroll_offset = 0;
        let new_idx = page.navigate_down(5);

        assert_eq!(new_idx, 6);
        assert_eq!(page.scroll_offset, 0);
    }

    #[test]
    fn test_navigate_down_at_bottom() {
        let mut page = Page::new(20, 10);
        let new_idx = page.navigate_down(19);

        assert_eq!(new_idx, 19);
        assert_eq!(page.scroll_offset, 0);
    }

    #[test]
    fn test_navigate_down_with_scroll_adjustment() {
        let mut page = Page::new(20, 10);
        page.scroll_offset = 0;
        let page_info = page.get_page_info();
        let last_visible = page_info.last_visible_idx;
        let new_idx = page.navigate_down(last_visible);

        assert!(new_idx > last_visible);
        assert!(page.scroll_offset > 0);
    }

    #[test]
    fn test_clamp_selection_to_visible_above() {
        let mut page = Page::new(100, 10);
        page.scroll_offset = 50;
        page.clamp_selection_to_visible(40);

        assert_eq!(page.scroll_offset, 40);
    }

    #[test]
    fn test_clamp_selection_to_visible_below() {
        let mut page = Page::new(100, 10);
        page.scroll_offset = 0;
        page.clamp_selection_to_visible(20);

        assert!(page.scroll_offset > 0);
    }

    #[test]
    fn test_clamp_selection_to_visible_within_range() {
        let mut page = Page::new(100, 10);
        page.scroll_offset = 5;
        page.clamp_selection_to_visible(10);

        assert_eq!(page.scroll_offset, 5);
    }

    #[test]
    fn test_indicator_disappears_at_top_with_scroll_offset_one() {
        let mut page = Page::new(20, 10);
        page.scroll_offset = 1;
        let info = page.get_page_info();

        // When scroll_offset == 1, render_start becomes 0 (no items above)
        assert!(!info.show_above_indicator, "Should not show above indicator when scroll_offset=1");
        assert_eq!(info.first_visible_idx, 0);
    }

    #[test]
    fn test_indicator_shows_with_scroll_offset_two() {
        let mut page = Page::new(20, 10);
        page.scroll_offset = 2;
        let info = page.get_page_info();

        // When scroll_offset == 2, items above indicator should show
        assert!(info.show_above_indicator, "Should show above indicator when scroll_offset=2");
        assert_eq!(info.items_above_count, 2);
    }

    #[test]
    fn test_jump_half_page_down_reaches_last_element() {
        let mut page = Page::new(25, 10);

        // Jump from item 0 to middle
        let idx1 = page.jump_half_page_down(0);
        assert_eq!(idx1, 4); // middle of page 0 (0-9)

        // Jump from middle to end of page 0
        let idx2 = page.jump_half_page_down(idx1);
        assert_eq!(idx2, 9);

        // Jump from end of page 0 to start of page 1
        let idx3 = page.jump_half_page_down(idx2);
        assert_eq!(idx3, 10);

        // Jump through page 1...
        let idx4 = page.jump_half_page_down(idx3);
        assert_eq!(idx4, 14); // middle of page 1 (10-19)

        let idx5 = page.jump_half_page_down(idx4);
        assert_eq!(idx5, 19); // end of page 1

        // Jump to start of page 2
        let idx6 = page.jump_half_page_down(idx5);
        assert_eq!(idx6, 20); // start of page 2 (20-24)

        // Jump through page 2
        let idx7 = page.jump_half_page_down(idx6);
        assert_eq!(idx7, 22); // middle of page 2 (20-24)

        let idx8 = page.jump_half_page_down(idx7);
        assert_eq!(idx8, 24); // Should reach the last item!
    }

    #[test]
    fn test_jump_half_page_down_reaches_last_with_smaller_last_page() {
        let mut page = Page::new(23, 8);

        // With page_size=8: pages are 0-7, 8-15, 16-22
        // Jump from the end of page 1 (15) should eventually reach 22
        let mut current = 0;
        let mut iterations = 0;

        while current != 22 && iterations < 10 {
            current = page.jump_half_page_down(current);
            iterations += 1;
        }

        assert_eq!(current, 22, "Should reach last item (22)");
    }

    #[test]
    fn test_page_info_no_above_indicator_at_zero_scroll() {
        let page = Page::new(100, 10);
        let info = page.get_page_info();

        assert!(!info.show_above_indicator);
        assert_eq!(info.items_above_count, 0);
    }

    #[test]
    fn test_page_info_has_above_indicator_after_scrolling() {
        let mut page = Page::new(100, 10);
        page.scroll_offset = 5;
        let info = page.get_page_info();

        assert!(info.show_above_indicator);
        assert_eq!(info.items_above_count, 5);
    }

    #[test]
    fn test_jump_from_last_page_to_absolute_last_element() {
        let mut page = Page::new(17, 8);

        // Pages: 0-7, 8-15, 16-16
        // Start at item 16 (last page with only 1 item)
        let current = 16;

        // This should stay at 16 since it's already the last item
        let result = page.jump_half_page_down(current);
        assert_eq!(result, 16);
    }

    #[test]
    fn test_jump_from_second_to_last_to_last() {
        let mut page = Page::new(20, 9);

        // Pages: 0-8, 9-17, 18-19
        // Start at item 18 (last page)
        let idx1 = page.jump_half_page_down(18);

        // Middle of last page (18-19) would be... let's see: (18+19)/2 = 18
        // So current_idx (18) < middle_idx? No, they're equal
        // So it falls through to "At or past bottom" case
        // Since we're on the last page, it should try to reach 19
        assert_eq!(idx1, 19, "Should reach the absolute last element");
    }
}
