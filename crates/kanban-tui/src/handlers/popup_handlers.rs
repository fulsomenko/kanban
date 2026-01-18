use crate::app::App;
use crossterm::event::KeyCode;
use kanban_domain::{dependencies::CardGraphExt, FieldUpdate, SortField, SortOrder};

impl App {
    pub fn handle_import_board_popup(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.pop_mode();
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
                self.pop_mode();
                self.import_selection.clear();
            }
            _ => {}
        }
    }

    pub fn handle_set_card_priority_popup(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.pop_mode();
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
                        if let Some(card) = self.ctx.cards.get(card_idx) {
                            use kanban_domain::{CardPriority, CardUpdate};
                            let priority = match priority_idx {
                                0 => CardPriority::Low,
                                1 => CardPriority::Medium,
                                2 => CardPriority::High,
                                3 => CardPriority::Critical,
                                _ => CardPriority::Medium,
                            };
                            let card_id = card.id;
                            let cmd = Box::new(crate::state::commands::UpdateCard {
                                card_id,
                                updates: CardUpdate {
                                    priority: Some(priority),
                                    ..Default::default()
                                },
                            });
                            if let Err(e) = self.execute_command(cmd) {
                                tracing::error!("Failed to update card priority: {}", e);
                            }
                        }
                    }
                }
                self.pop_mode();
            }
            _ => {}
        }
    }

    pub fn handle_order_cards_popup(&mut self, key_code: KeyCode) -> bool {
        match key_code {
            KeyCode::Esc => {
                self.pop_mode();
                self.sort_field_selection.clear();
                false
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.sort_field_selection.next(7);
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
                        5 => SortField::Position,
                        6 => SortField::Default,
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
                        if let Some(board) = self.ctx.boards.get(board_idx) {
                            let board_id = board.id;
                            let cmd = Box::new(crate::state::commands::SetBoardTaskSort {
                                board_id,
                                field,
                                order,
                            });
                            if let Err(e) = self.execute_command(cmd) {
                                tracing::error!("Failed to set board task sort: {}", e);
                            }
                        }
                    }

                    let is_sprint_detail = self.active_sprint_index.is_some();
                    self.pop_mode();
                    self.sort_field_selection.clear();

                    tracing::info!("Sorting by {:?} ({:?})", field, order);

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
                self.pop_mode();
                self.sprint_assign_selection.clear();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(board_idx) = self.active_board_index {
                    if let Some(board) = self.ctx.boards.get(board_idx) {
                        let sprint_count = self
                            .ctx
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
                        let card_id = {
                            if let Some(card) = self.ctx.cards.get(card_idx) {
                                card.id
                            } else {
                                return;
                            }
                        };

                        if selection_idx == 0 {
                            // Unassign from sprint
                            let cmd = Box::new(crate::state::commands::UpdateCard {
                                card_id,
                                updates: kanban_domain::CardUpdate {
                                    sprint_id: FieldUpdate::Clear,
                                    assigned_prefix: FieldUpdate::Clear,
                                    ..Default::default()
                                },
                            });
                            if let Err(e) = self.execute_command(cmd) {
                                tracing::error!("Failed to unassign card from sprint: {}", e);
                            } else {
                                // Clear sprint log via direct mutation (domain operation)
                                if let Some(card) = self.ctx.cards.get_mut(card_idx) {
                                    card.end_current_sprint_log();
                                }
                                tracing::info!("Unassigned card from sprint");
                            }
                        } else if let Some(board_idx) = self.active_board_index {
                            if let Some(board_id) = self.ctx.boards.get(board_idx).map(|b| b.id) {
                                let board_sprints: Vec<_> = self
                                    .ctx
                                    .sprints
                                    .iter()
                                    .filter(|s| s.board_id == board_id)
                                    .collect();
                                if let Some(sprint) = board_sprints.get(selection_idx - 1) {
                                    let sprint_id = sprint.id;
                                    let sprint_number = sprint.sprint_number;
                                    let sprint_status = format!("{:?}", sprint.status);

                                    // Get effective prefix and sprint info before calling execute_command
                                    let effective_prefix = {
                                        if let Some(board) = self.ctx.boards.get(board_idx) {
                                            sprint.effective_prefix(board, "task").to_string()
                                        } else {
                                            "task".to_string()
                                        }
                                    };

                                    let sprint_name = {
                                        if let Some(board) = self.ctx.boards.get(board_idx) {
                                            sprint.get_name(board).map(|s| s.to_string())
                                        } else {
                                            None
                                        }
                                    };

                                    // Build batch of commands
                                    let mut commands: Vec<
                                        Box<dyn crate::state::commands::Command>,
                                    > = Vec::new();

                                    // First, assign to sprint
                                    let assign_cmd =
                                        Box::new(kanban_domain::commands::AssignCardToSprint {
                                            card_id,
                                            sprint_id,
                                            sprint_number,
                                            sprint_name,
                                            sprint_status,
                                        })
                                            as Box<dyn crate::state::commands::Command>;
                                    commands.push(assign_cmd);

                                    // Then, update the assigned prefix
                                    let update_cmd = Box::new(crate::state::commands::UpdateCard {
                                        card_id,
                                        updates: kanban_domain::CardUpdate {
                                            assigned_prefix: FieldUpdate::Set(
                                                effective_prefix.clone(),
                                            ),
                                            ..Default::default()
                                        },
                                    })
                                        as Box<dyn crate::state::commands::Command>;
                                    commands.push(update_cmd);

                                    // Execute all commands as a batch
                                    if let Err(e) = self.execute_commands_batch(commands) {
                                        tracing::error!("Failed to assign card to sprint: {}", e);
                                    } else {
                                        tracing::info!(
                                            "Assigned card to sprint with id: {}",
                                            sprint_id
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                self.pop_mode();
                self.sprint_assign_selection.clear();
            }
            _ => {}
        }
    }

    pub fn handle_assign_multiple_cards_to_sprint_popup(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.pop_mode();
                self.sprint_assign_selection.clear();
                self.selected_cards.clear();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(board_idx) = self.active_board_index {
                    if let Some(board) = self.ctx.boards.get(board_idx) {
                        let sprint_count = self
                            .ctx
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

                    if selection_idx == 0 {
                        // Unassign cards from sprint - batch all unassignments
                        let mut unassign_commands: Vec<Box<dyn crate::state::commands::Command>> =
                            Vec::new();
                        for card_id in &card_ids {
                            let cmd = Box::new(kanban_domain::commands::UnassignCardFromSprint {
                                card_id: *card_id,
                            })
                                as Box<dyn crate::state::commands::Command>;
                            unassign_commands.push(cmd);
                        }

                        if let Err(e) = self.execute_commands_batch(unassign_commands) {
                            tracing::error!("Failed to unassign cards from sprint: {}", e);
                        } else {
                            tracing::info!(
                                "Unassigned {} cards from sprint",
                                self.selected_cards.len()
                            );
                        }
                    } else if let Some(board_idx) = self.active_board_index {
                        if let Some(board_id) = self.ctx.boards.get(board_idx).map(|b| b.id) {
                            let board_sprints: Vec<_> = self
                                .ctx
                                .sprints
                                .iter()
                                .filter(|s| s.board_id == board_id)
                                .collect();
                            if let Some(sprint) = board_sprints.get(selection_idx - 1) {
                                let sprint_id = sprint.id;
                                let sprint_number = sprint.sprint_number;
                                let sprint_status = format!("{:?}", sprint.status);

                                // Get effective prefix and sprint info before the loop
                                let effective_prefix = {
                                    if let Some(board) = self.ctx.boards.get(board_idx) {
                                        sprint.effective_prefix(board, "task").to_string()
                                    } else {
                                        "task".to_string()
                                    }
                                };

                                let sprint_name = {
                                    if let Some(board) = self.ctx.boards.get(board_idx) {
                                        sprint.get_name(board).map(|s| s.to_string())
                                    } else {
                                        None
                                    }
                                };

                                // Build batch of commands for all cards
                                let mut commands: Vec<Box<dyn crate::state::commands::Command>> =
                                    Vec::new();
                                for card_id in &card_ids {
                                    // First, assign to sprint
                                    let assign_cmd =
                                        Box::new(kanban_domain::commands::AssignCardToSprint {
                                            card_id: *card_id,
                                            sprint_id,
                                            sprint_number,
                                            sprint_name: sprint_name.clone(),
                                            sprint_status: sprint_status.clone(),
                                        })
                                            as Box<dyn crate::state::commands::Command>;
                                    commands.push(assign_cmd);

                                    // Then, update the assigned prefix
                                    let update_cmd = Box::new(crate::state::commands::UpdateCard {
                                        card_id: *card_id,
                                        updates: kanban_domain::CardUpdate {
                                            assigned_prefix: FieldUpdate::Set(
                                                effective_prefix.clone(),
                                            ),
                                            ..Default::default()
                                        },
                                    })
                                        as Box<dyn crate::state::commands::Command>;
                                    commands.push(update_cmd);
                                }

                                // Execute all commands as a batch (single pause/resume cycle)
                                if let Err(e) = self.execute_commands_batch(commands) {
                                    tracing::error!("Failed to assign cards to sprint: {}", e);
                                }

                                tracing::info!(
                                    "Assigned {} cards to sprint with id: {}",
                                    self.selected_cards.len(),
                                    sprint_id
                                );
                            }
                        }
                    }
                }
                self.pop_mode();
                self.sprint_assign_selection.clear();
                self.selected_cards.clear();
            }
            _ => {}
        }
    }

    pub fn handle_manage_parents_popup(&mut self, key_code: KeyCode) {
        self.handle_relationship_popup(key_code, true);
    }

    pub fn handle_manage_children_popup(&mut self, key_code: KeyCode) {
        self.handle_relationship_popup(key_code, false);
    }

    fn handle_relationship_popup(&mut self, key_code: KeyCode, is_parent_mode: bool) {
        // Filter cards by search
        let filtered_cards: Vec<_> = if self.relationship_search.is_empty() {
            self.relationship_card_ids.clone()
        } else {
            let search_lower = self.relationship_search.to_lowercase();
            self.relationship_card_ids
                .iter()
                .filter(|card_id| {
                    self.ctx
                        .cards
                        .iter()
                        .find(|c| c.id == **card_id)
                        .map(|c| c.title.to_lowercase().contains(&search_lower))
                        .unwrap_or(false)
                })
                .copied()
                .collect()
        };

        let list_len = filtered_cards.len();

        // Handle search mode separately
        if self.relationship_search_active {
            match key_code {
                KeyCode::Esc => {
                    // Exit search mode but stay in dialog
                    self.relationship_search_active = false;
                }
                KeyCode::Enter => {
                    // Confirm search and exit search mode
                    self.relationship_search_active = false;
                }
                KeyCode::Backspace => {
                    self.relationship_search.pop();
                    self.update_relationship_selection_after_search();
                }
                KeyCode::Char(c) => {
                    self.relationship_search.push(c);
                    self.update_relationship_selection_after_search();
                }
                _ => {}
            }
            return;
        }

        // Navigation mode
        match key_code {
            KeyCode::Esc => {
                self.pop_mode();
                self.relationship_card_ids.clear();
                self.relationship_selected.clear();
                self.relationship_selection.clear();
                self.relationship_search.clear();
                self.relationship_search_active = false;
            }
            KeyCode::Char('/') => {
                // Enter search mode
                self.relationship_search_active = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.relationship_selection.next(list_len);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.relationship_selection.prev();
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                // Toggle relationship
                if let Some(idx) = self.relationship_selection.get() {
                    if let Some(selected_card_id) = filtered_cards.get(idx).copied() {
                        if let Some(card_idx) = self.active_card_index {
                            if let Some(current_card) = self.ctx.cards.get(card_idx) {
                                let current_card_id = current_card.id;

                                if self.relationship_selected.contains(&selected_card_id) {
                                    // Remove relationship
                                    let result = if is_parent_mode {
                                        // Current card is child, selected card is parent
                                        self.ctx
                                            .graph
                                            .cards
                                            .remove_parent(current_card_id, selected_card_id)
                                    } else {
                                        // Current card is parent, selected card is child
                                        self.ctx
                                            .graph
                                            .cards
                                            .remove_parent(selected_card_id, current_card_id)
                                    };

                                    if result.is_ok() {
                                        self.relationship_selected.remove(&selected_card_id);
                                        self.ctx.state_manager.mark_dirty();
                                        let snapshot = crate::state::DataSnapshot::from_app(self);
                                        self.ctx.state_manager.queue_snapshot(snapshot);
                                    }
                                } else {
                                    // Add relationship
                                    let result = if is_parent_mode {
                                        // Current card is child, selected card is parent
                                        self.ctx
                                            .graph
                                            .cards
                                            .set_parent(current_card_id, selected_card_id)
                                    } else {
                                        // Current card is parent, selected card is child
                                        self.ctx
                                            .graph
                                            .cards
                                            .set_parent(selected_card_id, current_card_id)
                                    };

                                    if result.is_ok() {
                                        self.relationship_selected.insert(selected_card_id);
                                        self.ctx.state_manager.mark_dirty();
                                        let snapshot = crate::state::DataSnapshot::from_app(self);
                                        self.ctx.state_manager.queue_snapshot(snapshot);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn update_relationship_selection_after_search(&mut self) {
        let filtered_count = if self.relationship_search.is_empty() {
            self.relationship_card_ids.len()
        } else {
            let search_lower = self.relationship_search.to_lowercase();
            self.relationship_card_ids
                .iter()
                .filter(|card_id| {
                    self.ctx
                        .cards
                        .iter()
                        .find(|c| c.id == **card_id)
                        .map(|c| c.title.to_lowercase().contains(&search_lower))
                        .unwrap_or(false)
                })
                .count()
        };

        if filtered_count > 0 {
            self.relationship_selection.set(Some(0));
        } else {
            self.relationship_selection.clear();
        }
    }
}
