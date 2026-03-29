mod context;
pub use context::{BulkOperationFailure, BulkOperationResult, DataSnapshot, KanbanContext};

#[cfg(feature = "test-helpers")]
pub mod test_helpers;

use kanban_domain::KanbanError;
use kanban_persistence::{PersistenceStore, StoreRegistry};
use std::sync::Arc;

pub fn default_registry() -> StoreRegistry {
    let mut registry = StoreRegistry::new();
    #[cfg(feature = "sqlite-storage")]
    registry.register(Box::new(kanban_persistence_sqlite::SqliteStoreFactory));
    #[cfg(feature = "json-storage")]
    registry.register(Box::new(kanban_persistence_json::JsonStoreFactory));
    registry
}

pub fn make_store(locator: &str) -> Result<Arc<dyn PersistenceStore + Send + Sync>, KanbanError> {
    Ok(default_registry().create_store(locator)?)
}
