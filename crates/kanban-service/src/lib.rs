pub mod backend;
pub mod config;
mod context;
mod path;
mod store_manager;
pub use backend::KanbanBackend;
pub use config::AppConfigDto;
pub use context::{BatchOperationFailure, BatchOperationResult, KanbanContext, MAX_UNDO_DEPTH};
pub use path::validate_path;
pub use store_manager::StoreManager;

#[cfg(feature = "test-helpers")]
pub mod test_helpers;

pub use kanban_core::AppConfig;

// Re-export the full KanbanOperations surface so callers only need kanban_service::*
pub use kanban_domain::{
    ArchivedCard, Board, BoardUpdate, Card, CardListFilter, CardPriority, CardStatus, CardSummary,
    CardUpdate, Column, ColumnUpdate, CreateCardOptions, DependencyGraph, FieldUpdate, KanbanError,
    KanbanOperations, KanbanResult, Snapshot, Sprint, SprintStatus, SprintUpdate,
};

#[cfg(feature = "json")]
pub use kanban_persistence_json::JsonStoreFactory;
#[cfg(feature = "sqlite")]
pub use kanban_persistence_sqlite::SqliteStoreFactory;

/// Returns a `StoreRegistry` pre-populated with available backends.
/// SQLite is registered first so its magic-byte check takes priority.
#[cfg(any(feature = "json", feature = "sqlite"))]
pub fn default_registry() -> kanban_persistence::StoreRegistry {
    let mut registry = kanban_persistence::StoreRegistry::new();
    #[cfg(feature = "sqlite")]
    registry.register(Box::new(SqliteStoreFactory));
    #[cfg(feature = "json")]
    registry.register(Box::new(JsonStoreFactory));
    registry
}
