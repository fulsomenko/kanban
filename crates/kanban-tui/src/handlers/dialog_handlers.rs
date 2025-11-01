use crate::app::{App, AppMode, BoardFocus};
use crate::dialog::{handle_dialog_input, DialogAction};
use crossterm::event::KeyCode;
use kanban_domain::Card;

impl App {
    pub fn handle_create_board_dialog(&mut self, key_code: KeyCode) {
        match handle_dialog_input(&mut self.input, key_code, false) {
            DialogAction::Confirm => {
                self.create_board();
                self.mode = AppMode::Normal;
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.mode = AppMode::Normal;
                self.input.clear();
            }
            DialogAction::None => {}
        }
    }

    pub fn handle_create_card_dialog(&mut self, key_code: KeyCode) {
        match handle_dialog_input(&mut self.input, key_code, false) {
            DialogAction::Confirm => {
                self.create_card();
                self.mode = AppMode::Normal;
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.mode = AppMode::Normal;
                self.input.clear();
            }
            DialogAction::None => {}
        }
    }

    pub fn handle_create_sprint_dialog(&mut self, key_code: KeyCode) {
        match handle_dialog_input(&mut self.input, key_code, true) {
            DialogAction::Confirm => {
                self.create_sprint();
                self.mode = AppMode::BoardDetail;
                self.board_focus = BoardFocus::Sprints;
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.mode = AppMode::BoardDetail;
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
                self.mode = AppMode::Normal;
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.mode = AppMode::Normal;
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
                self.mode = AppMode::Normal;
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.mode = AppMode::Normal;
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
                self.mode = AppMode::Normal;
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.mode = AppMode::Normal;
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
                        self.mode = AppMode::CardDetail;
                        self.input.clear();
                        return false;
                    }
                } else {
                    tracing::error!("Invalid points value");
                    self.mode = AppMode::CardDetail;
                    self.input.clear();
                    return false;
                };

                if let Some(card_idx) = self.active_card_index {
                    if let Some(board_idx) = self.active_board_index {
                        if let Some(board) = self.boards.get(board_idx) {
                            let board_cards: Vec<_> = self
                                .cards
                                .iter()
                                .filter(|card| {
                                    self.columns.iter().any(|col| {
                                        col.id == card.column_id && col.board_id == board.id
                                    })
                                })
                                .collect();

                            if let Some(card) = board_cards.get(card_idx) {
                                let card_id = card.id;
                                if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id)
                                {
                                    card.set_points(points);
                                    tracing::info!("Set points to: {:?}", points);
                                }
                            }
                        }
                    }
                }
                self.mode = AppMode::CardDetail;
                self.input.clear();
                false
            }
            DialogAction::Cancel => {
                self.mode = AppMode::CardDetail;
                self.input.clear();
                false
            }
            DialogAction::None => false,
        }
    }

    pub fn handle_set_branch_prefix_dialog(&mut self, key_code: KeyCode) {
        match handle_dialog_input(&mut self.input, key_code, true) {
            DialogAction::Confirm => {
                let prefix_str = self.input.as_str().trim();
                if prefix_str.is_empty() {
                    if let Some(board_idx) = self.board_selection.get() {
                        if let Some(board) = self.boards.get_mut(board_idx) {
                            board.update_branch_prefix(None);
                            tracing::info!("Cleared branch prefix");
                        }
                    }
                } else if Card::validate_branch_prefix(prefix_str) {
                    if let Some(board_idx) = self.board_selection.get() {
                        if let Some(board) = self.boards.get_mut(board_idx) {
                            board.update_branch_prefix(Some(prefix_str.to_string()));
                            tracing::info!("Set branch prefix to: {}", prefix_str);
                        }
                    }
                } else {
                    tracing::error!("Invalid prefix: use alphanumeric, hyphens, underscores only");
                }
                self.mode = AppMode::BoardDetail;
                self.board_focus = BoardFocus::Settings;
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.mode = AppMode::BoardDetail;
                self.board_focus = BoardFocus::Settings;
                self.input.clear();
            }
            DialogAction::None => {}
        }
    }

    pub fn handle_set_sprint_prefix_dialog(&mut self, key_code: KeyCode) {
        match handle_dialog_input(&mut self.input, key_code, true) {
            DialogAction::Confirm => {
                let prefix_str = self.input.as_str().trim();
                if let Some(sprint_idx) = self.active_sprint_index {
                    if let Some(sprint) = self.sprints.get_mut(sprint_idx) {
                        if prefix_str.is_empty() {
                            sprint.update_prefix(None);
                            tracing::info!("Cleared sprint prefix");
                        } else if Card::validate_branch_prefix(prefix_str) {
                            sprint.update_prefix(Some(prefix_str.to_string()));
                            tracing::info!("Set sprint prefix to: {}", prefix_str);
                        } else {
                            tracing::error!(
                                "Invalid prefix: use alphanumeric, hyphens, underscores only"
                            );
                        }
                    }
                }
                self.mode = AppMode::SprintDetail;
                self.input.clear();
            }
            DialogAction::Cancel => {
                self.mode = AppMode::SprintDetail;
                self.input.clear();
            }
            DialogAction::None => {}
        }
    }

    pub fn handle_confirm_sprint_prefix_collision_popup(&mut self, key_code: KeyCode) {
        use crossterm::event::KeyCode;
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::SprintDetail;
            }
            KeyCode::Enter | KeyCode::Char('y') => {
                // User confirmed they want to continue with the colliding prefix
                // The actual prefix application should happen before this mode is entered
                self.mode = AppMode::SprintDetail;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // User declined, go back to prefix dialog
                self.mode = AppMode::SetSprintPrefix;
            }
            _ => {}
        }
    }
}
