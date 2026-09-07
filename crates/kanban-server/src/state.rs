use kanban_core::ClientId;
use kanban_domain::{Invalidation, Model};
use kanban_service::api::{ChangeEventFrame, ChangeKind, EntityType};
use kanban_service::{KanbanContext, KanbanResult};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

/// Run a mutation against `ctx`, returning its value and discarding the
/// `Invalidation` it produced.
pub(crate) fn mutate<T>(
    ctx: &mut KanbanContext,
    op: impl FnOnce(&mut KanbanContext) -> KanbanResult<(T, Invalidation)>,
) -> KanbanResult<T> {
    let (value, invalidation) = op(ctx)?;
    let _ = invalidation;
    Ok(value)
}

/// Like [`mutate`], for operations that return only an `Invalidation`.
pub(crate) fn mutate_unit(
    ctx: &mut KanbanContext,
    op: impl FnOnce(&mut KanbanContext) -> KanbanResult<Invalidation>,
) -> KanbanResult<()> {
    let invalidation = op(ctx)?;
    let _ = invalidation;
    Ok(())
}

/// The context and the Model it feeds live under one guard, so a reader can
/// never observe a Model that a committed mutation has already invalidated.
/// `Deref`/`DerefMut` to the context keep every existing `state.ctx.lock()`
/// call site reading as it did before the Model joined it.
pub struct Session {
    pub ctx: KanbanContext,
    pub model: Model,
}

impl std::ops::Deref for Session {
    type Target = KanbanContext;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl std::ops::DerefMut for Session {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}

/// Shared state for every axum handler.
///
/// `tokio::sync::Mutex`, not `RwLock`: `KanbanContext`'s write path is async
/// (`save`/`reload`), so holding a sync `RwLock` write guard across an
/// `.await` would be a `Send`/deadlock hazard.
#[derive(Clone)]
pub struct AppState {
    pub ctx: Arc<Mutex<Session>>,
    pub instance_id: Uuid,
    pub event_tx: tokio::sync::broadcast::Sender<ChangeEventFrame>,
    /// True for a SQLite locator, where `watch::watch_for_external_changes`
    /// installs no watcher and an external writer is therefore invisible;
    /// the Model is cleared on every acquire instead.
    pub reset_model_per_request: bool,
}

impl AppState {
    pub fn new(ctx: KanbanContext) -> Self {
        Self::with_reset(ctx, false)
    }

    pub fn with_reset(ctx: KanbanContext, reset_model_per_request: bool) -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(256);
        Self {
            ctx: Arc::new(Mutex::new(Session {
                ctx,
                model: Model::default(),
            })),
            instance_id: Uuid::new_v4(),
            event_tx,
            reset_model_per_request,
        }
    }

    /// Acquires the session lock, clearing the Model first when
    /// `reset_model_per_request` is set.
    pub async fn lock_session(&self) -> MutexGuard<'_, Session> {
        let mut guard = self.ctx.lock().await;
        if self.reset_model_per_request {
            let _ = guard.model.invalidate(Invalidation::All);
        }
        guard
    }

    fn emit(
        &self,
        entity_type: Option<EntityType>,
        entity_id: Option<Uuid>,
        kind: Option<ChangeKind>,
    ) {
        let _ = self.event_tx.send(ChangeEventFrame::for_entity(
            self.instance_id,
            Uuid::new_v4(),
            ClientId::nil(),
            entity_type,
            entity_id,
            kind,
        ));
    }

    /// Broadcast a change event naming the entity a mutation touched. Shared
    /// across every entity's write routes so each doesn't reimplement it;
    /// call after the context lock guard has been dropped. A missing
    /// subscriber (no SSE consumer connected yet) is not an error, hence the
    /// discarded result.
    pub fn broadcast_change(&self, entity_type: EntityType, entity_id: Uuid, kind: ChangeKind) {
        self.emit(Some(entity_type), Some(entity_id), Some(kind));
    }

    /// Broadcast a change event whose origin is outside this process (an
    /// external writer changed the file), so the specific entity touched is
    /// unknowable.
    pub fn broadcast_unscoped_change(&self) {
        self.emit(None, None, None);
    }

    /// Durably persist any pending changes, then broadcast that `entity_id`
    /// changed. Call this from *inside* the context lock (needs
    /// `&KanbanContext` — `save()` takes `&self`, so no reacquire is needed),
    /// immediately after a successful mutation and before the lock guard
    /// drops. A write whose `save()` fails must not report success to the
    /// client — callers propagate the error via `AppError::from(&e)` exactly
    /// like every other `KanbanResult` in this crate, so a 201/200 response
    /// is only ever returned once the data is actually on disk (or in
    /// SQLite's case, harmlessly redundant — `flush()` is a no-op cost there
    /// since each statement already committed).
    pub async fn persist_and_broadcast(
        &self,
        ctx: &KanbanContext,
        entity_type: EntityType,
        entity_id: Uuid,
        kind: ChangeKind,
    ) -> KanbanResult<()> {
        ctx.save().await?;
        self.broadcast_change(entity_type, entity_id, kind);
        Ok(())
    }
}

#[cfg(all(test, feature = "test-helpers"))]
mod tests {
    use super::*;
    use kanban_backend_memory::InMemoryStore;
    use kanban_service::{AppConfig, KanbanBackend, KanbanOperations};
    use std::cell::Cell;

    async fn seeded_ctx() -> (KanbanContext, Uuid) {
        let backend: Arc<dyn KanbanBackend> = Arc::new(InMemoryStore::new());
        let mut ctx = KanbanContext::open(backend, AppConfig::default())
            .await
            .unwrap();
        let board = ctx.create_board("Board1".into(), None).unwrap();
        (ctx, board.id)
    }

    #[tokio::test]
    async fn test_mutate_returns_the_operations_value() {
        let (mut ctx, board_id) = seeded_ctx().await;

        let updated = mutate(&mut ctx, |c| {
            c.update_board_impl(
                board_id,
                kanban_domain::BoardUpdate {
                    name: Some("Renamed".into()),
                    ..Default::default()
                },
            )
        })
        .unwrap();

        assert_eq!(updated.name, "Renamed");
    }

    #[tokio::test]
    async fn test_mutate_propagates_the_error_and_invalidates_nothing() {
        let (mut ctx, _board_id) = seeded_ctx().await;
        let absent_id = Uuid::new_v4();

        let result = mutate(&mut ctx, |c| {
            c.update_board_impl(absent_id, kanban_domain::BoardUpdate::default())
        });

        assert!(result.is_err());
        assert_eq!(ctx.list_boards().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_mutate_runs_the_operation_exactly_once() {
        let (mut ctx, board_id) = seeded_ctx().await;
        let calls = Cell::new(0u32);

        mutate(&mut ctx, |c| {
            calls.set(calls.get() + 1);
            c.update_board_impl(board_id, kanban_domain::BoardUpdate::default())
        })
        .unwrap();

        assert_eq!(calls.get(), 1);
    }
}
