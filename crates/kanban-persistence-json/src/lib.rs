pub mod atomic_writer;
pub mod conflict;
pub mod json_file_store;
pub mod migration;

pub use conflict::FileMetadata;
pub use json_file_store::{JsonEnvelope, JsonFileStore};

use kanban_persistence::{PersistenceError, PersistenceStore, StoreFactory};
use std::sync::Arc;

pub struct JsonStoreFactory;

impl StoreFactory for JsonStoreFactory {
    fn name(&self) -> &str {
        "json"
    }

    fn supported_patterns(&self) -> &[&str] {
        &["*.json", "<any file path>"]
    }

    fn matches(&self, locator: &str) -> bool {
        let ext = std::path::Path::new(locator)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        ext == "json" || !locator.contains("://")
    }

    fn create(
        &self,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
        Ok(Arc::new(JsonFileStore::new(locator)))
    }
}
