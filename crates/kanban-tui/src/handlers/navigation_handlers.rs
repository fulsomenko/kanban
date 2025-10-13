use crate::app::{App, AppMode, Focus};
use crossterm::event::KeyCode;

impl App {
    pub fn handle_focus_switch(&mut self, focus_target: Focus) {
        match focus_target {
            Focus::Boards => {
                self.focus = Focus::Boards;
            }
            Focus::Cards => {
                if self.active_board_index.is_some() {
                    self.focus = Focus::Cards;
                }
            }
        }
    }

    pub fn handle_navigation_down(&mut self) {
        match self.focus {
            Focus::Boards => {
                self.board_selection.next(self.boards.len());
            }
            Focus::Cards => {
                if let Some(board_idx) = self.active_board_index {
                    if let Some(board) = self.boards.get(board_idx) {
                        let card_count = self.get_board_card_count(board.id);
                        self.card_selection.next(card_count);
                    }
                }
            }
        }
    }

    pub fn handle_navigation_up(&mut self) {
        match self.focus {
            Focus::Boards => {
                self.board_selection.prev();
            }
            Focus::Cards => {
                self.card_selection.prev();
            }
        }
    }

    pub fn handle_selection_activate(&mut self) {
        match self.focus {
            Focus::Boards => {
                if self.board_selection.get().is_some() {
                    self.active_board_index = self.board_selection.get();
                    self.card_selection.clear();

                    if let Some(board_idx) = self.active_board_index {
                        if let Some(board) = self.boards.get(board_idx) {
                            self.current_sort_field = Some(board.task_sort_field);
                            self.current_sort_order = Some(board.task_sort_order);

                            let card_count = self.get_board_card_count(board.id);
                            if card_count > 0 {
                                self.card_selection.set(Some(0));
                            }
                        }
                    }

                    self.focus = Focus::Cards;
                }
            }
            Focus::Cards => {
                if let Some(sorted_idx) = self.card_selection.get() {
                    if let Some(board_idx) = self.active_board_index {
                        if let Some(board) = self.boards.get(board_idx) {
                            let sorted_cards = self.get_sorted_board_cards(board.id);
                            if let Some(selected_card) = sorted_cards.get(sorted_idx) {
                                let card_id = selected_card.id;
                                let actual_idx = self.cards.iter().position(|c| c.id == card_id);
                                self.active_card_index = actual_idx;
                                self.mode = AppMode::CardDetail;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn handle_escape_key(&mut self) {
        if self.active_board_index.is_some() {
            self.active_board_index = None;
            self.card_selection.clear();
            self.focus = Focus::Boards;
        }
    }
}
