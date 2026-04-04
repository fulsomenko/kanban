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

    fn matches_content(&self, header: &[u8]) -> bool {
        header.starts_with(b"SQLite format 3\0")
    }

    fn create(
        &self,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
        Ok(Arc::new(SqliteStore::new(locator)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_content_valid_sqlite_header() {
        let factory = SqliteStoreFactory;
        assert!(factory.matches_content(b"SQLite format 3\0"));
        assert!(factory.matches_content(b"SQLite format 3\0extra data"));
    }

    #[test]
    fn test_matches_content_invalid_header() {
        let factory = SqliteStoreFactory;
        assert!(!factory.matches_content(b""));
        assert!(!factory.matches_content(b"{\"boards\": []}"));
        assert!(!factory.matches_content(b"SQLite format 2\0"));
        assert!(!factory.matches_content(b"not sqlite"));
    }
}
