use kanban_core::SelectionState;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActiveCard {
    index: usize,
    id: uuid::Uuid,
}

impl ActiveCard {
    pub fn new(index: usize, id: uuid::Uuid) -> Self {
        Self { index, id }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn id(&self) -> uuid::Uuid {
        self.id
    }
}

#[derive(Default)]
pub struct SelectionHub {
    pub board: SelectionState,
    pub active_board_index: Option<usize>,
    pub active_card: Option<ActiveCard>,
    pub sprint: SelectionState,
    pub sprint_scroll: Cell<usize>,
    pub active_sprint_index: Option<usize>,
    pub card_navigation_history: Vec<usize>,
    pub settings_config: SelectionState,
    pub settings_config_file: SelectionState,
    pub settings_storage: SelectionState,
}
