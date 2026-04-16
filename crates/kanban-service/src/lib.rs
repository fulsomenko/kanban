pub mod backend;
pub mod config;
mod context;
mod null_store;
mod path;
mod store_manager;
pub use backend::KanbanBackend;
pub use config::AppConfigDto;
pub use context::{BatchOperationFailure, BatchOperationResult, KanbanContext};
pub use null_store::NullStore;
pub use path::validate_path;
pub use store_manager::StoreManager;

#[cfg(feature = "test-helpers")]
pub mod test_helpers;

pub use kanban_core::AppConfig;

#[cfg(feature = "json")]
pub use kanban_persistence_json::JsonStoreFactory;

#[cfg(feature = "sqlite")]
pub use kanban_persistence_sqlite::SqliteStoreFactory;

/// Returns a `StoreRegistry` pre-populated with all backends that were
/// compiled in. SQLite is registered first so content-sniffing prefers it;
/// JSON is registered as the catch-all fallback.
#[cfg(any(feature = "json", feature = "sqlite"))]
pub fn default_registry() -> kanban_persistence::StoreRegistry {
    let mut registry = kanban_persistence::StoreRegistry::new();
    #[cfg(feature = "sqlite")]
    registry.register(Box::new(SqliteStoreFactory));
    #[cfg(feature = "json")]
    registry.register(Box::new(JsonStoreFactory));
    registry
}
