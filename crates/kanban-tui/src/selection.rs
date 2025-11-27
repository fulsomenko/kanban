#[derive(Clone)]
pub struct SelectionState {
    selected_index: Option<usize>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self {
            selected_index: None,
        }
    }

    pub fn get(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn set(&mut self, index: Option<usize>) {
        self.selected_index = index;
    }

    pub fn clear(&mut self) {
        self.selected_index = None;
    }

    pub fn next(&mut self, max_count: usize) {
        if max_count == 0 {
            return;
        }
        self.selected_index = Some(match self.selected_index {
            Some(idx) => (idx + 1).min(max_count - 1),
            None => 0,
        });
    }

    pub fn prev(&mut self) {
        self.selected_index = Some(match self.selected_index {
            Some(idx) => idx.saturating_sub(1),
            None => 0,
        });
    }

    pub fn auto_select_first_if_empty(&mut self, has_items: bool) {
        if self.selected_index.is_none() && has_items {
            self.selected_index = Some(0);
        }
    }

    pub fn jump_to_first(&mut self) {
        self.selected_index = Some(0);
    }

    pub fn jump_to_last(&mut self, len: usize) {
        if len > 0 {
            self.selected_index = Some(len - 1);
        }
    }
}

impl Default for SelectionState {
    fn default() -> Self {
        Self::new()
    }
}
