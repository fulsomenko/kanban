use kanban_core::SelectionState;

pub struct SelectionHub {
    pub board: SelectionState,
    pub active_board_index: Option<usize>,
    pub active_card_index: Option<usize>,
    pub sprint: SelectionState,
    pub active_sprint_index: Option<usize>,
}

impl SelectionHub {
    pub fn new() -> Self {
        Self {
            board: SelectionState::new(),
            active_board_index: None,
            active_card_index: None,
            sprint: SelectionState::new(),
            active_sprint_index: None,
        }
    }
}

impl Default for SelectionHub {
    fn default() -> Self {
        Self::new()
    }
}
