//! Generic selection state utilities.
//!
//! Provides a reusable single-selection state machine that can be used
//! by any UI framework for list selection.

/// State for single-item selection in a list.
#[derive(Clone, Debug, Default)]
pub struct SelectionState {
    selected_index: Option<usize>,
}

impl SelectionState {
    /// Create a new selection state with no selection.
    pub fn new() -> Self {
        Self {
            selected_index: None,
        }
    }

    /// Get the currently selected index.
    pub fn get(&self) -> Option<usize> {
        self.selected_index
    }

    /// Set the selected index.
    pub fn set(&mut self, index: Option<usize>) {
        self.selected_index = index;
    }

    /// Clear the selection.
    pub fn clear(&mut self) {
        self.selected_index = None;
    }

    /// Move selection to the next item.
    pub fn next(&mut self, max_count: usize) {
        if max_count == 0 {
            return;
        }
        self.selected_index = Some(match self.selected_index {
            Some(idx) => (idx + 1).min(max_count - 1),
            None => 0,
        });
    }

    /// Move selection to the previous item.
    pub fn prev(&mut self) {
        self.selected_index = Some(match self.selected_index {
            Some(idx) => idx.saturating_sub(1),
            None => 0,
        });
    }

    /// Auto-select the first item if nothing is selected and items exist.
    pub fn auto_select_first_if_empty(&mut self, has_items: bool) {
        if self.selected_index.is_none() && has_items {
            self.selected_index = Some(0);
        }
    }

    /// Jump to the first item.
    pub fn jump_to_first(&mut self) {
        self.selected_index = Some(0);
    }

    /// Jump to the last item.
    pub fn jump_to_last(&mut self, len: usize) {
        if len > 0 {
            self.selected_index = Some(len - 1);
        }
    }

    /// Check if an index is selected.
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_index == Some(index)
    }

    /// Check if anything is selected.
    pub fn has_selection(&self) -> bool {
        self.selected_index.is_some()
    }

    /// Clamp selection to valid range after list size changes.
    pub fn clamp(&mut self, max_count: usize) {
        if let Some(idx) = self.selected_index {
            if max_count == 0 {
                self.selected_index = None;
            } else if idx >= max_count {
                self.selected_index = Some(max_count - 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_selection_is_empty() {
        let selection = SelectionState::new();
        assert!(selection.get().is_none());
        assert!(!selection.has_selection());
    }

    #[test]
    fn test_set_and_get() {
        let mut selection = SelectionState::new();
        selection.set(Some(5));
        assert_eq!(selection.get(), Some(5));
        assert!(selection.has_selection());
    }

    #[test]
    fn test_clear() {
        let mut selection = SelectionState::new();
        selection.set(Some(5));
        selection.clear();
        assert!(selection.get().is_none());
    }

    #[test]
    fn test_next() {
        let mut selection = SelectionState::new();

        // From None
        selection.next(5);
        assert_eq!(selection.get(), Some(0));

        // Normal increment
        selection.next(5);
        assert_eq!(selection.get(), Some(1));

        // At boundary
        selection.set(Some(4));
        selection.next(5);
        assert_eq!(selection.get(), Some(4));
    }

    #[test]
    fn test_prev() {
        let mut selection = SelectionState::new();

        // From None
        selection.prev();
        assert_eq!(selection.get(), Some(0));

        // Normal decrement
        selection.set(Some(3));
        selection.prev();
        assert_eq!(selection.get(), Some(2));

        // At boundary
        selection.set(Some(0));
        selection.prev();
        assert_eq!(selection.get(), Some(0));
    }

    #[test]
    fn test_auto_select_first() {
        let mut selection = SelectionState::new();

        // With items
        selection.auto_select_first_if_empty(true);
        assert_eq!(selection.get(), Some(0));

        // Already selected - no change
        selection.set(Some(5));
        selection.auto_select_first_if_empty(true);
        assert_eq!(selection.get(), Some(5));

        // No items
        let mut selection2 = SelectionState::new();
        selection2.auto_select_first_if_empty(false);
        assert!(selection2.get().is_none());
    }

    #[test]
    fn test_jump_to_first_last() {
        let mut selection = SelectionState::new();

        selection.set(Some(5));
        selection.jump_to_first();
        assert_eq!(selection.get(), Some(0));

        selection.jump_to_last(10);
        assert_eq!(selection.get(), Some(9));

        // Empty list
        selection.jump_to_last(0);
        assert_eq!(selection.get(), Some(9)); // No change
    }

    #[test]
    fn test_is_selected() {
        let mut selection = SelectionState::new();
        selection.set(Some(3));

        assert!(selection.is_selected(3));
        assert!(!selection.is_selected(0));
        assert!(!selection.is_selected(5));
    }

    #[test]
    fn test_clamp() {
        let mut selection = SelectionState::new();
        selection.set(Some(10));

        // Clamp to smaller size
        selection.clamp(5);
        assert_eq!(selection.get(), Some(4));

        // Clamp to empty
        selection.clamp(0);
        assert!(selection.get().is_none());

        // Clamp when within range - no change
        selection.set(Some(3));
        selection.clamp(10);
        assert_eq!(selection.get(), Some(3));
    }

    #[test]
    fn test_next_empty_list() {
        let mut selection = SelectionState::new();
        selection.next(0);
        assert!(selection.get().is_none());
    }
}
