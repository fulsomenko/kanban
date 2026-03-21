use std::collections::HashSet;
use uuid::Uuid;
use kanban_core::SelectionState;
use kanban_domain::{SortField, SortOrder};
use crate::filters::FilterDialogState;
use crate::search::SearchState;

pub struct FilterState {
    pub active_sprint_filters: HashSet<Uuid>,
    pub hide_assigned_cards: bool,
    pub current_sort_field: Option<SortField>,
    pub current_sort_order: Option<SortOrder>,
    pub sort_field_selection: SelectionState,
    pub search: SearchState,
    pub dialog_state: Option<FilterDialogState>,
}

impl FilterState {
    pub fn new() -> Self {
        Self {
            active_sprint_filters: HashSet::new(),
            hide_assigned_cards: false,
            current_sort_field: None,
            current_sort_order: None,
            sort_field_selection: SelectionState::new(),
            search: SearchState::new(),
            dialog_state: None,
        }
    }
}

impl Default for FilterState {
    fn default() -> Self {
        Self::new()
    }
}
