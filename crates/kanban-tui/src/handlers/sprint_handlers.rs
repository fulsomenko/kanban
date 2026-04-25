use crate::app::{App, BoardFocus, DialogMode};
use kanban_domain::commands::{
    ActivateSprint, BoardCommand, Command, CompleteSprint, CreateSprint, SprintCommand, UpdateBoard,
};
use kanban_domain::{BoardUpdate, FieldUpdate, SprintStatus};
use uuid::Uuid;

impl App {
    pub fn handle_create_sprint_key(&mut self) {
        if self.focus.board_focus == BoardFocus::Sprints && self.selection.board.get().is_some() {
            self.open_dialog(DialogMode::CreateSprint);
            self.input.clear();
        }
    }

    pub fn handle_activate_sprint_key(&mut self) {
        if let Some(sprint_idx) = self.selection.active_sprint_index {
            // Collect sprint info before mutations
            let sprint_info = {
                if let Some(sprint) = self.model.sprints().get(sprint_idx) {
                    if sprint.status == SprintStatus::Planning {
                        Some((
                            sprint.id,
                            sprint.formatted_name(
                                &self.model.boards()[self.selection.board.get().unwrap_or(0)],
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

            if let Some((sprint_id, sprint_name)) = sprint_info {
                let board_idx = self
                    .selection
                    .active_board_index
                    .or(self.selection.board.get());
                if let Some(board_idx) = board_idx {
                    if let Some(board) = self.model.boards().get(board_idx) {
                        let duration = board.sprint_duration_days.unwrap_or(14);
                        let board_id = board.id;

                        // Execute ActivateSprint and UpdateBoard as batch
                        let activate_cmd =
                            Command::Sprint(SprintCommand::Activate(ActivateSprint {
                                sprint_id,
                                duration_days: duration,
                            }));

                        let board_cmd = Command::Board(BoardCommand::Update(UpdateBoard {
                            board_id,
                            updates: BoardUpdate {
                                active_sprint_id: FieldUpdate::Set(sprint_id),
                                ..Default::default()
                            },
                        }));

                        if let Err(e) = self.execute_commands_batch(vec![activate_cmd, board_cmd]) {
                            tracing::error!("Failed to activate sprint: {}", e);
                            self.set_error(format!("Failed to activate sprint: {}", e));
                            return;
                        }

                        tracing::info!("Activated sprint: {}", sprint_name);
                    }
                }
            }
        }
    }

    pub fn handle_complete_sprint_key(&mut self) {
        if let Some(sprint_idx) = self.selection.active_sprint_index {
            // Collect sprint and board info before mutations
            let sprint_info = {
                if let Some(sprint) = self.model.sprints().get(sprint_idx) {
                    if sprint.status == SprintStatus::Active
                        || sprint.status == SprintStatus::Planning
                    {
                        let board_idx = self
                            .selection
                            .active_board_index
                            .or(self.selection.board.get());
                        board_idx.and_then(|board_idx| {
                            self.model.boards().get(board_idx).map(|board| {
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
                let complete_cmd =
                    Command::Sprint(SprintCommand::Complete(CompleteSprint { sprint_id }));

                let board_cmd = Command::Board(BoardCommand::Update(UpdateBoard {
                    board_id,
                    updates: BoardUpdate {
                        active_sprint_id: FieldUpdate::Clear,
                        ..Default::default()
                    },
                }));

                if let Err(e) = self.execute_commands_batch(vec![complete_cmd, board_cmd]) {
                    tracing::error!("Failed to complete sprint: {}", e);
                    self.set_error(format!("Failed to complete sprint: {}", e));
                    return;
                }

                self.filter.active_sprint_filters.remove(&sprint_id);

                tracing::info!("Completed sprint: {}", sprint_name);

                self.pop_mode();
                self.focus.board_focus = BoardFocus::Sprints;
                self.selection.active_sprint_index = None;

                {
                    use kanban_domain::query::sprint::get_sprint_uncompleted_cards;
                    let has_planning = self.model.sprints().iter().any(|s| {
                        s.board_id == board_id
                            && s.status == SprintStatus::Planning
                            && s.id != sprint_id
                    });

                    if has_planning
                        && !get_sprint_uncompleted_cards(sprint_id, self.model.cards()).is_empty()
                    {
                        self.dialog_input.carry_over_source_sprint_id = Some(sprint_id);
                        self.dialog_input.carry_over_sprint_selection.set(Some(0));
                        self.open_dialog(DialogMode::CarryOverSprint);
                    }
                }
            }
        }
    }

    pub fn handle_carry_over_for_sprint(&mut self, from_sprint_id: Uuid) {
        let board_id = match self.model.sprints().iter().find(|s| s.id == from_sprint_id) {
            Some(sprint) => sprint.board_id,
            None => return,
        };

        let has_planning_sprint = self
            .model
            .sprints()
            .iter()
            .any(|s| s.board_id == board_id && s.status == SprintStatus::Planning);

        if has_planning_sprint {
            self.dialog_input.carry_over_source_sprint_id = Some(from_sprint_id);
            self.dialog_input.carry_over_sprint_selection.set(Some(0));
            self.open_dialog(DialogMode::CarryOverSprint);
        } else {
            self.set_error("No Planning sprint available for carry-over");
        }
    }

    pub fn create_sprint(&mut self) {
        let board_idx = self
            .selection
            .active_board_index
            .or(self.selection.board.get());
        if let Some(board_idx) = board_idx {
            let (board_id, name) = {
                if let Some(board) = self.model.boards().get(board_idx) {
                    let input_text = self.input.as_str().trim();
                    let name = if input_text.is_empty() {
                        None
                    } else {
                        Some(input_text.to_string())
                    };
                    (board.id, name)
                } else {
                    return;
                }
            };

            let default_sprint_prefix = self
                .app_config
                .effective_default_sprint_prefix()
                .to_string();

            let sprint_id = uuid::Uuid::new_v4();
            let prior_sprint_count = self
                .model
                .sprints()
                .iter()
                .filter(|s| s.board_id == board_id)
                .count();

            let cmd = Command::Sprint(SprintCommand::Create(CreateSprint {
                id: sprint_id,
                board_id,
                name,
                default_sprint_prefix: default_sprint_prefix.clone(),
                explicit_prefix: None,
                auto_consume_name: true,
            }));

            if let Err(e) = self.execute_command(cmd) {
                tracing::error!("Failed to create sprint: {}", e);
                self.set_error(format!("Failed to create sprint: {}", e));
                return;
            }

            tracing::info!("Created sprint (id: {})", sprint_id);

            self.selection.sprint.set(Some(prior_sprint_count));
        }
    }
}
