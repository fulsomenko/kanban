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
}

#[derive(Clone)]
pub struct Page {
    pub total_items: usize,
    pub viewport_height: usize,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone)]
struct ViewportInfo {
    has_items_above: bool,
    has_items_below: bool,
    items_to_show: usize,
}

impl Page {
    pub fn new(total_items: usize, viewport_height: usize) -> Self {
        Self {
            total_items,
            viewport_height,
            scroll_offset: 0,
        }
    }

    pub fn set_total_items(&mut self, total_items: usize) {
        self.total_items = total_items;
        if self.scroll_offset > total_items.saturating_sub(1) {
            self.scroll_offset = total_items.saturating_sub(1);
        }
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
    }

    pub fn get_page_info(&self) -> PageInfo {
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
        }
    }

    pub fn jump_half_page_down(&mut self, current_idx: usize) -> usize {
        if self.total_items == 0 {
            return 0;
        }

        let page_info = self.get_page_info();
        let jump_distance = page_info.items_per_page / 2;

        let new_idx = (current_idx + jump_distance).min(self.total_items - 1);

        self.scroll_to_make_visible(new_idx);

        new_idx
    }

    pub fn jump_half_page_up(&mut self, current_idx: usize) -> usize {
        if self.total_items == 0 {
            return 0;
        }

        let page_info = self.get_page_info();
        let jump_distance = page_info.items_per_page / 2;

        let new_idx = current_idx.saturating_sub(jump_distance);

        self.scroll_to_make_visible(new_idx);

        new_idx
    }

    pub fn jump_full_page_down(&mut self, current_idx: usize) -> usize {
        if self.total_items == 0 {
            return 0;
        }

        let page_info = self.get_page_info();
        let jump_distance = page_info.items_per_page;

        let new_idx = (current_idx + jump_distance).min(self.total_items - 1);

        self.scroll_to_make_visible(new_idx);

        new_idx
    }

    pub fn jump_full_page_up(&mut self, current_idx: usize) -> usize {
        if self.total_items == 0 {
            return 0;
        }

        let page_info = self.get_page_info();
        let jump_distance = page_info.items_per_page;

        let new_idx = current_idx.saturating_sub(jump_distance);

        self.scroll_to_make_visible(new_idx);

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
    }

    fn calculate_viewport_info(&self) -> ViewportInfo {
        let has_items_above = self.scroll_offset > 1;
        let available_space = self.viewport_height.saturating_sub(has_items_above as usize);
        let has_items_below = self.scroll_offset + available_space < self.total_items;

        let num_indicator_lines = (has_items_above as usize) + (has_items_below as usize);
        let items_to_show = self.viewport_height.saturating_sub(num_indicator_lines);

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
    fn test_jump_half_page_down_from_start() {
        let mut page = Page::new(100, 10);
        let new_idx = page.jump_half_page_down(0);

        assert_eq!(new_idx, 4);
        assert_eq!(page.scroll_offset, 0);
    }

    #[test]
    fn test_jump_half_page_down_middle() {
        let mut page = Page::new(100, 10);
        let new_idx = page.jump_half_page_down(25);

        assert_eq!(new_idx, 29);
    }

    #[test]
    fn test_jump_half_page_down_near_end() {
        let mut page = Page::new(100, 10);
        let new_idx = page.jump_half_page_down(95);

        assert_eq!(new_idx, 99);
    }

    #[test]
    fn test_jump_half_page_up_from_middle() {
        let mut page = Page::new(100, 10);
        page.scroll_offset = 20;
        let new_idx = page.jump_half_page_up(50);

        assert_eq!(new_idx, 46);
    }

    #[test]
    fn test_jump_half_page_up_from_start() {
        let mut page = Page::new(100, 10);
        let new_idx = page.jump_half_page_up(5);

        assert_eq!(new_idx, 1);
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
}
