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

/// Returns a `StoreRegistry` pre-populated with the JSON backend.
/// SQLite files are handled directly via `KanbanContext::open_sqlite`.
#[cfg(feature = "json")]
pub fn default_registry() -> kanban_persistence::StoreRegistry {
    let mut registry = kanban_persistence::StoreRegistry::new();
    registry.register(Box::new(JsonStoreFactory));
    registry
}
