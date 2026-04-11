pub mod config;
mod context;
mod store_manager;
pub use config::AppConfigDto;
pub use context::{BatchOperationFailure, BatchOperationResult, KanbanContext};
pub use store_manager::StoreManager;

#[cfg(feature = "test-helpers")]
pub mod test_helpers;

pub use kanban_core::AppConfig;

use kanban_domain::KanbanError;
use kanban_persistence::{PersistenceStore, StoreRegistry};
use std::sync::Arc;

pub fn default_registry() -> StoreRegistry {
    let mut registry = StoreRegistry::new();
    // SQLite first for priority; JSON last as catch-all fallback for plain file paths.
    #[cfg(feature = "sqlite-storage")]
    registry.register(Box::new(kanban_persistence_sqlite::SqliteStoreFactory));
    #[cfg(feature = "json-storage")]
    registry.register(Box::new(kanban_persistence_json::JsonStoreFactory));
    registry
}

#[cfg(all(feature = "json-storage", feature = "sqlite-storage"))]
fn default_manager() -> StoreManager {
    StoreManager::with_default_backends()
}

#[cfg(not(all(feature = "json-storage", feature = "sqlite-storage")))]
fn default_manager() -> StoreManager {
    StoreManager::new(default_registry())
}

pub fn sync_backend_with_file(locator: &str, config: &mut AppConfig) -> bool {
    default_manager().sync_backend_with_file(locator, config)
}

pub fn detect_backend(locator: &str) -> Option<String> {
    default_manager().detect_backend(locator)
}

pub fn make_store(
    backend: &str,
    locator: &str,
) -> Result<Arc<dyn PersistenceStore + Send + Sync>, KanbanError> {
    default_manager().make_store(backend, locator)
}

pub fn make_store_with_config(
    file: Option<&str>,
    config: &AppConfig,
) -> Result<Arc<dyn PersistenceStore + Send + Sync>, KanbanError> {
    default_manager().make_store_with_config(file, config)
}

pub async fn validate_and_load_store(
    backend: &str,
    path: &str,
) -> Result<kanban_domain::Snapshot, KanbanError> {
    default_manager().validate_and_load_store(backend, path).await
}

/// Exports a board selection to a new SQLite file.
///
/// **Note:** The dependency graph is not part of the `AllBoardsExport` format
/// and will not be present in the exported file. This is by design — the export
/// format is board-centric, not a full snapshot. Use `migrate_store` instead
/// if you need to preserve card dependencies.
pub async fn export_to_sqlite(
    export: kanban_domain::export::AllBoardsExport,
    filename: &str,
) -> Result<(), KanbanError> {
    default_manager().export_to_sqlite(export, filename).await
}

pub async fn migrate_store(
    from_backend: &str,
    from_path: &str,
    to_backend: &str,
    to_path: &str,
) -> Result<(), KanbanError> {
    default_manager()
        .migrate_store(from_backend, from_path, to_backend, to_path)
        .await
}
