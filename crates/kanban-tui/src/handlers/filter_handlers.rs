use crate::app::{App, AppMode, Focus};
use crate::filters::{CardFilters, FilterDialogState, FilterDialogSection};
use crossterm::event::KeyCode;

impl App {
    pub fn handle_open_filter_dialog(&mut self) {
        if self.focus != Focus::Cards || self.active_board_index.is_none() {
            return;
        }

        let filters = CardFilters {
            show_unassigned_sprints: self.hide_assigned_cards,
            selected_sprint_ids: self.active_sprint_filter.iter().cloned().collect(),
            date_from: None,
            date_to: None,
        };

        self.filter_dialog_state = Some(FilterDialogState::new(filters));
        self.mode = AppMode::FilterOptions;
    }

    pub fn handle_filter_options_popup(&mut self, key_code: KeyCode) {
        use crossterm::event::KeyCode;

        if let Some(ref mut dialog_state) = self.filter_dialog_state {
            match key_code {
                KeyCode::Esc => {
                    self.filter_dialog_state = None;
                    self.mode = AppMode::Normal;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    dialog_state.section_selection = (dialog_state.section_selection + 1) % 3;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    dialog_state.section_selection = if dialog_state.section_selection == 0 {
                        2
                    } else {
                        dialog_state.section_selection - 1
                    };
                }
                KeyCode::Char(' ') => {
                    match dialog_state.current_section {
                        FilterDialogSection::UnassignedSprints => {
                            dialog_state.filters.show_unassigned_sprints =
                                !dialog_state.filters.show_unassigned_sprints;
                        }
                        _ => {}
                    }
                }
                KeyCode::Enter => {
                    self.apply_filters();
                    self.filter_dialog_state = None;
                    self.mode = AppMode::Normal;
                }
                _ => {}
            }
        }
    }

    fn apply_filters(&mut self) {
        if let Some(dialog_state) = &self.filter_dialog_state {
            self.hide_assigned_cards = dialog_state.filters.show_unassigned_sprints;

            if !dialog_state.filters.selected_sprint_ids.is_empty() {
                if let Some(sprint_id) = dialog_state.filters.selected_sprint_ids.iter().next() {
                    self.active_sprint_filter = Some(*sprint_id);
                }
            } else {
                self.active_sprint_filter = None;
            }

            self.refresh_view();
            tracing::info!("Applied filters");
        }
    }
}
