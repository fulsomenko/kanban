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

use crate::layout_strategy::ColumnBoundary;

#[derive(Debug, Clone)]
struct ComputedPage {
    start_index: usize,
    end_index: usize,
    scroll_offset: usize,  // Where to scroll to display this page
}

#[derive(Clone)]
pub struct Page {
    pub total_items: usize,
    pub viewport_height: usize,
    pub scroll_offset: usize,
    current_page: usize, // explicit page tracking
    column_boundaries: Vec<ColumnBoundary>, // For header accounting
    computed_pages: Vec<ComputedPage>, // Pre-computed page boundaries
}

#[derive(Debug, Clone)]
struct ViewportInfo {
    has_items_above: bool,
    has_items_below: bool,
    items_to_show: usize,
}

impl Page {
    pub fn new(total_items: usize, viewport_height: usize) -> Self {
        let mut page = Self {
            total_items,
            viewport_height,
            scroll_offset: 0,
            current_page: 0,
            column_boundaries: Vec::new(),
            computed_pages: Vec::new(),
        };
        page.recompute_pages();
        page
    }

    pub fn set_column_boundaries(&mut self, boundaries: Vec<ColumnBoundary>) {
        self.column_boundaries = boundaries;
        self.recompute_pages();
    }

    /// Pre-compute all page boundaries based on current viewport, items, and headers
    fn recompute_pages(&mut self) {
        self.computed_pages = self.compute_pages();
    }

    /// Compute page boundaries accounting for headers and indicators
    fn compute_pages(&self) -> Vec<ComputedPage> {
        if self.total_items == 0 {
            return Vec::new();
        }

        let mut pages = Vec::new();
        let mut current_scroll = 0;

        while current_scroll < self.total_items {
            // Calculate overhead at this scroll position
            let header_count = self.count_headers_at_offset(current_scroll);
            let has_below = current_scroll + self.viewport_height < self.total_items;
            let has_above = current_scroll > 0;
            let indicator_count = (has_above as usize) + (has_below as usize);
            let total_overhead = header_count + indicator_count;

            // Items that fit on this page
            let items_per_page = self.viewport_height.saturating_sub(total_overhead);

            if items_per_page == 0 {
                // Edge case: viewport too small or all overhead
                break;
            }

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

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
        self.sync_page_from_scroll();
        self.recompute_pages();
    }

    /// Sync current_page from scroll_offset
    fn sync_page_from_scroll(&mut self) {
        if self.viewport_height == 0 {
            self.current_page = 0;
        } else {
            self.current_page = self.scroll_offset / self.viewport_height;
        }
    }

    pub fn get_current_page(&self) -> usize {
        self.current_page
    }

    pub fn get_total_pages(&self) -> usize {
        if self.total_items == 0 || self.viewport_height == 0 {
            return 0;
        }
        self.total_items.div_ceil(self.viewport_height)
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
                }
            }
        }

        // Already on last page or not on any page, stay where we are
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

        let jump_distance = self.viewport_height;
        let new_idx = (current_idx + jump_distance).min(self.total_items - 1);

        // Check if jumping to a new page
        let current_page = current_idx / self.viewport_height;
        let new_page = new_idx / self.viewport_height;

        if new_page > current_page {
            // Jumping to next page: scroll to show it
            self.scroll_offset = new_page * self.viewport_height;
        }

        new_idx
    }

    pub fn jump_full_page_up(&mut self, current_idx: usize) -> usize {
        if self.total_items == 0 {
            return 0;
        }

        let jump_distance = self.viewport_height;
        let new_idx = current_idx.saturating_sub(jump_distance);

        // Check if jumping to a previous page
        let current_page = current_idx / self.viewport_height;
        let new_page = new_idx / self.viewport_height;

        if new_page < current_page {
            // Jumping to previous page: scroll to show it
            self.scroll_offset = new_page * self.viewport_height;
        }

        new_idx
    }

    pub fn scroll_to_make_visible(&mut self, item_idx: usize) {
        if self.viewport_height == 0 {
            return;
        }

        let scroll_start = self.scroll_offset;
        let scroll_end = scroll_start + self.viewport_height;

        if item_idx < scroll_start {
            self.scroll_offset = item_idx;
        } else if item_idx >= scroll_end {
            self.scroll_offset = item_idx.saturating_sub(self.viewport_height - 1);
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

    /// Count how many column headers will appear in the viewport at the given scroll offset
    fn count_headers_at_offset(&self, scroll_offset: usize) -> usize {
        if self.column_boundaries.is_empty() {
            return 0;
        }

        // Count how many column headers will appear in the viewport
        // A boundary is relevant if any of its cards appear in the visible range
        let viewport_end = scroll_offset + self.viewport_height;

        self.column_boundaries
            .iter()
            .filter(|boundary| {
                let boundary_end = boundary.start_index + boundary.card_count;
                // Boundary is relevant if it overlaps with viewport range
                boundary.start_index < viewport_end && boundary_end > scroll_offset
            })
            .count()
    }

    fn calculate_viewport_info(&self) -> ViewportInfo {
        let has_items_above = self.scroll_offset > 1;
        let available_space = self.viewport_height.saturating_sub(has_items_above as usize);
        let has_items_below = self.scroll_offset + available_space < self.total_items;

        // Account for both indicators and column headers
        let header_count = self.count_headers_at_offset(self.scroll_offset);
        let num_indicator_lines = (has_items_above as usize) + (has_items_below as usize);
        let total_overhead_lines = header_count + num_indicator_lines;
        let items_to_show = self.viewport_height.saturating_sub(total_overhead_lines);

        ViewportInfo {
            has_items_above,
            has_items_below,
            items_to_show,
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

        assert_eq!(info.items_per_page, 9);
        assert!(!info.show_above_indicator);
        assert!(info.show_below_indicator);
        assert_eq!(info.items_below_count, 11);
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
        // Page 0 has items 0-8, middle = 4
        // From item 4 (at middle): jump to bottom (8)
        let new_idx = page.jump_half_page_down(4);
        assert_eq!(new_idx, 8);
    }

    #[test]
    fn test_jump_half_page_down_from_bottom_of_page() {
        let mut page = Page::new(100, 10);
        // Page 0 has items 0-8, middle = 4
        // From item 8 (at bottom): jump to first item of next page (9)
        let new_idx = page.jump_half_page_down(8);
        assert_eq!(new_idx, 9);
    }

    #[test]
    fn test_jump_half_page_down_three_step_navigation() {
        let mut page = Page::new(100, 10);
        // Page 0: items 0-8, middle = 4
        // Step 1: From top (0) → middle (4)
        let idx1 = page.jump_half_page_down(0);
        assert_eq!(idx1, 4);

        // Step 2: From middle (4) → bottom (8)
        let idx2 = page.jump_half_page_down(idx1);
        assert_eq!(idx2, 8);

        // Step 3: From bottom (8) → next page start (9)
        let idx3 = page.jump_half_page_down(idx2);
        assert_eq!(idx3, 9);
    }

    #[test]
    fn test_jump_half_page_up_from_bottom_of_page() {
        let mut page = Page::new(100, 10);
        // Page 0 has items 0-8, middle = 4
        // From item 8 (bottom half): jump to middle (4)
        let new_idx = page.jump_half_page_up(8);
        assert_eq!(new_idx, 4);
    }

    #[test]
    fn test_jump_half_page_up_from_middle_of_page() {
        let mut page = Page::new(100, 10);
        // Page 0 has items 0-8, middle = 4
        // From item 4 (at middle): jump to top (0)
        let new_idx = page.jump_half_page_up(4);
        assert_eq!(new_idx, 0);
    }

    #[test]
    fn test_jump_half_page_up_from_top_of_page() {
        let mut page = Page::new(100, 10);
        // Page 0 has items 0-8, middle = 4
        // From item 0 (at top): jump to last item of previous page
        // But we're on first page, so stay at 0
        let new_idx = page.jump_half_page_up(0);
        assert_eq!(new_idx, 0);
    }

    #[test]
    fn test_jump_half_page_up_three_step_navigation() {
        let mut page = Page::new(100, 10);
        // Page 1: items 9-16, middle = (9+16)/2 = 12
        // Starting from item 16 (bottom of page 1)
        let idx1 = page.jump_half_page_up(16);
        assert_eq!(idx1, 12); // Jump to middle

        // From middle (12)
        let idx2 = page.jump_half_page_up(idx1);
        assert_eq!(idx2, 9); // Jump to top

        // From top (9)
        let idx3 = page.jump_half_page_up(idx2);
        assert_eq!(idx3, 8); // Jump to end of previous page
    }

    #[test]
    fn test_jump_multiple_pages_down() {
        let mut page = Page::new(100, 10);
        // With viewport 10:
        // Page 0: items 0-8, middle = 4
        // Page 1: items 9-16, middle = 12
        // Page 2: items 17-24, middle = 20

        // From top of page 0 -> middle -> bottom -> next page
        let idx1 = page.jump_half_page_down(0);   // 0 -> 4 (middle of page 0)
        assert_eq!(idx1, 4);
        let idx2 = page.jump_half_page_down(idx1); // 4 -> 8 (bottom of page 0)
        assert_eq!(idx2, 8);
        let idx3 = page.jump_half_page_down(idx2); // 8 -> 9 (start of page 1)
        assert_eq!(idx3, 9);
        let idx4 = page.jump_half_page_down(idx3); // 9 -> 12 (middle of page 1)
        assert_eq!(idx4, 12);
    }

    #[test]
    fn test_jump_multiple_pages_up() {
        let mut page = Page::new(100, 10);
        // With viewport 10:
        // Page 0: items 0-8, middle = 4
        // Page 1: items 9-16, middle = 12

        // From bottom of page 1 -> middle -> top -> previous page
        let idx1 = page.jump_half_page_up(16);    // 16 -> 12 (middle of page 1)
        assert_eq!(idx1, 12);
        let idx2 = page.jump_half_page_up(idx1);  // 12 -> 9 (top of page 1)
        assert_eq!(idx2, 9);
        let idx3 = page.jump_half_page_up(idx2);  // 9 -> 8 (end of page 0)
        assert_eq!(idx3, 8);
        let idx4 = page.jump_half_page_up(idx3);  // 8 -> 4 (middle of page 0)
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

        assert!(info.show_above_indicator);
        assert!(info.show_below_indicator);
        assert_eq!(info.items_per_page, 8);
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
}
