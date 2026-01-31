use crate::app::{App, AppMode, CardField, DialogMode, Focus};
use crate::card_list::CardListId;
use crate::events::EventHandler;
use kanban_domain::commands::{
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
            let new_status = if card.status == CardStatus::Done {
                CardStatus::Todo
            } else {
                CardStatus::Done
            };

            let toggle_result = self.active_board_index.and_then(|idx| {
                self.ctx.boards.get(idx).and_then(|board| {
                    kanban_domain::card_lifecycle::compute_completion_toggle(
                        card,
                        board,
                        &self.ctx.columns,
                        &self.ctx.cards,
                    )
                })
            });

            let mut updates = CardUpdate {
                status: Some(new_status),
                ..Default::default()
            };

            if let Some(ref result) = toggle_result {
                updates.column_id = Some(result.target_column_id);
                updates.position = Some(result.new_position);
                updates.status = Some(result.new_status);
            }

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

        let mut update_commands: Vec<Box<dyn kanban_domain::commands::Command>> = Vec::new();

        for card_id in card_ids {
            let card = match self.ctx.cards.iter().find(|c| c.id == card_id) {
                Some(c) => c.clone(),
                None => continue,
            };

            let new_status = if card.status == CardStatus::Done {
                CardStatus::Todo
            } else {
                CardStatus::Done
            };

            let toggle_result = self.active_board_index.and_then(|idx| {
                self.ctx.boards.get(idx).and_then(|board| {
                    kanban_domain::card_lifecycle::compute_completion_toggle(
                        &card,
                        board,
                        &self.ctx.columns,
                        &self.ctx.cards,
                    )
                })
            });

            let mut updates = CardUpdate {
                status: Some(new_status),
                ..Default::default()
            };

            if let Some(ref result) = toggle_result {
                updates.column_id = Some(result.target_column_id);
                updates.position = Some(result.new_position);
                updates.status = Some(result.new_status);
            }

            let cmd = Box::new(UpdateCard { card_id, updates })
                as Box<dyn kanban_domain::commands::Command>;
            update_commands.push(cmd);
            toggled_count += 1;
        }

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

                let mark_as_complete = self
                    .ctx
                    .boards
                    .get(idx)
                    .map(|board| {
                        kanban_domain::card_lifecycle::should_auto_complete_new_card(
                            column.id,
                            board,
                            &self.ctx.columns,
                        )
                    })
                    .unwrap_or(false);

                let create_cmd = Box::new(CreateCard {
                    board_id: bid,
                    column_id: column.id,
                    title: self.input.as_str().to_string(),
                    position,
                });

                if let Err(e) = self.execute_command(create_cmd) {
                    tracing::error!("Failed to create card: {}", e);
                    return;
                }

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
                        });

                        if let Err(e) = self.execute_command(update_cmd) {
                            tracing::error!("Failed to update card status: {}", e);
                        }
                    }
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
        self.handle_move_card(kanban_domain::card_lifecycle::MoveDirection::Left);
    }

    pub fn handle_move_card_right(&mut self) {
        self.handle_move_card(kanban_domain::card_lifecycle::MoveDirection::Right);
    }

    fn handle_move_card(&mut self, direction: kanban_domain::card_lifecycle::MoveDirection) {
        if self.focus != Focus::Cards {
            return;
        }

        if let Some(card) = self.get_selected_card_in_context() {
            let board = self
                .active_board_index
                .and_then(|idx| self.ctx.boards.get(idx));
            let board = match board {
                Some(b) => b,
                None => return,
            };

            let move_result = kanban_domain::card_lifecycle::compute_card_column_move(
                card,
                board,
                &self.ctx.columns,
                &self.ctx.cards,
                direction,
            );

            let move_result = match move_result {
                Some(r) => r,
                None => return,
            };

            let card_id = card.id;
            let mut commands: Vec<Box<dyn kanban_domain::commands::Command>> = Vec::new();

            commands.push(Box::new(MoveCard {
                card_id,
                new_column_id: move_result.target_column_id,
                new_position: move_result.new_position,
            }));

            if let Some(new_status) = move_result.new_status {
                commands.push(Box::new(UpdateCard {
                    card_id,
                    updates: CardUpdate {
                        status: Some(new_status),
                        ..Default::default()
                    },
                }));
            }

            if let Err(e) = self.execute_commands_batch(commands) {
                let dir = match direction {
                    kanban_domain::card_lifecycle::MoveDirection::Left => "left",
                    kanban_domain::card_lifecycle::MoveDirection::Right => "right",
                };
                tracing::error!("Failed to move card {}: {}", dir, e);
                return;
            }

            if self.is_kanban_view() {
                if let Some(current_col_idx) = self.column_selection.get() {
                    match direction {
                        kanban_domain::card_lifecycle::MoveDirection::Left => {
                            if current_col_idx > 0 {
                                self.column_selection.set(Some(current_col_idx - 1));
                            }
                        }
                        kanban_domain::card_lifecycle::MoveDirection::Right => {
                            let num_cols = self
                                .active_board_index
                                .and_then(|idx| self.ctx.boards.get(idx))
                                .map(|b| {
                                    self.ctx
                                        .columns
                                        .iter()
                                        .filter(|c| c.board_id == b.id)
                                        .count()
                                })
                                .unwrap_or(0);
                            if current_col_idx < num_cols - 1 {
                                self.column_selection.set(Some(current_col_idx + 1));
                            }
                        }
                    }
                }
            }

            self.refresh_view();
            self.select_card_by_id(card_id);
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
        use crate::app::CardAnimation;
        use kanban_domain::AnimationType;
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
        kanban_domain::card_lifecycle::compact_column_positions(&mut self.ctx.cards, column_id);
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
        use crate::app::CardAnimation;
        use kanban_domain::AnimationType;
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

        let board_id = self
            .active_board_index
            .and_then(|idx| self.ctx.boards.get(idx))
            .map(|b| b.id);

        let target_column_id = board_id
            .and_then(|bid| {
                kanban_domain::card_lifecycle::resolve_restore_column(
                    original_column_id,
                    bid,
                    &self.ctx.columns,
                )
            })
            .unwrap_or(original_column_id);

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
        use crate::app::CardAnimation;
        use kanban_domain::AnimationType;
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

    pub fn handle_manage_children_from_list(&mut self) {
        use kanban_domain::dependencies::CardGraphExt;

        // Get the currently selected card from the list view
        let card = match self.get_selected_card_in_context() {
            Some(c) => c,
            None => return,
        };

        let card_id = card.id;

        // Get the board ID for filtering
        let board_id = match self.active_board_index {
            Some(idx) => match self.ctx.boards.get(idx) {
                Some(board) => board.id,
                None => return,
            },
            None => return,
        };

        // Get ancestors to exclude (would create cycle)
        let ancestors = self.ctx.graph.cards.ancestors(card_id);

        // Get cards from current board, excluding self and ancestors
        let column_ids: std::collections::HashSet<_> = self
            .ctx
            .columns
            .iter()
            .filter(|c| c.board_id == board_id)
            .map(|c| c.id)
            .collect();

        let eligible_cards: Vec<_> = self
            .ctx
            .cards
            .iter()
            .filter(|c| column_ids.contains(&c.column_id))
            .filter(|c| c.id != card_id)
            .filter(|c| !ancestors.contains(&c.id))
            .map(|c| c.id)
            .collect();

        // Get current children (for checkbox display)
        let current_children: std::collections::HashSet<_> =
            self.ctx.graph.cards.children(card_id).into_iter().collect();

        // Store the card index so the popup knows which card we're managing
        self.active_card_index = self.ctx.cards.iter().position(|c| c.id == card_id);

        // Set up dialog state
        self.relationship_card_ids = eligible_cards;
        self.relationship_selected = current_children;
        self.relationship_selection.set(Some(0));
        self.relationship_search.clear();

        self.open_dialog(DialogMode::ManageChildren);
    }
}
