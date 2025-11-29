pub mod snapshot;

use crate::app::App;
use kanban_core::KanbanResult;
use kanban_persistence::{JsonFileStore, PersistenceMetadata, PersistenceStore, StoreSnapshot};
use kanban_domain::commands::Command;
use kanban_domain::commands::CommandContext;
use std::collections::VecDeque;
use std::sync::Arc;

pub use snapshot::DataSnapshot;
pub use kanban_domain::commands;

/// Manages state mutations and persistence
/// Decouples business logic from persistence concerns
pub struct StateManager {
    store: Option<Arc<JsonFileStore>>,
    command_queue: VecDeque<String>,
    dirty: bool,
    instance_id: uuid::Uuid,
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
        }
    }

    /// Execute a command and mark state as dirty
    pub fn execute(&mut self, app: &mut App, command: Box<dyn Command>) -> KanbanResult<()> {
        let description = command.description();
        tracing::debug!("Executing: {}", description);

        // Create context from app data
        let mut context = CommandContext {
            boards: &mut app.boards,
            columns: &mut app.columns,
            cards: &mut app.cards,
            sprints: &mut app.sprints,
            archived_cards: &mut app.archived_cards,
        };

        // Execute business logic
        command.execute(&mut context)?;

        // Mark dirty and queue command
        self.dirty = true;
        self.command_queue.push_back(description);

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

    /// Save state to disk if dirty
    /// Called periodically from the event loop
    pub async fn save_if_needed(&mut self, snapshot: &DataSnapshot) -> KanbanResult<()> {
        if !self.dirty {
            return Ok(());
        }

        if let Some(ref store) = self.store {
            let data = snapshot.to_json_bytes()?;
            let persistence_snapshot = StoreSnapshot {
                data,
                metadata: PersistenceMetadata::new(2, self.instance_id),
            };

            store.save(persistence_snapshot).await?;
            self.dirty = false;

            let cmd_count = self.command_queue.len();
            tracing::info!("Saved {} commands to disk", cmd_count);
            self.command_queue.clear();
        }

        Ok(())
    }

    /// Force save immediately (for critical operations)
    pub async fn save_now(&mut self, snapshot: &DataSnapshot) -> KanbanResult<()> {
        self.dirty = true;
        self.save_if_needed(snapshot).await
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
