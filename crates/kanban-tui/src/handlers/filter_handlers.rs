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
                KeyCode::Char('j') | KeyCode::Down => {
                    match dialog_state.current_section {
                        FilterDialogSection::UnassignedSprints => {
                            dialog_state.next_section();
                        }
                        FilterDialogSection::Sprints => {
                            if let Some(board_idx) = self.active_board_index {
                                if let Some(board) = self.boards.get(board_idx) {
                                    let sprint_count = self
                                        .sprints
                                        .iter()
                                        .filter(|s| s.board_id == board.id)
                                        .count();
                                    if dialog_state.item_selection < sprint_count.saturating_sub(1) {
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
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    match dialog_state.current_section {
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
                    }
                }
                KeyCode::Char(' ') => {
                    match dialog_state.current_section {
                        FilterDialogSection::UnassignedSprints => {
                            dialog_state.filters.show_unassigned_sprints =
                                !dialog_state.filters.show_unassigned_sprints;
                            tracing::info!(
                                "Toggled unassigned sprints filter: {}",
                                dialog_state.filters.show_unassigned_sprints
                            );
                            self.apply_filters();
                        }
                        FilterDialogSection::Sprints => {
                            if let Some(board_idx) = self.active_board_index {
                                if let Some(board) = self.boards.get(board_idx) {
                                    let board_sprints: Vec<_> = self
                                        .sprints
                                        .iter()
                                        .filter(|s| s.board_id == board.id)
                                        .collect();

                                    if let Some(sprint) = board_sprints.get(dialog_state.item_selection) {
                                        if dialog_state.filters.selected_sprint_ids.contains(&sprint.id) {
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
