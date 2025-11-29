pub mod snapshot;

use crate::app::App;
use kanban_core::KanbanResult;
use kanban_persistence::{JsonFileStore, PersistenceMetadata, PersistenceStore, StoreSnapshot};
use kanban_domain::commands::Command;
use kanban_domain::commands::CommandContext;
use kanban_domain::{ArchivedCard, Board, Card, Column, Sprint};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use snapshot::DataSnapshot;
pub use kanban_domain::commands;

/// Minimum time between saves to prevent excessive disk writes
const MIN_SAVE_INTERVAL: Duration = Duration::from_millis(500);

/// Manages state mutations and persistence
/// Decouples business logic from persistence concerns
pub struct StateManager {
    store: Option<Arc<JsonFileStore>>,
    command_queue: VecDeque<String>,
    dirty: bool,
    instance_id: uuid::Uuid,
    last_save_time: Option<Instant>,
    conflict_pending: bool,
}

impl StateManager {
    /// Create a new state manager with optional persistence store
    pub fn new(save_file: Option<String>) -> Self {
        let (store, instance_id) = if let Some(path) = save_file {
            let store = Arc::new(JsonFileStore::new(&path));
            let id = store.instance_id();
            (Some(store), id)
        } else {
            (None, uuid::Uuid::new_v4())
        };

        Self {
            store,
            command_queue: VecDeque::new(),
            dirty: false,
            instance_id,
            last_save_time: None,
            conflict_pending: false,
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
        self.command_queue.push_back(description);

        Ok(())
    }

    /// Execute a command and mark state as dirty (app-based convenience method)
    pub fn execute(&mut self, app: &mut App, command: Box<dyn Command>) -> KanbanResult<()> {
        self.execute_with_context(
            &mut app.boards,
            &mut app.columns,
            &mut app.cards,
            &mut app.sprints,
            &mut app.archived_cards,
            command,
        )
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

    /// Save state to disk if dirty and debounce interval has elapsed
    /// Called periodically from the event loop
    /// Returns ConflictDetected error if external file modifications detected
    pub async fn save_if_needed(&mut self, snapshot: &DataSnapshot) -> KanbanResult<()> {
        if !self.dirty {
            return Ok(());
        }

        // Check debounce interval
        let should_save = match self.last_save_time {
            None => true,
            Some(last_save) => last_save.elapsed() >= MIN_SAVE_INTERVAL,
        };

        if !should_save {
            return Ok(());
        }

        if let Some(ref store) = self.store {
            let data = snapshot.to_json_bytes()?;
            let persistence_snapshot = StoreSnapshot {
                data,
                metadata: PersistenceMetadata::new(2, self.instance_id),
            };

            match store.save(persistence_snapshot).await {
                Ok(_) => {
                    self.dirty = false;
                    self.last_save_time = Some(Instant::now());
                    self.conflict_pending = false;

                    let cmd_count = self.command_queue.len();
                    tracing::info!("Saved {} commands to disk", cmd_count);
                    self.command_queue.clear();
                    Ok(())
                }
                Err(kanban_core::KanbanError::ConflictDetected { path, .. }) => {
                    self.conflict_pending = true;
                    tracing::warn!("File conflict detected at {}", path);
                    Err(kanban_core::KanbanError::ConflictDetected { path, source: None })
                }
                Err(e) => Err(e),
            }
        } else {
            Ok(())
        }
    }

    /// Force save immediately, bypassing debounce (for critical operations)
    pub async fn save_now(&mut self, snapshot: &DataSnapshot) -> KanbanResult<()> {
        if let Some(ref store) = self.store {
            let data = snapshot.to_json_bytes()?;
            let persistence_snapshot = StoreSnapshot {
                data,
                metadata: PersistenceMetadata::new(2, self.instance_id),
            };

            store.save(persistence_snapshot).await?;
            self.dirty = false;
            self.last_save_time = Some(Instant::now());

            let cmd_count = self.command_queue.len();
            tracing::info!("Force saved {} commands to disk", cmd_count);
            self.command_queue.clear();
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
            self.last_save_time = Some(Instant::now());
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

            // Deserialize and rebuild app state from loaded data
            let data: DataSnapshot = serde_json::from_slice(&snapshot.data)
                .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;

            app.boards = data.boards;
            app.columns = data.columns;
            app.cards = data.cards;
            app.sprints = data.sprints;
            app.archived_cards = data.archived_cards;

            self.dirty = false;
            self.command_queue.clear();

            tracing::info!("Reloaded state from disk");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_manager_creation() {
        let manager = StateManager::new(None);
        assert!(!manager.is_dirty());
    }

    #[test]
    fn test_dirty_flag_after_execute() {
        let mut manager = StateManager::new(None);

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
        manager.execute(&mut App::new(None), command).unwrap();

        assert!(manager.is_dirty());
    }
}
