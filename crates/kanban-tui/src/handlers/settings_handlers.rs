use crate::app::{App, AppMode, DialogMode, ExportDialogState, Focus, SettingsFocus};
use crate::edit_format::EditFormat;
use crate::events::EventHandler;
use crossterm::event::KeyCode;
use kanban_core::AppConfigDto;
use kanban_domain::export::{BoardExporter, AllBoardsExport};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

impl App {
    pub fn settings_item_count(&self, panel: SettingsFocus) -> usize {
        match panel {
            SettingsFocus::Configuration => {
                if self.app_config.has_data_file { 7 } else { 5 }
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

    pub fn apply_storage_location_change(
        &mut self,
        old_config: kanban_core::AppConfig,
        old_storage_location: &str,
    ) -> bool {
        let new_storage_location = self.app_config.effective_storage_location();

        if let Some(detected) = kanban_service::detect_backend(&new_storage_location) {
            let configured = self.app_config.effective_storage_backend().to_string();
            if detected != configured {
                self.app_config.storage_backend = Some(detected.clone());
                self.set_success(format!(
                    "storage_backend changed to '{}' to match file at '{}'",
                    detected, new_storage_location
                ));
            }
        }

        if !self.app_config.has_data_file || new_storage_location == old_storage_location {
            return true;
        }

        let new_path = std::path::Path::new(&new_storage_location);
        let file_existed = new_path.exists();

        let new_backend = self.app_config.effective_storage_backend().to_string();
        let old_backend = old_config.effective_storage_backend();

        if !file_existed {
            let old_path = std::path::Path::new(old_storage_location);
            if old_path.exists() {
                if let Err(e) = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(
                        kanban_service::migrate_store_for_backend(Some(old_backend), old_storage_location, Some(&new_backend), &new_storage_location),
                    )
                }) {
                    self.app_config = old_config;
                    self.set_error(format!("Migration failed: {}", e));
                    return false;
                }
            }
        }

        let snapshot = match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                kanban_service::validate_and_load_store_for_backend(Some(&new_backend), &new_storage_location),
            )
        }) {
            Ok(s) => s,
            Err(e) => {
                self.app_config = old_config;
                self.set_error(format!("Invalid storage file: {}", e));
                return false;
            }
        };

        match self.ctx.state_manager.replace_store(&new_backend, &new_storage_location) {
            Ok((save_rx, completion_rx)) => {
                use crate::state::snapshot::TuiSnapshot;
                snapshot.apply_to_app(self);
                self.ctx.state_manager.mark_clean();
                self.ctx.state_manager.clear_history();

                self.selection.active_board_index =
                    if self.ctx.boards.is_empty() { None } else { Some(0) };
                self.selection.board.set(
                    if self.ctx.boards.is_empty() { None } else { Some(0) },
                );
                self.selection.active_card_index = None;
                self.selection.card_navigation_history.clear();

                self.persistence.save_file = Some(new_storage_location.clone());
                self.persistence.save_completion_rx = Some(completion_rx);
                self.spawn_save_worker(save_rx);
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
                return false;
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
                self.handle_settings_key(KeyCode::Char('e'), terminal, event_handler)
            }
            KeyCode::Char('1') | KeyCode::Char('2') | KeyCode::Char('3')
            | KeyCode::Char('j') | KeyCode::Down
            | KeyCode::Char('k') | KeyCode::Up
            | KeyCode::Char('h') | KeyCode::Left
            | KeyCode::Char('l') | KeyCode::Right
            | KeyCode::Enter => {
                self.handle_settings_key_nav(key)
            }
            KeyCode::Char('e') => {
                let format = EditFormat::parse(self.app_config.effective_editing_format());
                let ext = format.file_extension();
                let old_location = self.app_config.effective_configuration_location();
                let old_storage_location = self.app_config.effective_storage_location();
                let old_config = self.app_config.clone();
                let mut config = self.app_config.clone();
                let temp_file =
                    std::env::temp_dir().join(format!("kanban_config_edit.{}", ext));
                if let Err(e) = App::edit_entity_impl::<AppConfigDto, _>(
                    &mut config,
                    terminal,
                    event_handler,
                    temp_file,
                    format,
                ) {
                    self.set_error(format!("Failed to edit config: {}", e));
                    return true;
                }
                self.app_config = config;

                if !self.apply_storage_location_change(old_config, &old_storage_location) {
                    return true;
                }

                if self.app_config.has_non_default_values() {
                    if let Err(e) = self.app_config.save() {
                        self.set_error(format!("Failed to save config: {}", e));
                    } else {
                        let new_location = self.app_config.effective_configuration_location();
                        if new_location != old_location {
                            let old_path = std::path::Path::new(&old_location);
                            if old_path.exists() {
                                let _ = std::fs::remove_file(old_path);
                            }
                        }
                    }
                } else {
                    let location = self.app_config.effective_configuration_location();
                    let path = std::path::Path::new(&location);
                    if path.exists() {
                        let _ = std::fs::remove_file(path);
                    }
                }
                self.ctx.default_card_prefix = self.app_config.effective_default_card_prefix().to_string();
                self.ctx.default_sprint_prefix = self.app_config.effective_default_sprint_prefix().to_string();
                true
            }
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
                self.selection.settings_config.auto_select_first_if_empty(true);
            }
            KeyCode::Char('2') => {
                self.focus.settings_focus = SettingsFocus::ConfigFile;
                self.selection.settings_config_file.auto_select_first_if_empty(true);
            }
            KeyCode::Char('3') => {
                self.focus.settings_focus = SettingsFocus::Storage;
                self.selection.settings_storage.auto_select_first_if_empty(true);
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
                    self.selection.settings_config.auto_select_first_if_empty(true);
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.focus.settings_focus != SettingsFocus::Storage {
                    self.focus.settings_focus = SettingsFocus::Storage;
                    self.selection.settings_storage.auto_select_first_if_empty(true);
                }
            }
            KeyCode::Enter => {
                if self.focus.settings_focus == SettingsFocus::Storage
                    && self.selection.settings_storage.get() == Some(3)
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
                if current >= count - 1 {
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
                KeyCode::Char(c) => {
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
                self.set_error("SQLite export not yet implemented".to_string());
            }
        }

        self.export_dialog = None;
        self.pop_mode();
    }
}
