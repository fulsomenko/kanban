use crate::app::{App, AppMode, BoardFocus, DialogMode, Focus};
use kanban_domain::commands::{
    BoardCommand, ColumnCommand, Command, CreateBoard, CreateColumn, UpdateBoard,
};
use kanban_domain::{BoardUpdate, TaskListView};

impl App {
    pub fn handle_create_board_key(&mut self) {
        if self.focus.active == Focus::Boards {
            self.open_dialog(DialogMode::CreateBoard);
            self.input.clear();
        }
    }

    pub fn handle_rename_board_key(&mut self) {
        if self.focus.active == Focus::Boards && self.selection.board.get().is_some() {
            if let Some(board_idx) = self.selection.board.get() {
                if let Some(board) = self.model.boards().get(board_idx) {
                    self.input.set(board.name.clone());
                    self.open_dialog(DialogMode::RenameBoard);
                }
            }
        }
    }

    pub fn handle_edit_board_key(&mut self) {
        if self.focus.active == Focus::Boards && self.selection.board.get().is_some() {
            self.push_mode(AppMode::BoardDetail);
            self.focus.board_focus = BoardFocus::Name;
        }
    }

    pub fn handle_export_board_key(&mut self) {
        if self.focus.active == Focus::Boards && self.selection.board.get().is_some() {
            if let Some(board_idx) = self.selection.board.get() {
                if let Some(board) = self.model.boards().get(board_idx) {
                    let filename = format!(
                        "{}-{}.json",
                        board.name.replace(" ", "-").to_lowercase(),
                        chrono::Utc::now().format("%Y%m%d-%H%M%S")
                    );
                    self.input.set(filename);
                    self.open_dialog(DialogMode::ExportBoard);
                }
            }
        }
    }

    pub fn handle_export_all_key(&mut self) {
        if self.focus.active == Focus::Boards && !self.model.boards().is_empty() {
            let filename = format!(
                "kanban-all-{}.json",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            );
            self.input.set(filename);
            self.open_dialog(DialogMode::ExportAll);
        }
    }

    pub fn handle_import_board_key(&mut self) {
        if self.focus.active == Focus::Boards {
            self.scan_import_files();
            if !self.dialog_input.import_files.is_empty() {
                self.dialog_input.import_selection.set(Some(0));
                self.open_dialog(DialogMode::ImportBoard);
            }
        }
    }

    pub fn create_board(&mut self) {
        let board_name = self.input.as_str().to_string();

        let board_id = uuid::Uuid::new_v4();
        let position = self.model.boards().len() as i32;
        let new_index = position as usize;

        let mut commands: Vec<Command> = vec![Command::Board(BoardCommand::Create(CreateBoard {
            id: board_id,
            name: board_name.clone(),
            card_prefix: None,
            position,
        }))];

        for (name, position) in [("TODO", 0i32), ("Doing", 1i32), ("Complete", 2i32)] {
            commands.push(Command::Column(ColumnCommand::Create(CreateColumn {
                id: uuid::Uuid::new_v4(),
                board_id,
                name: name.to_string(),
                position,
            })));
        }

        // Single batch so undo reverses the whole "create a board"
        // action in one step.
        if let Err(e) = self.execute_commands_batch(commands) {
            tracing::error!("Failed to create board: {}", e);
            self.set_error(format!("Failed to create board: {}", e));
            return;
        }

        tracing::info!("Created board: {} (id: {})", board_name, board_id);

        self.selection.board.set(Some(new_index));
        self.switch_view_strategy(TaskListView::default());
    }

    pub fn rename_board(&mut self) {
        if let Some(idx) = self.selection.board.get() {
            if let Some(board) = self.model.boards().get(idx) {
                let board_id = board.id;
                let new_name = self.input.as_str().to_string();

                // Execute UpdateBoard command
                let cmd = Command::Board(BoardCommand::Update(UpdateBoard {
                    board_id,
                    updates: BoardUpdate {
                        name: Some(new_name.clone()),
                        ..Default::default()
                    },
                }));

                if let Err(e) = self.execute_command(cmd) {
                    tracing::error!("Failed to rename board: {}", e);
                    self.set_error(format!("Failed to rename board: {}", e));
                    return;
                }

                tracing::info!("Renamed board to: {}", new_name);
            }
        }
    }

    fn scan_import_files(&mut self) {
        self.dialog_input.import_files.clear();
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        if let Some(filename) = entry.file_name().to_str() {
                            if filename.ends_with(".json") {
                                self.dialog_input.import_files.push(filename.to_string());
                            }
                        }
                    }
                }
            }
        }
        self.dialog_input.import_files.sort();
    }
}
