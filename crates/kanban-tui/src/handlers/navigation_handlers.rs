use crate::app::{App, AppMode, Focus};
use kanban_domain::TaskListView;

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
                        if self.is_kanban_view() {
                            let focused_col_idx = self.column_selection.get().unwrap_or(0);
                            let mut board_columns: Vec<_> = self
                                .columns
                                .iter()
                                .filter(|col| col.board_id == board.id)
                                .collect();
                            board_columns.sort_by_key(|col| col.position);

                            if let Some(focused_column) = board_columns.get(focused_col_idx) {
                                let column_card_count = self.get_sorted_board_cards(board.id)
                                    .iter()
                                    .filter(|card| card.column_id == focused_column.id)
                                    .count();
                                self.card_selection.next(column_card_count);
                            }
                        } else {
                            let card_count = self.get_board_card_count(board.id);
                            self.card_selection.next(card_count);
                        }
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

    fn is_kanban_view(&self) -> bool {
        if let Some(board_idx) = self.active_board_index.or(self.board_selection.get()) {
            if let Some(board) = self.boards.get(board_idx) {
                return board.task_list_view == TaskListView::ColumnView;
            }
        }
        false
    }

    pub fn handle_kanban_column_left(&mut self) {
        if !self.is_kanban_view() || self.focus != Focus::Cards {
            return;
        }

        if let Some(board_idx) = self.active_board_index {
            if let Some(board) = self.boards.get(board_idx) {
                let column_count = self
                    .columns
                    .iter()
                    .filter(|col| col.board_id == board.id)
                    .count();

                if column_count == 0 {
                    return;
                }

                if let Some(current_col_idx) = self.column_selection.get() {
                    if current_col_idx > 0 {
                        self.column_selection.set(Some(current_col_idx - 1));
                        self.card_selection.set(Some(0));
                        tracing::info!("Moved to column {}", current_col_idx - 1);
                    }
                } else {
                    self.column_selection.set(Some(0));
                    self.card_selection.set(Some(0));
                }
            }
        }
    }

    pub fn handle_kanban_column_right(&mut self) {
        if !self.is_kanban_view() || self.focus != Focus::Cards {
            return;
        }

        if let Some(board_idx) = self.active_board_index {
            if let Some(board) = self.boards.get(board_idx) {
                let column_count = self
                    .columns
                    .iter()
                    .filter(|col| col.board_id == board.id)
                    .count();

                if column_count == 0 {
                    return;
                }

                let current_col_idx = self.column_selection.get().unwrap_or(0);
                if current_col_idx < column_count - 1 {
                    self.column_selection.set(Some(current_col_idx + 1));
                    self.card_selection.set(Some(0));
                    tracing::info!("Moved to column {}", current_col_idx + 1);
                }
            }
        }
    }

    pub fn handle_column_or_focus_switch(&mut self, index: usize) {
        if self.is_kanban_view() && self.focus == Focus::Cards {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.boards.get(board_idx) {
                    let column_count = self
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .count();

                    if index < column_count {
                        self.column_selection.set(Some(index));
                        self.card_selection.set(Some(0));
                        tracing::info!("Switched to column {}", index);
                    }
                }
            }
        } else {
            match index {
                0 => self.handle_focus_switch(Focus::Boards),
                1 => self.handle_focus_switch(Focus::Cards),
                _ => {}
            }
        }
    }
}
