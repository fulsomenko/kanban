pub mod snapshot;

use crate::app::App;
use kanban_core::KanbanResult;
use kanban_domain::commands::Command;
use kanban_domain::commands::CommandContext;
use kanban_domain::{ArchivedCard, Board, Card, Column, Sprint};
use kanban_persistence::{JsonFileStore, PersistenceMetadata, PersistenceStore, StoreSnapshot};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

#[cfg(feature = "sqlite")]
use kanban_persistence::SqliteStore;

pub use kanban_domain::commands;
pub use snapshot::DataSnapshot;

/// Manages state mutations and persistence with immediate auto-saving
///
/// # Save Behavior
///
/// Changes are saved immediately after each command execution:
/// - No debounce delay - every change is persisted instantly
/// - No data loss window - state is always current on disk
/// - Simple and predictable behavior
///
/// The immediate save approach works well for kanban boards because:
/// - User actions are discrete and human-paced (not rapid-fire)
/// - Modern SSDs handle frequent writes efficiently (1-5ms per save)
/// - Conflict detection and file watching remain responsive
///
/// # Example
/// ```ignore
/// // Execute command - automatically saves afterward
/// state_manager.execute(app, command)?;
/// state_manager.save(&snapshot).await?;
/// ```
pub struct StateManager {
    store: Option<Arc<dyn PersistenceStore + Send + Sync>>,
    command_queue: VecDeque<String>,
    dirty: bool,
    instance_id: uuid::Uuid,
    conflict_pending: bool,
    needs_refresh: bool,
    save_tx: Option<mpsc::Sender<DataSnapshot>>,
    save_completion_tx: Option<mpsc::UnboundedSender<()>>,
    pending_saves: usize,
    file_watcher: Option<Arc<kanban_persistence::FileWatcher>>,
}

impl StateManager {
    /// Create a new state manager with optional persistence store
    ///
    /// Returns a tuple of:
    /// - StateManager instance
    /// - Optional receiver for async save processing (snapshots to save)
    /// - Optional receiver for save completion notifications
    ///
    /// Storage backend is selected based on file extension:
    /// - `.db` or `.sqlite` -> SQLite (requires `sqlite` feature)
    /// - `.json` or other -> JSON file
    pub fn new(
        save_file: Option<String>,
    ) -> (
        Self,
        Option<mpsc::Receiver<DataSnapshot>>,
        Option<mpsc::UnboundedReceiver<()>>,
    ) {
        // Use bounded channel (capacity: 100) to prevent unbounded memory growth
        // If queue is full, save_if_needed() will log a warning instead of blocking
        const SAVE_QUEUE_CAPACITY: usize = 100;

        let (store, instance_id, save_channel): (
            Option<Arc<dyn PersistenceStore + Send + Sync>>,
            uuid::Uuid,
            Option<(mpsc::Sender<DataSnapshot>, mpsc::Receiver<DataSnapshot>)>,
        ) = if let Some(ref path) = save_file {
            let (store, id): (Arc<dyn PersistenceStore + Send + Sync>, uuid::Uuid) =
                Self::create_store(path);
            let (tx, rx) = mpsc::channel(SAVE_QUEUE_CAPACITY);
            (Some(store), id, Some((tx, rx)))
        } else {
            (None, uuid::Uuid::new_v4(), None)
        };

        let (save_tx, save_rx) = if let Some((tx, rx)) = save_channel {
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        let (save_completion_tx, save_completion_rx) = mpsc::unbounded_channel();

        let manager = Self {
            store,
            command_queue: VecDeque::new(),
            dirty: false,
            instance_id,
            conflict_pending: false,
            needs_refresh: false,
            save_tx,
            save_completion_tx: Some(save_completion_tx),
            pending_saves: 0,
            file_watcher: None,
        };

        (manager, save_rx, Some(save_completion_rx))
    }

    /// Create the appropriate store based on file extension
    fn create_store(path: &str) -> (Arc<dyn PersistenceStore + Send + Sync>, uuid::Uuid) {
        let path_ref = Path::new(path);
        let extension = path_ref
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("json");

        match extension {
            #[cfg(feature = "sqlite")]
            "db" | "sqlite" => {
                tracing::info!("Using SQLite storage backend for: {}", path);
                let store = Arc::new(SqliteStore::new(path));
                let id = store.instance_id();
                (store, id)
            }
            _ => {
                tracing::info!("Using JSON file storage backend for: {}", path);
                let store = Arc::new(JsonFileStore::new(path));
                let id = store.instance_id();
                (store, id)
            }
        }
    }

    /// Execute a command and mark state as dirty
    /// Takes individual mutable references to avoid borrow checker issues when called from App methods
    pub fn execute_with_context(
        &mut self,
        boards: &mut Vec<Board>,
        columns: &mut Vec<Column>,
        cards: &mut Vec<Card>,
        sprints: &mut Vec<Sprint>,
        archived_cards: &mut Vec<ArchivedCard>,
        command: Box<dyn Command>,
    ) -> KanbanResult<()> {
        let description = command.description();
        tracing::debug!("Executing: {}", description);

        // Create context from data
        let mut context = CommandContext {
            boards,
            columns,
            cards,
            sprints,
            archived_cards,
        };

        // Execute business logic
        command.execute(&mut context)?;

        // Mark dirty and queue command
        self.dirty = true;
        self.needs_refresh = true;
        self.command_queue.push_back(description);

        Ok(())
    }

    /// Execute a command and mark state as dirty (app-based convenience method)
    ///
    /// After execution, queues a snapshot for async saving if a save channel is configured.
    pub fn execute(&mut self, app: &mut App, command: Box<dyn Command>) -> KanbanResult<()> {
        // Execute command
        self.execute_with_context(
            &mut app.ctx.boards,
            &mut app.ctx.columns,
            &mut app.ctx.cards,
            &mut app.ctx.sprints,
            &mut app.ctx.archived_cards,
            command,
        )?;

        // Queue snapshot for async save if channel is available
        if let Some(ref tx) = self.save_tx {
            let snapshot = DataSnapshot::from_app(app);
            tracing::debug!("Queueing snapshot for async save");
            // Use try_send (non-blocking) since we're in a synchronous context
            // Backpressure is handled by queue_snapshot method
            match tx.try_send(snapshot) {
                Ok(_) => {
                    self.pending_saves += 1;
                    tracing::debug!(
                        "Snapshot queued successfully (pending: {})",
                        self.pending_saves
                    );
                }
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    tracing::warn!(
                        "Save queue is full ({} pending), skipping this save. \
                        This may indicate the disk is slow or the save worker is overloaded.",
                        self.pending_saves
                    );
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    tracing::error!("Failed to queue save: channel closed");
                }
            }
        } else {
            tracing::debug!("No save channel available - skipping save");
        }

        Ok(())
    }

    /// Execute multiple commands in a batch
    pub fn execute_batch(
        &mut self,
        app: &mut App,
        commands: Vec<Box<dyn Command>>,
    ) -> KanbanResult<()> {
        for command in commands {
            self.execute(app, command)?;
        }
        Ok(())
    }

    /// Check if state is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get the instance ID for this manager
    pub fn instance_id(&self) -> uuid::Uuid {
        self.instance_id
    }

    /// Get the store reference
    pub fn store(&self) -> Option<&Arc<dyn PersistenceStore + Send + Sync>> {
        self.store.as_ref()
    }

    /// Check if there's a pending conflict
    pub fn has_conflict(&self) -> bool {
        self.conflict_pending
    }

    /// Clear the conflict flag (called after user resolves conflict)
    pub fn clear_conflict(&mut self) {
        self.conflict_pending = false;
    }

    /// Clear the store reference (called when import fails to prevent accidental saves)
    pub fn clear_store(&mut self) {
        self.store = None;
    }

    /// Check if view needs to be refreshed
    pub fn needs_refresh(&self) -> bool {
        self.needs_refresh
    }

    /// Clear the refresh flag (called after view is refreshed)
    pub fn clear_refresh(&mut self) {
        self.needs_refresh = false;
    }

    /// Mark state as dirty and needing refresh (for direct mutations that bypass execute_command)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.needs_refresh = true;
    }

    /// Force overwrite external changes (user chose to keep their changes)
    pub async fn force_overwrite(&mut self, snapshot: &DataSnapshot) -> KanbanResult<()> {
        self.conflict_pending = false;

        if let Some(ref store) = self.store {
            let data = snapshot.to_json_bytes()?;
            let persistence_snapshot = StoreSnapshot {
                data,
                metadata: PersistenceMetadata::new(self.instance_id),
            };

            store.save(persistence_snapshot).await?;
            self.dirty = false;
            self.command_queue.clear();

            tracing::info!("Force overwrote external changes");
        }

        Ok(())
    }

    /// Reload from disk (user chose to discard their changes)
    pub async fn reload_from_disk(&mut self, app: &mut App) -> KanbanResult<()> {
        self.conflict_pending = false;

        if let Some(ref store) = self.store {
            let (snapshot, _metadata) = store.load().await?;

            // Deserialize and apply loaded data to app
            let data: DataSnapshot = serde_json::from_slice(&snapshot.data)
                .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;

            data.apply_to_app(app);

            self.dirty = false;
            self.command_queue.clear();

            tracing::info!("Reloaded state from disk");
        }

        Ok(())
    }

    /// Mark state as clean, clearing dirty flag and command queue
    /// Used after successful external reload to prevent re-save
    pub fn mark_clean(&mut self) {
        self.dirty = false;
        self.command_queue.clear();
    }

    /// Close the save channel to signal the worker to finish processing and exit
    /// Called during graceful shutdown before waiting for the worker to finish
    pub fn close_save_channel(&mut self) {
        self.save_tx = None;
    }

    /// Check if the save channel is available for sending snapshots
    pub fn has_save_channel(&self) -> bool {
        self.save_tx.is_some()
    }

    /// Queue a snapshot for async saving
    /// Used by App::execute_command to ensure snapshots are queued
    /// Increments pending_saves to track unsaved changes
    ///
    /// Pauses file watcher before queueing to prevent detecting our own writes as external changes.
    /// The save worker will resume the watcher after the save completes.
    ///
    /// Uses try_send to handle bounded channel capacity (100 snapshots).
    /// If channel is full, logs warning and skips save to prevent blocking UI.
    pub fn queue_snapshot(&mut self, snapshot: DataSnapshot) {
        // Pause file watching before queuing save to prevent detecting our own writes
        if let Some(ref watcher) = self.file_watcher {
            watcher.pause();
            tracing::debug!("File watcher paused before queuing snapshot");
        }

        if let Some(ref tx) = self.save_tx {
            tracing::debug!(
                "Queueing snapshot for async save (pending: {} -> {})",
                self.pending_saves,
                self.pending_saves + 1
            );
            match tx.try_send(snapshot) {
                Ok(_) => {
                    self.pending_saves += 1;
                    tracing::debug!("Snapshot queued successfully");
                }
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    tracing::warn!(
                        "Save queue is full ({} pending), skipping this save. \
                        This may indicate the disk is slow or the save worker is overloaded.",
                        self.pending_saves
                    );
                    // Resume watcher if we couldn't queue the snapshot
                    if let Some(ref watcher) = self.file_watcher {
                        watcher.resume();
                        tracing::debug!("File watcher resumed (save queue full)");
                    }
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    tracing::error!("Failed to queue save: channel closed");
                    // Resume watcher if channel is closed
                    if let Some(ref watcher) = self.file_watcher {
                        watcher.resume();
                        tracing::debug!("File watcher resumed (channel closed)");
                    }
                }
            }
        } else {
            tracing::debug!("No save channel available - skipping save");
            // Resume watcher if no save channel
            if let Some(ref watcher) = self.file_watcher {
                watcher.resume();
                tracing::debug!("File watcher resumed (no save channel)");
            }
        }
    }

    /// Check if there are pending saves waiting to be written
    pub fn has_pending_saves(&self) -> bool {
        self.pending_saves > 0
    }

    /// Signal that a save has been completed (called by save worker)
    pub fn save_completed(&mut self) {
        if self.pending_saves > 0 {
            self.pending_saves -= 1;
            tracing::debug!(
                "Save completed (pending: {} -> {})",
                self.pending_saves + 1,
                self.pending_saves
            );

            // If no more pending saves, clear the dirty flag
            if self.pending_saves == 0 {
                self.dirty = false;
                tracing::debug!("All saves complete - clearing dirty flag");
            }
        }
    }

    /// Get the save completion sender for the worker to use
    pub fn save_completion_tx(&self) -> Option<&mpsc::UnboundedSender<()>> {
        self.save_completion_tx.as_ref()
    }

    /// Set the file watcher for coordinating pause/resume with saves
    /// Called after the file watcher is initialized in App::run()
    pub fn set_file_watcher(&mut self, watcher: Arc<kanban_persistence::FileWatcher>) {
        self.file_watcher = Some(watcher);
        tracing::debug!("File watcher set on StateManager");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_manager_creation() {
        let (manager, _rx, _completion_rx) = StateManager::new(None);
        assert!(!manager.is_dirty());
    }

    #[test]
    fn test_dirty_flag_after_execute() {
        let (mut manager, _rx, _completion_rx) = StateManager::new(None);

        struct DummyCommand;
        impl Command for DummyCommand {
            fn execute(&self, _context: &mut CommandContext) -> KanbanResult<()> {
                Ok(())
            }

            fn description(&self) -> String {
                "dummy".to_string()
            }
        }

        let command = Box::new(DummyCommand);
        let (mut app, _app_rx) = App::new(None);
        manager.execute(&mut app, command).unwrap();

        assert!(manager.is_dirty());
    }
}
