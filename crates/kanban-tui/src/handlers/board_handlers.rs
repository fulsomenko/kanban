use crate::app::{App, AppMode, BoardFocus, Focus};
use crate::state::commands::{CreateBoard, CreateColumn, UpdateBoard};
use kanban_domain::{BoardUpdate, TaskListView};

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
        let board_name = self.input.as_str().to_string();

        // Execute CreateBoard command first to get the board ID
        let create_board_cmd = Box::new(CreateBoard {
            name: board_name.clone(),
            card_prefix: None,
        });

        if let Err(e) = self.execute_command(create_board_cmd) {
            tracing::error!("Failed to create board: {}", e);
            return;
        }

        // Get the board ID from the newly created board
        let board_id = if let Some(board) = self.boards.last() {
            board.id
        } else {
            return;
        };

        // Now batch the column creation commands
        let mut column_commands: Vec<Box<dyn crate::state::commands::Command>> = Vec::new();
        let default_columns = vec![("TODO", 0i32), ("Doing", 1i32), ("Complete", 2i32)];

        for (name, position) in default_columns {
            let create_col_cmd = Box::new(CreateColumn {
                board_id,
                name: name.to_string(),
                position,
            }) as Box<dyn crate::state::commands::Command>;
            column_commands.push(create_col_cmd);
        }

        // Execute all column creation commands as a batch (single pause/resume cycle)
        if let Err(e) = self.execute_commands_batch(column_commands) {
            tracing::error!("Failed to create default columns: {}", e);
            return;
        }

        let task_list_view = TaskListView::default(); // Default view for new boards

        tracing::info!("Created board: {} (id: {})", board_name, board_id);
        tracing::info!("Created default columns: TODO, Doing, Complete");

        let new_index = self.boards.len() - 1;
        self.board_selection.set(Some(new_index));
        self.switch_view_strategy(task_list_view);
    }

    pub fn rename_board(&mut self) {
        if let Some(idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get(idx) {
                let board_id = board.id;
                let new_name = self.input.as_str().to_string();

                // Execute UpdateBoard command
                let cmd = Box::new(UpdateBoard {
                    board_id,
                    updates: BoardUpdate {
                        name: Some(new_name.clone()),
                        ..Default::default()
                    },
                });

                if let Err(e) = self.execute_command(cmd) {
                    tracing::error!("Failed to rename board: {}", e);
                    return;
                }

                tracing::info!("Renamed board to: {}", new_name);
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
