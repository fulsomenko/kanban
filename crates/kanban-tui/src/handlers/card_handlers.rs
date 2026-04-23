use crate::app::{App, AppMode, CardField, DialogMode, Focus};
use crate::card_list::CardListId;
use crate::events::EventHandler;
use kanban_domain::commands::{
    BoardCommand, CardCommand, Command, CreateCard, MoveCard, RestoreCard, SetBoardTaskSort,
    UpdateCard,
};
use kanban_domain::{ArchivedCard, CardStatus, CardUpdate, KanbanOperations, SortOrder, Sprint};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

impl App {
    pub fn handle_create_card_key(&mut self) {
        if self.focus.active == Focus::Cards && self.selection.active_board_index.is_some() {
            self.open_dialog(DialogMode::CreateCard);
            self.input.clear();
        }
    }

    fn get_focused_column_id(&mut self) -> Option<uuid::Uuid> {
        if let Some(task_list) = self.view.strategy.get_active_task_list() {
            if let CardListId::Column(column_id) = task_list.id {
                return Some(column_id);
            }
        }
        None
    }

    pub fn handle_toggle_card_completion(&mut self) {
        if self.focus.active != Focus::Cards {
            return;
        }

        if !self.multi_select.selected_cards.is_empty() {
            self.toggle_selected_cards_completion();
        } else {
            self.toggle_card_completion();
        }
    }

    pub fn handle_card_selection_toggle(&mut self) {
        if self.focus.active == Focus::Cards {
            if self.multi_select.selection_mode_active {
                // Exit selection mode (keep selections)
                self.multi_select.selection_mode_active = false;
            } else {
                // Enter selection mode and select current card
                self.multi_select.selection_mode_active = true;
                if let Some(card) = self.get_selected_card_in_context() {
                    self.multi_select.selected_cards.insert(card.id);
                }
            }
        }
    }

    pub fn handle_clear_card_selection(&mut self) {
        self.multi_select.selected_cards.clear();
    }

    pub fn handle_select_all_cards_in_view(&mut self) {
        if self.focus.active != Focus::Cards {
            return;
        }

        if let Some(task_list) = self.view.strategy.get_active_task_list() {
            for card_id in &task_list.cards {
                self.multi_select.selected_cards.insert(*card_id);
            }
            if !task_list.cards.is_empty() {
                self.multi_select.selection_mode_active = true;
            }
        }
    }

    pub fn handle_set_selected_cards_priority(&mut self) {
        if self.focus.active != Focus::Cards || self.multi_select.selected_cards.is_empty() {
            return;
        }

        self.dialog_input.priority_selection.set(Some(0));
        self.open_dialog(DialogMode::SetMultipleCardsPriority);
    }

    pub fn handle_assign_to_sprint_key(&mut self) {
        if self.focus.active != Focus::Cards {
            return;
        }

        if !self.multi_select.selected_cards.is_empty() {
            self.dialog_input.sprint_assign_selection.clear();
            self.open_dialog(DialogMode::AssignMultipleCardsToSprint);
        } else if self.get_selected_card_id().is_some() {
            if let Some(board_idx) = self.selection.active_board_index {
                let boards = self.ctx.boards();
                if let Some(board) = boards.get(board_idx) {
                    let sprints = self.ctx.sprints();
                    let sprint_count = Sprint::assignable(&sprints, board.id).len();
                    if sprint_count > 0 {
                        if let Some(selected_card) = self.get_selected_card_in_context() {
                            let card_id = selected_card.id;
                            let actual_idx = self.ctx.cards().iter().position(|c| c.id == card_id);
                            self.selection.active_card_index = actual_idx;
                        }
                        let selection_idx = self.get_current_sprint_selection_index();
                        self.dialog_input
                            .sprint_assign_selection
                            .set(Some(selection_idx));
                        self.open_dialog(DialogMode::AssignCardToSprint);
                    }
                }
            }
        }
    }

    pub fn handle_order_cards_key(&mut self) {
        if self.focus.active == Focus::Cards && self.selection.active_board_index.is_some() {
            let sort_idx = self.get_current_sort_field_selection_index();
            self.filter.sort_field_selection.set(Some(sort_idx));
            self.open_dialog(DialogMode::OrderCards);
        }
    }

    pub fn handle_toggle_sort_order_key(&mut self) {
        if self.focus.active == Focus::Cards && self.selection.active_board_index.is_some() {
            if let Some(current_order) = self.filter.current_sort_order {
                let new_order = match current_order {
                    SortOrder::Ascending => SortOrder::Descending,
                    SortOrder::Descending => SortOrder::Ascending,
                };
                self.filter.current_sort_order = Some(new_order);

                if let Some(board_idx) = self.selection.active_board_index {
                    let boards = self.ctx.boards();
                    if let Some(board) = boards.get(board_idx) {
                        if let Some(field) = self.filter.current_sort_field {
                            let cmd = Command::Board(BoardCommand::SetTaskSort(SetBoardTaskSort {
                                board_id: board.id,
                                field,
                                order: new_order,
                            }));

                            if let Err(e) = self.execute_command(cmd) {
                                tracing::error!("Failed to set board task sort: {}", e);
                                self.set_error(format!("Failed to set board task sort: {}", e));
                                return;
                            }
                        }
                    }
                }

                tracing::info!("Toggled sort order to: {:?}", new_order);
                self.needs_redraw = true;
            }
        }
    }

    pub fn handle_toggle_hide_assigned(&mut self) {
        if self.focus.active == Focus::Cards && self.selection.active_board_index.is_some() {
            self.filter.hide_assigned_cards = !self.filter.hide_assigned_cards;
            let status = if self.filter.hide_assigned_cards {
                "enabled"
            } else {
                "disabled"
            };
            tracing::info!("Hide assigned cards: {}", status);

            self.needs_redraw = true;
        }
    }

    pub fn handle_toggle_sprint_filter(&mut self) {
        if self.focus.active == Focus::Cards && self.selection.active_board_index.is_some() {
            if let Some(board_idx) = self.selection.active_board_index {
                let boards = self.ctx.boards();
                if let Some(board) = boards.get(board_idx) {
                    if let Some(active_sprint_id) = board.active_sprint_id {
                        if self
                            .filter
                            .active_sprint_filters
                            .contains(&active_sprint_id)
                        {
                            self.filter.active_sprint_filters.remove(&active_sprint_id);
                            tracing::info!("Disabled sprint filter - showing all cards");
                        } else {
                            self.filter.active_sprint_filters.clear();
                            self.filter.active_sprint_filters.insert(active_sprint_id);
                            tracing::info!("Enabled sprint filter - showing active sprint only");
                        }

                        self.needs_redraw = true;
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
        if self.focus.active == Focus::Cards {
            if let Some(selected_card) = self.get_selected_card_in_context() {
                let card_id = selected_card.id;
                let actual_idx = self.ctx.cards().iter().position(|c| c.id == card_id);
                self.selection.active_card_index = actual_idx;

                if let Err(e) =
                    self.edit_card_field(terminal, event_handler, CardField::Description)
                {
                    tracing::error!("Failed to edit card description: {}", e);
                    self.set_error(format!("Failed to edit card description: {}", e));
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

            let boards = self.ctx.boards();
            let columns = self.ctx.columns();
            let cards = self.ctx.cards();
            let toggle_result = self.selection.active_board_index.and_then(|idx| {
                boards.get(idx).and_then(|board| {
                    kanban_domain::card_lifecycle::compute_completion_toggle(
                        &card, board, &columns, &cards,
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

            let cmd = Command::Card(CardCommand::Update(UpdateCard { card_id, updates }));
            if let Err(e) = self.execute_command(cmd) {
                tracing::error!("Failed to toggle card completion: {}", e);
                self.set_error(format!("Failed to toggle card completion: {}", e));
                return;
            }

            self.select_card_by_id(card_id);
        }
    }

    fn toggle_selected_cards_completion(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self.multi_select.selected_cards.iter().copied().collect();
        let mut toggled_count = 0;
        let first_card_id = card_ids.first().copied();

        let mut update_commands: Vec<Command> = Vec::new();

        for card_id in card_ids {
            let all_cards = self.ctx.cards();
            let card = match all_cards.iter().find(|c| c.id == card_id) {
                Some(c) => c.clone(),
                None => continue,
            };

            let new_status = if card.status == CardStatus::Done {
                CardStatus::Todo
            } else {
                CardStatus::Done
            };

            let boards = self.ctx.boards();
            let columns = self.ctx.columns();
            let cards = self.ctx.cards();
            let toggle_result = self.selection.active_board_index.and_then(|idx| {
                boards.get(idx).and_then(|board| {
                    kanban_domain::card_lifecycle::compute_completion_toggle(
                        &card, board, &columns, &cards,
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

            let cmd = Command::Card(CardCommand::Update(UpdateCard { card_id, updates }));
            update_commands.push(cmd);
            toggled_count += 1;
        }

        if !update_commands.is_empty() {
            if let Err(e) = self.execute_commands_batch(update_commands) {
                tracing::error!("Failed to toggle card completion: {}", e);
                self.set_error(format!("Failed to toggle card completion: {}", e));
                return;
            }
        }

        tracing::info!("Toggled {} cards completion status", toggled_count);
        self.multi_select.selected_cards.clear();
        self.multi_select.selection_mode_active = false;
        if let Some(card_id) = first_card_id {
            self.select_card_by_id(card_id);
        }
    }

    pub fn create_card(&mut self) {
        if let Some(idx) = self.selection.active_board_index {
            let focused_col_id = self.get_focused_column_id();
            let board_info = self.ctx.boards().get(idx).map(|b| (b.id, b.card_counter));

            if let Some((bid, card_number)) = board_info {
                let target_column_id = if let Some(focused_col_id) = focused_col_id {
                    Some(focused_col_id)
                } else {
                    self.ctx
                        .columns()
                        .iter()
                        .find(|col| col.board_id == bid)
                        .map(|col| col.id)
                };

                let column = if let Some(col_id) = target_column_id {
                    self.ctx
                        .columns()
                        .iter()
                        .find(|col| col.id == col_id)
                        .cloned()
                } else {
                    None
                };

                let column = match column {
                    Some(col) => col,
                    None => match self.ctx.create_column(bid, "Todo".to_string(), Some(0)) {
                        Ok(col) => col,
                        Err(e) => {
                            tracing::error!("Failed to create column: {}", e);
                            self.set_error(format!("Failed to create column: {}", e));
                            return;
                        }
                    },
                };

                let cards = self.ctx.cards();
                let position =
                    kanban_domain::card_lifecycle::next_position_in_column(&cards, column.id);

                let boards = self.ctx.boards();
                let columns = self.ctx.columns();
                let mark_as_complete = boards
                    .get(idx)
                    .map(|board| {
                        kanban_domain::card_lifecycle::should_auto_complete_new_card(
                            column.id, board, &columns,
                        )
                    })
                    .unwrap_or(false);

                let create_cmd = Command::Card(CardCommand::Create(CreateCard {
                    id: uuid::Uuid::new_v4(),
                    card_number,
                    board_id: bid,
                    column_id: column.id,
                    title: self.input.as_str().to_string(),
                    position,
                    options: kanban_domain::CreateCardOptions::default(),
                    timestamp: chrono::Utc::now(),
                }));

                if let Err(e) = self.execute_command(create_cmd) {
                    tracing::error!("Failed to create card: {}", e);
                    self.set_error(format!("Failed to create card: {}", e));
                    return;
                }

                if mark_as_complete {
                    if let Some(card) = self
                        .ctx
                        .cards()
                        .iter()
                        .rev()
                        .find(|c| c.column_id == column.id)
                    {
                        let card_id = card.id;
                        let update_cmd = Command::Card(CardCommand::Update(UpdateCard {
                            card_id,
                            updates: CardUpdate {
                                status: Some(CardStatus::Done),
                                ..Default::default()
                            },
                        }));

                        if let Err(e) = self.execute_command(update_cmd) {
                            tracing::error!("Failed to update card status: {}", e);
                            self.set_error(format!("Failed to update card status: {}", e));
                        }
                    }
                }

                // Select the most recently created card
                if let Some(card) = self
                    .ctx
                    .cards()
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
        if self.focus.active != Focus::Cards {
            return;
        }

        if !self.multi_select.selected_cards.is_empty() {
            self.move_selected_cards(direction);
            return;
        }

        if let Some(card) = self.get_selected_card_in_context() {
            let boards = self.ctx.boards();
            let board = self
                .selection
                .active_board_index
                .and_then(|idx| boards.get(idx));
            let board = match board {
                Some(b) => b,
                None => return,
            };

            let columns = self.ctx.columns();
            let cards = self.ctx.cards();
            let move_result = kanban_domain::card_lifecycle::compute_card_column_move(
                &card, board, &columns, &cards, direction,
            );

            let move_result = match move_result {
                Some(r) => r,
                None => return,
            };

            let card_id = card.id;
            let mut commands: Vec<Command> = Vec::new();

            commands.push(Command::Card(CardCommand::Move(MoveCard {
                card_id,
                new_column_id: move_result.target_column_id,
                new_position: move_result.new_position,
            })));

            if let Some(new_status) = move_result.new_status {
                commands.push(Command::Card(CardCommand::Update(UpdateCard {
                    card_id,
                    updates: CardUpdate {
                        status: Some(new_status),
                        ..Default::default()
                    },
                })));
            }

            if let Err(e) = self.execute_commands_batch(commands) {
                let dir = match direction {
                    kanban_domain::card_lifecycle::MoveDirection::Left => "left",
                    kanban_domain::card_lifecycle::MoveDirection::Right => "right",
                };
                tracing::error!("Failed to move card {}: {}", dir, e);
                self.set_error(format!("Failed to move card {}: {}", dir, e));
                return;
            }

            if self.is_kanban_view() {
                if let Some(current_col_idx) = self.dialog_input.column_selection.get() {
                    match direction {
                        kanban_domain::card_lifecycle::MoveDirection::Left => {
                            if current_col_idx > 0 {
                                self.dialog_input
                                    .column_selection
                                    .set(Some(current_col_idx - 1));
                            }
                        }
                        kanban_domain::card_lifecycle::MoveDirection::Right => {
                            let boards = self.ctx.boards();
                            let columns = self.ctx.columns();
                            let num_cols = self
                                .selection
                                .active_board_index
                                .and_then(|idx| boards.get(idx))
                                .map(|b| columns.iter().filter(|c| c.board_id == b.id).count())
                                .unwrap_or(0);
                            if current_col_idx < num_cols - 1 {
                                self.dialog_input
                                    .column_selection
                                    .set(Some(current_col_idx + 1));
                            }
                        }
                    }
                }
            }

            self.select_card_by_id(card_id);
        }
    }

    fn move_selected_cards(&mut self, direction: kanban_domain::card_lifecycle::MoveDirection) {
        let boards = self.ctx.boards();
        let board = self
            .selection
            .active_board_index
            .and_then(|idx| boards.get(idx));
        let board = match board {
            Some(b) => b,
            None => return,
        };

        let card_ids: Vec<uuid::Uuid> = self.multi_select.selected_cards.iter().copied().collect();
        let first_card_id = card_ids.first().copied();
        let mut commands: Vec<Command> = Vec::new();
        let mut moved_count = 0;

        for card_id in &card_ids {
            let all_cards = self.ctx.cards();
            let card = match all_cards.iter().find(|c| c.id == *card_id) {
                Some(c) => c,
                None => continue,
            };

            let columns = self.ctx.columns();
            let cards = self.ctx.cards();
            let move_result = kanban_domain::card_lifecycle::compute_card_column_move(
                card, board, &columns, &cards, direction,
            );

            let move_result = match move_result {
                Some(r) => r,
                None => continue,
            };

            commands.push(Command::Card(CardCommand::Move(MoveCard {
                card_id: *card_id,
                new_column_id: move_result.target_column_id,
                new_position: move_result.new_position,
            })));

            if let Some(new_status) = move_result.new_status {
                commands.push(Command::Card(CardCommand::Update(UpdateCard {
                    card_id: *card_id,
                    updates: CardUpdate {
                        status: Some(new_status),
                        ..Default::default()
                    },
                })));
            }

            moved_count += 1;
        }

        if !commands.is_empty() {
            if let Err(e) = self.execute_commands_batch(commands) {
                let dir = match direction {
                    kanban_domain::card_lifecycle::MoveDirection::Left => "left",
                    kanban_domain::card_lifecycle::MoveDirection::Right => "right",
                };
                tracing::error!("Failed to move cards {}: {}", dir, e);
                self.set_error(format!("Failed to move cards {}: {}", dir, e));
                return;
            }
        }

        tracing::info!("Moved {} cards", moved_count);
        self.multi_select.selected_cards.clear();
        self.multi_select.selection_mode_active = false;
        if let Some(card_id) = first_card_id {
            self.select_card_by_id(card_id);
        }
    }

    pub fn handle_archive_card(&mut self) {
        if self.focus.active != Focus::Cards {
            return;
        }

        if !self.multi_select.selected_cards.is_empty() {
            self.start_delete_animations_for_selected();
        } else if let Some(card_id) = self.get_selected_card_id() {
            self.start_delete_animation(card_id);
        }
    }

    fn start_delete_animations_for_selected(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self.multi_select.selected_cards.iter().copied().collect();
        for card_id in card_ids {
            self.start_delete_animation(card_id);
        }
        self.multi_select.selected_cards.clear();
        self.multi_select.selection_mode_active = false;
    }

    pub fn start_delete_animation(&mut self, card_id: uuid::Uuid) {
        use crate::app::CardAnimation;
        use kanban_domain::AnimationType;
        use std::time::Instant;

        if self.ctx.cards().iter().any(|c| c.id == card_id) {
            self.animation.animating.insert(
                card_id,
                CardAnimation {
                    animation_type: AnimationType::Archiving,
                    start_time: Instant::now(),
                },
            );
        }
    }

    pub fn compact_column_positions(&mut self, column_id: uuid::Uuid) {
        let cmd = Command::Card(CardCommand::CompactPositions(
            kanban_domain::commands::CompactColumnPositions { column_id },
        ));
        if let Err(e) = self.ctx.execute_command(cmd) {
            tracing::error!("Failed to compact column positions: {}", e);
            self.set_error(format!("Failed to compact column positions: {}", e));
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
            .cards()
            .iter()
            .find(|c| c.column_id == deleted_column_id && c.position >= deleted_position)
        {
            self.select_card_by_id(next_card.id);
        } else if let Some(prev_card) = self
            .ctx
            .cards()
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

        if !self.multi_select.selected_cards.is_empty() {
            self.start_restore_animations_for_selected();
        } else if let Some(card_id) = self.get_selected_card_id() {
            self.start_restore_animation(card_id);
        }
    }

    fn start_restore_animations_for_selected(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self.multi_select.selected_cards.iter().copied().collect();
        for card_id in card_ids {
            self.start_restore_animation(card_id);
        }
        self.multi_select.selected_cards.clear();
        self.multi_select.selection_mode_active = false;
    }

    fn start_restore_animation(&mut self, card_id: uuid::Uuid) {
        use crate::app::CardAnimation;
        use kanban_domain::AnimationType;
        use std::time::Instant;

        if self
            .ctx
            .archived_cards()
            .iter()
            .any(|dc| dc.card.id == card_id)
        {
            self.animation.animating.insert(
                card_id,
                CardAnimation {
                    animation_type: AnimationType::Restoring,
                    start_time: Instant::now(),
                },
            );
        }
    }

    pub fn restore_card(&mut self, archived_card: ArchivedCard) {
        let card_id = archived_card.card.id;
        let original_column_id = archived_card.original_column_id;
        let original_position = archived_card.original_position;
        let card_title = archived_card.card.title.clone();

        let boards = self.ctx.boards();
        let board_id = self
            .selection
            .active_board_index
            .and_then(|idx| boards.get(idx))
            .map(|b| b.id);

        let columns = self.ctx.columns();
        let target_column_id = board_id
            .and_then(|bid| {
                kanban_domain::card_lifecycle::resolve_restore_column(
                    original_column_id,
                    bid,
                    &columns,
                )
            })
            .unwrap_or(original_column_id);

        let cmd = Command::Card(CardCommand::Restore(RestoreCard {
            card_id,
            column_id: target_column_id,
            position: original_position,
            timestamp: chrono::Utc::now(),
        }));

        if let Err(e) = self.execute_command(cmd) {
            tracing::error!("Failed to restore card: {}", e);
            self.set_error(format!("Failed to restore card: {}", e));
            return;
        }

        tracing::info!("Card '{}' restored to original position", card_title);
    }

    pub fn handle_delete_card_permanent(&mut self) {
        if self.mode != AppMode::ArchivedCardsView {
            return;
        }

        if !self.multi_select.selected_cards.is_empty() {
            self.start_permanent_delete_animations_for_selected();
        } else if let Some(card_id) = self.get_selected_card_id() {
            self.start_permanent_delete_animation(card_id);
        }
    }

    fn start_permanent_delete_animations_for_selected(&mut self) {
        let card_ids: Vec<uuid::Uuid> = self.multi_select.selected_cards.iter().copied().collect();
        for card_id in card_ids {
            self.start_permanent_delete_animation(card_id);
        }
        self.multi_select.selected_cards.clear();
        self.multi_select.selection_mode_active = false;
    }

    fn start_permanent_delete_animation(&mut self, card_id: uuid::Uuid) {
        use crate::app::CardAnimation;
        use kanban_domain::AnimationType;
        use std::time::Instant;

        if self
            .ctx
            .archived_cards()
            .iter()
            .any(|dc| dc.card.id == card_id)
        {
            self.animation.animating.insert(
                card_id,
                CardAnimation {
                    animation_type: AnimationType::Deleting,
                    start_time: Instant::now(),
                },
            );
        }
    }

    pub fn handle_toggle_archived_cards_view(&mut self) {
        match self.mode {
            AppMode::Normal => {
                self.mode = AppMode::ArchivedCardsView;
                self.populate_render_data();
                self.refresh_strategy();

                // Initialize selection in view strategy
                if let Some(list) = self.view.strategy.get_active_task_list_mut() {
                    if !list.is_empty() {
                        list.set_selected_index(Some(0));
                        list.ensure_selected_visible(self.view.viewport_height);
                    }
                }
                self.needs_redraw = true;
            }
            AppMode::ArchivedCardsView => {
                self.mode = AppMode::Normal;
                self.populate_render_data();
                self.refresh_strategy();

                // Re-initialize selection when returning to normal view
                if let Some(list) = self.view.strategy.get_active_task_list_mut() {
                    if !list.is_empty() {
                        list.set_selected_index(Some(0));
                        list.ensure_selected_visible(self.view.viewport_height);
                    }
                }
                self.needs_redraw = true;
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
        let boards = self.ctx.boards();
        let board_id = match self.selection.active_board_index {
            Some(idx) => match boards.get(idx) {
                Some(board) => board.id,
                None => return,
            },
            None => return,
        };

        // Get ancestors to exclude (would create cycle)
        let graph = self.ctx.graph();
        let ancestors = graph.cards.ancestors(card_id);

        // Get cards from current board, excluding self and ancestors
        let columns = self.ctx.columns();
        let column_ids: std::collections::HashSet<_> = columns
            .iter()
            .filter(|c| c.board_id == board_id)
            .map(|c| c.id)
            .collect();

        let cards = self.ctx.cards();
        let eligible_cards: Vec<_> = cards
            .iter()
            .filter(|c| column_ids.contains(&c.column_id))
            .filter(|c| c.id != card_id)
            .filter(|c| !ancestors.contains(&c.id))
            .map(|c| c.id)
            .collect();

        // Get current children (for checkbox display)
        let graph = self.ctx.graph();
        let current_children: std::collections::HashSet<_> =
            graph.cards.children(card_id).into_iter().collect();

        // Store the card index so the popup knows which card we're managing
        let cards = self.ctx.cards();
        self.selection.active_card_index = cards.iter().position(|c| c.id == card_id);

        // Set up dialog state
        self.relationship.card_ids = eligible_cards;
        self.relationship.selected = current_children;
        self.relationship.selection.set(Some(0));
        self.relationship.search.clear();

        self.open_dialog(DialogMode::ManageChildren);
    }
}
