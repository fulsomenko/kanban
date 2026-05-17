//! # kanban-service — orchestration layer
//!
//! The service crate sits between the persistence backends and the
//! interactive frontends (CLI, TUI, MCP). It owns the
//! [`KanbanContext`] which:
//!
//! - delegates entity CRUD to a pluggable [`backend::KanbanBackend`];
//! - runs every command batch in a transactional scope via
//!   [`backend::KanbanBackend::with_transaction`];
//! - tracks per-session undo/redo state in an
//!   [`undo_stack::UndoStack`].
//!
//! ## Undo / Redo model (KAN-191)
//!
//! Every undoable [`kanban_domain::commands::Command`] implements
//! `capture_inverse`, which produces the forward CRUD operations that
//! reverse its effect. The `(forward, inverse)` pair is pushed onto
//! [`undo_stack::UndoStack`] at execute time; `undo()` pops the inverse
//! and executes it against current state through the normal command
//! pipeline.
//!
//! There is no snapshot-and-replay path. The previous `baseline_snapshot`
//! and replay-on-undo machinery were removed in KAN-191 Phase 7.
//!
//! ## Two stories, two lifetimes
//!
//! | Concept       | Lifetime              | Owner            | Purpose                    |
//! |---------------|-----------------------|------------------|----------------------------|
//! | **UndoStack** | Per-session, in-RAM   | `KanbanContext`  | User "take back / reapply" |
//! | **CommandLog**| Append-only, persisted| `KanbanBackend`  | Audit history (KAN-36)     |
//!
//! `KanbanContext::execute` pushes onto BOTH: the UndoStack gets the
//! `(forward, inverse)` pair; the audit log (via
//! `backend.append_commands`) gets the forward batch.

pub mod backend;
mod cascade;
pub mod config;
mod context;
#[cfg(feature = "json")]
pub mod json_backend;
mod path;
#[cfg(feature = "sqlite")]
pub mod sqlite_backend;
mod store_manager;
pub mod undo_stack;
pub use backend::KanbanBackend;
pub use config::AppConfigDto;
pub use context::{BatchOperationFailure, BatchOperationResult, KanbanContext};
pub use path::validate_path;
pub use store_manager::StoreManager;

#[cfg(feature = "test-helpers")]
pub mod test_helpers;

pub use kanban_core::AppConfig;

pub use kanban_domain::{
    ArchivedCard, Board, BoardId, BoardUpdate, Card, CardId, CardListFilter, CardPriority,
    CardStatus, CardSummary, CardUpdate, Column, ColumnId, ColumnUpdate, CreateCardOptions,
    DependencyGraph, FieldUpdate, KanbanError, KanbanOperations, KanbanResult, Snapshot, Sprint,
    SprintId, SprintStatus, SprintUpdate,
};

#[cfg(feature = "json")]
pub use kanban_persistence_json::JsonStoreFactory;
#[cfg(feature = "sqlite")]
pub use kanban_persistence_sqlite::SqliteStoreFactory;

/// Open a [`KanbanContext`] from a file locator with zero I/O.
/// The backend (JSON or SQLite) is detected automatically.
/// Data is loaded lazily on the first [`DataStore`] or [`CommandStore`] call.
#[cfg(any(feature = "json", feature = "sqlite"))]
pub async fn open_context(locator: &str, config: AppConfig) -> KanbanResult<KanbanContext> {
    let mut config = config;
    let sm = StoreManager::new(default_registry());
    sm.sync_backend_with_file(locator, &mut config);
    let backend = sm.make_backend(locator, &config).await?;
    KanbanContext::open(backend, config).await
}

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
