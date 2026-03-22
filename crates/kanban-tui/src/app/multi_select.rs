use std::collections::HashSet;
use uuid::Uuid;

#[derive(Default)]
pub struct MultiSelectState {
    pub selected_cards: HashSet<Uuid>,
    pub selection_mode_active: bool,
}

impl MultiSelectState {
    pub fn new() -> Self {
        Default::default()
    }
}
