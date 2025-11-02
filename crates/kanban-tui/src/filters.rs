use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum FilterDialogSection {
    Sprints,
    DateRange,
    Tags,
}

#[derive(Debug, Clone, Default)]
pub struct CardFilters {
    pub show_unassigned_sprints: bool,
    pub selected_sprint_ids: HashSet<uuid::Uuid>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub selected_tags: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct FilterDialogState {
    pub current_section: FilterDialogSection,
    pub section_index: usize,
    pub item_selection: usize,
    pub filters: CardFilters,
}

impl FilterDialogState {
    pub fn new(filters: CardFilters) -> Self {
        Self {
            current_section: FilterDialogSection::Sprints,
            section_index: 0,
            item_selection: 0,
            filters,
        }
    }

    pub fn next_section(&mut self) {
        self.section_index = (self.section_index + 1) % 3;
        self.item_selection = 0;
        self.current_section = match self.section_index {
            0 => FilterDialogSection::Sprints,
            1 => FilterDialogSection::DateRange,
            2 => FilterDialogSection::Tags,
            _ => FilterDialogSection::Sprints,
        };
    }

    pub fn prev_section(&mut self) {
        self.section_index = if self.section_index == 0 {
            2
        } else {
            self.section_index - 1
        };
        self.item_selection = 0;
        self.current_section = match self.section_index {
            0 => FilterDialogSection::Sprints,
            1 => FilterDialogSection::DateRange,
            2 => FilterDialogSection::Tags,
            _ => FilterDialogSection::Sprints,
        };
    }
}
