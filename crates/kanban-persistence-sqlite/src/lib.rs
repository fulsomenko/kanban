mod builders;
mod helpers;
pub mod sqlite_store;
mod upserts;

pub use sqlite_store::SqliteStore;

use kanban_persistence::{PersistenceError, PersistenceStore, StoreFactory};
use std::sync::Arc;

pub struct SqliteStoreFactory;

impl StoreFactory for SqliteStoreFactory {
    fn name(&self) -> &str {
        "sqlite"
    }

    fn supported_patterns(&self) -> &[&str] {
        &["*.sqlite", "*.sqlite3", "*.db"]
    }

    fn matches(&self, locator: &str) -> bool {
        let ext = std::path::Path::new(locator)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        matches!(ext, "sqlite" | "sqlite3" | "db")
    }

    fn create(
        &self,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
        Ok(Arc::new(SqliteStore::new(locator)))
    }
}
