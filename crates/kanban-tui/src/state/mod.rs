pub mod snapshot;

use crate::app::App;
use kanban_core::KanbanResult;
use kanban_domain::commands::Command;
use kanban_domain::commands::CommandContext;
use kanban_domain::{ArchivedCard, Board, Card, Column, Sprint};
use kanban_persistence::{JsonFileStore, PersistenceMetadata, PersistenceStore, StoreSnapshot};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::mpsc;

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
    store: Option<Arc<JsonFileStore>>,
    command_queue: VecDeque<String>,
    dirty: bool,
    instance_id: uuid::Uuid,
    conflict_pending: bool,
    needs_refresh: bool,
    save_tx: Option<mpsc::UnboundedSender<DataSnapshot>>,
}

impl StateManager {
    /// Create a new state manager with optional persistence store
    ///
    /// Returns the StateManager and an optional receiver for async save processing.
    /// If save_file is None, the receiver will also be None (no saves needed).
    pub fn new(save_file: Option<String>) -> (Self, Option<mpsc::UnboundedReceiver<DataSnapshot>>) {
        let (store, instance_id, save_channel) = if let Some(path) = save_file {
            let store = Arc::new(JsonFileStore::new(&path));
            let id = store.instance_id();
            let (tx, rx) = mpsc::unbounded_channel();
            (Some(store), id, Some((tx, rx)))
        } else {
            (None, uuid::Uuid::new_v4(), None)
        };

        let (save_tx, save_rx) = if let Some((tx, rx)) = save_channel {
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        let manager = Self {
            store,
            command_queue: VecDeque::new(),
            dirty: false,
            instance_id,
            conflict_pending: false,
            needs_refresh: false,
            save_tx,
        };

        (manager, save_rx)
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
            &mut app.boards,
            &mut app.columns,
            &mut app.cards,
            &mut app.sprints,
            &mut app.archived_cards,
            command,
        )?;

        // Queue snapshot for async save if channel is available
        if let Some(ref tx) = self.save_tx {
            let snapshot = DataSnapshot::from_app(app);
            tracing::debug!("Queueing snapshot for async save");
            // Send is non-blocking and only fails if receiver is dropped
            match tx.send(snapshot) {
                Ok(_) => {
                    tracing::debug!("Snapshot queued successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to queue save: channel closed: {:?}", e);
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
    pub fn store(&self) -> Option<&Arc<JsonFileStore>> {
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
                metadata: PersistenceMetadata::new(2, self.instance_id),
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
    pub fn queue_snapshot(&self, snapshot: DataSnapshot) {
        if let Some(ref tx) = self.save_tx {
            tracing::debug!("Queueing snapshot for async save");
            match tx.send(snapshot) {
                Ok(_) => {
                    tracing::debug!("Snapshot queued successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to queue save: channel closed: {:?}", e);
                }
            }
        } else {
            tracing::debug!("No save channel available - skipping save");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_manager_creation() {
        let (manager, _rx) = StateManager::new(None);
        assert!(!manager.is_dirty());
    }

    #[test]
    fn test_dirty_flag_after_execute() {
        let (mut manager, _rx) = StateManager::new(None);

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
