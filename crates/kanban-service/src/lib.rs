mod context;
pub use context::{BulkOperationFailure, BulkOperationResult, DataSnapshot, KanbanContext};

#[cfg(feature = "test-helpers")]
pub mod test_helpers;

use kanban_domain::KanbanError;
use kanban_persistence::PersistenceStore;
use std::sync::Arc;

pub fn make_store(path: &str) -> Result<Arc<dyn PersistenceStore + Send + Sync>, KanbanError> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    #[cfg(feature = "sqlite-storage")]
    if matches!(ext, "db" | "sqlite") {
        return Ok(Arc::new(kanban_persistence_sqlite::SqliteStore::new(path)));
    }
    #[cfg(not(feature = "sqlite-storage"))]
    if matches!(ext, "db" | "sqlite") {
        return Err(KanbanError::Internal(format!(
            "SQLite storage requested ({path}) but the 'sqlite-storage' feature is not enabled. \
             Recompile with `--features sqlite` or use a .json file."
        )));
    }
    let _ = ext;
    #[cfg(feature = "json-storage")]
    {
        Ok(Arc::new(kanban_persistence_json::JsonFileStore::new(path)))
    }
    #[cfg(not(feature = "json-storage"))]
    {
        Err(KanbanError::Internal(
            "No storage backend enabled".to_string(),
        ))
    }
}
