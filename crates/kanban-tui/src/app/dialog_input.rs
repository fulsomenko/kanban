use crate::components::sprint_picker::SprintPicker;
use kanban_core::SelectionState;
use std::cell::Cell;
use uuid::Uuid;

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
}
