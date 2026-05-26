use crate::components::sprint_picker::SprintPicker;
use kanban_core::SelectionState;
use std::cell::Cell;
use uuid::Uuid;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CreateCardFocus {
    #[default]
    Title,
    Sprint,
}

#[derive(Default)]
pub struct DialogInputState {
    pub import_files: Vec<String>,
    pub import_selection: SelectionState,
    pub priority_selection: SelectionState,
    pub column_selection: SelectionState,
    pub column_scroll: Cell<usize>,
    pub sprint_assign_selection: SelectionState,
    pub task_list_view_selection: SelectionState,
    pub carry_over_sprint_selection: SelectionState,
    pub carry_over_source_sprint_id: Option<Uuid>,
    pub create_card_sprint_picker: SprintPicker,
    pub create_card_focus: CreateCardFocus,
}

impl DialogInputState {
    pub fn create_card_focus_is_title(&self) -> bool {
        self.create_card_focus == CreateCardFocus::Title
    }

    pub fn toggle_create_card_focus(&mut self) {
        self.create_card_focus = match self.create_card_focus {
            CreateCardFocus::Title => CreateCardFocus::Sprint,
            CreateCardFocus::Sprint => CreateCardFocus::Title,
        };
    }

    pub fn reset_create_card_focus(&mut self) {
        self.create_card_focus = CreateCardFocus::Title;
    }
}
