use crate::app::{App, AppMode};
use crossterm::event::KeyCode;
use kanban_domain::{SortField, SortOrder};

impl App {
    pub fn handle_import_board_popup(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.import_selection.clear();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.import_selection.next(self.import_files.len());
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.import_selection.prev();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(idx) = self.import_selection.get() {
                    if let Some(filename) = self.import_files.get(idx).cloned() {
                        if let Err(e) = self.import_board_from_file(&filename) {
                            tracing::error!("Failed to import board: {}", e);
                        }
                    }
                }
                self.mode = AppMode::Normal;
                self.import_selection.clear();
            }
            _ => {}
        }
    }

    pub fn handle_set_card_priority_popup(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::CardDetail;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.priority_selection.next(4);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.priority_selection.prev();
            }
            KeyCode::Enter => {
                if let Some(priority_idx) = self.priority_selection.get() {
                    if let Some(card_idx) = self.active_card_index {
                        if let Some(card) = self.cards.get_mut(card_idx) {
                            use kanban_domain::CardPriority;
                            let priority = match priority_idx {
                                0 => CardPriority::Low,
                                1 => CardPriority::Medium,
                                2 => CardPriority::High,
                                3 => CardPriority::Critical,
                                _ => CardPriority::Medium,
                            };
                            card.update_priority(priority);
                        }
                    }
                }
                self.mode = AppMode::CardDetail;
            }
            _ => {}
        }
    }

    pub fn handle_order_cards_popup(&mut self, key_code: KeyCode) -> bool {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.sort_field_selection.clear();
                false
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.sort_field_selection.next(6);
                false
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.sort_field_selection.prev();
                false
            }
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('a') | KeyCode::Char('d') => {
                if let Some(field_idx) = self.sort_field_selection.get() {
                    let field = match field_idx {
                        0 => SortField::Points,
                        1 => SortField::Priority,
                        2 => SortField::CreatedAt,
                        3 => SortField::UpdatedAt,
                        4 => SortField::Status,
                        5 => SortField::Default,
                        _ => return false,
                    };

                    let order = if self.current_sort_field == Some(field)
                        && matches!(key_code, KeyCode::Enter | KeyCode::Char(' '))
                    {
                        match self.current_sort_order {
                            Some(SortOrder::Ascending) => SortOrder::Descending,
                            Some(SortOrder::Descending) => SortOrder::Ascending,
                            None => SortOrder::Ascending,
                        }
                    } else {
                        match key_code {
                            KeyCode::Char('d') => SortOrder::Descending,
                            _ => SortOrder::Ascending,
                        }
                    };

                    self.current_sort_field = Some(field);
                    self.current_sort_order = Some(order);

                    if let Some(board_idx) = self.active_board_index {
                        if let Some(board) = self.boards.get_mut(board_idx) {
                            board.update_task_sort(field, order);
                        }
                    }

                    // Return to the appropriate mode based on where we came from
                    let is_sprint_detail = self.active_sprint_index.is_some();
                    let prev_mode = if is_sprint_detail {
                        AppMode::SprintDetail
                    } else {
                        AppMode::Normal
                    };
                    self.mode = prev_mode;
                    self.sort_field_selection.clear();

                    tracing::info!("Sorting by {:?} ({:?})", field, order);

                    // Apply sorting to the appropriate context
                    if is_sprint_detail {
                        self.apply_sort_to_sprint_lists(field, order);
                    } else {
                        self.refresh_view();
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub fn handle_assign_card_to_sprint_popup(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.sprint_assign_selection.clear();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(board_idx) = self.active_board_index {
                    if let Some(board) = self.boards.get(board_idx) {
                        let sprint_count = self
                            .sprints
                            .iter()
                            .filter(|s| s.board_id == board.id)
                            .count();
                        self.sprint_assign_selection.next(sprint_count + 1);
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.sprint_assign_selection.prev();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(selection_idx) = self.sprint_assign_selection.get() {
                    if let Some(card_idx) = self.active_card_index {
                        if let Some(card) = self.cards.get_mut(card_idx) {
                            if selection_idx == 0 {
                                card.end_current_sprint_log();
                                card.sprint_id = None;
                                tracing::info!("Unassigned card from sprint");
                            } else if let Some(board_idx) = self.active_board_index {
                                if let Some(board) = self.boards.get(board_idx) {
                                    let board_sprints: Vec<_> = self
                                        .sprints
                                        .iter()
                                        .filter(|s| s.board_id == board.id)
                                        .collect();
                                    if let Some(sprint) = board_sprints.get(selection_idx - 1) {
                                        if let Some(old_sprint_id) = card.sprint_id {
                                            if old_sprint_id != sprint.id {
                                                card.end_current_sprint_log();
                                            }
                                        }
                                        card.assign_to_sprint(
                                            sprint.id,
                                            sprint.sprint_number,
                                            sprint.get_name(board).map(|s| s.to_string()),
                                            format!("{:?}", sprint.status),
                                        );
                                        tracing::info!(
                                            "Assigned card to sprint: {}",
                                            sprint.formatted_name(board, "sprint")
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                self.mode = AppMode::Normal;
                self.sprint_assign_selection.clear();
            }
            _ => {}
        }
    }

    pub fn handle_assign_multiple_cards_to_sprint_popup(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.sprint_assign_selection.clear();
                self.selected_cards.clear();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(board_idx) = self.active_board_index {
                    if let Some(board) = self.boards.get(board_idx) {
                        let sprint_count = self
                            .sprints
                            .iter()
                            .filter(|s| s.board_id == board.id)
                            .count();
                        self.sprint_assign_selection.next(sprint_count + 1);
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.sprint_assign_selection.prev();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(selection_idx) = self.sprint_assign_selection.get() {
                    let card_ids: Vec<uuid::Uuid> = self.selected_cards.iter().copied().collect();
                    for card_id in card_ids {
                        if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) {
                            if selection_idx == 0 {
                                card.end_current_sprint_log();
                                card.sprint_id = None;
                            } else if let Some(board_idx) = self.active_board_index {
                                if let Some(board) = self.boards.get(board_idx) {
                                    let board_sprints: Vec<_> = self
                                        .sprints
                                        .iter()
                                        .filter(|s| s.board_id == board.id)
                                        .collect();
                                    if let Some(sprint) = board_sprints.get(selection_idx - 1) {
                                        if let Some(old_sprint_id) = card.sprint_id {
                                            if old_sprint_id != sprint.id {
                                                card.end_current_sprint_log();
                                            }
                                        }
                                        card.assign_to_sprint(
                                            sprint.id,
                                            sprint.sprint_number,
                                            sprint.get_name(board).map(|s| s.to_string()),
                                            format!("{:?}", sprint.status),
                                        );
                                    }
                                }
                            }
                        }
                    }

                    if let Some(board_idx) = self.active_board_index {
                        if let Some(board) = self.boards.get(board_idx) {
                            let board_sprints: Vec<_> = self
                                .sprints
                                .iter()
                                .filter(|s| s.board_id == board.id)
                                .collect();
                            if selection_idx == 0 {
                                tracing::info!(
                                    "Unassigned {} cards from sprint",
                                    self.selected_cards.len()
                                );
                            } else if let Some(sprint) = board_sprints.get(selection_idx - 1) {
                                tracing::info!(
                                    "Assigned {} cards to sprint: {}",
                                    self.selected_cards.len(),
                                    sprint.formatted_name(board, "sprint")
                                );
                            }
                        }
                    }
                }
                self.mode = AppMode::Normal;
                self.sprint_assign_selection.clear();
                self.selected_cards.clear();
            }
            _ => {}
        }
    }
}
