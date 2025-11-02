use crate::app::{App, AppMode, BoardFocus};
use kanban_domain::{Sprint, SprintStatus};

impl App {
    pub fn handle_create_sprint_key(&mut self) {
        if self.board_focus == BoardFocus::Sprints && self.board_selection.get().is_some() {
            self.mode = AppMode::CreateSprint;
            self.input.clear();
        }
    }

    pub fn handle_activate_sprint_key(&mut self) {
        if let Some(sprint_idx) = self.active_sprint_index {
            if let Some(sprint) = self.sprints.get_mut(sprint_idx) {
                if sprint.status == SprintStatus::Planning {
                    let board_idx = self.active_board_index.or(self.board_selection.get());
                    if let Some(board_idx) = board_idx {
                        if let Some(board) = self.boards.get_mut(board_idx) {
                            let duration = board.sprint_duration_days.unwrap_or(14);
                            let sprint_id = sprint.id;
                            sprint.activate(duration);
                            board.active_sprint_id = Some(sprint_id);
                        }
                        if let Some(board) = self.boards.get(board_idx) {
                            tracing::info!(
                                "Activated sprint: {}",
                                sprint.formatted_name(board, "sprint")
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn handle_complete_sprint_key(&mut self) {
        if let Some(sprint_idx) = self.active_sprint_index {
            if let Some(sprint) = self.sprints.get_mut(sprint_idx) {
                if sprint.status == SprintStatus::Active || sprint.status == SprintStatus::Planning
                {
                    let sprint_id = sprint.id;
                    sprint.complete();
                    let board_idx = self.active_board_index.or(self.board_selection.get());
                    if let Some(board_idx) = board_idx {
                        if let Some(board) = self.boards.get(board_idx) {
                            tracing::info!(
                                "Completed sprint: {}",
                                sprint.formatted_name(board, "sprint")
                            );
                        }
                    }

                    let board_idx = self.active_board_index.or(self.board_selection.get());
                    if let Some(board_idx) = board_idx {
                        if let Some(board) = self.boards.get_mut(board_idx) {
                            if board.active_sprint_id == Some(sprint_id) {
                                board.active_sprint_id = None;
                                self.active_sprint_filter = None;
                            }
                        }
                    }

                    self.mode = AppMode::BoardDetail;
                    self.board_focus = BoardFocus::Sprints;
                    self.active_sprint_index = None;
                }
            }
        }
    }

    pub fn create_sprint(&mut self) {
        let board_idx = self.active_board_index.or(self.board_selection.get());
        if let Some(board_idx) = board_idx {
            let (sprint_number, name_index, board_id, effective_sprint_prefix) = {
                if let Some(board) = self.boards.get_mut(board_idx) {
                    let effective_sprint_prefix = board.sprint_prefix.as_deref().unwrap_or("sprint").to_string();
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

            let sprint = Sprint::new(board_id, sprint_number, name_index, None);
            if let Some(board) = self.boards.get(board_idx) {
                tracing::info!(
                    "Creating sprint: {} (id: {})",
                    sprint.formatted_name(board, &effective_sprint_prefix),
                    sprint.id
                );
            }
            self.sprints.push(sprint);

            let board_sprints: Vec<_> = self
                .sprints
                .iter()
                .filter(|s| s.board_id == board_id)
                .collect();
            let new_index = board_sprints.len() - 1;
            self.sprint_selection.set(Some(new_index));
        }
    }
}
