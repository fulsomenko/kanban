use crate::app::{App, AppMode, BoardFocus, Focus};

impl App {
    pub fn handle_create_board_key(&mut self) {
        if self.focus == Focus::Boards {
            self.mode = AppMode::CreateBoard;
            self.input.clear();
        }
    }

    pub fn handle_rename_board_key(&mut self) {
        if self.focus == Focus::Boards && self.board_selection.get().is_some() {
            if let Some(board_idx) = self.board_selection.get() {
                if let Some(board) = self.boards.get(board_idx) {
                    self.input.set(board.name.clone());
                    self.mode = AppMode::RenameBoard;
                }
            }
        }
    }

    pub fn handle_edit_board_key(&mut self) {
        if self.focus == Focus::Boards && self.board_selection.get().is_some() {
            self.mode = AppMode::BoardDetail;
            self.board_focus = BoardFocus::Name;
        }
    }

    pub fn handle_export_board_key(&mut self) {
        if self.focus == Focus::Boards && self.board_selection.get().is_some() {
            if let Some(board_idx) = self.board_selection.get() {
                if let Some(board) = self.boards.get(board_idx) {
                    let filename = format!(
                        "{}-{}.json",
                        board.name.replace(" ", "-").to_lowercase(),
                        chrono::Utc::now().format("%Y%m%d-%H%M%S")
                    );
                    self.input.set(filename);
                    self.mode = AppMode::ExportBoard;
                }
            }
        }
    }

    pub fn handle_export_all_key(&mut self) {
        if self.focus == Focus::Boards && !self.boards.is_empty() {
            let filename = format!(
                "kanban-all-{}.json",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            );
            self.input.set(filename);
            self.mode = AppMode::ExportAll;
        }
    }

    pub fn handle_import_board_key(&mut self) {
        if self.focus == Focus::Boards {
            self.scan_import_files();
            if !self.import_files.is_empty() {
                self.import_selection.set(Some(0));
                self.mode = AppMode::ImportBoard;
            }
        }
    }

    pub fn create_board(&mut self) {
        let board = kanban_domain::Board::new(self.input.as_str().to_string(), None);
        let board_id = board.id;
        let task_list_view = board.task_list_view;
        tracing::info!("Creating board: {} (id: {})", board.name, board.id);

        self.boards.push(board);

        let default_columns = vec![("TODO", 0), ("Doing", 1), ("Complete", 2)];

        for (name, position) in default_columns {
            let column = kanban_domain::Column::new(board_id, name.to_string(), position);
            tracing::info!(
                "Creating default column: {} (position: {})",
                column.name,
                column.position
            );
            self.columns.push(column);
        }

        let new_index = self.boards.len() - 1;
        self.board_selection.set(Some(new_index));
        self.switch_view_strategy(task_list_view);
    }

    pub fn rename_board(&mut self) {
        if let Some(idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get_mut(idx) {
                board.update_name(self.input.as_str().to_string());
                tracing::info!("Renamed board to: {}", board.name);
            }
        }
    }

    fn scan_import_files(&mut self) {
        self.import_files.clear();
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        if let Some(filename) = entry.file_name().to_str() {
                            if filename.ends_with(".json") {
                                self.import_files.push(filename.to_string());
                            }
                        }
                    }
                }
            }
        }
        self.import_files.sort();
    }
}
