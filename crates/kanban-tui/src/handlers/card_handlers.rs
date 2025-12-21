use crate::app::{App, AppMode, CardField, DialogMode, Focus};
use crate::card_list::CardListId;
use crate::events::EventHandler;
use crate::state::commands::{
    ArchiveCard, CreateCard, DeleteCard, MoveCard, RestoreCard, SetBoardTaskSort, UpdateCard,
};
use kanban_domain::{ArchivedCard, CardStatus, CardUpdate, Column, SortOrder};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

impl App {
    pub fn handle_create_card_key(&mut self) {
        if self.focus == Focus::Cards && self.active_board_index.is_some() {
            self.open_dialog(DialogMode::CreateCard);
            self.input.clear();
        }
    }

    fn get_focused_column_id(&mut self) -> Option<uuid::Uuid> {
        if let Some(task_list) = self.view_strategy.get_active_task_list() {
            if let CardListId::Column(column_id) = task_list.id {
                return Some(column_id);
            }
        }
        None
    }

    pub fn handle_toggle_card_completion(&mut self) {
        if self.focus != Focus::Cards {
            return;
        }

        if !self.selected_cards.is_empty() {
            self.toggle_selected_cards_completion();
        } else {
            self.toggle_card_completion();
        }
    }

    pub fn handle_card_selection_toggle(&mut self) {
        if self.focus == Focus::Cards {
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
            self.open_dialog(DialogMode::AssignMultipleCardsToSprint);
        } else if self.get_selected_card_id().is_some() {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    let sprint_count = self
                        .ctx
                        .sprints
                        .iter()
                        .filter(|s| s.board_id == board.id)
                        .count();
                    if sprint_count > 0 {
                        if let Some(selected_card) = self.get_selected_card_in_context() {
                            let card_id = selected_card.id;
                            let actual_idx = self.ctx.cards.iter().position(|c| c.id == card_id);
                            self.active_card_index = actual_idx;
                        }
                        let selection_idx = self.get_current_sprint_selection_index();
                        self.sprint_assign_selection.set(Some(selection_idx));
                        self.open_dialog(DialogMode::AssignCardToSprint);
                    }
                }
            }
        }
    }

    pub fn handle_order_cards_key(&mut self) {
        if self.focus == Focus::Cards && self.active_board_index.is_some() {
            let sort_idx = self.get_current_sort_field_selection_index();
            self.sort_field_selection.set(Some(sort_idx));
            self.open_dialog(DialogMode::OrderCards);
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
                    if let Some(board) = self.ctx.boards.get(board_idx) {
                        if let Some(field) = self.current_sort_field {
                            let cmd = Box::new(SetBoardTaskSort {
                                board_id: board.id,
                                field,
                                order: new_order,
                            });

                            if let Err(e) = self.execute_command(cmd) {
                                tracing::error!("Failed to set board task sort: {}", e);
                                return;
                            }
                        }
                    }
                }

                tracing::info!("Toggled sort order to: {:?}", new_order);
                self.refresh_view();
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
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    if let Some(active_sprint_id) = board.active_sprint_id {
                        if self.active_sprint_filters.contains(&active_sprint_id) {
                            self.active_sprint_filters.remove(&active_sprint_id);
                            tracing::info!("Disabled sprint filter - showing all cards");
                        } else {
                            self.active_sprint_filters.clear();
                            self.active_sprint_filters.insert(active_sprint_id);
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
        if self.focus == Focus::Cards {
            if let Some(selected_card) = self.get_selected_card_in_context() {
                let card_id = selected_card.id;
                let actual_idx = self.ctx.cards.iter().position(|c| c.id == card_id);
                self.active_card_index = actual_idx;

                if let Err(e) =
                    self.edit_card_field(terminal, event_handler, CardField::Description)
                {
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
            let old_column_id = card.column_id;
            let new_status = if card.status == CardStatus::Done {
                CardStatus::Todo
            } else {
                CardStatus::Done
            };

            // Calculate target column and position before calling execute_command
            let target_column_and_position = if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    // Get sorted column IDs
                    let mut cols_with_pos: Vec<_> = self
                        .ctx
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .map(|col| (col.id, col.position))
                        .collect();
                    cols_with_pos.sort_by_key(|(_, pos)| *pos);
                    let board_columns: Vec<uuid::Uuid> =
                        cols_with_pos.into_iter().map(|(id, _)| id).collect();

                    let current_column_pos = board_columns
                        .iter()
                        .position(|col_id| *col_id == old_column_id);

                    if new_status == CardStatus::Done {
                        // Moving to Done: move to last column
                        if let Some(last_col) = board_columns.last() {
                            if *last_col != old_column_id {
                                let position = self
                                    .ctx
                                    .cards
                                    .iter()
                                    .filter(|c| c.column_id == *last_col)
                                    .count() as i32;
                                Some((*last_col, position, "last"))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        // Moving from Done to Todo: move to second-to-last column if in last column
                        if let Some(pos) = current_column_pos {
                            if pos == board_columns.len() - 1 && board_columns.len() > 1 {
                                // Currently in last column, move to second-to-last
                                let target_col = board_columns[board_columns.len() - 2];
                                let position = self
                                    .ctx
                                    .cards
                                    .iter()
                                    .filter(|c| c.column_id == target_col)
                                    .count() as i32;
                                Some((target_col, position, "second-to-last"))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Build update with status and optionally column/position
            let mut updates = CardUpdate {
                status: Some(new_status),
                ..Default::default()
            };

            if let Some((target_column_id, position, column_desc)) = target_column_and_position {
                updates.column_id = Some(target_column_id);
                updates.position = Some(position);
                tracing::info!(
                    "Moving card to {} column (status: {:?})",
                    column_desc,
                    new_status
                );
            } else {
                tracing::info!("Toggling card to status: {:?}", new_status);
            }

            // Execute UpdateCard command
            let cmd = Box::new(UpdateCard { card_id, updates });
            if let Err(e) = self.execute_command(cmd) {
                tracing::error!("Failed to toggle card completion: {}", e);
                return;
            }

            self.refresh_view();
            self.select_card_by_id(card_id);
        }
    }

    fn toggle_selected_cards_completion(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self.selected_cards.iter().copied().collect();
        let mut toggled_count = 0;
        let first_card_id = card_ids.first().copied();

        // Build list of column IDs for target calculations
        let board_column_ids: Vec<uuid::Uuid> = if let Some(board_idx) = self.active_board_index {
            if let Some(board) = self.ctx.boards.get(board_idx) {
                // Sort by position
                let mut cols_with_pos: Vec<_> = self
                    .ctx
                    .columns
                    .iter()
                    .filter(|col| col.board_id == board.id)
                    .map(|col| (col.id, col.position))
                    .collect();
                cols_with_pos.sort_by_key(|(_, pos)| *pos);
                cols_with_pos.into_iter().map(|(id, _)| id).collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Batch all card updates to avoid race conditions with file watcher
        let mut update_commands: Vec<Box<dyn crate::state::commands::Command>> = Vec::new();

        for card_id in card_ids {
            // First, get card info without mutable borrow
            let (old_column_id, old_status) =
                if let Some(card) = self.ctx.cards.iter().find(|c| c.id == card_id) {
                    (card.column_id, card.status)
                } else {
                    continue;
                };

            let new_status = if old_status == CardStatus::Done {
                CardStatus::Todo
            } else {
                CardStatus::Done
            };

            let current_column_pos = board_column_ids
                .iter()
                .position(|col_id| *col_id == old_column_id);

            // Determine target column
            let target_column_id = if new_status == CardStatus::Done {
                // Moving to Done: move to last column
                board_column_ids.last().and_then(|last_col| {
                    if *last_col != old_column_id {
                        Some(*last_col)
                    } else {
                        None
                    }
                })
            } else {
                // Moving from Done to Todo: move to second-to-last column if in last column
                if let Some(pos) = current_column_pos {
                    if pos == board_column_ids.len() - 1 && board_column_ids.len() > 1 {
                        Some(board_column_ids[board_column_ids.len() - 2])
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            // Calculate position in target column before command execution
            let target_position = target_column_id.map(|col_id| {
                self.ctx
                    .cards
                    .iter()
                    .filter(|c| c.column_id == col_id)
                    .count() as i32
            });

            // Build update with status and optionally column/position
            let mut updates = CardUpdate {
                status: Some(new_status),
                ..Default::default()
            };

            if let (Some(target_col_id), Some(position)) = (target_column_id, target_position) {
                updates.column_id = Some(target_col_id);
                updates.position = Some(position);
            }

            // Add to batch
            let cmd = Box::new(UpdateCard { card_id, updates })
                as Box<dyn crate::state::commands::Command>;
            update_commands.push(cmd);
            toggled_count += 1;
        }

        // Execute all updates as a batch
        if !update_commands.is_empty() {
            if let Err(e) = self.execute_commands_batch(update_commands) {
                tracing::error!("Failed to toggle card completion: {}", e);
                return;
            }
        }

        tracing::info!("Toggled {} cards completion status", toggled_count);
        self.selected_cards.clear();
        self.refresh_view();
        if let Some(card_id) = first_card_id {
            self.select_card_by_id(card_id);
        }
    }

    pub fn create_card(&mut self) {
        if let Some(idx) = self.active_board_index {
            let focused_col_id = self.get_focused_column_id();
            let board_id = self.ctx.boards.get(idx).map(|b| b.id);

            if let Some(bid) = board_id {
                let target_column_id = if let Some(focused_col_id) = focused_col_id {
                    Some(focused_col_id)
                } else {
                    self.ctx
                        .columns
                        .iter()
                        .find(|col| col.board_id == bid)
                        .map(|col| col.id)
                };

                let column = if let Some(col_id) = target_column_id {
                    self.ctx
                        .columns
                        .iter()
                        .find(|col| col.id == col_id)
                        .cloned()
                } else {
                    None
                };

                let column = match column {
                    Some(col) => col,
                    None => {
                        let new_column = Column::new(bid, "Todo".to_string(), 0);
                        self.ctx.columns.push(new_column.clone());
                        new_column
                    }
                };

                let position = self
                    .ctx
                    .cards
                    .iter()
                    .filter(|c| c.column_id == column.id)
                    .count() as i32;

                let column_name = column.name.clone();

                // Determine if we need to mark as complete (if in last column)
                let board_columns: Vec<_> = self
                    .ctx
                    .columns
                    .iter()
                    .filter(|col| col.board_id == bid)
                    .collect();

                let mark_as_complete = if board_columns.len() > 2 {
                    let mut sorted_cols = board_columns.clone();
                    sorted_cols.sort_by_key(|col| col.position);
                    if let Some(last_col) = sorted_cols.last() {
                        last_col.id == column.id
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Build batch: CreateCard + optional status update
                let mut commands: Vec<Box<dyn crate::state::commands::Command>> = Vec::new();

                let create_cmd = Box::new(CreateCard {
                    board_id: bid,
                    column_id: column.id,
                    title: self.input.as_str().to_string(),
                    position,
                }) as Box<dyn crate::state::commands::Command>;
                commands.push(create_cmd);

                if let Err(e) = self.execute_commands_batch(commands) {
                    tracing::error!("Failed to create card: {}", e);
                    return;
                }

                // After command execution, find the newly created card
                if mark_as_complete {
                    if let Some(card) = self
                        .ctx
                        .cards
                        .iter()
                        .rev()
                        .find(|c| c.column_id == column.id)
                    {
                        let card_id = card.id;
                        let update_cmd = Box::new(UpdateCard {
                            card_id,
                            updates: CardUpdate {
                                status: Some(CardStatus::Done),
                                ..Default::default()
                            },
                        })
                            as Box<dyn crate::state::commands::Command>;

                        if let Err(e) = self.execute_command(update_cmd) {
                            tracing::error!("Failed to update card status: {}", e);
                        }

                        tracing::info!(
                            "Creating card in column: {} [marked as complete]",
                            column_name
                        );
                    }
                } else {
                    tracing::info!("Creating card in column: {}", column_name);
                }

                self.refresh_view();
                // Select the most recently created card
                if let Some(card) = self
                    .ctx
                    .cards
                    .iter()
                    .rev()
                    .find(|c| c.column_id == column.id)
                {
                    self.select_card_by_id(card.id);
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
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    let card_id = card.id;
                    let current_column_id = card.column_id;
                    let current_status = card.status;

                    // Collect and sort column IDs before command execution
                    let mut cols_with_pos: Vec<_> = self
                        .ctx
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .map(|col| (col.id, col.position))
                        .collect();
                    cols_with_pos.sort_by_key(|(_, pos)| *pos);
                    let board_column_ids: Vec<uuid::Uuid> =
                        cols_with_pos.into_iter().map(|(id, _)| id).collect();

                    let current_position = board_column_ids
                        .iter()
                        .position(|col_id| *col_id == current_column_id);

                    if let Some(pos) = current_position {
                        if pos > 0 {
                            let target_column_id = board_column_ids[pos - 1];
                            let is_moving_from_last = pos == board_column_ids.len() - 1;
                            let num_cols = board_column_ids.len();

                            let new_position = self
                                .ctx
                                .cards
                                .iter()
                                .filter(|c| c.column_id == target_column_id)
                                .count() as i32;

                            // Build batch with MoveCard and optional status update
                            let mut commands: Vec<Box<dyn crate::state::commands::Command>> =
                                Vec::new();

                            let move_cmd = Box::new(MoveCard {
                                card_id,
                                new_column_id: target_column_id,
                                new_position,
                            })
                                as Box<dyn crate::state::commands::Command>;
                            commands.push(move_cmd);

                            // If moving from last column and card is Done, mark as Todo
                            if is_moving_from_last
                                && num_cols > 1
                                && current_status == CardStatus::Done
                            {
                                let status_cmd = Box::new(UpdateCard {
                                    card_id,
                                    updates: CardUpdate {
                                        status: Some(CardStatus::Todo),
                                        ..Default::default()
                                    },
                                })
                                    as Box<dyn crate::state::commands::Command>;
                                commands.push(status_cmd);
                            }

                            if let Err(e) = self.execute_commands_batch(commands) {
                                tracing::error!("Failed to move card left: {}", e);
                                return;
                            }

                            if is_moving_from_last
                                && num_cols > 1
                                && current_status == CardStatus::Done
                            {
                                tracing::info!(
                                    "Moved card from last column (unmarked as complete)"
                                );
                            } else {
                                tracing::info!("Moved card to previous column");
                            }

                            if self.is_kanban_view() {
                                if let Some(current_col_idx) = self.column_selection.get() {
                                    if current_col_idx > 0 {
                                        self.column_selection.set(Some(current_col_idx - 1));
                                    }
                                }
                                self.refresh_view();
                                self.select_card_by_id(card_id);
                            } else {
                                self.refresh_view();
                                self.select_card_by_id(card_id);
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
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    let card_id = card.id;
                    let current_column_id = card.column_id;
                    let current_status = card.status;

                    // Collect and sort column IDs before command execution
                    let mut cols_with_pos: Vec<_> = self
                        .ctx
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .map(|col| (col.id, col.position))
                        .collect();
                    cols_with_pos.sort_by_key(|(_, pos)| *pos);
                    let board_column_ids: Vec<uuid::Uuid> =
                        cols_with_pos.into_iter().map(|(id, _)| id).collect();

                    let current_position = board_column_ids
                        .iter()
                        .position(|col_id| *col_id == current_column_id);

                    if let Some(pos) = current_position {
                        if pos < board_column_ids.len() - 1 {
                            let target_column_id = board_column_ids[pos + 1];
                            let is_moving_to_last = pos + 1 == board_column_ids.len() - 1;
                            let num_cols = board_column_ids.len();

                            let new_position = self
                                .ctx
                                .cards
                                .iter()
                                .filter(|c| c.column_id == target_column_id)
                                .count() as i32;

                            // Build batch with MoveCard and optional status update
                            let mut commands: Vec<Box<dyn crate::state::commands::Command>> =
                                Vec::new();

                            let move_cmd = Box::new(MoveCard {
                                card_id,
                                new_column_id: target_column_id,
                                new_position,
                            })
                                as Box<dyn crate::state::commands::Command>;
                            commands.push(move_cmd);

                            // If moving to last column and card is not Done, mark as Done
                            if is_moving_to_last
                                && num_cols > 1
                                && current_status != CardStatus::Done
                            {
                                let status_cmd = Box::new(UpdateCard {
                                    card_id,
                                    updates: CardUpdate {
                                        status: Some(CardStatus::Done),
                                        ..Default::default()
                                    },
                                })
                                    as Box<dyn crate::state::commands::Command>;
                                commands.push(status_cmd);
                            }

                            if let Err(e) = self.execute_commands_batch(commands) {
                                tracing::error!("Failed to move card right: {}", e);
                                return;
                            }

                            if is_moving_to_last
                                && num_cols > 1
                                && current_status != CardStatus::Done
                            {
                                tracing::info!("Moved card to last column (marked as complete)");
                            } else {
                                tracing::info!("Moved card to next column");
                            }

                            if self.is_kanban_view() {
                                if let Some(current_col_idx) = self.column_selection.get() {
                                    if current_col_idx < num_cols - 1 {
                                        self.column_selection.set(Some(current_col_idx + 1));
                                    }
                                }
                                self.refresh_view();
                                self.select_card_by_id(card_id);
                            } else {
                                self.refresh_view();
                                self.select_card_by_id(card_id);
                            }
                        } else {
                            tracing::warn!("Card is already in the last column");
                        }
                    }
                }
            }
        }
    }

    pub fn handle_archive_card(&mut self) {
        if self.focus != Focus::Cards {
            return;
        }

        if !self.selected_cards.is_empty() {
            self.start_delete_animations_for_selected();
        } else if let Some(card_id) = self.get_selected_card_id() {
            self.start_delete_animation(card_id);
        }
    }

    fn start_delete_animations_for_selected(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self.selected_cards.iter().copied().collect();
        for card_id in card_ids {
            self.start_delete_animation(card_id);
        }
        self.selected_cards.clear();
    }

    fn start_delete_animation(&mut self, card_id: uuid::Uuid) {
        use crate::app::{AnimationType, CardAnimation};
        use std::time::Instant;

        if self.ctx.cards.iter().any(|c| c.id == card_id) {
            self.animating_cards.insert(
                card_id,
                CardAnimation {
                    animation_type: AnimationType::Archiving,
                    start_time: Instant::now(),
                },
            );
        }
    }

    #[allow(dead_code)]
    fn delete_card(&mut self, card_id: uuid::Uuid) -> bool {
        // Store info before executing command
        let deleted_info = self
            .ctx
            .cards
            .iter()
            .find(|c| c.id == card_id)
            .map(|c| (c.column_id, c.position, c.title.clone()));

        if let Some((deleted_column_id, deleted_position, card_title)) = deleted_info {
            // Execute ArchiveCard command
            let cmd = Box::new(ArchiveCard { card_id });
            if let Err(e) = self.execute_command(cmd) {
                tracing::error!("Failed to archive card: {}", e);
                return false;
            }

            // Compact positions in the deleted column to remove gaps
            self.compact_column_positions(deleted_column_id);

            // Update selection to the next appropriate card
            self.select_card_after_deletion(deleted_column_id, deleted_position);

            tracing::info!("Card '{}' archived", card_title);
            return true;
        }
        false
    }

    pub fn compact_column_positions(&mut self, column_id: uuid::Uuid) {
        // Get all cards in this column and sort by position
        let mut column_cards: Vec<_> = self
            .ctx
            .cards
            .iter_mut()
            .filter(|c| c.column_id == column_id)
            .collect();
        column_cards.sort_by_key(|c| c.position);

        // Reassign positions sequentially (0, 1, 2, ...)
        for (new_pos, card) in column_cards.iter_mut().enumerate() {
            card.position = new_pos as i32;
        }
    }

    pub fn select_card_after_deletion(
        &mut self,
        deleted_column_id: uuid::Uuid,
        deleted_position: i32,
    ) {
        // Try to find a card in the same column at or after the deleted position
        if let Some(next_card) = self
            .ctx
            .cards
            .iter()
            .find(|c| c.column_id == deleted_column_id && c.position >= deleted_position)
        {
            self.select_card_by_id(next_card.id);
        } else if let Some(prev_card) = self
            .ctx
            .cards
            .iter()
            .rev()
            .find(|c| c.column_id == deleted_column_id)
        {
            // Select the last remaining card in the column
            self.select_card_by_id(prev_card.id);
        }
        // Else: no selection (falls back to current behavior - no explicit selection)
    }

    pub fn handle_restore_card(&mut self) {
        if self.mode != AppMode::ArchivedCardsView {
            return;
        }

        if !self.selected_cards.is_empty() {
            self.start_restore_animations_for_selected();
        } else if let Some(card_id) = self.get_selected_card_id() {
            self.start_restore_animation(card_id);
        }
    }

    fn start_restore_animations_for_selected(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self.selected_cards.iter().copied().collect();
        for card_id in card_ids {
            self.start_restore_animation(card_id);
        }
        self.selected_cards.clear();
    }

    fn start_restore_animation(&mut self, card_id: uuid::Uuid) {
        use crate::app::{AnimationType, CardAnimation};
        use std::time::Instant;

        if self
            .ctx
            .archived_cards
            .iter()
            .any(|dc| dc.card.id == card_id)
        {
            self.animating_cards.insert(
                card_id,
                CardAnimation {
                    animation_type: AnimationType::Restoring,
                    start_time: Instant::now(),
                },
            );
        }
    }

    #[allow(dead_code)]
    fn restore_selected_cards(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self.selected_cards.iter().copied().collect();
        let mut restored_count = 0;

        for card_id in card_ids {
            if let Some(archived_card) = self
                .ctx
                .archived_cards
                .iter()
                .find(|dc| dc.card.id == card_id)
                .cloned()
            {
                self.restore_card(archived_card);
                restored_count += 1;
            }
        }

        tracing::info!("Restored {} card(s)", restored_count);
        self.selected_cards.clear();
        self.refresh_view();
    }

    pub fn restore_card(&mut self, archived_card: ArchivedCard) {
        let card_id = archived_card.card.id;
        let original_column_id = archived_card.original_column_id;
        let original_position = archived_card.original_position;
        let card_title = archived_card.card.title.clone();

        // Check if the original column still exists
        let target_column_id = if self
            .ctx
            .columns
            .iter()
            .any(|col| col.id == original_column_id)
        {
            original_column_id
        } else {
            // If original column doesn't exist, use first column
            self.ctx
                .columns
                .first()
                .map(|col| col.id)
                .unwrap_or(original_column_id)
        };

        // Execute RestoreCard command
        let cmd = Box::new(RestoreCard {
            card_id,
            column_id: target_column_id,
            position: original_position,
        });

        if let Err(e) = self.execute_command(cmd) {
            tracing::error!("Failed to restore card: {}", e);
            return;
        }

        tracing::info!("Card '{}' restored to original position", card_title);
    }

    pub fn handle_delete_card_permanent(&mut self) {
        if self.mode != AppMode::ArchivedCardsView {
            return;
        }

        if !self.selected_cards.is_empty() {
            self.start_permanent_delete_animations_for_selected();
        } else if let Some(card_id) = self.get_selected_card_id() {
            self.start_permanent_delete_animation(card_id);
        }
    }

    fn start_permanent_delete_animations_for_selected(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self.selected_cards.iter().copied().collect();
        for card_id in card_ids {
            self.start_permanent_delete_animation(card_id);
        }
        self.selected_cards.clear();
    }

    fn start_permanent_delete_animation(&mut self, card_id: uuid::Uuid) {
        use crate::app::{AnimationType, CardAnimation};
        use std::time::Instant;

        if self
            .ctx
            .archived_cards
            .iter()
            .any(|dc| dc.card.id == card_id)
        {
            self.animating_cards.insert(
                card_id,
                CardAnimation {
                    animation_type: AnimationType::Deleting,
                    start_time: Instant::now(),
                },
            );
        }
    }

    #[allow(dead_code)]
    fn permanent_delete_selected_cards(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self.selected_cards.iter().copied().collect();
        let mut deleted_count = 0;

        for card_id in card_ids {
            if let Some(card) = self
                .ctx
                .archived_cards
                .iter()
                .find(|dc| dc.card.id == card_id)
            {
                let card_title = card.card.title.clone();

                // Execute DeleteCard command
                let cmd = Box::new(DeleteCard { card_id });
                if let Err(e) = self.execute_command(cmd) {
                    tracing::error!("Failed to permanently delete card: {}", e);
                    continue;
                }

                tracing::info!("Permanently deleted card '{}'", card_title);
                deleted_count += 1;
            }
        }

        tracing::info!("Permanently deleted {} card(s)", deleted_count);
        self.selected_cards.clear();
        self.refresh_view();
    }

    #[allow(dead_code)]
    fn permanent_delete_card_at(&mut self, index: usize) {
        if index < self.ctx.archived_cards.len() {
            if let Some(card) = self.ctx.archived_cards.get(index) {
                let card_id = card.card.id;
                let card_title = card.card.title.clone();

                // Execute DeleteCard command
                let cmd = Box::new(DeleteCard { card_id });
                if let Err(e) = self.execute_command(cmd) {
                    tracing::error!("Failed to permanently delete card: {}", e);
                    return;
                }

                tracing::info!("Permanently deleted card '{}'", card_title);
                self.refresh_view();
            }
        }
    }

    pub fn handle_toggle_archived_cards_view(&mut self) {
        match self.mode {
            AppMode::Normal => {
                self.mode = AppMode::ArchivedCardsView;
                self.refresh_view();

                // Initialize selection in view strategy
                if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    if !list.is_empty() {
                        list.set_selected_index(Some(0));
                        list.ensure_selected_visible(self.viewport_height);
                    }
                }
            }
            AppMode::ArchivedCardsView => {
                self.mode = AppMode::Normal;
                self.refresh_view();

                // Re-initialize selection when returning to normal view
                if let Some(list) = self.view_strategy.get_active_task_list_mut() {
                    if !list.is_empty() {
                        list.set_selected_index(Some(0));
                        list.ensure_selected_visible(self.viewport_height);
                    }
                }
            }
            _ => {}
        }
    }
}
