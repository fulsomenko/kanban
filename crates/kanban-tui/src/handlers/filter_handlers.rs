use crate::app::{App, AppMode, Focus};
use crate::filters::{CardFilters, FilterDialogSection, FilterDialogState};
use crossterm::event::KeyCode;

impl App {
    pub fn handle_open_filter_dialog(&mut self) {
        if self.focus != Focus::Cards || self.active_board_index.is_none() {
            return;
        }

        let filters = CardFilters {
            show_unassigned_sprints: self.hide_assigned_cards,
            selected_sprint_ids: self.active_sprint_filters.clone(),
            date_from: None,
            date_to: None,
            selected_tags: Default::default(),
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
                KeyCode::Char('j') | KeyCode::Down => match dialog_state.current_section {
                    FilterDialogSection::Sprints => {
                        if let Some(board_idx) = self.active_board_index {
                            if let Some(board) = self.boards.get(board_idx) {
                                let sprint_count = self
                                    .sprints
                                    .iter()
                                    .filter(|s| s.board_id == board.id)
                                    .count();
                                let total_items = 1 + sprint_count;
                                if dialog_state.item_selection < total_items.saturating_sub(1) {
                                    dialog_state.item_selection += 1;
                                } else {
                                    dialog_state.next_section();
                                }
                            }
                        }
                    }
                    _ => {
                        dialog_state.next_section();
                    }
                },
                KeyCode::Char('k') | KeyCode::Up => match dialog_state.current_section {
                    FilterDialogSection::Sprints => {
                        if dialog_state.item_selection > 0 {
                            dialog_state.item_selection -= 1;
                        } else {
                            dialog_state.prev_section();
                        }
                    }
                    _ => {
                        dialog_state.prev_section();
                    }
                },
                KeyCode::Char(' ') => {
                    if dialog_state.current_section == FilterDialogSection::Sprints {
                        if dialog_state.item_selection == 0 {
                            dialog_state.filters.show_unassigned_sprints =
                                !dialog_state.filters.show_unassigned_sprints;
                            tracing::info!(
                                "Toggled unassigned sprints filter: {}",
                                dialog_state.filters.show_unassigned_sprints
                            );
                            self.apply_filters();
                        } else if let Some(board_idx) = self.active_board_index {
                            if let Some(board) = self.boards.get(board_idx) {
                                let board_sprints: Vec<_> = self
                                    .sprints
                                    .iter()
                                    .filter(|s| s.board_id == board.id)
                                    .collect();

                                let sprint_idx = dialog_state.item_selection - 1;
                                if let Some(sprint) = board_sprints.get(sprint_idx) {
                                    if dialog_state
                                        .filters
                                        .selected_sprint_ids
                                        .contains(&sprint.id)
                                    {
                                        dialog_state.filters.selected_sprint_ids.remove(&sprint.id);
                                    } else {
                                        dialog_state.filters.selected_sprint_ids.insert(sprint.id);
                                    }
                                    tracing::info!(
                                        "Toggled sprint: {}",
                                        sprint.formatted_name(board, "sprint")
                                    );
                                    self.apply_filters();
                                }
                            }
                        }
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
            self.active_sprint_filters = dialog_state.filters.selected_sprint_ids.clone();

            self.refresh_view();
            tracing::info!(
                "Applied filters: unassigned={}, sprints={}",
                self.hide_assigned_cards,
                self.active_sprint_filters.len()
            );
        }
    }
}
