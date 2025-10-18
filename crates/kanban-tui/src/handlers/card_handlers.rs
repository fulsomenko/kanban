use crate::app::{App, AppMode, CardField, Focus};
use crate::events::EventHandler;
use kanban_domain::{Card, CardStatus, Column, SortOrder};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

impl App {
    pub fn handle_create_card_key(&mut self) {
        if self.focus == Focus::Cards && self.active_board_index.is_some() {
            self.mode = AppMode::CreateCard;
            self.input.clear();
        }
    }

    pub fn handle_toggle_card_completion(&mut self) {
        if self.focus != Focus::Cards {
            return;
        }

        if !self.selected_cards.is_empty() {
            self.toggle_selected_cards_completion();
        } else if self.card_selection.get().is_some() {
            self.toggle_card_completion();
        }
    }

    pub fn handle_card_selection_toggle(&mut self) {
        if self.focus == Focus::Cards && self.card_selection.get().is_some() {
            if let Some(card) = self.get_selected_card_in_context() {
                let card_id = card.id;
                if self.selected_cards.contains(&card_id) {
                    self.selected_cards.remove(&card_id);
                } else {
                    self.selected_cards.insert(card_id);
                }
            }
        }
    }

    pub fn handle_assign_to_sprint_key(&mut self) {
        if self.focus != Focus::Cards {
            return;
        }

        if !self.selected_cards.is_empty() {
            self.sprint_assign_selection.clear();
            self.mode = AppMode::AssignMultipleCardsToSprint;
        } else if self.card_selection.get().is_some() {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.boards.get(board_idx) {
                    let sprint_count = self
                        .sprints
                        .iter()
                        .filter(|s| s.board_id == board.id)
                        .count();
                    if sprint_count > 0 {
                        if let Some(selected_card) = self.get_selected_card_in_context() {
                            let card_id = selected_card.id;
                            let actual_idx = self.cards.iter().position(|c| c.id == card_id);
                            self.active_card_index = actual_idx;
                        }
                        self.sprint_assign_selection.set(Some(0));
                        self.mode = AppMode::AssignCardToSprint;
                    }
                }
            }
        }
    }

    pub fn handle_order_cards_key(&mut self) {
        if self.focus == Focus::Cards && self.active_board_index.is_some() {
            self.sort_field_selection.set(Some(0));
            self.mode = AppMode::OrderCards;
        }
    }

    pub fn handle_toggle_sort_order_key(&mut self) {
        if self.focus == Focus::Cards && self.active_board_index.is_some() {
            if let Some(current_order) = self.current_sort_order {
                let new_order = match current_order {
                    SortOrder::Ascending => SortOrder::Descending,
                    SortOrder::Descending => SortOrder::Ascending,
                };
                self.current_sort_order = Some(new_order);

                if let Some(board_idx) = self.active_board_index {
                    if let Some(board) = self.boards.get_mut(board_idx) {
                        if let Some(field) = self.current_sort_field {
                            board.update_task_sort(field, new_order);
                        }
                    }
                }

                tracing::info!("Toggled sort order to: {:?}", new_order);
            }
        }
    }

    pub fn handle_toggle_hide_assigned(&mut self) {
        if self.focus == Focus::Cards && self.active_board_index.is_some() {
            self.hide_assigned_cards = !self.hide_assigned_cards;
            let status = if self.hide_assigned_cards {
                "enabled"
            } else {
                "disabled"
            };
            tracing::info!("Hide assigned cards: {}", status);

            self.refresh_view();
        }
    }

    pub fn handle_toggle_sprint_filter(&mut self) {
        if self.focus == Focus::Cards && self.active_board_index.is_some() {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.boards.get(board_idx) {
                    if let Some(active_sprint_id) = board.active_sprint_id {
                        if self.active_sprint_filter == Some(active_sprint_id) {
                            self.active_sprint_filter = None;
                            tracing::info!("Disabled sprint filter - showing all cards");
                        } else {
                            self.active_sprint_filter = Some(active_sprint_id);
                            tracing::info!("Enabled sprint filter - showing active sprint only");
                        }

                        self.refresh_view();
                    } else {
                        tracing::warn!("No active sprint set for filtering");
                    }
                }
            }
        }
    }

    pub fn handle_edit_card_key(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
    ) -> bool {
        let mut should_restart = false;
        if self.focus == Focus::Cards && self.card_selection.get().is_some() {
            if let Some(selected_card) = self.get_selected_card_in_context() {
                let card_id = selected_card.id;
                let actual_idx = self.cards.iter().position(|c| c.id == card_id);
                self.active_card_index = actual_idx;

                if let Err(e) = self.edit_card_field(terminal, event_handler, CardField::Description) {
                    tracing::error!("Failed to edit card description: {}", e);
                }
                should_restart = true;
            }
        }
        should_restart
    }

    fn toggle_card_completion(&mut self) {
        if let Some(card) = self.get_selected_card_in_context() {
            let card_id = card.id;
            let new_status = if card.status == CardStatus::Done {
                CardStatus::Todo
            } else {
                CardStatus::Done
            };

            let last_column_id = if new_status == CardStatus::Done {
                if let Some(board_idx) = self.active_board_index {
                    if let Some(board) = self.boards.get(board_idx) {
                        let mut board_columns: Vec<_> = self
                            .columns
                            .iter()
                            .filter(|col| col.board_id == board.id)
                            .collect();
                        board_columns.sort_by_key(|col| col.position);
                        board_columns.last().map(|col| col.id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let new_position = if let Some(target_column_id) = last_column_id {
                Some(self
                    .cards
                    .iter()
                    .filter(|c| c.column_id == target_column_id)
                    .count() as i32)
            } else {
                None
            };

            if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) {
                let old_column_id = card.column_id;
                card.update_status(new_status);

                if let (Some(target_column_id), Some(position)) = (last_column_id, new_position) {
                    if old_column_id != target_column_id {
                        card.move_to_column(target_column_id, position);
                        tracing::info!(
                            "Moved card '{}' to last column (status: {:?})",
                            card.title,
                            new_status
                        );
                    } else {
                        tracing::info!(
                            "Toggled card '{}' to status: {:?}",
                            card.title,
                            new_status
                        );
                    }
                } else {
                    tracing::info!(
                        "Toggled card '{}' to status: {:?}",
                        card.title,
                        new_status
                    );
                }
            }
        }
    }

    fn toggle_selected_cards_completion(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self.selected_cards.iter().copied().collect();
        let mut toggled_count = 0;

        let last_column_id = if let Some(board_idx) = self.active_board_index {
            if let Some(board) = self.boards.get(board_idx) {
                let mut board_columns: Vec<_> = self
                    .columns
                    .iter()
                    .filter(|col| col.board_id == board.id)
                    .collect();
                board_columns.sort_by_key(|col| col.position);
                board_columns.last().map(|col| col.id)
            } else {
                None
            }
        } else {
            None
        };

        for card_id in card_ids {
            let new_position = if let Some(target_column_id) = last_column_id {
                Some(
                    self.cards
                        .iter()
                        .filter(|c| c.column_id == target_column_id)
                        .count() as i32,
                )
            } else {
                None
            };

            if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) {
                let old_column_id = card.column_id;
                let new_status = if card.status == CardStatus::Done {
                    CardStatus::Todo
                } else {
                    CardStatus::Done
                };
                card.update_status(new_status);

                if new_status == CardStatus::Done {
                    if let (Some(target_column_id), Some(position)) = (last_column_id, new_position)
                    {
                        if old_column_id != target_column_id {
                            card.move_to_column(target_column_id, position);
                        }
                    }
                }

                toggled_count += 1;
            }
        }

        tracing::info!("Toggled {} cards completion status", toggled_count);
        self.selected_cards.clear();
    }

    pub fn create_card(&mut self) {
        if let Some(idx) = self.active_board_index {
            if let Some(board) = self.boards.get_mut(idx) {
                let column = self
                    .columns
                    .iter()
                    .find(|col| col.board_id == board.id)
                    .cloned();

                let column = match column {
                    Some(col) => col,
                    None => {
                        let new_column = Column::new(board.id, "Todo".to_string(), 0);
                        self.columns.push(new_column.clone());
                        new_column
                    }
                };

                let position = self
                    .cards
                    .iter()
                    .filter(|c| c.column_id == column.id)
                    .count() as i32;
                let card = Card::new(board, column.id, self.input.as_str().to_string(), position);
                let new_card_id = card.id;
                let board_id = board.id;
                tracing::info!("Creating card: {} (id: {})", card.title, card.id);
                self.cards.push(card);

                let sorted_cards = self.get_sorted_board_cards(board_id);
                if let Some(pos) = sorted_cards.iter().position(|c| c.id == new_card_id) {
                    self.card_selection.set(Some(pos));
                }
            }
        }
    }

    pub fn handle_move_card_left(&mut self) {
        if self.focus != Focus::Cards {
            return;
        }

        if let Some(card) = self.get_selected_card_in_context() {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.boards.get(board_idx) {
                    let card_id = card.id;
                    let current_column_id = card.column_id;

                    let mut board_columns: Vec<_> = self
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .collect();
                    board_columns.sort_by_key(|col| col.position);

                    let current_position = board_columns
                        .iter()
                        .position(|col| col.id == current_column_id);

                    if let Some(pos) = current_position {
                        if pos > 0 {
                            let target_column_id = board_columns[pos - 1].id;

                            let new_position = self
                                .cards
                                .iter()
                                .filter(|c| c.column_id == target_column_id)
                                .count() as i32;

                            if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) {
                                card.move_to_column(target_column_id, new_position);
                                tracing::info!(
                                    "Moved card '{}' to previous column",
                                    card.title
                                );
                            }

                            if self.is_kanban_view() {
                                if let Some(current_col_idx) = self.column_selection.get() {
                                    if current_col_idx > 0 {
                                        self.column_selection.set(Some(current_col_idx - 1));
                                    }
                                }
                            } else {
                                let sorted_cards = self.get_sorted_board_cards(board.id);
                                if let Some(new_idx) =
                                    sorted_cards.iter().position(|c| c.id == card_id)
                                {
                                    self.card_selection.set(Some(new_idx));
                                }
                            }
                        } else {
                            tracing::warn!("Card is already in the first column");
                        }
                    }
                }
            }
        }
    }

    pub fn handle_move_card_right(&mut self) {
        if self.focus != Focus::Cards {
            return;
        }

        if let Some(card) = self.get_selected_card_in_context() {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.boards.get(board_idx) {
                    let card_id = card.id;
                    let current_column_id = card.column_id;

                    let mut board_columns: Vec<_> = self
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .collect();
                    board_columns.sort_by_key(|col| col.position);

                    let current_position = board_columns
                        .iter()
                        .position(|col| col.id == current_column_id);

                    if let Some(pos) = current_position {
                        if pos < board_columns.len() - 1 {
                            let target_column_id = board_columns[pos + 1].id;

                            let new_position = self
                                .cards
                                .iter()
                                .filter(|c| c.column_id == target_column_id)
                                .count() as i32;

                            if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) {
                                card.move_to_column(target_column_id, new_position);
                                tracing::info!(
                                    "Moved card '{}' to next column",
                                    card.title
                                );
                            }

                            if self.is_kanban_view() {
                                let column_count = board_columns.len();
                                if let Some(current_col_idx) = self.column_selection.get() {
                                    if current_col_idx < column_count - 1 {
                                        self.column_selection.set(Some(current_col_idx + 1));
                                    }
                                }
                            } else {
                                let sorted_cards = self.get_sorted_board_cards(board.id);
                                if let Some(new_idx) =
                                    sorted_cards.iter().position(|c| c.id == card_id)
                                {
                                    self.card_selection.set(Some(new_idx));
                                }
                            }
                        } else {
                            tracing::warn!("Card is already in the last column");
                        }
                    }
                }
            }
        }
    }
}
