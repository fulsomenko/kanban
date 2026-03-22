use kanban_core::SelectionState;

#[derive(Default)]
pub struct DialogInputState {
    pub import_files: Vec<String>,
    pub import_selection: SelectionState,
    pub priority_selection: SelectionState,
    pub column_selection: SelectionState,
    pub sprint_assign_selection: SelectionState,
    pub task_list_view_selection: SelectionState,
}
