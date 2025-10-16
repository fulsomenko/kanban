use crate::app::{App, AppMode, BoardFocus};
use crossterm::event::KeyCode;
use kanban_domain::{Column, TaskListView};

impl App {
    pub fn handle_create_column_key(&mut self) {
        if self.board_focus == BoardFocus::Columns {
            if let Some(board_idx) = self.board_selection.get() {
                if self.boards.get(board_idx).is_some() {
                    self.mode = AppMode::CreateColumn;
                    self.input.clear();
                }
            }
        }
    }

    pub fn handle_rename_column_key(&mut self) {
        if self.board_focus == BoardFocus::Columns && self.column_selection.get().is_some() {
            if let Some(board_idx) = self.board_selection.get() {
                if let Some(board) = self.boards.get(board_idx) {
                    let board_columns: Vec<_> = self
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .collect();

                    if let Some(column_idx) = self.column_selection.get() {
                        if let Some(column) = board_columns.get(column_idx) {
                            self.input.set(column.name.clone());
                            self.mode = AppMode::RenameColumn;
                        }
                    }
                }
            }
        }
    }

    pub fn handle_delete_column_key(&mut self) {
        if self.board_focus == BoardFocus::Columns && self.column_selection.get().is_some() {
            if let Some(board_idx) = self.board_selection.get() {
                if let Some(board) = self.boards.get(board_idx) {
                    let column_count = self.columns.iter().filter(|col| col.board_id == board.id).count();

                    if column_count > 1 {
                        self.mode = AppMode::DeleteColumnConfirm;
                    } else {
                        tracing::warn!("Cannot delete the last column");
                    }
                }
            }
        }
    }

    pub fn handle_move_column_up(&mut self) {
        if self.board_focus == BoardFocus::Columns && self.column_selection.get().is_some() {
            if let Some(board_idx) = self.board_selection.get() {
                if let Some(board) = self.boards.get(board_idx) {
                    let mut board_columns: Vec<_> = self
                        .columns
                        .iter_mut()
                        .filter(|col| col.board_id == board.id)
                        .collect();

                    board_columns.sort_by_key(|col| col.position);

                    if let Some(selected_idx) = self.column_selection.get() {
                        if selected_idx > 0 && selected_idx < board_columns.len() {
                            let prev_pos = board_columns[selected_idx - 1].position;
                            let curr_pos = board_columns[selected_idx].position;
                            board_columns[selected_idx - 1].update_position(curr_pos);
                            board_columns[selected_idx].update_position(prev_pos);
                            self.column_selection.prev();
                            tracing::info!("Moved column up");
                        }
                    }
                }
            }
        }
    }

    pub fn handle_move_column_down(&mut self) {
        if self.board_focus == BoardFocus::Columns && self.column_selection.get().is_some() {
            if let Some(board_idx) = self.board_selection.get() {
                if let Some(board) = self.boards.get(board_idx) {
                    let mut board_columns: Vec<_> = self
                        .columns
                        .iter_mut()
                        .filter(|col| col.board_id == board.id)
                        .collect();

                    board_columns.sort_by_key(|col| col.position);

                    if let Some(selected_idx) = self.column_selection.get() {
                        if selected_idx < board_columns.len() - 1 {
                            let curr_pos = board_columns[selected_idx].position;
                            let next_pos = board_columns[selected_idx + 1].position;
                            board_columns[selected_idx + 1].update_position(curr_pos);
                            board_columns[selected_idx].update_position(next_pos);
                            let column_count = board_columns.len();
                            self.column_selection.next(column_count);
                            tracing::info!("Moved column down");
                        }
                    }
                }
            }
        }
    }

    pub fn handle_toggle_task_list_view(&mut self) {
        if let Some(board_idx) = self.board_selection.get() {
            if self.boards.get(board_idx).is_some() {
                self.task_list_view_selection.set(Some(0));
                self.mode = AppMode::SelectTaskListView;
            }
        }
    }

    pub fn create_column(&mut self) {
        if let Some(board_idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get(board_idx) {
                let column_name = self.input.as_str().trim().to_string();

                if column_name.is_empty() {
                    tracing::warn!("Column name cannot be empty");
                    return;
                }

                let position = self
                    .columns
                    .iter()
                    .filter(|col| col.board_id == board.id)
                    .map(|col| col.position)
                    .max()
                    .unwrap_or(-1) + 1;

                let column = Column::new(board.id, column_name.clone(), position);
                tracing::info!("Creating column: {} (position: {})", column.name, column.position);
                self.columns.push(column);

                let board_column_count = self.columns.iter().filter(|col| col.board_id == board.id).count();
                let new_column_index = board_column_count.saturating_sub(1);
                self.column_selection.set(Some(new_column_index));
            }
        }
    }

    pub fn rename_column(&mut self) {
        if let Some(board_idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get(board_idx) {
                if let Some(column_idx) = self.column_selection.get() {
                    let board_columns: Vec<_> = self
                        .columns
                        .iter_mut()
                        .filter(|col| col.board_id == board.id)
                        .collect();

                    if let Some(column) = board_columns.get(column_idx) {
                        let new_name = self.input.as_str().trim().to_string();

                        if new_name.is_empty() {
                            tracing::warn!("Column name cannot be empty");
                            return;
                        }

                        let column_id = column.id;
                        if let Some(col) = self.columns.iter_mut().find(|c| c.id == column_id) {
                            col.update_name(new_name.clone());
                            tracing::info!("Renamed column to: {}", new_name);
                        }
                    }
                }
            }
        }
    }

    pub fn delete_column(&mut self) {
        if let Some(board_idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get(board_idx) {
                if let Some(column_idx) = self.column_selection.get() {
                    let board_columns: Vec<_> = self
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .collect();

                    if board_columns.len() <= 1 {
                        tracing::warn!("Cannot delete the last column");
                        return;
                    }

                    if let Some(column_to_delete) = board_columns.get(column_idx) {
                        let column_id = column_to_delete.id;
                        let column_name = column_to_delete.name.clone();

                        let first_column_id = board_columns.first().map(|c| c.id);

                        if let Some(target_column_id) = first_column_id {
                            if target_column_id != column_id {
                                let cards_to_move: Vec<_> = self
                                    .cards
                                    .iter()
                                    .filter(|card| card.column_id == column_id)
                                    .map(|card| card.id)
                                    .collect();

                                let card_count = cards_to_move.len();
                                for card_id in cards_to_move {
                                    if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) {
                                        card.move_to_column(target_column_id, card.position);
                                    }
                                }

                                tracing::info!("Moved {} cards to first column", card_count);
                            }
                        }

                        self.columns.retain(|col| col.id != column_id);
                        tracing::info!("Deleted column: {}", column_name);

                        let remaining_columns = self.columns.iter().filter(|col| col.board_id == board.id).count();
                        if remaining_columns > 0 {
                            if column_idx >= remaining_columns {
                                self.column_selection.set(Some(remaining_columns - 1));
                            } else {
                                self.column_selection.set(Some(column_idx));
                            }
                        } else {
                            self.column_selection.clear();
                        }
                    }
                }
            }
        }
    }

    pub fn handle_create_column_dialog(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::BoardDetail;
                self.board_focus = BoardFocus::Columns;
                self.input.clear();
            }
            KeyCode::Enter => {
                self.create_column();
                self.mode = AppMode::BoardDetail;
                self.board_focus = BoardFocus::Columns;
                self.input.clear();
            }
            KeyCode::Char(c) => {
                self.input.insert_char(c);
            }
            KeyCode::Backspace => {
                self.input.backspace();
            }
            KeyCode::Left => {
                self.input.move_left();
            }
            KeyCode::Right => {
                self.input.move_right();
            }
            _ => {}
        }
    }

    pub fn handle_rename_column_dialog(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::BoardDetail;
                self.board_focus = BoardFocus::Columns;
                self.input.clear();
            }
            KeyCode::Enter => {
                self.rename_column();
                self.mode = AppMode::BoardDetail;
                self.board_focus = BoardFocus::Columns;
                self.input.clear();
            }
            KeyCode::Char(c) => {
                self.input.insert_char(c);
            }
            KeyCode::Backspace => {
                self.input.backspace();
            }
            KeyCode::Left => {
                self.input.move_left();
            }
            KeyCode::Right => {
                self.input.move_right();
            }
            _ => {}
        }
    }

    pub fn handle_delete_column_confirm_popup(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::BoardDetail;
                self.board_focus = BoardFocus::Columns;
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.delete_column();
                self.mode = AppMode::BoardDetail;
                self.board_focus = BoardFocus::Columns;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.mode = AppMode::BoardDetail;
                self.board_focus = BoardFocus::Columns;
            }
            _ => {}
        }
    }

    pub fn handle_select_task_list_view_popup(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.task_list_view_selection.clear();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.task_list_view_selection.next(3);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.task_list_view_selection.prev();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(view_idx) = self.task_list_view_selection.get() {
                    let view = match view_idx {
                        0 => TaskListView::Flat,
                        1 => TaskListView::GroupedByColumn,
                        2 => TaskListView::ColumnView,
                        _ => TaskListView::Flat,
                    };

                    if let Some(board_idx) = self.active_board_index {
                        if let Some(board) = self.boards.get_mut(board_idx) {
                            board.update_task_list_view(view);
                            tracing::info!("Updated task list view to: {:?}", view);
                        }
                    }
                }
                self.mode = AppMode::Normal;
                self.task_list_view_selection.clear();
            }
            _ => {}
        }
    }
}
