use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum FilterDialogSection {
    UnassignedSprints,
    Sprints,
    DateRange,
}

#[derive(Debug, Clone, Default)]
pub struct CardFilters {
    pub show_unassigned_sprints: bool,
    pub selected_sprint_ids: HashSet<uuid::Uuid>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FilterDialogState {
    pub current_section: FilterDialogSection,
    pub section_selection: usize,
    pub filters: CardFilters,
}

impl FilterDialogState {
    pub fn new(filters: CardFilters) -> Self {
        Self {
            current_section: FilterDialogSection::UnassignedSprints,
            section_selection: 0,
            filters,
        }
    }
}
