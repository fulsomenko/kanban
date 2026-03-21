use kanban_core::SelectionState;

pub struct DialogInputState {
    pub import_files: Vec<String>,
    pub import_selection: SelectionState,
    pub priority_selection: SelectionState,
    pub column_selection: SelectionState,
    pub sprint_assign_selection: SelectionState,
    pub task_list_view_selection: SelectionState,
}

impl DialogInputState {
    pub fn new() -> Self {
        Self {
            import_files: Vec::new(),
            import_selection: SelectionState::new(),
            priority_selection: SelectionState::new(),
            column_selection: SelectionState::new(),
            sprint_assign_selection: SelectionState::new(),
            task_list_view_selection: SelectionState::new(),
        }
    }
}

impl Default for DialogInputState {
    fn default() -> Self {
        Self::new()
    }
}
