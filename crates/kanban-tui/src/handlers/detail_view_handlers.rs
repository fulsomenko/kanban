use crate::app::{App, AppMode, BoardField, BoardFocus, CardField, CardFocus};
use crate::events::EventHandler;
use crossterm::event::KeyCode;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

impl App {
    pub fn handle_card_detail_key(
        &mut self,
        key_code: KeyCode,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
    ) -> bool {
        let mut should_restart = false;
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.active_card_index = None;
                self.card_focus = CardFocus::Title;
            }
            KeyCode::Char('1') => {
                self.card_focus = CardFocus::Title;
            }
            KeyCode::Char('2') => {
                self.card_focus = CardFocus::Metadata;
            }
            KeyCode::Char('3') => {
                self.card_focus = CardFocus::Description;
            }
            KeyCode::Char('y') => {
                self.copy_branch_name();
            }
            KeyCode::Char('Y') => {
                self.copy_git_checkout_command();
            }
            KeyCode::Char('e') => match self.card_focus {
                CardFocus::Title => {
                    if let Err(e) = self.edit_card_field(terminal, event_handler, CardField::Title)
                    {
                        tracing::error!("Failed to edit title: {}", e);
                    }
                    should_restart = true;
                }
                CardFocus::Description => {
                    if let Err(e) =
                        self.edit_card_field(terminal, event_handler, CardField::Description)
                    {
                        tracing::error!("Failed to edit description: {}", e);
                    }
                    should_restart = true;
                }
                CardFocus::Metadata => {
                    self.input.clear();
                    self.mode = AppMode::SetCardPoints;
                }
            },
            KeyCode::Char('s') => {
                if let Some(board_idx) = self.active_board_index {
                    if let Some(board) = self.boards.get(board_idx) {
                        let sprint_count = self
                            .sprints
                            .iter()
                            .filter(|s| s.board_id == board.id)
                            .count();
                        if sprint_count > 0 {
                            self.sprint_assign_selection.set(Some(0));
                            self.mode = AppMode::AssignCardToSprint;
                        }
                    }
                }
            }
            KeyCode::Char('p') => {
                self.priority_selection.set(Some(0));
                self.mode = AppMode::SetCardPriority;
            }
            _ => {}
        }
        should_restart
    }

    pub fn handle_board_detail_key(
        &mut self,
        key_code: KeyCode,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
    ) -> bool {
        let mut should_restart = false;
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.board_focus = BoardFocus::Name;
            }
            KeyCode::Char('1') => {
                self.board_focus = BoardFocus::Name;
            }
            KeyCode::Char('2') => {
                self.board_focus = BoardFocus::Description;
            }
            KeyCode::Char('3') => {
                self.board_focus = BoardFocus::Settings;
            }
            KeyCode::Char('4') => {
                self.board_focus = BoardFocus::Sprints;
            }
            KeyCode::Char('5') => {
                self.board_focus = BoardFocus::Columns;
            }
            KeyCode::Char('e') => match self.board_focus {
                BoardFocus::Name => {
                    if let Err(e) = self.edit_board_field(terminal, event_handler, BoardField::Name)
                    {
                        tracing::error!("Failed to edit board name: {}", e);
                    }
                    should_restart = true;
                }
                BoardFocus::Description => {
                    if let Err(e) =
                        self.edit_board_field(terminal, event_handler, BoardField::Description)
                    {
                        tracing::error!("Failed to edit board description: {}", e);
                    }
                    should_restart = true;
                }
                BoardFocus::Settings => {
                    if let Err(e) =
                        self.edit_board_field(terminal, event_handler, BoardField::Settings)
                    {
                        tracing::error!("Failed to edit board settings: {}", e);
                    }
                    should_restart = true;
                }
                BoardFocus::Sprints => {}
                BoardFocus::Columns => {}
            },
            KeyCode::Char('n') => {
                if self.board_focus == BoardFocus::Sprints {
                    self.handle_create_sprint_key();
                } else if self.board_focus == BoardFocus::Columns {
                    self.handle_create_column_key();
                }
            }
            KeyCode::Char('r') => {
                if self.board_focus == BoardFocus::Columns {
                    self.handle_rename_column_key();
                }
            }
            KeyCode::Char('d') => {
                if self.board_focus == BoardFocus::Columns {
                    self.handle_delete_column_key();
                }
            }
            KeyCode::Char('J') => {
                if self.board_focus == BoardFocus::Columns {
                    self.handle_move_column_down();
                }
            }
            KeyCode::Char('K') => {
                if self.board_focus == BoardFocus::Columns {
                    self.handle_move_column_up();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.board_focus == BoardFocus::Sprints {
                    if let Some(board_idx) = self.board_selection.get() {
                        if let Some(board) = self.boards.get(board_idx) {
                            let sprint_count = self
                                .sprints
                                .iter()
                                .filter(|s| s.board_id == board.id)
                                .count();
                            self.sprint_selection.next(sprint_count);
                        }
                    }
                } else if self.board_focus == BoardFocus::Columns {
                    if let Some(board_idx) = self.board_selection.get() {
                        if let Some(board) = self.boards.get(board_idx) {
                            let column_count = self
                                .columns
                                .iter()
                                .filter(|col| col.board_id == board.id)
                                .count();
                            self.column_selection.next(column_count);
                        }
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.board_focus == BoardFocus::Sprints {
                    self.sprint_selection.prev();
                } else if self.board_focus == BoardFocus::Columns {
                    self.column_selection.prev();
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if self.board_focus == BoardFocus::Sprints {
                    if let Some(sprint_idx) = self.sprint_selection.get() {
                        if let Some(board_idx) = self.board_selection.get() {
                            if let Some(board) = self.boards.get(board_idx) {
                                let board_sprints: Vec<_> = self
                                    .sprints
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, s)| s.board_id == board.id)
                                    .collect();
                                if let Some((actual_idx, _)) = board_sprints.get(sprint_idx) {
                                    self.active_sprint_index = Some(*actual_idx);
                                    self.mode = AppMode::SprintDetail;
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('p') => {
                if self.board_focus == BoardFocus::Settings {
                    if let Some(board_idx) = self.board_selection.get() {
                        if let Some(board) = self.boards.get(board_idx) {
                            let current_prefix = board.branch_prefix.clone().unwrap_or_else(String::new);
                            self.input.set(current_prefix);
                            self.mode = AppMode::SetBranchPrefix;
                        }
                    }
                }
            }
            _ => {}
        }
        should_restart
    }

    pub fn handle_sprint_detail_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::BoardDetail;
                self.board_focus = BoardFocus::Sprints;
                self.active_sprint_index = None;
            }
            KeyCode::Char('a') => {
                self.handle_activate_sprint_key();
            }
            KeyCode::Char('c') => {
                self.handle_complete_sprint_key();
            }
            _ => {}
        }
    }
}
