pub mod sqlite_store;

pub use sqlite_store::SqliteStore;

use kanban_persistence::{PersistenceError, PersistenceStore, StoreFactory};
use std::sync::Arc;

pub struct SqliteStoreFactory;

impl StoreFactory for SqliteStoreFactory {
    fn name(&self) -> &str {
        "sqlite"
    }

    fn matches_content(&self, header: &[u8]) -> bool {
        header.starts_with(b"SQLite format 3\0")
    }

    fn create(
        &self,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
        let handle = tokio::runtime::Handle::current();
        let store = tokio::task::block_in_place(|| handle.block_on(SqliteStore::open(locator)))
            .map_err(|e| PersistenceError::Database(e.to_string()))?;
        Ok(Arc::new(store))
    }
}

