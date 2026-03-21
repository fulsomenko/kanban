use std::collections::HashSet;
use uuid::Uuid;

pub struct MultiSelectState {
    pub selected_cards: HashSet<Uuid>,
    pub selection_mode_active: bool,
}

impl MultiSelectState {
    pub fn new() -> Self {
        Self {
            selected_cards: HashSet::new(),
            selection_mode_active: false,
        }
    }
}

impl Default for MultiSelectState {
    fn default() -> Self {
        Self::new()
    }
}
