mod context;
pub use context::{BulkOperationFailure, BulkOperationResult, DataSnapshot, KanbanContext};

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

pub fn detect_backend(locator: &str) -> Option<String> {
    default_registry().detect_backend(locator).map(String::from)
}

pub fn make_store(locator: &str) -> Result<Arc<dyn PersistenceStore + Send + Sync>, KanbanError> {
    Ok(default_registry().create_store(locator)?)
}

pub fn make_store_for_backend(
    backend: &str,
    locator: &str,
) -> Result<Arc<dyn PersistenceStore + Send + Sync>, KanbanError> {
    Ok(default_registry().create_by_name(backend, locator)?)
}

pub fn default_extension_for(backend: &str) -> Option<String> {
    default_registry()
        .default_extension_for(backend)
        .map(|s| s.to_string())
}

pub fn make_store_with_config(
    file: Option<&str>,
    config: &AppConfig,
) -> Result<Arc<dyn PersistenceStore + Send + Sync>, KanbanError> {
    match file {
        Some(path) => make_store(path),
        None => make_store(&config.effective_storage_location()),
    }
}

pub async fn validate_and_load_store(
    path: &str,
) -> Result<kanban_domain::Snapshot, KanbanError> {
    validate_and_load_store_for_backend(None, path).await
}

pub async fn validate_and_load_store_for_backend(
    backend: Option<&str>,
    path: &str,
) -> Result<kanban_domain::Snapshot, KanbanError> {
    let store = match backend {
        Some(b) => make_store_for_backend(b, path)?,
        None => make_store(path)?,
    };
    if !store.exists().await {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Storage file does not exist: {}", path),
        )
        .into());
    }
    let (snapshot, _metadata) = store.load().await?;
    let data = kanban_persistence::snapshot_from_json_bytes(&snapshot.data)?;
    Ok(data)
}

pub async fn migrate_store(from_path: &str, to_path: &str) -> Result<(), KanbanError> {
    migrate_store_for_backend(None, from_path, None, to_path).await
}

pub async fn migrate_store_for_backend(
    from_backend: Option<&str>,
    from_path: &str,
    to_backend: Option<&str>,
    to_path: &str,
) -> Result<(), KanbanError> {
    let from = std::path::Path::new(from_path);
    let to = std::path::Path::new(to_path);
    if !from.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source file not found: {}", from.display()),
        )
        .into());
    }
    if to.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "Destination already exists: {}. Remove it first or use a different path.",
                to.display()
            ),
        )
        .into());
    }
    let source = match from_backend {
        Some(b) => make_store_for_backend(b, from_path)?,
        None => make_store(from_path)?,
    };
    let (snapshot, _) = source.load().await?;
    let target = match to_backend {
        Some(b) => make_store_for_backend(b, to_path)?,
        None => make_store(to_path)?,
    };
    target.save(snapshot).await?;
    Ok(())
}
