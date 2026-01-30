use crate::app::{App, BoardFocus, DialogMode};
use crossterm::event::KeyCode;
use kanban_domain::commands::{
    CreateColumn, DeleteColumn, MoveCard, SetBoardTaskListView, UpdateColumn,
};
use kanban_domain::{ColumnUpdate, TaskListView};

impl App {
    pub fn handle_create_column_key(&mut self) {
        if self.board_focus == BoardFocus::Columns {
            if let Some(board_idx) = self.board_selection.get() {
                if self.ctx.boards.get(board_idx).is_some() {
                    self.open_dialog(DialogMode::CreateColumn);
                    self.input.clear();
                }
            }
        }
    }

    pub fn handle_rename_column_key(&mut self) {
        if self.board_focus == BoardFocus::Columns && self.column_selection.get().is_some() {
            if let Some(board_idx) = self.board_selection.get() {
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    let board_columns: Vec<_> = self
                        .ctx
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .collect();

                    if let Some(column_idx) = self.column_selection.get() {
                        if let Some(column) = board_columns.get(column_idx) {
                            self.input.set(column.name.clone());
                            self.open_dialog(DialogMode::RenameColumn);
                        }
                    }
                }
            }
        }
    }

    pub fn handle_delete_column_key(&mut self) {
        if self.board_focus == BoardFocus::Columns && self.column_selection.get().is_some() {
            if let Some(board_idx) = self.board_selection.get() {
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    let column_count = self
                        .ctx
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .count();

                    if column_count > 1 {
                        self.open_dialog(DialogMode::DeleteColumnConfirm);
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
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    // Collect and sort column data before mutating
                    let mut board_columns: Vec<_> = self
                        .ctx
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .map(|col| (col.id, col.position))
                        .collect();

                    board_columns.sort_by_key(|(_, pos)| *pos);

                    if let Some(selected_idx) = self.column_selection.get() {
                        if selected_idx > 0 && selected_idx < board_columns.len() {
                            let prev_col_id = board_columns[selected_idx - 1].0;
                            let curr_col_id = board_columns[selected_idx].0;
                            let prev_pos = board_columns[selected_idx - 1].1;
                            let curr_pos = board_columns[selected_idx].1;

                            // Swap positions using batched commands
                            let cmd1 = Box::new(UpdateColumn {
                                column_id: prev_col_id,
                                updates: ColumnUpdate {
                                    position: Some(curr_pos),
                                    ..Default::default()
                                },
                            })
                                as Box<dyn kanban_domain::commands::Command>;

                            let cmd2 = Box::new(UpdateColumn {
                                column_id: curr_col_id,
                                updates: ColumnUpdate {
                                    position: Some(prev_pos),
                                    ..Default::default()
                                },
                            })
                                as Box<dyn kanban_domain::commands::Command>;

                            if let Err(e) = self.execute_commands_batch(vec![cmd1, cmd2]) {
                                tracing::error!("Failed to move column: {}", e);
                                return;
                            }

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
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    // Collect and sort column data before mutating
                    let mut board_columns: Vec<_> = self
                        .ctx
                        .columns
                        .iter()
                        .filter(|col| col.board_id == board.id)
                        .map(|col| (col.id, col.position))
                        .collect();

                    board_columns.sort_by_key(|(_, pos)| *pos);

                    if let Some(selected_idx) = self.column_selection.get() {
                        if selected_idx < board_columns.len() - 1 {
                            let curr_col_id = board_columns[selected_idx].0;
                            let next_col_id = board_columns[selected_idx + 1].0;
                            let curr_pos = board_columns[selected_idx].1;
                            let next_pos = board_columns[selected_idx + 1].1;

                            // Swap positions using batched commands
                            let cmd1 = Box::new(UpdateColumn {
                                column_id: next_col_id,
                                updates: ColumnUpdate {
                                    position: Some(curr_pos),
                                    ..Default::default()
                                },
                            })
                                as Box<dyn kanban_domain::commands::Command>;

                            let cmd2 = Box::new(UpdateColumn {
                                column_id: curr_col_id,
                                updates: ColumnUpdate {
                                    position: Some(next_pos),
                                    ..Default::default()
                                },
                            })
                                as Box<dyn kanban_domain::commands::Command>;

                            if let Err(e) = self.execute_commands_batch(vec![cmd1, cmd2]) {
                                tracing::error!("Failed to move column: {}", e);
                                return;
                            }

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
        if self.focus != crate::app::Focus::Cards {
            return;
        }

        if let Some(board_idx) = self.active_board_index {
            if let Some(board) = self.ctx.boards.get(board_idx) {
                let current_view_idx = match board.task_list_view {
                    TaskListView::Flat => 0,
                    TaskListView::GroupedByColumn => 1,
                    TaskListView::ColumnView => 2,
                };
                self.task_list_view_selection.set(Some(current_view_idx));
                self.open_dialog(DialogMode::SelectTaskListView);
            }
        }
    }

    pub fn create_column(&mut self) {
        if let Some(board_idx) = self.board_selection.get() {
            // Collect board_id before command execution
            let board_id = self.ctx.boards.get(board_idx).map(|board| board.id);

            if let Some(board_id) = board_id {
                let column_name = self.input.as_str().trim().to_string();

                if column_name.is_empty() {
                    tracing::warn!("Column name cannot be empty");
                    return;
                }

                let position = self
                    .ctx
                    .columns
                    .iter()
                    .filter(|col| col.board_id == board_id)
                    .map(|col| col.position)
                    .max()
                    .unwrap_or(-1)
                    + 1;

                let cmd = Box::new(CreateColumn {
                    board_id,
                    name: column_name.clone(),
                    position,
                });

                if let Err(e) = self.execute_command(cmd) {
                    tracing::error!("Failed to create column: {}", e);
                    return;
                }

                tracing::info!("Created column: {} (position: {})", column_name, position);

                let board_column_count = self
                    .ctx
                    .columns
                    .iter()
                    .filter(|col| col.board_id == board_id)
                    .count();
                let new_column_index = board_column_count.saturating_sub(1);
                self.column_selection.set(Some(new_column_index));
            }
        }
    }

    pub fn rename_column(&mut self) {
        if let Some(board_idx) = self.board_selection.get() {
            // Collect column ID before mutable borrow
            let column_info = {
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    if let Some(column_idx) = self.column_selection.get() {
                        let board_columns: Vec<_> = self
                            .ctx
                            .columns
                            .iter()
                            .filter(|col| col.board_id == board.id)
                            .collect();

                        board_columns.get(column_idx).map(|col| col.id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(column_id) = column_info {
                let new_name = self.input.as_str().trim().to_string();

                if new_name.is_empty() {
                    tracing::warn!("Column name cannot be empty");
                    return;
                }

                let cmd = Box::new(UpdateColumn {
                    column_id,
                    updates: ColumnUpdate {
                        name: Some(new_name.clone()),
                        ..Default::default()
                    },
                });

                if let Err(e) = self.execute_command(cmd) {
                    tracing::error!("Failed to rename column: {}", e);
                    return;
                }

                tracing::info!("Renamed column to: {}", new_name);
            }
        }
    }

    pub fn delete_column(&mut self) {
        if let Some(board_idx) = self.board_selection.get() {
            // Collect all necessary data before mutating
            let delete_info = {
                if let Some(board) = self.ctx.boards.get(board_idx) {
                    if let Some(column_idx) = self.column_selection.get() {
                        let board_columns: Vec<_> = self
                            .ctx
                            .columns
                            .iter()
                            .filter(|col| col.board_id == board.id)
                            .map(|col| (col.id, col.name.clone()))
                            .collect();

                        if board_columns.len() <= 1 {
                            return;
                        }

                        let column_to_delete = board_columns.get(column_idx).cloned();
                        let first_column_id = board_columns.first().map(|(id, _)| *id);

                        if let Some((column_id, column_name)) = column_to_delete {
                            let cards_to_move: Vec<(uuid::Uuid, i32)> = self
                                .ctx
                                .cards
                                .iter()
                                .filter(|card| card.column_id == column_id)
                                .map(|card| (card.id, card.position))
                                .collect();

                            Some((
                                column_id,
                                column_name,
                                first_column_id,
                                cards_to_move,
                                column_idx,
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((column_id, column_name, first_column_id, cards_to_move, column_idx)) =
                delete_info
            {
                tracing::warn!("Cannot delete the last column");

                if let Some(target_column_id) = first_column_id {
                    if target_column_id != column_id {
                        let card_count = cards_to_move.len();

                        // Batch all card moves together to avoid race conditions
                        let mut move_commands: Vec<Box<dyn kanban_domain::commands::Command>> =
                            Vec::new();
                        for (card_id, position) in cards_to_move {
                            let cmd = Box::new(MoveCard {
                                card_id,
                                new_column_id: target_column_id,
                                new_position: position,
                            })
                                as Box<dyn kanban_domain::commands::Command>;
                            move_commands.push(cmd);
                        }

                        if let Err(e) = self.execute_commands_batch(move_commands) {
                            tracing::error!("Failed to move cards: {}", e);
                            return;
                        }

                        tracing::info!("Moved {} cards to first column", card_count);
                    }
                }

                let cmd = Box::new(DeleteColumn { column_id });
                if let Err(e) = self.execute_command(cmd) {
                    tracing::error!("Failed to delete column: {}", e);
                    return;
                }

                tracing::info!("Deleted column: {}", column_name);

                let remaining_columns = self
                    .ctx
                    .columns
                    .iter()
                    .filter(|col| {
                        if let Some(board) = self.ctx.boards.get(board_idx) {
                            col.board_id == board.id
                        } else {
                            false
                        }
                    })
                    .count();

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

    pub fn handle_create_column_dialog(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.pop_mode();
                self.board_focus = BoardFocus::Columns;
                self.input.clear();
            }
            KeyCode::Enter => {
                self.create_column();
                self.pop_mode();
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
                self.pop_mode();
                self.board_focus = BoardFocus::Columns;
                self.input.clear();
            }
            KeyCode::Enter => {
                self.rename_column();
                self.pop_mode();
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
                self.pop_mode();
                self.board_focus = BoardFocus::Columns;
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.delete_column();
                self.pop_mode();
                self.board_focus = BoardFocus::Columns;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.pop_mode();
                self.board_focus = BoardFocus::Columns;
            }
            _ => {}
        }
    }

    pub fn handle_select_task_list_view_popup(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.pop_mode();
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

                    let selected_card_id = self.get_selected_card_id();

                    if let Some(board_idx) = self.active_board_index {
                        if let Some(board) = self.ctx.boards.get(board_idx) {
                            let cmd = Box::new(SetBoardTaskListView {
                                board_id: board.id,
                                view,
                            });

                            if let Err(e) = self.execute_command(cmd) {
                                tracing::error!("Failed to set task list view: {}", e);
                                self.pop_mode();
                                self.task_list_view_selection.clear();
                                return;
                            }

                            self.switch_view_strategy(view);

                            if let Some(card_id) = selected_card_id {
                                self.select_card_by_id(card_id);
                            }

                            tracing::info!("Updated task list view to: {:?}", view);
                        }
                    }
                }
                self.pop_mode();
                self.task_list_view_selection.clear();
            }
            _ => {}
        }
    }
}
