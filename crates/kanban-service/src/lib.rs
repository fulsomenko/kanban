mod context;
pub use context::{BulkOperationFailure, BulkOperationResult, DataSnapshot, KanbanContext};

#[cfg(feature = "test-helpers")]
pub mod test_helpers;

use kanban_persistence::PersistenceStore;
use std::sync::Arc;

pub fn make_store(path: &str) -> Arc<dyn PersistenceStore + Send + Sync> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    #[cfg(feature = "sqlite-storage")]
    if matches!(ext, "db" | "sqlite") {
        return Arc::new(kanban_persistence_sqlite::SqliteStore::new(path));
    }
    let _ = ext;
    #[cfg(feature = "json-storage")]
    {
        Arc::new(kanban_persistence_json::JsonFileStore::new(path))
    }
    #[cfg(not(feature = "json-storage"))]
    {
        panic!("No storage backend enabled")
    }
}
