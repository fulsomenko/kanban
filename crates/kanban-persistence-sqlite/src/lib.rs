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
        if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
            return Err(PersistenceError::Database(
                "SqliteStoreFactory::create requires a multi-thread Tokio runtime; \
                 block_in_place is unavailable on a current_thread runtime. \
                 Use #[tokio::test(flavor = \"multi_thread\")] in tests."
                    .to_string(),
            ));
        }
        let store = tokio::task::block_in_place(|| handle.block_on(SqliteStore::open(locator)))
            .map_err(|e| PersistenceError::Database(e.to_string()))?;
        Ok(Arc::new(store))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_persistence::StoreFactory;

    #[test]
    fn test_sqlite_factory_matches_content_sqlite_magic_bytes() {
        let header = b"SQLite format 3\0extra";
        assert!(SqliteStoreFactory.matches_content(header));
    }

    #[test]
    fn test_sqlite_factory_matches_content_rejects_json() {
        let header = b"{\"boards\": []}";
        assert!(!SqliteStoreFactory.matches_content(header));
    }

    #[test]
    fn test_sqlite_factory_matches_content_rejects_empty() {
        assert!(!SqliteStoreFactory.matches_content(b""));
    }

    #[test]
    fn test_sqlite_factory_name_is_sqlite() {
        assert_eq!(SqliteStoreFactory.name(), "sqlite");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_sqlite_factory_create_returns_persistence_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let store = SqliteStoreFactory.create(path.to_str().unwrap()).unwrap();
        assert!(store.exists().await);
    }
}
