use crate::app::{App, BoardFocus, DialogMode};
use crate::state::commands::{ActivateSprint, CompleteSprint, CreateSprint, UpdateBoard};
use kanban_domain::{BoardUpdate, FieldUpdate, SprintStatus};

impl App {
    pub fn handle_create_sprint_key(&mut self) {
        if self.board_focus == BoardFocus::Sprints && self.board_selection.get().is_some() {
            self.open_dialog(DialogMode::CreateSprint);
            self.input.clear();
        }
    }

    pub fn handle_activate_sprint_key(&mut self) {
        if let Some(sprint_idx) = self.active_sprint_index {
            // Collect sprint info before mutations
            let sprint_info = {
                if let Some(sprint) = self.ctx.sprints.get(sprint_idx) {
                    if sprint.status == SprintStatus::Planning {
                        Some((
                            sprint.id,
                            sprint.formatted_name(
                                &self.ctx.boards[self.board_selection.get().unwrap_or(0)],
                                "sprint",
                            ),
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((sprint_id, _)) = sprint_info {
                let board_idx = self.active_board_index.or(self.board_selection.get());
                if let Some(board_idx) = board_idx {
                    if let Some(board) = self.ctx.boards.get(board_idx) {
                        let duration = board.sprint_duration_days.unwrap_or(14);
                        let board_id = board.id;

                        // Execute ActivateSprint and UpdateBoard as batch
                        let activate_cmd = Box::new(ActivateSprint {
                            sprint_id,
                            duration_days: duration,
                        })
                            as Box<dyn crate::state::commands::Command>;

                        let board_cmd = Box::new(UpdateBoard {
                            board_id,
                            updates: BoardUpdate {
                                active_sprint_id: FieldUpdate::Set(sprint_id),
                                ..Default::default()
                            },
                        })
                            as Box<dyn crate::state::commands::Command>;

                        if let Err(e) = self.execute_commands_batch(vec![activate_cmd, board_cmd]) {
                            tracing::error!("Failed to activate sprint: {}", e);
                            return;
                        }

                        if let Some(board) = self.ctx.boards.get(board_idx) {
                            tracing::info!(
                                "Activated sprint: {}",
                                self.ctx
                                    .sprints
                                    .get(sprint_idx)
                                    .map(|s| s.formatted_name(board, "sprint"))
                                    .unwrap_or_default()
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn handle_complete_sprint_key(&mut self) {
        if let Some(sprint_idx) = self.active_sprint_index {
            // Collect sprint and board info before mutations
            let sprint_info = {
                if let Some(sprint) = self.ctx.sprints.get(sprint_idx) {
                    if sprint.status == SprintStatus::Active
                        || sprint.status == SprintStatus::Planning
                    {
                        let board_idx = self.active_board_index.or(self.board_selection.get());
                        board_idx.and_then(|board_idx| {
                            self.ctx.boards.get(board_idx).map(|board| {
                                (sprint.id, board.id, sprint.formatted_name(board, "sprint"))
                            })
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((sprint_id, board_id, sprint_name)) = sprint_info {
                // Execute CompleteSprint and UpdateBoard as batch
                let complete_cmd = Box::new(CompleteSprint { sprint_id })
                    as Box<dyn crate::state::commands::Command>;

                let board_cmd = Box::new(UpdateBoard {
                    board_id,
                    updates: BoardUpdate {
                        active_sprint_id: FieldUpdate::Clear,
                        ..Default::default()
                    },
                }) as Box<dyn crate::state::commands::Command>;

                if let Err(e) = self.execute_commands_batch(vec![complete_cmd, board_cmd]) {
                    tracing::error!("Failed to complete sprint: {}", e);
                    return;
                }

                self.active_sprint_filters.remove(&sprint_id);

                tracing::info!("Completed sprint: {}", sprint_name);

                self.pop_mode();
                self.board_focus = BoardFocus::Sprints;
                self.active_sprint_index = None;
            }
        }
    }

    pub fn create_sprint(&mut self) {
        let board_idx = self.active_board_index.or(self.board_selection.get());
        if let Some(board_idx) = board_idx {
            let (sprint_number, name_index, board_id, effective_sprint_prefix) = {
                if let Some(board) = self.ctx.boards.get_mut(board_idx) {
                    let effective_sprint_prefix = board
                        .sprint_prefix
                        .as_deref()
                        .unwrap_or("sprint")
                        .to_string();
                    // Ensure the counter for this prefix is initialized based on existing sprints
                    board.ensure_sprint_counter_initialized(
                        &effective_sprint_prefix,
                        &self.ctx.sprints,
                    );
                    let sprint_number = board.get_next_sprint_number(&effective_sprint_prefix);
                    let input_text = self.input.as_str().trim();
                    let name_index = if input_text.is_empty() {
                        board.consume_sprint_name()
                    } else {
                        Some(board.add_sprint_name_at_used_index(input_text.to_string()))
                    };
                    (sprint_number, name_index, board.id, effective_sprint_prefix)
                } else {
                    return;
                }
            };

            // Execute CreateSprint command
            let cmd = Box::new(CreateSprint {
                board_id,
                sprint_number,
                name_index,
                prefix: Some(effective_sprint_prefix.clone()),
            });

            if let Err(e) = self.execute_command(cmd) {
                tracing::error!("Failed to create sprint: {}", e);
                return;
            }

            // Log the newly created sprint
            let board_sprints: Vec<_> = self
                .ctx
                .sprints
                .iter()
                .filter(|s| s.board_id == board_id)
                .collect();

            if let Some(new_sprint) = board_sprints.last() {
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    tracing::info!(
                        "Creating sprint: {} (id: {})",
                        new_sprint.formatted_name(board, &effective_sprint_prefix),
                        new_sprint.id
                    );
                }
            }

            let new_index = board_sprints.len() - 1;
            self.sprint_selection.set(Some(new_index));
        }
    }
}
