use crate::app::{App, AppMode, DialogMode, ExportDialogState, Focus, SettingsFocus};
use crate::edit_format::EditFormat;
use crate::editor::edit_in_external_editor;
use crate::events::EventHandler;
use crossterm::event::KeyCode;
use kanban_domain::export::{AllBoardsExport, BoardExporter};
use kanban_service::AppConfigDto;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

const EXPORT_BUTTON_STORAGE_INDEX: usize = 3;

impl App {
    pub fn settings_item_count(&self, panel: SettingsFocus) -> usize {
        match panel {
            SettingsFocus::Configuration => {
                if self.has_data_file {
                    if self.cli_file_override {
                        9
                    } else {
                        7
                    }
                } else {
                    5
                }
            }
            SettingsFocus::ConfigFile => 3,
            SettingsFocus::Storage => 4,
        }
    }

    pub fn handle_open_settings(&mut self) {
        if self.focus.active == Focus::Boards {
            self.focus.settings_focus = SettingsFocus::Configuration;
            self.selection.settings_config.set(Some(0));
            self.push_mode(AppMode::Settings);
        }
    }

    pub fn apply_config_edit(
        &mut self,
        new_content: &str,
        format: &EditFormat,
    ) -> Result<bool, String> {
        if !matches!(self.migration_state, crate::app::MigrationState::Idle) {
            return Err("Migration in progress, please wait".to_string());
        }
        let old_location =
            kanban_service::config::effective_configuration_location(&self.app_config);
        let old_storage_location =
            kanban_service::config::resolve_storage_location(&self.app_config);
        let old_config = self.app_config.clone();
        let mut config = self.app_config.clone();
        if self.cli_file_override {
            config.storage_backend = self.original_storage_backend.clone();
            config.storage_location = self.original_storage_location.clone();
        }

        let updated_dto: AppConfigDto = format
            .deserialize(new_content)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        updated_dto
            .validate_and_apply(&mut config)
            .map_err(|e| format!("Invalid config: {}", e))?;

        if kanban_service::config::has_non_default_values(&config) {
            kanban_service::config::save(&config)
                .map_err(|e| format!("Failed to save config: {}", e))?;
            let new_location = kanban_service::config::effective_configuration_location(&config);
            if new_location != old_location {
                let old_path = std::path::Path::new(&old_location);
                if old_path.exists() {
                    if let Err(e) = std::fs::remove_file(old_path) {
                        tracing::warn!(
                            "Failed to remove old config file {}: {}",
                            old_path.display(),
                            e
                        );
                    }
                }
            }
        } else {
            let location = kanban_service::config::effective_configuration_location(&config);
            let path = std::path::Path::new(&location);
            if path.exists() {
                if let Err(e) = std::fs::remove_file(path) {
                    tracing::warn!("Failed to remove config file {}: {}", path.display(), e);
                }
            }
        }

        self.app_config = config;
        self.apply_storage_location_change(old_config, &old_storage_location);
        Ok(true)
    }

    pub fn apply_storage_location_change(
        &mut self,
        old_config: kanban_core::AppConfig,
        old_storage_location: &str,
    ) {
        use crate::app::MigrationState;

        let new_storage_location =
            kanban_service::config::resolve_storage_location(&self.app_config);

        if kanban_service::sync_backend_with_file(&new_storage_location, &mut self.app_config) {
            self.set_success(format!(
                "storage_backend changed to '{}' to match file at '{}'",
                self.app_config.effective_storage_backend(),
                new_storage_location
            ));
        }

        if !self.has_data_file || new_storage_location == old_storage_location {
            return;
        }

        let new_backend = self.app_config.effective_storage_backend().to_string();
        let old_backend = old_config.effective_storage_backend().to_string();
        let old_storage_location_owned = old_storage_location.to_string();
        let old_path_exists = std::path::Path::new(old_storage_location).exists();

        let (tx, rx) = tokio::sync::oneshot::channel();

        let new_storage_clone = new_storage_location.clone();
        let new_backend_clone = new_backend.clone();
        tokio::spawn(async move {
            let file_existed = std::path::Path::new(&new_storage_clone).exists();
            let result: Result<(kanban_domain::Snapshot, bool), String> = async {
                if !file_existed && old_path_exists {
                    kanban_service::migrate_store(
                        &old_backend,
                        &old_storage_location_owned,
                        &new_backend_clone,
                        &new_storage_clone,
                    )
                    .await
                    .map_err(|e| format!("Migration failed: {}", e))?;
                }

                let snapshot =
                    kanban_service::validate_and_load_store(&new_backend_clone, &new_storage_clone)
                        .await
                        .map_err(|e| format!("Invalid storage file: {}", e))?;
                Ok((snapshot, file_existed))
            }
            .await;

            let _ = tx.send(result);
        });

        self.migration_state = MigrationState::Migrating {
            old_config,
            old_storage_location: old_storage_location.to_string(),
            result_rx: rx,
        };
        self.set_success("Migrating storage...".to_string());
    }

    pub fn handle_migration_complete(
        &mut self,
        old_config: kanban_core::AppConfig,
        result: Result<(kanban_domain::Snapshot, bool), String>,
    ) {
        self.migration_state = crate::app::MigrationState::Idle;

        let (snapshot, file_existed) = match result {
            Ok(s) => s,
            Err(e) => {
                self.app_config = old_config;
                self.set_error(e);
                return;
            }
        };

        let new_storage_location =
            kanban_service::config::resolve_storage_location(&self.app_config);
        let new_backend = self.app_config.effective_storage_backend().to_string();

        match self
            .ctx
            .state_manager
            .replace_store(&new_backend, &new_storage_location)
        {
            Ok((save_rx, completion_rx)) => {
                use crate::state::snapshot::TuiSnapshot;
                snapshot.apply_to_app(self);
                self.ctx.state_manager.mark_clean();
                self.ctx.state_manager.clear_history();

                self.selection.active_board_index = if self.ctx.boards.is_empty() {
                    None
                } else {
                    Some(0)
                };
                self.selection.board.set(if self.ctx.boards.is_empty() {
                    None
                } else {
                    Some(0)
                });
                self.selection.active_card_index = None;
                self.selection.card_navigation_history.clear();

                self.persistence.save_file = Some(new_storage_location.clone());
                self.persistence.save_completion_rx = Some(completion_rx);
                self.spawn_save_worker(save_rx);
                self.cli_file_override = false;
                let msg = if file_existed {
                    format!("Loaded from {}", new_storage_location)
                } else {
                    format!("Migrated to {}", new_storage_location)
                };
                self.set_success(msg);
            }
            Err(e) => {
                self.app_config = old_config;
                self.set_error(format!("Store swap failed: {}", e));
            }
        }
    }

    pub async fn await_migration(&mut self) {
        use crate::app::MigrationState;
        let (old_config, rx) =
            match std::mem::replace(&mut self.migration_state, MigrationState::Idle) {
                MigrationState::Migrating {
                    old_config,
                    result_rx,
                    ..
                } => (old_config, result_rx),
                MigrationState::Idle => return,
            };
        if let Ok(result) = rx.await {
            self.handle_migration_complete(old_config, result);
        }
    }

    fn open_config_editor(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
    ) -> bool {
        let format = EditFormat::parse(self.app_config.effective_editing_format());
        let ext = format.file_extension();
        let mut dto = AppConfigDto::from_config(&self.app_config, self.has_data_file);
        if self.cli_file_override {
            dto.storage_backend = if self.config_storage_backend.is_empty() {
                None
            } else {
                Some(self.config_storage_backend.clone())
            };
            dto.storage_location = if self.config_storage_location.is_empty() {
                None
            } else {
                Some(self.config_storage_location.clone())
            };
        }
        let serialized = format.serialize(&dto).unwrap_or_else(|_| "{}".to_string());
        let current_content = if self.cli_file_override {
            format.comment_storage_fields(&serialized)
        } else {
            serialized
        };
        let temp_file = std::env::temp_dir().join(format!("kanban_config_edit.{}", ext));
        match edit_in_external_editor(terminal, event_handler, temp_file, &current_content) {
            Ok(Some(new_content)) => {
                if let Err(e) = self.apply_config_edit(&new_content, &format) {
                    self.set_error(e);
                }
            }
            Ok(None) => {}
            Err(e) => {
                self.set_error(format!("Failed to edit config: {}", e));
            }
        }
        true
    }

    pub fn handle_settings_key(
        &mut self,
        key: KeyCode,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
    ) -> bool {
        match key {
            KeyCode::Enter if self.focus.settings_focus == SettingsFocus::Configuration => {
                self.open_config_editor(terminal, event_handler)
            }
            KeyCode::Char('1')
            | KeyCode::Char('2')
            | KeyCode::Char('3')
            | KeyCode::Char('j')
            | KeyCode::Down
            | KeyCode::Char('k')
            | KeyCode::Up
            | KeyCode::Char('h')
            | KeyCode::Left
            | KeyCode::Char('l')
            | KeyCode::Right
            | KeyCode::Enter => self.handle_settings_key_nav(key),
            KeyCode::Char('e') => self.open_config_editor(terminal, event_handler),
            KeyCode::Char('x') => {
                let board_count = self.ctx.boards.len();
                if board_count == 0 {
                    self.set_error("No boards to export".to_string());
                    return false;
                }
                self.export_dialog = Some(ExportDialogState::new(board_count));
                self.push_mode(AppMode::Dialog(DialogMode::ExportBoards));
                false
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.pop_mode();
                false
            }
            _ => false,
        }
    }

    pub fn handle_settings_key_nav(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('1') => {
                self.focus.settings_focus = SettingsFocus::Configuration;
                self.selection
                    .settings_config
                    .auto_select_first_if_empty(true);
            }
            KeyCode::Char('2') => {
                self.focus.settings_focus = SettingsFocus::ConfigFile;
                self.selection
                    .settings_config_file
                    .auto_select_first_if_empty(true);
            }
            KeyCode::Char('3') => {
                self.focus.settings_focus = SettingsFocus::Storage;
                self.selection
                    .settings_storage
                    .auto_select_first_if_empty(true);
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.handle_settings_nav_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.handle_settings_nav_up();
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if self.focus.settings_focus == SettingsFocus::Storage {
                    self.focus.settings_focus = SettingsFocus::Configuration;
                    self.selection
                        .settings_config
                        .auto_select_first_if_empty(true);
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.focus.settings_focus != SettingsFocus::Storage {
                    self.focus.settings_focus = SettingsFocus::Storage;
                    self.selection
                        .settings_storage
                        .auto_select_first_if_empty(true);
                }
            }
            KeyCode::Enter => {
                if self.focus.settings_focus == SettingsFocus::Storage
                    && self.selection.settings_storage.get() == Some(EXPORT_BUTTON_STORAGE_INDEX)
                {
                    return self.trigger_export();
                }
            }
            _ => {}
        }
        false
    }

    pub fn handle_settings_nav_down(&mut self) {
        match self.focus.settings_focus {
            SettingsFocus::Configuration => {
                let count = self.settings_item_count(SettingsFocus::Configuration);
                let current = self.selection.settings_config.get().unwrap_or(0);
                if self.cli_file_override && self.has_data_file {
                    match current {
                        c if c >= count - 1 => {
                            self.focus.settings_focus = SettingsFocus::ConfigFile;
                            self.selection.settings_config_file.set(Some(0));
                        }
                        4 => self.selection.settings_config.set(Some(7)),
                        _ => self.selection.settings_config.next(count),
                    }
                } else if current >= count - 1 {
                    self.focus.settings_focus = SettingsFocus::ConfigFile;
                    self.selection.settings_config_file.set(Some(0));
                } else {
                    self.selection.settings_config.next(count);
                }
            }
            SettingsFocus::ConfigFile => {
                let count = self.settings_item_count(SettingsFocus::ConfigFile);
                let current = self.selection.settings_config_file.get().unwrap_or(0);
                if current >= count - 1 {
                    self.focus.settings_focus = SettingsFocus::Configuration;
                    self.selection.settings_config.set(Some(0));
                } else {
                    self.selection.settings_config_file.next(count);
                }
            }
            SettingsFocus::Storage => {
                let count = self.settings_item_count(SettingsFocus::Storage);
                let current = self.selection.settings_storage.get().unwrap_or(0);
                if current >= count - 1 {
                    self.selection.settings_storage.set(Some(0));
                } else {
                    self.selection.settings_storage.next(count);
                }
            }
        }
    }

    fn handle_settings_nav_up(&mut self) {
        match self.focus.settings_focus {
            SettingsFocus::Configuration => {
                let current = self.selection.settings_config.get().unwrap_or(0);
                if current == 0 {
                    let count = self.settings_item_count(SettingsFocus::ConfigFile);
                    self.focus.settings_focus = SettingsFocus::ConfigFile;
                    self.selection.settings_config_file.set(Some(count - 1));
                } else if self.cli_file_override && self.has_data_file && current == 7 {
                    self.selection.settings_config.set(Some(4));
                } else {
                    self.selection.settings_config.prev();
                }
            }
            SettingsFocus::ConfigFile => {
                let current = self.selection.settings_config_file.get().unwrap_or(0);
                if current == 0 {
                    let count = self.settings_item_count(SettingsFocus::Configuration);
                    self.focus.settings_focus = SettingsFocus::Configuration;
                    self.selection.settings_config.set(Some(count - 1));
                } else {
                    self.selection.settings_config_file.prev();
                }
            }
            SettingsFocus::Storage => {
                let current = self.selection.settings_storage.get().unwrap_or(0);
                if current == 0 {
                    let count = self.settings_item_count(SettingsFocus::Storage);
                    self.selection.settings_storage.set(Some(count - 1));
                } else {
                    self.selection.settings_storage.prev();
                }
            }
        }
    }

    fn trigger_export(&mut self) -> bool {
        let board_count = self.ctx.boards.len();
        if board_count == 0 {
            self.set_error("No boards to export".to_string());
            return false;
        }
        self.export_dialog = Some(ExportDialogState::new(board_count));
        self.push_mode(AppMode::Dialog(DialogMode::ExportBoards));
        false
    }

    pub fn handle_export_boards_dialog(&mut self, key_code: KeyCode) {
        let Some(ref mut dialog) = self.export_dialog else {
            return;
        };

        match dialog.step {
            crate::app::ExportStep::SelectBoards => match key_code {
                KeyCode::Char(' ') => {
                    dialog.toggle(dialog.cursor);
                }
                KeyCode::Char('a') => {
                    dialog.select_all();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let len = dialog.board_selections.len();
                    if len > 0 {
                        dialog.cursor = (dialog.cursor + 1) % len;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let len = dialog.board_selections.len();
                    if len > 0 {
                        dialog.cursor = (dialog.cursor + len - 1) % len;
                    }
                }
                KeyCode::Enter => {
                    if dialog.any_selected() {
                        dialog.step = crate::app::ExportStep::ExportOptions;
                    }
                }
                KeyCode::Esc => {
                    self.export_dialog = None;
                    self.pop_mode();
                }
                _ => {}
            },
            crate::app::ExportStep::ExportOptions => match key_code {
                KeyCode::Tab | KeyCode::BackTab => {
                    dialog.format = match dialog.format {
                        crate::app::ExportFormat::Json => crate::app::ExportFormat::Sqlite,
                        crate::app::ExportFormat::Sqlite => crate::app::ExportFormat::Json,
                    };
                    let stem = dialog
                        .filename
                        .rsplit_once('.')
                        .map(|(s, _)| s)
                        .unwrap_or(&dialog.filename)
                        .to_string();
                    dialog.filename = match dialog.format {
                        crate::app::ExportFormat::Json => format!("{}.json", stem),
                        crate::app::ExportFormat::Sqlite => format!("{}.sqlite", stem),
                    };
                }
                KeyCode::Backspace => {
                    dialog.filename.pop();
                }
                KeyCode::Char(c)
                    if !matches!(
                        c,
                        '/' | '\\' | '\0' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                    ) =>
                {
                    dialog.filename.push(c);
                }
                KeyCode::Enter => {
                    self.execute_export();
                }
                KeyCode::Esc => {
                    dialog.step = crate::app::ExportStep::SelectBoards;
                }
                _ => {}
            },
        }
    }

    fn execute_export(&mut self) {
        let Some(ref dialog) = self.export_dialog else {
            return;
        };

        let filename = dialog.filename.clone();
        let selected_indices: Vec<usize> = dialog
            .board_selections
            .iter()
            .enumerate()
            .filter(|(_, &selected)| selected)
            .map(|(i, _)| i)
            .collect();

        if selected_indices.is_empty() || filename.is_empty() {
            self.set_error("No boards selected or filename empty".to_string());
            return;
        }

        let board_exports: Vec<_> = selected_indices
            .iter()
            .filter_map(|&i| self.ctx.boards.get(i))
            .map(|board| {
                BoardExporter::export_board(
                    board,
                    &self.ctx.columns,
                    &self.ctx.cards,
                    &self.ctx.archived_cards,
                    &self.ctx.sprints,
                )
            })
            .collect();

        let export = AllBoardsExport::from_boards(board_exports);

        match dialog.format {
            crate::app::ExportFormat::Json => {
                match BoardExporter::export_to_file(&export, &filename) {
                    Ok(()) => self.set_success(format!("Exported to {}", filename)),
                    Err(e) => self.set_error(format!("Export failed: {}", e)),
                }
            }
            crate::app::ExportFormat::Sqlite => {
                let filename_clone = filename.clone();
                let export_clone = export.clone();
                let (tx, rx) = tokio::sync::oneshot::channel();
                tokio::spawn(async move {
                    let result = kanban_service::export_to_sqlite(export_clone, &filename_clone)
                        .await
                        .map(|_| filename_clone)
                        .map_err(|e| format!("Export failed: {}", e));
                    let _ = tx.send(result);
                });
                self.export_result_rx = Some(rx);
                self.set_success("Exporting...".to_string());
            }
        }

        self.export_dialog = None;
        self.pop_mode();
    }
}
