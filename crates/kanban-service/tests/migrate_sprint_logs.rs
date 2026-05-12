//! Integration tests for `KanbanContext::migrate_sprint_logs` (KAN-430).
//!
//! Run against both `JsonDataStore` and `SqliteBackend` via a macro to catch any
//! backend-specific divergence. The pure migration logic itself is unit-tested
//! in `kanban_domain::card_lifecycle::tests`.

use kanban_domain::{Board, Card, Column, Sprint};
use kanban_persistence_json::JsonFileStore;
use kanban_service::{
    json_backend::JsonDataStore, sqlite_backend::SqliteBackend, AppConfig, KanbanBackend,
    KanbanContext,
};
use std::sync::Arc;
use tempfile::tempdir;

async fn open_json_ctx() -> (KanbanContext, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.json");
    let backend: Arc<dyn KanbanBackend> =
        Arc::new(JsonDataStore::new(Arc::new(JsonFileStore::new(&path))));
    let ctx = KanbanContext::open(backend, AppConfig::default())
        .await
        .unwrap();
    (ctx, dir)
}

async fn open_sqlite_ctx() -> (KanbanContext, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.sqlite");
    let backend: Arc<dyn KanbanBackend> =
        Arc::new(SqliteBackend::open(path.to_str().unwrap()).await.unwrap());
    let ctx = KanbanContext::open(backend, AppConfig::default())
        .await
        .unwrap();
    (ctx, dir)
}

macro_rules! migrate_sprint_logs_tests {
    ($mod_name:ident, $open_ctx:expr) => {
        mod $mod_name {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn test_migrate_sprint_logs_backfills_card_with_sprint_id_and_empty_logs() {
                let (mut ctx, _dir) = $open_ctx.await;
                let backend = ctx.backend();

                let mut board = Board::new("B".to_string(), Some("TST".to_string()));
                let col = Column::new(board.id, "Col".to_string(), 0);
                let sprint = Sprint::new(board.id, 1, None, Some("Alpha".to_string()));
                let sprint_id = sprint.id;
                let mut card = Card::new(&mut board, col.id, "Card".to_string(), 0);
                let card_id = card.id;
                card.sprint_id = Some(sprint_id);
                assert!(card.sprint_logs.is_empty());
                backend.upsert_board(board).unwrap();
                backend.upsert_column(col).unwrap();
                backend.upsert_sprint(sprint).unwrap();
                backend.upsert_card(card).unwrap();

                let migrated = ctx.migrate_sprint_logs().unwrap();
                assert_eq!(migrated, 1);

                let card = backend.get_card(card_id).unwrap().unwrap();
                assert_eq!(
                    card.sprint_logs.len(),
                    1,
                    "sprint log should be backfilled for card with sprint_id but empty logs"
                );
                assert_eq!(card.sprint_logs[0].sprint_number, 1);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_migrate_sprint_logs_no_op_when_nothing_to_migrate() {
                let (mut ctx, _dir) = $open_ctx.await;
                let backend = ctx.backend();

                let mut board = Board::new("B".to_string(), Some("TST".to_string()));
                let col = Column::new(board.id, "Col".to_string(), 0);
                let card = Card::new(&mut board, col.id, "Card".to_string(), 0);
                backend.upsert_board(board).unwrap();
                backend.upsert_column(col).unwrap();
                backend.upsert_card(card).unwrap();

                let migrated = ctx.migrate_sprint_logs().unwrap();
                assert_eq!(
                    migrated, 0,
                    "migrate_sprint_logs should report zero when no card needs backfilling"
                );
            }
        }
    };
}

migrate_sprint_logs_tests!(json_backend, open_json_ctx());
migrate_sprint_logs_tests!(sqlite_backend, open_sqlite_ctx());
