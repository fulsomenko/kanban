use crate::app::{App, BoardFocus, DialogMode};
use crate::dialog::{handle_dialog_input, DialogAction};
use crossterm::event::KeyCode;
use kanban_domain::{Card, FieldUpdate};

/// Context for handling different types of prefix dialogs
enum PrefixDialogContext {
    /// Board-level sprint prefix
    BoardSprint,
    /// Sprint-level sprint prefix override
    Sprint,
    /// Sprint-level card prefix override
    SprintCard,
}

impl App {
    pub fn handle_create_board_dialog(&mut self, key_code: KeyCode) {
        match handle_dialog_input(&mut self.input, key_code, false) {
            DialogAction::Confirm => {
                self.create_board();
                self.pop_mode();
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.pop_mode();
                self.input.clear();
            }
            DialogAction::None => {}
        }
    }

    pub fn handle_create_card_dialog(&mut self, key_code: KeyCode) {
        match handle_dialog_input(&mut self.input, key_code, false) {
            DialogAction::Confirm => {
                self.create_card();
                self.pop_mode();
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.pop_mode();
                self.input.clear();
            }
            DialogAction::None => {}
        }
    }

    pub fn handle_create_sprint_dialog(&mut self, key_code: KeyCode) {
        match handle_dialog_input(&mut self.input, key_code, true) {
            DialogAction::Confirm => {
                self.create_sprint();
                self.pop_mode();
                self.board_focus = BoardFocus::Sprints;
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.pop_mode();
                self.board_focus = BoardFocus::Sprints;
                self.input.clear();
            }
            DialogAction::None => {}
        }
    }

    pub fn handle_rename_board_dialog(&mut self, key_code: KeyCode) {
        match handle_dialog_input(&mut self.input, key_code, false) {
            DialogAction::Confirm => {
                self.rename_board();
                self.pop_mode();
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.pop_mode();
                self.input.clear();
            }
            DialogAction::None => {}
        }
    }

    pub fn handle_export_board_dialog(&mut self, key_code: KeyCode) {
        match handle_dialog_input(&mut self.input, key_code, false) {
            DialogAction::Confirm => {
                if let Err(e) = self.export_board_with_filename() {
                    tracing::error!("Failed to export board: {}", e);
                }
                self.pop_mode();
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.pop_mode();
                self.input.clear();
            }
            DialogAction::None => {}
        }
    }

    pub fn handle_export_all_dialog(&mut self, key_code: KeyCode) {
        match handle_dialog_input(&mut self.input, key_code, false) {
            DialogAction::Confirm => {
                if let Err(e) = self.export_all_boards_with_filename() {
                    tracing::error!("Failed to export all boards: {}", e);
                }
                self.pop_mode();
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.pop_mode();
                self.input.clear();
            }
            DialogAction::None => {}
        }
    }

    pub fn handle_set_card_points_dialog(&mut self, key_code: KeyCode) -> bool {
        match handle_dialog_input(&mut self.input, key_code, true) {
            DialogAction::Confirm => {
                let input_str = self.input.as_str().trim();
                let points = if input_str.is_empty() {
                    None
                } else if let Ok(p) = input_str.parse::<u8>() {
                    if (1..=5).contains(&p) {
                        Some(p)
                    } else {
                        tracing::error!("Points must be between 1-5");
                        self.pop_mode();
                        self.input.clear();
                        return false;
                    }
                } else {
                    tracing::error!("Invalid points value");
                    self.pop_mode();
                    self.input.clear();
                    return false;
                };

                let card_id = self
                    .active_card_index
                    .and_then(|idx| self.ctx.cards.get(idx))
                    .map(|c| c.id)
                    .or_else(|| self.get_selected_card_in_context().map(|c| c.id));

                if let Some(card_id) = card_id {
                    let cmd = Box::new(kanban_domain::commands::UpdateCard {
                        card_id,
                        updates: kanban_domain::CardUpdate {
                            points: points.into(),
                            ..Default::default()
                        },
                    });
                    if let Err(e) = self.execute_command(cmd) {
                        tracing::error!("Failed to set card points: {}", e);
                    } else {
                        tracing::info!("Set points to: {:?}", points);
                    }
                }
                self.pop_mode();
                self.input.clear();
                false
            }
            DialogAction::Cancel => {
                self.pop_mode();
                self.input.clear();
                false
            }
            DialogAction::None => false,
        }
    }

    /// Generic handler for prefix dialogs that handles common validation and state management
    fn handle_prefix_dialog_impl(&mut self, key_code: KeyCode, context: PrefixDialogContext) {
        match handle_dialog_input(&mut self.input, key_code, true) {
            DialogAction::Confirm => {
                let prefix_str = self.input.as_str().trim().to_string();

                if prefix_str.is_empty() {
                    match context {
                        PrefixDialogContext::BoardSprint => {
                            if let Some(board_idx) = self.board_selection.get() {
                                if let Some(board_id) = self.ctx.boards.get(board_idx).map(|b| b.id)
                                {
                                    let cmd = Box::new(kanban_domain::commands::UpdateBoard {
                                        board_id,
                                        updates: kanban_domain::BoardUpdate {
                                            sprint_prefix: FieldUpdate::Clear,
                                            ..Default::default()
                                        },
                                    });
                                    if let Err(e) = self.execute_command(cmd) {
                                        tracing::error!("Failed to clear sprint prefix: {}", e);
                                    } else {
                                        tracing::info!("Cleared sprint prefix");
                                    }
                                }
                            }
                        }
                        PrefixDialogContext::Sprint => {
                            if let Some(sprint_idx) = self.active_sprint_index {
                                if let Some(sprint_id) =
                                    self.ctx.sprints.get(sprint_idx).map(|s| s.id)
                                {
                                    let cmd = Box::new(kanban_domain::commands::UpdateSprint {
                                        sprint_id,
                                        updates: kanban_domain::SprintUpdate {
                                            prefix: FieldUpdate::Clear,
                                            ..Default::default()
                                        },
                                    });
                                    if let Err(e) = self.execute_command(cmd) {
                                        tracing::error!("Failed to clear sprint prefix: {}", e);
                                    } else {
                                        tracing::info!("Cleared sprint prefix");
                                    }
                                }
                            }
                        }
                        PrefixDialogContext::SprintCard => {
                            if let Some(sprint_idx) = self.active_sprint_index {
                                if let Some(sprint_id) =
                                    self.ctx.sprints.get(sprint_idx).map(|s| s.id)
                                {
                                    let cmd = Box::new(kanban_domain::commands::UpdateSprint {
                                        sprint_id,
                                        updates: kanban_domain::SprintUpdate {
                                            card_prefix: FieldUpdate::Clear,
                                            ..Default::default()
                                        },
                                    });
                                    if let Err(e) = self.execute_command(cmd) {
                                        tracing::error!(
                                            "Failed to clear sprint card prefix override: {}",
                                            e
                                        );
                                    } else {
                                        tracing::info!("Cleared sprint card prefix override");
                                    }
                                }
                            }
                        }
                    }
                } else if Card::validate_branch_prefix(&prefix_str) {
                    match context {
                        PrefixDialogContext::BoardSprint => {
                            if let Some(board_idx) = self.board_selection.get() {
                                if let Some(board_id) = self.ctx.boards.get(board_idx).map(|b| b.id)
                                {
                                    let cmd = Box::new(kanban_domain::commands::UpdateBoard {
                                        board_id,
                                        updates: kanban_domain::BoardUpdate {
                                            sprint_prefix: FieldUpdate::Set(prefix_str.clone()),
                                            ..Default::default()
                                        },
                                    });
                                    if let Err(e) = self.execute_command(cmd) {
                                        tracing::error!("Failed to set sprint prefix: {}", e);
                                    } else {
                                        tracing::info!("Set sprint prefix to: {}", prefix_str);
                                        if let Some(board) = self.ctx.boards.get_mut(board_idx) {
                                            board.ensure_sprint_counter_initialized(
                                                &prefix_str,
                                                &self.ctx.sprints,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        PrefixDialogContext::Sprint => {
                            if let Some(sprint_idx) = self.active_sprint_index {
                                if let Some(sprint_id) =
                                    self.ctx.sprints.get(sprint_idx).map(|s| s.id)
                                {
                                    let cmd = Box::new(kanban_domain::commands::UpdateSprint {
                                        sprint_id,
                                        updates: kanban_domain::SprintUpdate {
                                            prefix: FieldUpdate::Set(prefix_str.clone()),
                                            ..Default::default()
                                        },
                                    });
                                    if let Err(e) = self.execute_command(cmd) {
                                        tracing::error!("Failed to set sprint prefix: {}", e);
                                    } else {
                                        tracing::info!("Set sprint prefix to: {}", prefix_str);
                                    }
                                }
                            }
                            let board_idx = self.active_board_index.or(self.board_selection.get());
                            if let Some(board_idx) = board_idx {
                                if let Some(board) = self.ctx.boards.get_mut(board_idx) {
                                    board.ensure_sprint_counter_initialized(
                                        &prefix_str,
                                        &self.ctx.sprints,
                                    );
                                }
                            }
                        }
                        PrefixDialogContext::SprintCard => {
                            if let Some(sprint_idx) = self.active_sprint_index {
                                if let Some(sprint_id) =
                                    self.ctx.sprints.get(sprint_idx).map(|s| s.id)
                                {
                                    let cmd = Box::new(kanban_domain::commands::UpdateSprint {
                                        sprint_id,
                                        updates: kanban_domain::SprintUpdate {
                                            card_prefix: FieldUpdate::Set(prefix_str.clone()),
                                            ..Default::default()
                                        },
                                    });
                                    if let Err(e) = self.execute_command(cmd) {
                                        tracing::error!(
                                            "Failed to set sprint card prefix override: {}",
                                            e
                                        );
                                    } else {
                                        tracing::info!(
                                            "Set sprint card prefix override to: {}",
                                            prefix_str
                                        );
                                    }
                                }
                            }
                        }
                    }
                } else {
                    tracing::error!("Invalid prefix: use alphanumeric, hyphens, underscores only");
                }

                self.pop_mode();
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.pop_mode();
                self.input.clear();
            }
            DialogAction::None => {}
        }
    }

    pub fn handle_set_branch_prefix_dialog(&mut self, key_code: KeyCode) {
        self.handle_prefix_dialog_impl(key_code, PrefixDialogContext::BoardSprint);
    }

    pub fn handle_set_sprint_prefix_dialog(&mut self, key_code: KeyCode) {
        self.handle_prefix_dialog_impl(key_code, PrefixDialogContext::Sprint);
    }

    pub fn handle_set_sprint_card_prefix_dialog(&mut self, key_code: KeyCode) {
        self.handle_prefix_dialog_impl(key_code, PrefixDialogContext::SprintCard);
    }

    pub fn handle_confirm_sprint_prefix_collision_popup(&mut self, key_code: KeyCode) {
        use crossterm::event::KeyCode;
        match key_code {
            KeyCode::Esc => {
                self.pop_mode();
            }
            KeyCode::Enter | KeyCode::Char('y') => {
                // User confirmed they want to continue with the colliding prefix
                // The actual prefix application should happen before this mode is entered
                self.pop_mode();
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // User declined, go back to prefix dialog
                self.pop_mode();
                self.open_dialog(DialogMode::SetSprintPrefix);
            }
            _ => {}
        }
    }

    pub fn handle_conflict_resolution_popup(&mut self, key_code: KeyCode) {
        use crossterm::event::KeyCode;
        match key_code {
            KeyCode::Char('o') | KeyCode::Char('O') => {
                // Keep our changes - force overwrite
                // Store the action to be processed in the event loop
                self.pending_key = Some('o');
                self.pop_mode();
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                // Take their changes - reload from disk
                self.pending_key = Some('t');
                self.pop_mode();
            }
            KeyCode::Esc => {
                // Retry later - just go back to previous mode
                self.ctx.state_manager.clear_conflict();
                self.pop_mode();
            }
            _ => {}
        }
    }

    pub fn handle_external_change_detected_popup(&mut self, key_code: KeyCode) {
        use crossterm::event::KeyCode;
        match key_code {
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Reload from external file - discard local changes
                self.pending_key = Some('r');
                self.pop_mode();
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                // Keep local changes - continue editing
                self.pop_mode();
            }
            KeyCode::Esc => {
                // Dismiss dialog - continue with current state
                self.pop_mode();
            }
            _ => {}
        }
    }
}
