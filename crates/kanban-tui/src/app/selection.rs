use kanban_core::SelectionState;

#[derive(Default)]
pub struct SelectionHub {
    pub board: SelectionState,
    pub active_board_index: Option<usize>,
    pub active_card_index: Option<usize>,
    pub sprint: SelectionState,
    pub active_sprint_index: Option<usize>,
    pub card_navigation_history: Vec<usize>,
}
