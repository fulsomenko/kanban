pub mod mode;
pub use mode::{AppMode, DialogMode};

pub mod focus;
pub use focus::{BoardFocus, CardFocus, Focus, FocusState, SettingsFocus};

pub mod sprint_view;
pub use sprint_view::{SprintTaskPanel, SprintViewState};

pub mod animation;
pub use animation::{AnimationState, CardAnimation};

pub mod selection;
pub use selection::SelectionHub;

pub mod filter;
pub use filter::FilterState;

pub mod multi_select;
pub use multi_select::MultiSelectState;

pub mod dialog_input;
pub use dialog_input::DialogInputState;

pub mod relationship;
pub use relationship::RelationshipState;

pub mod view;
pub use view::{RenderData, ViewState};

pub mod persistence;
pub use persistence::PersistenceState;

pub mod ui_state;
pub use ui_state::UiState;

use crate::{
    clipboard,
    components::Banner,
    editor::edit_in_external_editor,
    events::{Event, EventHandler},
    state::TuiSnapshot,
    tui_context::TuiContext,
    ui,
    view_strategy::{UnifiedViewStrategy, ViewRefreshContext, ViewStrategy},
};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use kanban_core::{AppConfig, Editable, InputState};
use kanban_domain::AnimationType;
use kanban_domain::KanbanResult;
use kanban_domain::{
    export::{AllBoardsExport, BoardExporter, BoardImporter},
    filter::{BoardFilter, CardFilter, SprintFilter, UnassignedOnlyFilter},
    partition_sprint_cards,
    sort::{get_sorter_for_field, OrderedSorter},
    sort_card_ids, Board, Card, SortField, SortOrder, Sprint,
};
use kanban_service::StoreManager;

use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Builds a `StoreManager` that mirrors the default CLI registry: SQLite
/// first (so content-sniffing prefers it) and JSON second as a catch-all
/// fallback. Used by [`App::new`] as the default backend configuration.
fn default_store_manager() -> StoreManager {
    StoreManager::new(kanban_service::default_registry())
}

pub struct App {
    pub store_manager: Arc<StoreManager>,
    pub should_quit: bool,
    pub quit_with_pending: bool, // Force quit even if saves are pending (second 'q' press)
    pub quit_with_migration: bool, // Force quit even if migration is in progress (second 'q' press)
    pub mode: AppMode,
    pub mode_stack: Vec<AppMode>,
    pub input: InputState,
    pub ctx: TuiContext,
    pub app_config: AppConfig,
    pub selection: SelectionHub,
    pub animation: AnimationState,
    pub filter: FilterState,
    pub dialog_input: DialogInputState,
    pub focus: FocusState,
    pub persistence: PersistenceState,
    pub multi_select: MultiSelectState,
    pub ui_state: UiState,
    pub sprint_view: SprintViewState,
    pub view: ViewState,
    pub render_data: RenderData,
    pub relationship: RelationshipState,
    pub pending_key: Option<char>,
    pub has_data_file: bool,
    pub cli_file_provided: bool,
    pub cli_file_override: bool,
    pub config_storage_backend: String,
    pub config_storage_location: String,
    pub original_storage_backend: Option<String>,
    pub original_storage_location: Option<String>,
    pub export_dialog: Option<ExportDialogState>,
    pub migration_state: MigrationState,
    pub export_result_rx: Option<tokio::sync::oneshot::Receiver<Result<String, String>>>,
    pub needs_redraw: bool,
    pub error_log: Arc<Mutex<crate::error_log::ErrorLogState>>,
    pub auto_open_seen_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportStep {
    SelectBoards,
    ExportOptions,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExportFormat {
    #[default]
    Json,
    Sqlite,
}

#[derive(Debug, Clone)]
pub struct ExportDialogState {
    pub board_selections: Vec<bool>,
    pub cursor: usize,
    pub step: ExportStep,
    pub format: ExportFormat,
    pub filename: String,
}

impl ExportDialogState {
    pub fn new(board_count: usize) -> Self {
        Self {
            board_selections: vec![false; board_count],
            cursor: 0,
            step: ExportStep::SelectBoards,
            format: ExportFormat::default(),
            filename: "export.json".to_string(),
        }
    }

    pub fn toggle(&mut self, index: usize) {
        if let Some(selected) = self.board_selections.get_mut(index) {
            *selected = !*selected;
        }
    }

    pub fn select_all(&mut self) {
        let all_selected = self.board_selections.iter().all(|&s| s);
        for s in &mut self.board_selections {
            *s = !all_selected;
        }
    }

    pub fn any_selected(&self) -> bool {
        self.board_selections.iter().any(|&s| s)
    }
}

pub enum MigrationState {
    Idle,
    Migrating {
        old_config: AppConfig,
        old_storage_location: String,
        result_rx: tokio::sync::oneshot::Receiver<Result<(kanban_domain::Snapshot, bool), String>>,
    },
}

pub enum CardField {
    Title,
    Description,
}

pub enum BoardField {
    Name,
    Description,
}

impl App {
    /// Convenience constructor using the default built-in backends (SQLite +
    /// JSON). Prefer [`App::new_with_store`] when embedding the TUI in a
    /// third-party binary that registers its own [`StoreFactory`].
    pub fn new(
        save_file: Option<String>,
    ) -> kanban_domain::KanbanResult<(
        Self,
        Option<tokio::sync::mpsc::Receiver<kanban_domain::Snapshot>>,
    )> {
        Self::new_with_store(default_store_manager(), save_file)
    }

    pub fn new_with_store(
        store_manager: StoreManager,
        save_file: Option<String>,
    ) -> kanban_domain::KanbanResult<(
        Self,
        Option<tokio::sync::mpsc::Receiver<kanban_domain::Snapshot>>,
    )> {
        let mut app_config = kanban_service::config::load();
        let config_resolved = kanban_service::config::resolve_storage_location(&app_config);
        let config_storage_backend = app_config.effective_storage_backend().to_string();
        let config_storage_location = config_resolved.clone();
        let original_storage_backend = app_config.storage_backend.clone();
        let original_storage_location = app_config.storage_location.clone();
        if let Some(ref file) = save_file {
            let path = std::path::Path::new(file);
            let resolved = if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(path))
                    .unwrap_or_else(|_| path.to_path_buf())
            };
            let canonical = resolved.canonicalize().unwrap_or(resolved);
            app_config.storage_location = Some(canonical.display().to_string());
            // File arg is the source of truth — ignore config's storage_backend
            app_config.storage_backend = None;
        }
        if store_manager
            .sync_backend_with_file(&app_config.effective_storage_location(), &mut app_config)
        {
            tracing::warn!(
                "Storage backend auto-corrected to '{}' based on file content",
                app_config.effective_storage_backend()
            );
        }
        let effective_file = kanban_service::config::resolve_storage_location(&app_config);
        let cli_file_override = save_file.is_some() && effective_file != config_resolved;
        // If the CLI-supplied file resolves to the same path as the configured
        // default, this is not a real override. Don't write the canonical
        // absolute path into app_config.storage_location — it is
        // indistinguishable from a user-set value and would be written to the
        // config file whenever any other setting is changed.
        if save_file.is_some() && !cli_file_override && original_storage_location.is_none() {
            app_config.storage_location = None;
        }
        let backend = app_config.effective_storage_backend().to_string();
        let store = store_manager.make_store(&backend, &effective_file)?;
        let (ctx, save_rx, save_completion_rx) = TuiContext::new(Some(store))?;
        let store_manager = Arc::new(store_manager);
        let app = Self {
            store_manager,
            should_quit: false,
            quit_with_pending: false,
            quit_with_migration: false,
            mode: AppMode::Normal,
            mode_stack: Vec::new(),
            input: InputState::new(),
            ctx,
            app_config,
            selection: SelectionHub::default(),
            animation: AnimationState::default(),
            filter: FilterState::default(),
            dialog_input: DialogInputState::default(),
            focus: FocusState::default(),
            persistence: PersistenceState::new(Some(effective_file), save_completion_rx),
            multi_select: MultiSelectState::default(),
            ui_state: UiState::default(),
            sprint_view: SprintViewState::default(),
            view: ViewState::default(),
            render_data: RenderData::default(),
            relationship: RelationshipState::default(),
            pending_key: None,
            has_data_file: true,
            cli_file_provided: save_file.is_some(),
            cli_file_override,
            config_storage_backend,
            config_storage_location,
            original_storage_backend,
            original_storage_location,
            export_dialog: None,
            migration_state: MigrationState::Idle,
            export_result_rx: None,
            needs_redraw: true,
            error_log: Arc::new(Mutex::new(crate::error_log::ErrorLogState::default())),
            auto_open_seen_count: 0,
        };

        Ok((app, save_rx))
    }

    /// Open a SQLite file using the relational `SqliteStore` backend.
    ///
    /// Unlike [`App::new_with_store`], this constructor is async and uses
    /// `KanbanContext::open_sqlite` so every CRUD operation writes directly to
    /// the database. No debounced blob-write save worker is started.
    pub async fn open_sqlite(
        path: &str,
        config: kanban_core::AppConfig,
    ) -> kanban_domain::KanbanResult<Self> {
        let mut app_config = config;
        let config_storage_backend = app_config.effective_storage_backend().to_string();
        let config_storage_location = kanban_service::config::resolve_storage_location(&app_config);
        let original_storage_backend = app_config.storage_backend.clone();
        let original_storage_location = app_config.storage_location.clone();

        let resolved = {
            let p = std::path::Path::new(path);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(p))
                    .unwrap_or_else(|_| p.to_path_buf())
            }
        };
        let effective_file = resolved.to_string_lossy().to_string();
        app_config.storage_location = Some(effective_file.clone());
        app_config.storage_backend = Some("sqlite".to_string());

        let inner =
            kanban_service::KanbanContext::open_sqlite(&effective_file, app_config.clone()).await?;

        let (ctx, _save_rx, save_completion_rx) =
            crate::tui_context::TuiContext::from_context(inner);
        let store_manager = Arc::new(default_store_manager());

        Ok(Self {
            store_manager,
            should_quit: false,
            quit_with_pending: false,
            quit_with_migration: false,
            mode: AppMode::Normal,
            mode_stack: Vec::new(),
            input: kanban_core::InputState::new(),
            ctx,
            app_config,
            selection: SelectionHub::default(),
            animation: AnimationState::default(),
            filter: FilterState::default(),
            dialog_input: DialogInputState::default(),
            focus: FocusState::default(),
            persistence: PersistenceState::new(Some(effective_file), save_completion_rx),
            multi_select: MultiSelectState::default(),
            ui_state: UiState::default(),
            sprint_view: SprintViewState::default(),
            view: ViewState::default(),
            render_data: RenderData::default(),
            relationship: RelationshipState::default(),
            pending_key: None,
            has_data_file: true,
            cli_file_provided: true,
            cli_file_override: true,
            config_storage_backend,
            config_storage_location,
            original_storage_backend,
            original_storage_location,
            export_dialog: None,
            migration_state: MigrationState::Idle,
            export_result_rx: None,
            needs_redraw: true,
            error_log: Arc::new(Mutex::new(crate::error_log::ErrorLogState::default())),
            auto_open_seen_count: 0,
        })
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn handle_quit_key(&mut self) {
        let needs_pending_confirm =
            self.ctx.save_coordinator.has_pending_saves() && !self.quit_with_pending;
        let needs_migration_confirm =
            matches!(self.migration_state, MigrationState::Migrating { .. })
                && !self.quit_with_migration;

        if needs_pending_confirm || needs_migration_confirm {
            if needs_pending_confirm && needs_migration_confirm {
                self.set_error(
                    "⏳ Saves pending and migration in progress... press 'q' again to force quit"
                        .to_string(),
                );
            } else if needs_pending_confirm {
                self.set_error(
                    "⏳ Saves pending... press 'q' again to force quit, or wait for completion"
                        .to_string(),
                );
                tracing::warn!("Quit attempted with pending saves, requiring confirmation");
            } else {
                self.set_error(
                    "Migration in progress... press 'q' again to abort and quit".to_string(),
                );
            }
            self.quit_with_pending = true;
            self.quit_with_migration = true;
            return;
        }

        self.quit();
    }

    pub fn spawn_save_worker(
        &mut self,
        mut rx: tokio::sync::mpsc::Receiver<kanban_domain::Snapshot>,
        deferred_watch_path: Option<std::path::PathBuf>,
    ) {
        use kanban_domain::KanbanError;
        use kanban_persistence::{PersistenceMetadata, StoreSnapshot};

        if let Some(store) = self.ctx.store() {
            let instance_id = store.instance_id();
            let file_watcher = self.persistence.file_watcher.clone();
            let save_completion_tx = self.ctx.save_coordinator.save_completion_tx().cloned();

            tracing::info!("Spawning save worker to process snapshots");
            let handle = tokio::spawn(async move {
                use kanban_persistence::ChangeDetector;
                tracing::info!("Save worker task started, waiting for snapshots");
                let mut watching_started = deferred_watch_path.is_none();
                while let Some(snapshot) = rx.recv().await {
                    tracing::debug!("Save worker received snapshot, starting save operation");

                    let data = match kanban_persistence::snapshot_to_json_bytes(&snapshot) {
                        Ok(d) => d,
                        Err(e) => {
                            tracing::error!("Failed to serialize snapshot: {}", e);
                            if let Some(ref watcher) = file_watcher {
                                watcher.resume();
                            }
                            continue;
                        }
                    };

                    let persistence_snapshot = StoreSnapshot {
                        data,
                        metadata: PersistenceMetadata::new(instance_id),
                    };

                    match store
                        .save(persistence_snapshot)
                        .await
                        .map_err(KanbanError::from)
                    {
                        Ok(_) => {
                            tracing::debug!("Save worker completed save");
                            if !watching_started {
                                if let (Some(ref watcher), Some(ref p)) =
                                    (&file_watcher, &deferred_watch_path)
                                {
                                    match watcher.start_watching(p.clone()).await {
                                        Ok(()) => {
                                            tracing::info!(
                                                "Deferred file watching started for: {}",
                                                p.display()
                                            );
                                            watching_started = true;
                                        }
                                        Err(e) => tracing::warn!(
                                            "Failed to start deferred file watching: {}",
                                            e
                                        ),
                                    }
                                }
                            }
                            if let Some(ref tx) = save_completion_tx {
                                if let Err(e) = tx.send(()) {
                                    tracing::error!("Failed to send save completion signal: {}", e);
                                }
                            }
                        }
                        Err(KanbanError::ConflictDetected { path, .. }) => {
                            tracing::warn!("Save worker detected conflict at {}", path);
                            if let Some(ref tx) = save_completion_tx {
                                if let Err(e) = tx.send(()) {
                                    tracing::error!("Failed to send save completion signal: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Save worker failed: {}", e);
                            if let Some(ref tx) = save_completion_tx {
                                if let Err(e) = tx.send(()) {
                                    tracing::error!("Failed to send save completion signal: {}", e);
                                }
                            }
                        }
                    }

                    if let Some(ref watcher) = file_watcher {
                        watcher.resume();
                    }
                }
                tracing::info!("Save worker exited recv loop (channel closed)");
            });
            self.persistence.save_worker_handle = Some(handle);
        } else {
            tracing::warn!("Could not spawn save worker: no store available");
        }
    }

    pub fn push_mode(&mut self, new_mode: AppMode) {
        self.mode_stack.push(self.mode.clone());
        self.mode = new_mode;
    }

    pub fn pop_mode(&mut self) {
        self.mode = self.mode_stack.pop().unwrap_or(AppMode::Normal);
    }

    pub fn is_dialog_mode(&self) -> bool {
        matches!(self.mode, AppMode::Dialog(_))
    }

    pub fn get_base_mode(&self) -> &AppMode {
        if self.is_dialog_mode() {
            self.mode_stack.last().unwrap_or(&AppMode::Normal)
        } else {
            &self.mode
        }
    }

    pub fn with_error_log<R>(&self, f: impl FnOnce(&crate::error_log::ErrorLogState) -> R) -> R {
        let log = self.error_log.lock().unwrap_or_else(|e| e.into_inner());
        f(&log)
    }

    pub fn with_error_log_mut<R>(
        &mut self,
        f: impl FnOnce(&mut crate::error_log::ErrorLogState) -> R,
    ) -> R {
        let mut log = self.error_log.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut log)
    }

    pub fn open_error_log(&mut self) {
        let entry_count = self.with_error_log_mut(|log| {
            log.has_unread_errors = false;
            log.unread_count = 0;
            log.entries.len()
        });
        self.ui_state.error_log_list.update_item_count(entry_count);
        self.ui_state.error_log_list.set_scroll_offset(0);
        self.push_mode(AppMode::ErrorLog);
    }

    pub fn set_error_log(&mut self, error_log: Arc<Mutex<crate::error_log::ErrorLogState>>) {
        self.error_log = error_log;
    }

    pub fn error_log_arc(&self) -> Arc<Mutex<crate::error_log::ErrorLogState>> {
        Arc::clone(&self.error_log)
    }

    fn handle_error_log_mode(&mut self, key_code: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        match key_code {
            KeyCode::Esc | KeyCode::Char('q') => self.pop_mode(),
            KeyCode::Char('j') | KeyCode::Down => {
                self.ui_state.error_log_list.navigate_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.ui_state.error_log_list.navigate_up();
            }
            _ => {}
        }
    }

    pub fn open_dialog(&mut self, dialog: DialogMode) {
        self.push_mode(AppMode::Dialog(dialog));
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.ui_state.banner = Some(Banner::error(message));
    }

    pub fn set_success(&mut self, message: impl Into<String>) {
        self.ui_state.banner = Some(Banner::success(message));
    }

    pub fn clear_banner(&mut self) {
        self.ui_state.banner = None;
    }

    fn keycode_matches_binding_key(
        key_code: &crossterm::event::KeyCode,
        binding_key: &str,
    ) -> bool {
        use crossterm::event::KeyCode;

        match key_code {
            KeyCode::Char(c) => {
                // Check if the entire binding_key is a single char match (handles "/" correctly)
                if binding_key.len() == 1 && binding_key.starts_with(*c) {
                    return true;
                }
                // Check if any part after splitting on '/' matches
                binding_key
                    .split('/')
                    .any(|k| k.trim().len() == 1 && k.trim().starts_with(*c))
            }
            KeyCode::Enter => binding_key.split('/').any(|k| {
                let trimmed = k.trim();
                trimmed == "Enter" || trimmed == "ENTER"
            }),
            KeyCode::Esc => binding_key.split('/').any(|k| {
                let trimmed = k.trim();
                trimmed == "Esc" || trimmed == "ESC"
            }),
            KeyCode::Backspace => binding_key.split('/').any(|k| {
                let trimmed = k.trim();
                trimmed == "Backspace" || trimmed == "BACKSPACE"
            }),
            KeyCode::Home => binding_key.split('/').any(|k| {
                let trimmed = k.trim();
                trimmed == "Home" || trimmed == "HOME"
            }),
            KeyCode::End => binding_key.split('/').any(|k| {
                let trimmed = k.trim();
                trimmed == "End" || trimmed == "END"
            }),
            KeyCode::Down => binding_key.split('/').any(|k| {
                let trimmed = k.trim();
                trimmed == "↓" || trimmed == "Down" || trimmed == "DOWN"
            }),
            KeyCode::Up => binding_key.split('/').any(|k| {
                let trimmed = k.trim();
                trimmed == "↑" || trimmed == "Up" || trimmed == "UP"
            }),
            KeyCode::Left => binding_key.split('/').any(|k| {
                let trimmed = k.trim();
                trimmed == "←" || trimmed == "Left" || trimmed == "LEFT"
            }),
            KeyCode::Right => binding_key.split('/').any(|k| {
                let trimmed = k.trim();
                trimmed == "→" || trimmed == "Right" || trimmed == "RIGHT"
            }),
            _ => false,
        }
    }

    fn execute_action(&mut self, action: &crate::keybindings::KeybindingAction) {
        use crate::keybindings::KeybindingAction;

        match action {
            KeybindingAction::NavigateDown => self.handle_navigation_down(),
            KeybindingAction::NavigateUp => self.handle_navigation_up(),
            KeybindingAction::NavigateLeft => self.handle_kanban_column_left(),
            KeybindingAction::NavigateRight => self.handle_kanban_column_right(),
            KeybindingAction::SelectItem => self.handle_selection_activate(),
            KeybindingAction::CreateCard => self.handle_create_card_key(),
            KeybindingAction::CreateBoard => self.handle_create_board_key(),
            KeybindingAction::CreateSprint => self.handle_create_sprint_key(),
            KeybindingAction::CreateColumn => self.handle_create_column_key(),
            KeybindingAction::RenameBoard => self.handle_rename_board_key(),
            KeybindingAction::RenameColumn => self.handle_rename_column_key(),
            KeybindingAction::EditCard => {}
            KeybindingAction::EditBoard => self.handle_edit_board_key(),
            KeybindingAction::ToggleCompletion => self.handle_toggle_card_completion(),
            KeybindingAction::AssignToSprint => self.handle_assign_to_sprint_key(),
            KeybindingAction::ArchiveCard => self.handle_archive_card(),
            KeybindingAction::RestoreCard => self.handle_restore_card(),
            KeybindingAction::DeleteCard => self.handle_delete_card_permanent(),
            KeybindingAction::MoveCardLeft => self.handle_move_card_left(),
            KeybindingAction::MoveCardRight => self.handle_move_card_right(),
            KeybindingAction::MoveColumnUp => self.handle_move_column_up(),
            KeybindingAction::MoveColumnDown => self.handle_move_column_down(),
            KeybindingAction::DeleteColumn => self.handle_delete_column_key(),
            KeybindingAction::ExportBoard => self.handle_export_board_key(),
            KeybindingAction::ExportAll => self.handle_export_all_key(),
            KeybindingAction::ImportBoard => self.handle_import_board_key(),
            KeybindingAction::OrderCards => self.handle_order_cards_key(),
            KeybindingAction::ToggleSortOrder => self.handle_toggle_sort_order_key(),
            KeybindingAction::ToggleFilter => self.handle_toggle_sprint_filter(),
            KeybindingAction::ToggleHideAssigned => self.handle_open_filter_dialog(),
            KeybindingAction::ToggleArchivedView => self.handle_toggle_archived_cards_view(),
            KeybindingAction::ToggleTaskListView => self.handle_toggle_task_list_view(),
            KeybindingAction::ToggleCardSelection => self.handle_card_selection_toggle(),
            KeybindingAction::ClearCardSelection => self.handle_clear_card_selection(),
            KeybindingAction::SelectAllCards => self.handle_select_all_cards_in_view(),
            KeybindingAction::SetSelectedCardsPriority => self.handle_set_selected_cards_priority(),
            KeybindingAction::Search => {
                if self.focus.active == Focus::Cards {
                    self.filter.search.activate();
                    self.mode = AppMode::Search;
                }
            }
            KeybindingAction::ShowHelp => {}
            KeybindingAction::Escape => self.handle_escape_key(),
            KeybindingAction::FocusPanel(panel) => self.handle_column_or_focus_switch(*panel),
            KeybindingAction::JumpToTop => self.handle_jump_to_top(),
            KeybindingAction::JumpToBottom => self.handle_jump_to_bottom(),
            KeybindingAction::JumpHalfViewportUp => self.handle_jump_half_viewport_up(),
            KeybindingAction::JumpHalfViewportDown => self.handle_jump_half_viewport_down(),
            KeybindingAction::ManageParents => self.handle_manage_parents(),
            KeybindingAction::ManageChildren => self.handle_manage_children(),
            KeybindingAction::CarryOver => {}
            KeybindingAction::Undo => {
                if let Err(e) = self.undo() {
                    self.set_error(format!("Undo failed: {}", e));
                }
            }
            KeybindingAction::Redo => {
                if let Err(e) = self.redo() {
                    self.set_error(format!("Redo failed: {}", e));
                }
            }
            KeybindingAction::OpenSettings => self.handle_open_settings(),
            KeybindingAction::ExportBoards => {}
        }
    }

    fn handle_key_event(
        &mut self,
        key: crossterm::event::KeyEvent,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
    ) -> bool {
        use crossterm::event::KeyCode;
        let mut should_restart_events = false;

        // Clear banner on any key press
        if self.ui_state.banner.is_some() {
            self.clear_banner();
            return false;
        }

        let is_input_mode = matches!(
            self.mode,
            AppMode::Search
                | AppMode::Dialog(DialogMode::CreateBoard)
                | AppMode::Dialog(DialogMode::CreateCard)
                | AppMode::Dialog(DialogMode::CreateSprint)
                | AppMode::Dialog(DialogMode::RenameBoard)
                | AppMode::Dialog(DialogMode::ExportBoard)
                | AppMode::Dialog(DialogMode::ExportAll)
                | AppMode::Dialog(DialogMode::SetCardPoints)
                | AppMode::Dialog(DialogMode::SetBranchPrefix)
                | AppMode::Dialog(DialogMode::CreateColumn)
                | AppMode::Dialog(DialogMode::RenameColumn)
                | AppMode::Dialog(DialogMode::SetSprintPrefix)
                | AppMode::Dialog(DialogMode::SetSprintCardPrefix)
        );

        if matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) && !is_input_mode {
            self.handle_quit_key();
            return false;
        }

        if matches!(key.code, KeyCode::F(12)) && !matches!(self.mode, AppMode::ErrorLog) {
            self.open_error_log();
            return false;
        }

        if matches!(key.code, KeyCode::Char('?'))
            && !is_input_mode
            && !matches!(self.mode, AppMode::Help(_))
        {
            let previous_mode = self.mode.clone();
            let provider = crate::keybindings::KeybindingRegistry::get_provider(self);
            let context = provider.get_context();
            self.ui_state
                .help_list
                .update_item_count(context.bindings.len());
            self.ui_state.help_list.set_scroll_offset(0);
            self.mode = AppMode::Help(Box::new(previous_mode));
            return false;
        }

        // Handle Ctrl+a for select all cards
        if matches!(self.mode, AppMode::Normal)
            && key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('a'))
        {
            self.pending_key = None;
            self.handle_select_all_cards_in_view();
            return false;
        }

        match self.mode {
            AppMode::Normal => match key.code {
                KeyCode::Char('/') => {
                    self.pending_key = None;
                    if self.focus.active == Focus::Cards {
                        self.filter.search.activate();
                        self.mode = AppMode::Search;
                    }
                }
                KeyCode::Char('g') => {
                    if self.pending_key == Some('g') {
                        self.pending_key = None;
                        self.handle_jump_to_top();
                    } else {
                        self.pending_key = Some('g');
                    }
                }
                KeyCode::Char('G') => {
                    self.pending_key = None;
                    self.handle_jump_to_bottom();
                }
                KeyCode::Char('{') => {
                    self.pending_key = None;
                    self.handle_jump_half_viewport_up();
                }
                KeyCode::Char('}') => {
                    self.pending_key = None;
                    self.handle_jump_half_viewport_down();
                }
                KeyCode::Char('n') => {
                    self.pending_key = None;
                    match self.focus.active {
                        Focus::Boards => self.handle_create_board_key(),
                        Focus::Cards => self.handle_create_card_key(),
                    }
                }
                KeyCode::Char('r') => {
                    self.pending_key = None;
                    self.handle_rename_board_key();
                }
                KeyCode::Char('e') => {
                    self.pending_key = None;
                    match self.focus.active {
                        Focus::Boards => self.handle_edit_board_key(),
                        Focus::Cards => {
                            should_restart_events =
                                self.handle_edit_card_key(terminal, event_handler);
                        }
                    }
                }
                KeyCode::Char('x') => {
                    self.pending_key = None;
                    self.handle_export_board_key();
                }
                KeyCode::Char('X') => {
                    self.pending_key = None;
                    self.handle_export_all_key();
                }
                KeyCode::Char('d') => {
                    self.pending_key = None;
                    self.handle_archive_card();
                }
                KeyCode::Char('D') => {
                    self.pending_key = None;
                    self.handle_toggle_archived_cards_view();
                }
                KeyCode::Char('i') => {
                    self.pending_key = None;
                    self.handle_import_board_key();
                }
                KeyCode::Char('a') => {
                    self.pending_key = None;
                    self.handle_assign_to_sprint_key();
                }
                KeyCode::Char('c') => {
                    self.pending_key = None;
                    self.handle_toggle_card_completion();
                }
                KeyCode::Char('s') => {
                    self.pending_key = None;
                    if self.focus.active == Focus::Cards {
                        self.handle_manage_children_from_list();
                    }
                }
                KeyCode::Char('o') => {
                    self.pending_key = None;
                    self.handle_order_cards_key();
                }
                KeyCode::Char('O') => {
                    self.pending_key = None;
                    self.handle_toggle_sort_order_key();
                }
                KeyCode::Char('T') => {
                    self.pending_key = None;
                    self.handle_open_filter_dialog();
                }
                KeyCode::Char('t') => {
                    self.pending_key = None;
                    self.handle_toggle_sprint_filter();
                }
                KeyCode::Char('v') => {
                    self.pending_key = None;
                    self.handle_card_selection_toggle();
                }
                KeyCode::Char('V') => {
                    self.pending_key = None;
                    self.handle_toggle_task_list_view();
                }
                KeyCode::Char('P') => {
                    self.pending_key = None;
                    self.handle_set_selected_cards_priority();
                }
                KeyCode::Char('H') => {
                    self.pending_key = None;
                    self.handle_move_card_left();
                }
                KeyCode::Char('L') => {
                    self.pending_key = None;
                    self.handle_move_card_right();
                }
                KeyCode::Char('h') => {
                    self.pending_key = None;
                    self.handle_kanban_column_left();
                }
                KeyCode::Char('l') => {
                    self.pending_key = None;
                    self.handle_kanban_column_right();
                }
                KeyCode::Char('1') => {
                    self.pending_key = None;
                    self.handle_column_or_focus_switch(0);
                }
                KeyCode::Char('2') => {
                    self.pending_key = None;
                    self.handle_column_or_focus_switch(1);
                }
                KeyCode::Char('3') => {
                    self.pending_key = None;
                    self.handle_column_or_focus_switch(2);
                }
                KeyCode::Char('4') => {
                    self.pending_key = None;
                    self.handle_column_or_focus_switch(3);
                }
                KeyCode::Char('5') => {
                    self.pending_key = None;
                    self.handle_column_or_focus_switch(4);
                }
                KeyCode::Char('6') => {
                    self.pending_key = None;
                    self.handle_column_or_focus_switch(5);
                }
                KeyCode::Char('7') => {
                    self.pending_key = None;
                    self.handle_column_or_focus_switch(6);
                }
                KeyCode::Char('8') => {
                    self.pending_key = None;
                    self.handle_column_or_focus_switch(7);
                }
                KeyCode::Char('9') => {
                    self.pending_key = None;
                    self.handle_column_or_focus_switch(8);
                }
                KeyCode::Esc => {
                    self.pending_key = None;
                    self.handle_escape_key();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.pending_key = None;
                    self.handle_navigation_down();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.pending_key = None;
                    self.handle_navigation_up();
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.pending_key = None;
                    self.handle_selection_activate();
                }
                KeyCode::Char('u') => {
                    self.pending_key = None;
                    if let Err(e) = self.undo() {
                        self.set_error(format!("Undo failed: {}", e));
                    }
                }
                KeyCode::Char('U') => {
                    self.pending_key = None;
                    if let Err(e) = self.redo() {
                        self.set_error(format!("Redo failed: {}", e));
                    }
                }
                KeyCode::Char('S') => {
                    self.pending_key = None;
                    self.handle_open_settings();
                }
                _ => {
                    self.pending_key = None;
                }
            },
            AppMode::CardDetail => {
                should_restart_events =
                    self.handle_card_detail_key(key.code, terminal, event_handler);
            }
            AppMode::BoardDetail => {
                should_restart_events =
                    self.handle_board_detail_key(key.code, terminal, event_handler);
            }
            AppMode::SprintDetail => self.handle_sprint_detail_key(key.code),
            AppMode::Search => self.handle_search_mode(key.code),
            AppMode::ArchivedCardsView => self.handle_archived_cards_view_mode(key.code),
            AppMode::Settings => {
                should_restart_events = self.handle_settings_key(key.code, terminal, event_handler);
            }
            AppMode::Help(_) => self.handle_help_mode(key.code),
            AppMode::ErrorLog => self.handle_error_log_mode(key.code),
            AppMode::Dialog(ref dialog) => match dialog {
                DialogMode::CreateBoard => self.handle_create_board_dialog(key.code),
                DialogMode::CreateCard => self.handle_create_card_dialog(key.code),
                DialogMode::CreateSprint => self.handle_create_sprint_dialog(key.code),
                DialogMode::RenameBoard => self.handle_rename_board_dialog(key.code),
                DialogMode::ExportBoard => self.handle_export_board_dialog(key.code),
                DialogMode::ExportAll => self.handle_export_all_dialog(key.code),
                DialogMode::ImportBoard => self.handle_import_board_popup(key.code),
                DialogMode::SetCardPoints => {
                    should_restart_events = self.handle_set_card_points_dialog(key.code);
                }
                DialogMode::SetCardPriority => self.handle_set_card_priority_popup(key.code),
                DialogMode::SetMultipleCardsPriority => {
                    self.handle_set_multiple_cards_priority_popup(key.code)
                }
                DialogMode::SetBranchPrefix => self.handle_set_branch_prefix_dialog(key.code),
                DialogMode::SetSprintPrefix => self.handle_set_sprint_prefix_dialog(key.code),
                DialogMode::SetSprintCardPrefix => {
                    self.handle_set_sprint_card_prefix_dialog(key.code)
                }
                DialogMode::OrderCards => {
                    should_restart_events = self.handle_order_cards_popup(key.code);
                }
                DialogMode::AssignCardToSprint => self.handle_assign_card_to_sprint_popup(key.code),
                DialogMode::AssignMultipleCardsToSprint => {
                    self.handle_assign_multiple_cards_to_sprint_popup(key.code)
                }
                DialogMode::CreateColumn => self.handle_create_column_dialog(key.code),
                DialogMode::RenameColumn => self.handle_rename_column_dialog(key.code),
                DialogMode::DeleteColumnConfirm => {
                    self.handle_delete_column_confirm_popup(key.code)
                }
                DialogMode::SelectTaskListView => self.handle_select_task_list_view_popup(key.code),
                DialogMode::ConfirmSprintPrefixCollision => {
                    self.handle_confirm_sprint_prefix_collision_popup(key.code)
                }
                DialogMode::FilterOptions => self.handle_filter_options_popup(key.code),
                DialogMode::ConflictResolution => self.handle_conflict_resolution_popup(key.code),
                DialogMode::ExternalChangeDetected => {
                    self.handle_external_change_detected_popup(key.code)
                }
                DialogMode::ManageParents => self.handle_manage_parents_popup(key.code),
                DialogMode::ManageChildren => self.handle_manage_children_popup(key.code),
                DialogMode::CarryOverSprint => self.handle_carry_over_sprint_popup(key.code),
                DialogMode::ExportBoards => self.handle_export_boards_dialog(key.code),
            },
        }
        should_restart_events
    }

    fn handle_search_mode(&mut self, key_code: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        match key_code {
            KeyCode::Char(c) => {
                self.filter.search.input.insert_char(c);
                self.needs_redraw = true;
            }
            KeyCode::Backspace => {
                self.filter.search.input.backspace();
                self.needs_redraw = true;
            }
            KeyCode::Enter => {
                self.mode = AppMode::Normal;
                self.needs_redraw = true;
            }
            KeyCode::Esc => {
                self.filter.search.deactivate();
                self.mode = AppMode::Normal;
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    fn handle_archived_cards_view_mode(&mut self, key_code: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        if self.focus.active != Focus::Cards {
            self.focus.active = Focus::Cards;
        }

        match key_code {
            KeyCode::Char('r') => self.handle_restore_card(),
            KeyCode::Char('x') => self.handle_delete_card_permanent(),
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.handle_toggle_archived_cards_view();
            }
            KeyCode::Char('v') => self.handle_card_selection_toggle(),
            KeyCode::Char('V') => self.handle_toggle_task_list_view(),
            KeyCode::Char('h') => self.handle_kanban_column_left(),
            KeyCode::Char('l') => self.handle_kanban_column_right(),
            KeyCode::Char('j') | KeyCode::Down => self.handle_navigation_down(),
            KeyCode::Char('k') | KeyCode::Up => self.handle_navigation_up(),
            _ => {}
        }
    }

    /// Scrolls the help list so the selected item is visible.
    ///
    /// Two passes are needed because `get_adjusted_viewport_height` reserves rows
    /// for scroll indicators, and an indicator can appear or disappear after the
    /// first `ensure_selected_visible` call — changing the available height. A
    /// second pass with the updated height corrects any residual mis-alignment.
    fn scroll_help_into_view(&mut self) {
        let raw = crate::ui::help_popup_viewport_height(self.view.last_frame_area);
        if raw == 0 {
            return;
        }
        let h0 = self.ui_state.help_list.get_adjusted_viewport_height(raw);
        self.ui_state.help_list.ensure_selected_visible(h0);
        let h1 = self.ui_state.help_list.get_adjusted_viewport_height(raw);
        if h1 != h0 {
            self.ui_state.help_list.ensure_selected_visible(h1);
        }
    }

    fn handle_help_mode(&mut self, key_code: crossterm::event::KeyCode) {
        use crate::keybindings::KeybindingRegistry;
        use crossterm::event::KeyCode;

        match key_code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.ui_state.help_pending_action = None;
                self.ui_state.help_list.navigate_down();
                self.scroll_help_into_view();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.ui_state.help_pending_action = None;
                self.ui_state.help_list.navigate_up();
                self.scroll_help_into_view();
            }
            KeyCode::Char('h') | KeyCode::Char('l') => {
                self.ui_state.help_pending_action = None;
            }
            KeyCode::Enter => {
                self.ui_state.help_pending_action = None;
                if let Some(index) = self.ui_state.help_list.get_selected_index() {
                    let provider = KeybindingRegistry::get_provider(self);
                    let context = provider.get_context();

                    if let Some(binding) = context.bindings.get(index) {
                        if let AppMode::Help(previous_mode) = &self.mode {
                            self.mode = (**previous_mode).clone();
                        } else {
                            self.mode = AppMode::Normal;
                        }
                        self.ui_state.help_list.reset();

                        self.execute_action(&binding.action);
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('?') => {
                self.ui_state.help_pending_action = None;
                if let AppMode::Help(previous_mode) = &self.mode {
                    self.mode = (**previous_mode).clone();
                } else {
                    self.mode = AppMode::Normal;
                }
                self.ui_state.help_list.reset();
            }
            _ => {
                let provider = KeybindingRegistry::get_provider(self);
                let context = provider.get_context();

                if let Some((index, binding)) = context
                    .bindings
                    .iter()
                    .enumerate()
                    .find(|(_, b)| Self::keycode_matches_binding_key(&key_code, &b.key))
                {
                    self.ui_state.help_list.jump_to(index);
                    self.scroll_help_into_view();
                    self.ui_state.help_pending_action = Some((Instant::now(), binding.action));
                }
            }
        }
    }

    fn handle_animation_tick(&mut self) {
        let now = Instant::now();
        let mut completed_animations = Vec::new();

        for (&card_id, animation) in &self.animation.animating {
            let elapsed = now.duration_since(animation.start_time).as_millis();
            if elapsed >= animation::ANIMATION_DURATION_MS {
                completed_animations.push((card_id, animation.animation_type));
            }
        }

        // Group animations by type for batch processing
        let mut archive_cards = Vec::new();
        let mut restore_cards = Vec::new();
        let mut delete_cards = Vec::new();
        let mut last_archive_column = None;
        let mut last_archive_position = None;

        for (card_id, animation_type) in completed_animations {
            self.animation.animating.remove(&card_id);
            match animation_type {
                AnimationType::Archiving => {
                    let cards = self.ctx.cards();
                    if let Some(card_pos) = cards.iter().position(|c| c.id == card_id) {
                        let card = &cards[card_pos];
                        last_archive_column = Some(card.column_id);
                        last_archive_position = Some(card.position);
                        archive_cards.push(card_id);
                    }
                }
                AnimationType::Restoring => {
                    restore_cards.push(card_id);
                }
                AnimationType::Deleting => {
                    delete_cards.push(card_id);
                }
            }
        }

        let had_archives = !archive_cards.is_empty();
        let had_deletes = !delete_cards.is_empty();

        // Execute batch archive commands
        if had_archives {
            if let Err(e) =
                self.execute_commands_batch(vec![kanban_domain::commands::Command::Card(
                    kanban_domain::commands::CardCommand::Archive(
                        kanban_domain::commands::ArchiveCards { ids: archive_cards },
                    ),
                )])
            {
                tracing::error!("Failed to archive cards: {}", e);
            } else if let (Some(column_id), Some(position)) =
                (last_archive_column, last_archive_position)
            {
                self.compact_column_positions(column_id);
                self.select_card_after_deletion(column_id, position);
            }
        }

        // Execute batch delete commands
        if had_deletes {
            let mut delete_commands: Vec<kanban_domain::commands::Command> = Vec::new();
            for card_id in delete_cards {
                let cmd = kanban_domain::commands::Command::Card(
                    kanban_domain::commands::CardCommand::Delete(
                        kanban_domain::commands::DeleteCard { card_id },
                    ),
                );
                delete_commands.push(cmd);
            }
            if let Err(e) = self.execute_commands_batch(delete_commands) {
                tracing::error!("Failed to delete cards: {}", e);
            }
        }

        // Handle restore animations individually (less common)
        for card_id in restore_cards {
            self.complete_restore_animation(card_id);
        }
    }

    fn complete_restore_animation(&mut self, card_id: uuid::Uuid) {
        if let Some(archived_card) = self
            .ctx
            .archived_cards()
            .iter()
            .find(|dc| dc.card.id == card_id)
            .cloned()
        {
            self.restore_card(archived_card);
        }
    }

    pub fn get_board_card_count(&self, board_id: uuid::Uuid) -> usize {
        let columns = self.ctx.columns();
        let board_filter = BoardFilter::new(board_id, &columns);
        let sprint_filter = if !self.filter.active_sprint_filters.is_empty() {
            Some(SprintFilter::in_sprints(
                self.filter.active_sprint_filters.iter().copied(),
            ))
        } else {
            None
        };

        let cards = self.ctx.cards();
        cards
            .iter()
            .filter(|c| {
                if !board_filter.matches(c) {
                    return false;
                }
                if let Some(ref sf) = sprint_filter {
                    if !sf.matches(c) {
                        return false;
                    }
                }
                if self.filter.hide_assigned_cards && !UnassignedOnlyFilter.matches(c) {
                    return false;
                }
                true
            })
            .count()
    }

    pub fn get_sorted_board_cards(&self, board_id: uuid::Uuid) -> Vec<Card> {
        let boards = self.ctx.boards();
        let board = boards.iter().find(|b| b.id == board_id).unwrap();
        let columns = self.ctx.columns();
        let board_filter = BoardFilter::new(board_id, &columns);
        let sprint_filter = if !self.filter.active_sprint_filters.is_empty() {
            Some(SprintFilter::in_sprints(
                self.filter.active_sprint_filters.iter().copied(),
            ))
        } else {
            None
        };

        let all_cards = self.ctx.cards();
        let mut cards: Vec<&Card> = all_cards
            .iter()
            .filter(|c| {
                if !board_filter.matches(c) {
                    return false;
                }
                if let Some(ref sf) = sprint_filter {
                    if !sf.matches(c) {
                        return false;
                    }
                }
                if self.filter.hide_assigned_cards && !UnassignedOnlyFilter.matches(c) {
                    return false;
                }
                true
            })
            .collect();

        let sorter = get_sorter_for_field(board.task_sort_field);
        let ordered_sorter = OrderedSorter::new(sorter, board.task_sort_order);
        ordered_sorter.sort_by(&mut cards);

        cards.into_iter().cloned().collect()
    }

    pub fn get_selected_card_in_context(&self) -> Option<Card> {
        if let Some(task_list) = self.view.strategy.get_active_task_list() {
            if let Some(card_id) = task_list.get_selected_card_id() {
                return self.get_card_by_id(card_id);
            }
        }
        None
    }

    pub fn get_selected_card_id(&self) -> Option<uuid::Uuid> {
        self.view
            .strategy
            .get_active_task_list()
            .and_then(|list| list.get_selected_card_id())
    }

    pub fn select_card_by_id(&mut self, card_id: uuid::Uuid) {
        if let Some(task_list) = self.view.strategy.get_active_task_list_mut() {
            task_list.select_card(card_id);
        }
    }

    pub fn get_card_by_id(&self, card_id: uuid::Uuid) -> Option<Card> {
        self.render_data.cards_by_id.get(&card_id).cloned()
    }

    pub fn get_card_for_detail_view(&self) -> Option<Card> {
        self.selection
            .active_card_id
            .and_then(|id| self.render_data.cards_by_id.get(&id).cloned())
    }

    pub fn populate_sprint_task_lists(&mut self, sprint_id: uuid::Uuid) {
        let cards = self.ctx.cards();
        let (uncompleted_ids, completed_ids) = partition_sprint_cards(sprint_id, &cards);

        self.sprint_view
            .uncompleted_cards
            .update_cards(uncompleted_ids);
        self.sprint_view.completed_cards.update_cards(completed_ids);

        self.sprint_view
            .uncompleted_component
            .update_cards(self.sprint_view.uncompleted_cards.cards.clone());
        self.sprint_view
            .completed_component
            .update_cards(self.sprint_view.completed_cards.cards.clone());

        // Default to uncompleted panel
        self.sprint_view.panel = SprintTaskPanel::Uncompleted;
    }

    pub fn apply_sort_to_sprint_lists(&mut self, sort_field: SortField, sort_order: SortOrder) {
        let cards = self.ctx.cards();
        let sorted_uncompleted_ids = sort_card_ids(
            &self.sprint_view.uncompleted_cards.cards,
            &cards,
            sort_field,
            sort_order,
        );
        let sorted_completed_ids = sort_card_ids(
            &self.sprint_view.completed_cards.cards,
            &cards,
            sort_field,
            sort_order,
        );

        self.sprint_view
            .uncompleted_cards
            .update_cards(sorted_uncompleted_ids);
        self.sprint_view
            .completed_cards
            .update_cards(sorted_completed_ids);

        self.sprint_view
            .uncompleted_component
            .update_cards(self.sprint_view.uncompleted_cards.cards.clone());
        self.sprint_view
            .completed_component
            .update_cards(self.sprint_view.completed_cards.cards.clone());
    }

    /// Execute multiple commands as a batch with a single pause/resume cycle
    /// This is the preferred method as it prevents race conditions where rapid successive saves
    /// detect previous writes as external. For single commands, this still works efficiently.
    pub fn execute_command(
        &mut self,
        command: kanban_domain::commands::Command,
    ) -> KanbanResult<()> {
        self.execute_commands_batch(vec![command])
    }

    /// Execute multiple commands as a batch with a single pause/resume cycle
    /// This prevents race conditions where rapid successive saves detect previous writes as external
    pub fn execute_commands_batch(
        &mut self,
        commands: Vec<kanban_domain::commands::Command>,
    ) -> KanbanResult<()> {
        self.ctx.execute_commands_batch(commands)?;
        Ok(())
    }

    pub fn prepare_frame(&mut self) {
        let cards_for_display: Vec<Card> = if self.mode == AppMode::ArchivedCardsView {
            self.ctx
                .archived_cards()
                .iter()
                .map(|dc| dc.card.clone())
                .collect()
        } else {
            self.ctx.cards()
        };
        self.render_data.cards_by_id =
            cards_for_display.into_iter().map(|c| (c.id, c)).collect();
        self.render_data.sprints = self.ctx.sprints();
        self.render_data.columns = self.ctx.columns();
        self.render_data.boards = self.ctx.boards();
        self.render_data.graph = self.ctx.graph();


        let board_idx = self
            .selection
            .active_board_index
            .or(self.selection.board.get());
        if let Some(idx) = board_idx {
            if let Some(board) = self.render_data.boards.get(idx) {
                let search_query = if self.filter.search.is_active {
                    Some(self.filter.search.query())
                } else {
                    None
                };
                let cards: Vec<Card> = self.render_data.cards_by_id.values().cloned().collect();
                let ctx = ViewRefreshContext {
                    board,
                    all_cards: &cards,
                    all_columns: &self.render_data.columns,
                    all_sprints: &self.render_data.sprints,
                    active_sprint_filters: self.filter.active_sprint_filters.clone(),
                    hide_assigned_cards: self.filter.hide_assigned_cards,
                    search_query,
                };
                self.view.strategy.refresh_task_lists(&ctx);
            }
        }
        self.sync_card_list_component();
    }

    /// Undo the last action
    pub fn undo(&mut self) -> KanbanResult<()> {
        if self.ctx.undo()? {
            self.needs_redraw = true;
        } else {
            self.set_error("Nothing to undo".to_string());
        }
        Ok(())
    }

    /// Redo the last undone action
    pub fn redo(&mut self) -> KanbanResult<()> {
        if self.ctx.redo()? {
            self.needs_redraw = true;
        } else {
            self.set_error("Nothing to redo".to_string());
        }
        Ok(())
    }

    pub fn sync_card_list_component(&mut self) {
        if let Some(active_list) = self.view.strategy.get_active_task_list() {
            self.view
                .card_list_component
                .update_cards(active_list.cards.clone());
        }
    }

    pub fn switch_view_strategy(&mut self, task_list_view: kanban_domain::TaskListView) {
        let new_strategy: Box<dyn ViewStrategy> = match task_list_view {
            kanban_domain::TaskListView::Flat => Box::new(UnifiedViewStrategy::flat()),
            kanban_domain::TaskListView::GroupedByColumn => {
                Box::new(UnifiedViewStrategy::grouped())
            }
            kanban_domain::TaskListView::ColumnView => Box::new(UnifiedViewStrategy::kanban()),
        };

        self.view.strategy = new_strategy;
        self.needs_redraw = true;
    }

    pub fn export_board_with_filename(&self) -> io::Result<()> {
        if let Some(board_idx) = self.selection.board.get() {
            let boards = self.ctx.boards();
            if let Some(board) = boards.get(board_idx) {
                let columns = self.ctx.columns();
                let cards = self.ctx.cards();
                let archived_cards = self.ctx.archived_cards();
                let sprints = self.ctx.sprints();
                let board_export =
                    BoardExporter::export_board(board, &columns, &cards, &archived_cards, &sprints);

                let export = AllBoardsExport {
                    boards: vec![board_export],
                };

                BoardExporter::export_to_file(&export, self.input.as_str())?;
            }
        }
        Ok(())
    }

    pub fn export_all_boards_with_filename(&self) -> io::Result<()> {
        let boards = self.ctx.boards();
        let columns = self.ctx.columns();
        let cards = self.ctx.cards();
        let archived_cards = self.ctx.archived_cards();
        let sprints = self.ctx.sprints();
        let export =
            BoardExporter::export_all_boards(&boards, &columns, &cards, &archived_cards, &sprints);
        BoardExporter::export_to_file(&export, self.input.as_str())?;
        Ok(())
    }

    pub fn auto_save(&self) -> io::Result<()> {
        if let Some(ref filename) = self.persistence.save_file {
            let boards = self.ctx.boards();
            let columns = self.ctx.columns();
            let cards = self.ctx.cards();
            let archived_cards = self.ctx.archived_cards();
            let sprints = self.ctx.sprints();
            let export = BoardExporter::export_all_boards(
                &boards,
                &columns,
                &cards,
                &archived_cards,
                &sprints,
            );
            BoardExporter::export_to_file(&export, filename)?;
        }
        Ok(())
    }

    fn check_ended_sprints(&self) {
        let sprints = self.ctx.sprints();
        let ended_sprints: Vec<_> = sprints.iter().filter(|s| s.is_ended()).collect();

        if !ended_sprints.is_empty() {
            tracing::warn!(
                "Found {} ended sprint(s) that need attention:",
                ended_sprints.len()
            );
            let boards = self.ctx.boards();
            for sprint in &ended_sprints {
                if let Some(board) = boards.iter().find(|b| b.id == sprint.board_id) {
                    tracing::warn!(
                        "  - {} (ended: {})",
                        sprint.formatted_name(board, "sprint"),
                        sprint
                            .end_date
                            .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    );
                }
            }
        }
    }

    pub fn edit_board_field(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
        field: BoardField,
    ) -> io::Result<()> {
        if let Some(board_idx) = self.selection.board.get() {
            let boards = self.ctx.boards();
            if let Some(board) = boards.get(board_idx) {
                let temp_dir = std::env::temp_dir();
                let (temp_file, current_content) = match field {
                    BoardField::Name => {
                        let temp_file = temp_dir.join(format!("kanban-board-{}-name.md", board.id));
                        (temp_file, board.name.clone())
                    }
                    BoardField::Description => {
                        let temp_file =
                            temp_dir.join(format!("kanban-board-{}-description.md", board.id));
                        let content = board.description.as_deref().unwrap_or("").to_string();
                        (temp_file, content)
                    }
                };

                let board_id = board.id;
                if let Some(new_content) =
                    edit_in_external_editor(terminal, event_handler, temp_file, &current_content)?
                {
                    let updates = match field {
                        BoardField::Name => {
                            if new_content.trim().is_empty() {
                                None
                            } else {
                                Some(kanban_domain::BoardUpdate {
                                    name: Some(new_content.trim().to_string()),
                                    ..Default::default()
                                })
                            }
                        }
                        BoardField::Description => {
                            let desc = if new_content.trim().is_empty() {
                                kanban_domain::FieldUpdate::Clear
                            } else {
                                kanban_domain::FieldUpdate::Set(new_content)
                            };
                            Some(kanban_domain::BoardUpdate {
                                description: desc,
                                ..Default::default()
                            })
                        }
                    };
                    if let Some(updates) = updates {
                        let cmd = kanban_domain::commands::Command::Board(
                            kanban_domain::commands::BoardCommand::Update(
                                kanban_domain::commands::UpdateBoard { board_id, updates },
                            ),
                        );
                        if let Err(e) = self.execute_command(cmd) {
                            tracing::error!("Failed to update board: {}", e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn edit_card_field(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
        field: CardField,
    ) -> io::Result<()> {
        if let Some(card_idx) = self.selection.active_card_index {
            let cards = self.ctx.cards();
            if let Some(card) = cards.get(card_idx) {
                let temp_dir = std::env::temp_dir();
                let (temp_file, current_content) = match field {
                    CardField::Title => {
                        let temp_file = temp_dir.join(format!("kanban-card-{}-title.md", card.id));
                        (temp_file, card.title.clone())
                    }
                    CardField::Description => {
                        let temp_file =
                            temp_dir.join(format!("kanban-card-{}-description.md", card.id));
                        let content = card.description.as_deref().unwrap_or("").to_string();
                        (temp_file, content)
                    }
                };

                let card_id = card.id;
                if let Some(new_content) =
                    edit_in_external_editor(terminal, event_handler, temp_file, &current_content)?
                {
                    let updates = match field {
                        CardField::Title => {
                            if new_content.trim().is_empty() {
                                None
                            } else {
                                Some(kanban_domain::CardUpdate {
                                    title: Some(new_content.trim().to_string()),
                                    ..Default::default()
                                })
                            }
                        }
                        CardField::Description => {
                            let desc = if new_content.trim().is_empty() {
                                kanban_domain::FieldUpdate::Clear
                            } else {
                                kanban_domain::FieldUpdate::Set(new_content)
                            };
                            Some(kanban_domain::CardUpdate {
                                description: desc,
                                ..Default::default()
                            })
                        }
                    };
                    if let Some(updates) = updates {
                        let cmd = kanban_domain::commands::Command::Card(
                            kanban_domain::commands::CardCommand::Update(
                                kanban_domain::commands::UpdateCard { card_id, updates },
                            ),
                        );
                        if let Err(e) = self.execute_command(cmd) {
                            tracing::error!("Failed to update card: {}", e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn edit_entity_json_impl<T: Editable<E>, E>(
        entity: &mut E,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
        temp_file: std::path::PathBuf,
    ) -> io::Result<()> {
        Self::edit_entity_impl::<T, E>(
            entity,
            terminal,
            event_handler,
            temp_file,
            crate::edit_format::EditFormat::Json,
        )
    }

    pub fn edit_entity_impl<T: Editable<E>, E>(
        entity: &mut E,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
        temp_file: std::path::PathBuf,
        format: crate::edit_format::EditFormat,
    ) -> io::Result<()> {
        let dto = T::from_entity(entity);
        let current_content = format.serialize(&dto).unwrap_or_else(|_| "{}".to_string());

        if let Some(new_content) =
            edit_in_external_editor(terminal, event_handler, temp_file, &current_content)?
        {
            match format.deserialize::<T>(&new_content) {
                Ok(updated_dto) => {
                    updated_dto.apply_to(entity);
                    tracing::info!("Updated entity via {} editor", format);
                }
                Err(e) => {
                    tracing::error!("Failed to parse {}: {}", format, e);
                }
            }
        }

        Ok(())
    }

    #[doc(hidden)]
    pub async fn load_initial_state(&mut self) {
        if let Some(store) = self.ctx.store() {
            if store.exists().await {
                match store.load().await {
                    Ok((snapshot, _metadata)) => {
                        match serde_json::from_slice::<kanban_domain::Snapshot>(&snapshot.data) {
                            Ok(data) => {
                                if let Err(e) = data.apply_to_app(self) {
                                    tracing::error!("Failed to apply snapshot: {}", e);
                                } else {
                                    self.ctx.mark_clean();
                                    if let Err(e) = self.ctx.clear_history() {
                                        tracing::error!("Failed to clear history: {}", e);
                                    }
                                    tracing::info!("Loaded initial state from store");
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to deserialize store data: {}", e);
                                self.persistence.save_file = None;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to load from store: {}", e);
                        self.persistence.save_file = None;
                    }
                }
            }
        }
        self.migrate_sprint_logs();
        self.check_ended_sprints();
        if self.selection.board.get().is_none() && !self.ctx.boards().is_empty() {
            self.selection.board.set(Some(0));
        }
        self.prepare_frame();
    }

    pub async fn run(
        &mut self,
        save_rx: Option<tokio::sync::mpsc::Receiver<kanban_domain::Snapshot>>,
    ) -> KanbanResult<()> {
        self.load_initial_state().await;

        let mut terminal = setup_terminal()?;

        // Initialize file watching if a save file is configured
        // (Done before spawning save worker so worker can pause/resume it)
        if let Some(ref save_file) = self.persistence.save_file {
            use kanban_persistence::ChangeDetector;
            tracing::info!("Initializing file watcher for: {}", save_file);
            let watcher = kanban_persistence::FileWatcher::new();
            let rx = watcher.subscribe();
            self.persistence.file_change_rx = Some(rx);
            tracing::debug!("File change broadcast receiver subscribed");

            let path = std::path::PathBuf::from(save_file);
            let deferred_watch_path = if path.exists() {
                if let Err(e) = watcher.start_watching(path.clone()).await {
                    tracing::warn!(
                        "Failed to start file watching for {}: {}",
                        path.display(),
                        e
                    );
                } else {
                    tracing::info!("File watcher started for: {}", path.display());
                }
                None
            } else {
                tracing::debug!(
                    "File does not exist yet, deferring file watching until first save: {}",
                    path.display()
                );
                Some(path)
            };

            // Store the watcher to keep the background task alive
            self.persistence.file_watcher = Some(watcher.clone());
            // Also set it on the state manager (wrapped in Arc) so queue_snapshot can pause it
            let watcher_arc = std::sync::Arc::new(watcher);
            self.ctx.save_coordinator.set_file_watcher(watcher_arc);

            // Spawn async save worker if save channel is configured
            if let Some(rx) = save_rx {
                self.spawn_save_worker(rx, deferred_watch_path);
            } else {
                tracing::debug!("No save channel receiver - no saves will be processed");
            }
        } else if let Some(rx) = save_rx {
            self.spawn_save_worker(rx, None);
        } else {
            tracing::debug!("No save channel receiver - no saves will be processed");
        }

        while !self.should_quit {
            let mut events = EventHandler::new();

            loop {
                if self.needs_redraw {
                    self.prepare_frame();
                    terminal.draw(|frame| ui::render(self, frame))?;
                    self.needs_redraw = false;
                }

                tokio::select! {
                    Some(event) = events.next() => {
                        match event {
                            Event::Key(key) => {
                                self.needs_redraw = true;
                                let should_restart = self.handle_key_event(key, &mut terminal, &events);
                                if should_restart {
                                    break;
                                }
                                // Drain buffered events before next draw to
                                // prevent input lag when rendering is slow.
                                let mut saw_tick = false;
                                while let Some(queued) = events.try_next() {
                                    match queued {
                                        Event::Key(k) => {
                                            let should_restart = self.handle_key_event(k, &mut terminal, &events);
                                            if should_restart {
                                                break;
                                            }
                                        }
                                        Event::Tick => {
                                            saw_tick = true;
                                        }
                                    }
                                }
                                if saw_tick {
                                    self.handle_animation_tick();
                                    if let Some(ref banner) = self.ui_state.banner {
                                        if banner.is_expired(std::time::Duration::from_secs(3)) {
                                            self.clear_banner();
                                            self.needs_redraw = true;
                                        }
                                    }
                                }
                            }
                            Event::Tick => {
                                if !self.animation.animating.is_empty() {
                                    self.needs_redraw = true;
                                }
                                self.handle_animation_tick();

                                // Auto-open error log only on new ERROR entries (not WARN)
                                let error_count =
                                    self.with_error_log(|log| log.error_count);
                                if error_count > self.auto_open_seen_count
                                    && !matches!(self.mode, AppMode::ErrorLog)
                                {
                                    self.auto_open_seen_count = error_count;
                                    self.open_error_log();
                                    self.needs_redraw = true;
                                }

                                // Auto-clear banner after 3 seconds
                                if let Some(ref banner) = self.ui_state.banner {
                                    if banner.is_expired(std::time::Duration::from_secs(3)) {
                                        self.clear_banner();
                                        self.needs_redraw = true;
                                    }
                                }

                                // Handle pending conflict resolution actions
                                // Only consume pending_key if it matches expected conflict actions
                                // to avoid breaking multi-key sequences like 'gg'
                                match self.pending_key {
                                    Some('o') => {
                                        self.pending_key = None;
                                        self.needs_redraw = true;
                                        // Pause file watcher to avoid conflict detection for our own save
                                        if let Some(ref watcher) = self.persistence.file_watcher {
                                            watcher.pause();
                                        }
                                        self.ctx.clear_conflict();
                                        if let Err(e) = self.ctx.save().await {
                                            tracing::error!("Failed to force overwrite: {}", e);
                                        }
                                        // Resume file watcher after save completes
                                        if let Some(ref watcher) = self.persistence.file_watcher {
                                            watcher.resume();
                                        }
                                    }
                                    Some('t') => {
                                        self.pending_key = None;
                                        self.needs_redraw = true;
                                        // Reload from disk
                                        if let Some(store) = self.ctx.store() {
                                            match store.load().await {
                                                Ok((snapshot, _metadata)) => {
                                                    match serde_json::from_slice::<kanban_domain::Snapshot>(&snapshot.data) {
                                                        Ok(data) => {
                                                            if let Err(e) = data.apply_to_app(self) {
                                                                tracing::error!("Failed to apply snapshot: {}", e);
                                                            } else {
                                                                if let Err(e) = self.ctx.clear_history() {
                                                                    tracing::error!("Failed to clear history: {}", e);
                                                                }
                                                                self.ctx.clear_conflict();
                                                                self.needs_redraw = true;
                                                                tracing::info!("Reloaded state from disk");
                                                            }
                                                        }
                                                        Err(e) => {
                                                            tracing::error!("Failed to deserialize reloaded state: {}", e);
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to reload from disk: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    Some('r') => {
                                        self.pending_key = None;
                                        self.needs_redraw = true;
                                        self.auto_reload_from_external_change().await;
                                    }
                                    // Don't consume pending_key for other values (e.g., 'g' for gg sequence)
                                    _ => {}
                                }

                                // Check if help menu pending action should execute
                                if let Some((start_time, action)) = &self.ui_state.help_pending_action {
                                    if start_time.elapsed().as_millis() >= 100 {
                                        self.needs_redraw = true;
                                        if let AppMode::Help(previous_mode) = &self.mode {
                                            self.mode = (**previous_mode).clone();
                                        } else {
                                            self.mode = AppMode::Normal;
                                        }
                                        self.ui_state.help_list.reset();

                                        let action = *action;
                                        self.ui_state.help_pending_action = None;
                                        self.execute_action(&action);
                                    }
                                }

                            }
                        }
                    }
                    result = async {
                        if let MigrationState::Migrating { ref mut result_rx, .. } = self.migration_state {
                            result_rx.await.ok()
                        } else {
                            std::future::pending().await
                        }
                    } => {
                        self.needs_redraw = true;
                        let old_config = match std::mem::replace(&mut self.migration_state, MigrationState::Idle) {
                            MigrationState::Migrating { old_config, .. } => old_config,
                            MigrationState::Idle => unreachable!(),
                        };
                        if let Some(result) = result {
                            self.handle_migration_complete(old_config, result);
                        }
                    }
                    export_result = async {
                        if let Some(ref mut rx) = &mut self.export_result_rx {
                            rx.await.ok()
                        } else {
                            std::future::pending().await
                        }
                    } => {
                        self.needs_redraw = true;
                        self.export_result_rx = None;
                        if let Some(result) = export_result {
                            match result {
                                Ok(filename) => self.set_success(format!("Exported to {}", filename)),
                                Err(e) => self.set_error(e),
                            }
                        }
                    }
                    _ = async {
                        if let Some(ref mut rx) = &mut self.persistence.save_completion_rx {
                            rx.recv().await
                        } else {
                            std::future::pending().await
                        }
                    } => {
                        // Save operation completed - update dirty flag
                        tracing::debug!("Save completion signal received");
                        self.ctx.save_coordinator.save_completed();
                        // Reset force quit flag and dirty flag if all saves are now complete
                        if !self.ctx.save_coordinator.has_pending_saves() {
                            self.ctx.mark_clean();
                            self.quit_with_pending = false;
                        }
                    }
                    Some(_change_event) = async {
                        if let Some(ref mut rx) = &mut self.persistence.file_change_rx {
                            match rx.recv().await {
                                Ok(event) => {
                                    tracing::debug!(
                                        "File change event received at {}",
                                        event.detected_at
                                    );
                                    Some(event)
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                                    tracing::warn!(
                                        "File watcher events lagged: {} events dropped",
                                        count
                                    );
                                    None
                                }
                                Err(e) => {
                                    tracing::error!("File change receiver error: {}", e);
                                    None
                                }
                            }
                        } else {
                            std::future::pending().await
                        }
                    } => {
                        self.needs_redraw = true;
                        // Check if this is our own write by comparing instance IDs
                        if let Some(store) = self.ctx.store() {
                            match store.load().await {
                                Ok((_snapshot, metadata)) => {
                                    // Compare instance IDs
                                    if metadata.instance_id == store.instance_id() {
                                        tracing::debug!(
                                            "File change from own instance ({}), ignoring",
                                            metadata.instance_id
                                        );
                                        continue; // Skip reload entirely
                                    }
                                    // It's external - proceed with existing logic below
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to load metadata for instance check: {}", e);
                                    // Fall through to existing logic (safer default)
                                }
                            }
                        }

                        // External file change detected - handle smart reload
                        if !self.ctx.is_dirty() {
                            // No local changes, auto-reload silently
                            tracing::info!("External change detected, auto-reloading");
                            self.auto_reload_from_external_change().await;
                            tracing::info!("Auto-reloaded due to external file change");
                        } else if self.mode != AppMode::Dialog(DialogMode::ConflictResolution)
                            && self.mode != AppMode::Dialog(DialogMode::ExternalChangeDetected)
                        {
                            // Local changes exist, prompt user
                            tracing::warn!("External file change detected with local changes");
                            self.open_dialog(DialogMode::ExternalChangeDetected);
                        }
                    }
                }

                if self.should_quit {
                    break;
                }
            }

            if self.should_quit {
                break;
            }
        }

        // If a migration completed at the same instant as quit, apply it before tearing down.
        self.await_migration().await;

        // Graceful shutdown: ensure all queued saves complete before exit
        self.ctx.save_coordinator.close_save_channel(); // Close save_tx channel to signal worker to finish

        // Wait for save worker to finish processing all queued saves
        if let Some(handle) = self.persistence.save_worker_handle.take() {
            handle.await.ok();
            tracing::info!("Save worker finished, all saves complete");
        }

        restore_terminal(&mut terminal)?;
        Ok(())
    }

    pub fn import_board_from_file(&mut self, filename: &str) -> io::Result<()> {
        let content = std::fs::read_to_string(filename)?;

        let first_new_index = self.ctx.boards().len();

        // Try V2 format first (preserves graph)
        if let Some(snapshot) = BoardImporter::try_load_snapshot(&content) {
            let cmd = kanban_domain::commands::Command::Board(
                kanban_domain::commands::BoardCommand::Import(
                    kanban_domain::commands::ImportEntities {
                        boards: snapshot.boards,
                        columns: snapshot.columns,
                        cards: snapshot.cards,
                        archived_cards: snapshot.archived_cards,
                        sprints: snapshot.sprints,
                        graph: Some(snapshot.graph),
                    },
                ),
            );
            if let Err(e) = self.ctx.execute_command(cmd) {
                self.set_error(e.to_string());
                tracing::error!("Failed to import V2 board: {}", e);
                return Ok(());
            }

            self.selection.board.set(Some(first_new_index));
            self.switch_view_strategy(kanban_domain::TaskListView::GroupedByColumn);
            return Ok(());
        }

        // Fall back to V1 format (no graph)
        let import = BoardImporter::import_from_json(&content)?;
        let entities = BoardImporter::extract_entities(import);

        let cmd =
            kanban_domain::commands::Command::Board(kanban_domain::commands::BoardCommand::Import(
                kanban_domain::commands::ImportEntities {
                    boards: entities.boards,
                    columns: entities.columns,
                    cards: entities.cards,
                    archived_cards: entities.archived_cards,
                    sprints: entities.sprints,
                    graph: None,
                },
            ));
        if let Err(e) = self.ctx.execute_command(cmd) {
            self.set_error(e.to_string());
            tracing::error!("Failed to import V1 board: {}", e);
            return Ok(());
        }

        self.selection.board.set(Some(first_new_index));
        self.switch_view_strategy(kanban_domain::TaskListView::GroupedByColumn);

        Ok(())
    }

    async fn auto_reload_from_external_change(&mut self) {
        let Some(store) = self.ctx.store() else {
            return;
        };
        match store.load().await {
            Ok((snapshot, _metadata)) => {
                match serde_json::from_slice::<kanban_domain::Snapshot>(&snapshot.data) {
                    Ok(data) => {
                        if let Err(e) = data.apply_to_app(self) {
                            tracing::error!("Failed to apply snapshot: {}", e);
                        } else {
                            if let Err(e) = self.ctx.clear_history() {
                                tracing::error!("Failed to clear history: {}", e);
                            }
                            self.ctx.mark_clean();
                            self.ctx.clear_conflict();
                            self.needs_redraw = true;
                            tracing::info!("Auto-reloaded state from external file change");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to deserialize reloaded state: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to reload from disk: {}", e);
            }
        }
    }

    fn migrate_sprint_logs(&mut self) {
        let cmd = kanban_domain::commands::Command::Card(
            kanban_domain::commands::CardCommand::MigrateSprintLogs(
                kanban_domain::commands::MigrateSprintLogs,
            ),
        );
        if let Err(e) = self.ctx.execute_command(cmd) {
            tracing::error!("Failed to migrate sprint logs: {}", e);
        }
    }

    /// Generic handler for copying card outputs to clipboard
    fn copy_card_output<F>(&mut self, output_type: &str, get_output: F)
    where
        F: Fn(&Card, &Board, &[Sprint], &str) -> String,
    {
        if let Some(card_idx) = self.selection.active_card_index {
            if let Some(board_idx) = self.selection.active_board_index {
                let boards = self.ctx.boards();
                if let Some(board) = boards.get(board_idx) {
                    let cards = self.ctx.cards();
                    if let Some(card) = cards.get(card_idx) {
                        let sprints = self.ctx.sprints();
                        let output = get_output(
                            card,
                            board,
                            &sprints,
                            self.app_config.effective_default_card_prefix(),
                        );
                        if let Err(e) = clipboard::copy_to_clipboard(&output) {
                            self.set_error(format!("Failed to copy: {}", e));
                        } else {
                            self.set_success(format!("Copied {}", output_type));
                        }
                    }
                }
            }
        }
    }

    pub fn copy_branch_name(&mut self) {
        self.copy_card_output("branch name", |card, board, sprints, prefix| {
            card.branch_name(board, sprints, prefix)
        });
    }

    pub fn copy_git_checkout_command(&mut self) {
        self.copy_card_output("command", |card, board, sprints, prefix| {
            card.git_checkout_command(board, sprints, prefix)
        });
    }

    pub fn get_current_priority_selection_index(&self) -> usize {
        if let Some(card_idx) = self.selection.active_card_index {
            let cards = self.ctx.cards();
            if let Some(card) = cards.get(card_idx) {
                use kanban_domain::CardPriority;
                return match card.priority {
                    CardPriority::Low => 0,
                    CardPriority::Medium => 1,
                    CardPriority::High => 2,
                    CardPriority::Critical => 3,
                };
            }
        }
        0
    }

    pub fn get_current_sprint_selection_index(&self) -> usize {
        if let Some(card_idx) = self.selection.active_card_index {
            let cards = self.ctx.cards();
            if let Some(card) = cards.get(card_idx) {
                if let Some(card_sprint_id) = card.sprint_id {
                    if let Some(board_idx) = self.selection.active_board_index {
                        let boards = self.ctx.boards();
                        if let Some(board) = boards.get(board_idx) {
                            let sprints = self.ctx.sprints();
                            let board_sprints = Sprint::assignable(&sprints, board.id);
                            for (idx, sprint) in board_sprints.iter().enumerate() {
                                if sprint.id == card_sprint_id {
                                    return idx + 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        0
    }

    pub fn get_current_sort_field_selection_index(&self) -> usize {
        if let Some(sort_field) = self.filter.current_sort_field {
            return match sort_field {
                SortField::Points => 0,
                SortField::Priority => 1,
                SortField::CreatedAt => 2,
                SortField::UpdatedAt => 3,
                SortField::Status => 4,
                SortField::Position => 5,
                SortField::Default => 6,
            };
        }
        0
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

impl Default for App {
    fn default() -> Self {
        let (app, _rx) = Self::new(None).expect("App::new(None) should never fail");
        app
    }
}

impl App {
    #[doc(hidden)]
    pub fn test_default() -> Self {
        let (ctx, _save_rx, save_completion_rx) =
            crate::tui_context::TuiContext::new(None).expect("TuiContext::new(None) failed");
        Self {
            store_manager: Arc::new(default_store_manager()),
            should_quit: false,
            quit_with_pending: false,
            quit_with_migration: false,
            mode: AppMode::Normal,
            mode_stack: Vec::new(),
            input: InputState::new(),
            ctx,
            app_config: kanban_core::AppConfig::default(),
            selection: SelectionHub::default(),
            animation: AnimationState::default(),
            filter: FilterState::default(),
            dialog_input: DialogInputState::default(),
            focus: FocusState::default(),
            persistence: PersistenceState::new(None, save_completion_rx),
            multi_select: MultiSelectState::default(),
            ui_state: UiState::default(),
            sprint_view: SprintViewState::default(),
            view: ViewState::default(),
            render_data: RenderData::default(),
            relationship: RelationshipState::default(),
            pending_key: None,
            has_data_file: true,
            cli_file_provided: false,
            cli_file_override: false,
            config_storage_backend: "json".into(),
            config_storage_location: "kanban.json".into(),
            original_storage_backend: None,
            original_storage_location: None,
            export_dialog: None,
            migration_state: MigrationState::Idle,
            export_result_rx: None,
            needs_redraw: true,
            error_log: Arc::new(Mutex::new(crate::error_log::ErrorLogState::default())),
            auto_open_seen_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn test_scroll_help_into_view_scrolls_deep_item() {
        let mut app = App::test_default();
        app.view.last_frame_area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 50,
        };
        app.ui_state.help_list.update_item_count(50);
        app.ui_state.help_list.jump_to(49);
        app.scroll_help_into_view();
        assert!(
            app.ui_state.help_list.get_scroll_offset() > 0,
            "help list should have scrolled to bring item 49 into view"
        );
    }
}
