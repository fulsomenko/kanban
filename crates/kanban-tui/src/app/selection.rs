use kanban_core::SelectionState;
use std::cell::Cell;

#[derive(Default)]
pub struct SelectionHub {
    pub board: SelectionState,
    pub active_board_index: Option<usize>,
    pub active_card_id: Option<uuid::Uuid>,
    pub sprint: SelectionState,
    pub sprint_scroll: Cell<usize>,
    pub active_sprint_index: Option<usize>,
    pub card_navigation_history: Vec<uuid::Uuid>,
    pub settings_config: SelectionState,
    pub settings_config_file: SelectionState,
    pub settings_storage: SelectionState,
}
