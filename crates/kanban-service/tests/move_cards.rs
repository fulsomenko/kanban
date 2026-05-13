//! Integration tests for `KanbanContext::move_cards` / `move_cards_detailed` (KAN-428).
//!
//! Run against both `JsonDataStore` and `SqliteBackend` via a macro to catch any
//! backend-specific divergence. Position-computation logic is unit-tested in
//! `kanban_domain::card_lifecycle::tests::compute_move_positions_*`.

use kanban_domain::{Board, Card, Column};
use kanban_persistence_json::JsonFileStore;
use kanban_service::{
    json_backend::JsonDataStore, sqlite_backend::SqliteBackend, AppConfig, KanbanBackend,
    KanbanContext, KanbanOperations,
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

macro_rules! move_cards_tests {
    ($mod_name:ident, $open_ctx:expr) => {
        mod $mod_name {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn test_move_cards_appends_after_existing_cards_in_target_column() {
                let (mut ctx, _dir) = $open_ctx.await;
                let backend = ctx.backend();

                let mut board = Board::new("B".to_string(), Some("TST".to_string()));
                let col_from = Column::new(board.id, "From".to_string(), 0);
                let col_to = Column::new(board.id, "To".to_string(), 1);
                let col_to_id = col_to.id;

                let existing1 = Card::new(&mut board, col_to_id, "E1".to_string(), 0);
                let existing2 = Card::new(&mut board, col_to_id, "E2".to_string(), 1);
                let move1 = Card::new(&mut board, col_from.id, "M1".to_string(), 0);
                let move2 = Card::new(&mut board, col_from.id, "M2".to_string(), 1);
                let move1_id = move1.id;
                let move2_id = move2.id;
                backend.upsert_board(board).unwrap();
                backend.upsert_column(col_from).unwrap();
                backend.upsert_column(col_to).unwrap();
                backend.upsert_card(existing1).unwrap();
                backend.upsert_card(existing2).unwrap();
                backend.upsert_card(move1).unwrap();
                backend.upsert_card(move2).unwrap();

                ctx.move_cards(vec![move1_id, move2_id], col_to_id).unwrap();

                let m1 = backend.get_card(move1_id).unwrap().unwrap();
                let m2 = backend.get_card(move2_id).unwrap().unwrap();
                assert_eq!(m1.column_id, col_to_id);
                assert_eq!(m2.column_id, col_to_id);
                assert_eq!(m1.position, 2);
                assert_eq!(m2.position, 3);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_move_cards_within_same_column_excludes_moving_cards_from_base() {
                let (mut ctx, _dir) = $open_ctx.await;
                let backend = ctx.backend();

                let mut board = Board::new("B".to_string(), Some("TST".to_string()));
                let col = Column::new(board.id, "Col".to_string(), 0);
                let col_id = col.id;
                let card1 = Card::new(&mut board, col_id, "C1".to_string(), 0);
                let card2 = Card::new(&mut board, col_id, "C2".to_string(), 1);
                let card3 = Card::new(&mut board, col_id, "C3".to_string(), 2);
                let c1_id = card1.id;
                let c3_id = card3.id;
                backend.upsert_board(board).unwrap();
                backend.upsert_column(col).unwrap();
                backend.upsert_card(card1).unwrap();
                backend.upsert_card(card2).unwrap();
                backend.upsert_card(card3).unwrap();

                ctx.move_cards(vec![c1_id, c3_id], col_id).unwrap();

                let c1 = backend.get_card(c1_id).unwrap().unwrap();
                let c3 = backend.get_card(c3_id).unwrap().unwrap();
                assert_eq!(c1.position, 1, "first moved card should be at base(1) + 0");
                assert_eq!(c3.position, 2, "second moved card should be at base(1) + 1");
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_move_cards_exceeding_wip_limit_returns_error_and_rolls_back() {
                // Setup must go through ctx so the cards survive snapshot rollback —
                // direct backend writes are not in the command log and are wiped
                // back to the open()-time baseline on rollback (SQLite uses
                // indexed snapshots seeded at open).
                let (mut ctx, _dir) = $open_ctx.await;
                let backend = ctx.backend();

                let board = ctx
                    .create_board("B".into(), Some("TST".into()))
                    .unwrap();
                let src_col = ctx.create_column(board.id, "Src".into(), None).unwrap();
                let dst_col = ctx.create_column(board.id, "Dst".into(), None).unwrap();
                ctx.update_column(
                    dst_col.id,
                    kanban_domain::ColumnUpdate {
                        wip_limit: kanban_domain::FieldUpdate::Set(1),
                        ..Default::default()
                    },
                )
                .unwrap();
                let card1 = ctx
                    .create_card(board.id, src_col.id, "C1".into(), Default::default())
                    .unwrap();
                let card2 = ctx
                    .create_card(board.id, src_col.id, "C2".into(), Default::default())
                    .unwrap();
                let src_id = src_col.id;
                let dst_id = dst_col.id;

                let result = ctx.move_cards(vec![card1.id, card2.id], dst_id);
                assert!(result.is_err(), "moving 2 cards into limit=1 column must error");

                // Atomic: nothing moved (snapshot rollback)
                let c1 = backend.get_card(card1.id).unwrap().unwrap();
                let c2 = backend.get_card(card2.id).unwrap().unwrap();
                assert_eq!(c1.column_id, src_id);
                assert_eq!(c2.column_id, src_id);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_move_cards_detailed_reports_invalid_ids_as_failures() {
                let (mut ctx, _dir) = $open_ctx.await;
                let backend = ctx.backend();

                let mut board = Board::new("B".to_string(), Some("TST".to_string()));
                let col_from = Column::new(board.id, "From".to_string(), 0);
                let col_to = Column::new(board.id, "To".to_string(), 1);
                let col_to_id = col_to.id;
                let card = Card::new(&mut board, col_from.id, "C".to_string(), 0);
                let valid_id = card.id;
                let invalid_id = uuid::Uuid::new_v4();
                backend.upsert_board(board).unwrap();
                backend.upsert_column(col_from).unwrap();
                backend.upsert_column(col_to).unwrap();
                backend.upsert_card(card).unwrap();

                let result = ctx.move_cards_detailed(vec![valid_id, invalid_id], col_to_id);

                assert_eq!(result.succeeded, vec![valid_id]);
                assert_eq!(result.failed.len(), 1);
                assert_eq!(result.failed[0].id, invalid_id);

                let valid = backend.get_card(valid_id).unwrap().unwrap();
                assert_eq!(valid.column_id, col_to_id);
            }

            // KAN-428 followup: pin behaviour change — move_cards now errors and
            // rolls back when any input ID is unknown (previously, invalid IDs
            // were silently skipped by the removed MoveCards batch command).
            // Callers that need partial-success semantics use move_cards_detailed.
            #[tokio::test(flavor = "multi_thread")]
            async fn test_move_cards_with_invalid_id_errors_and_rolls_back() {
                let (mut ctx, _dir) = $open_ctx.await;
                let backend = ctx.backend();

                let board = ctx.create_board("B".into(), Some("TST".into())).unwrap();
                let col_from = ctx.create_column(board.id, "From".into(), None).unwrap();
                let col_to = ctx.create_column(board.id, "To".into(), None).unwrap();
                let card = ctx
                    .create_card(board.id, col_from.id, "C".into(), Default::default())
                    .unwrap();
                let invalid_id = uuid::Uuid::new_v4();
                let from_id = col_from.id;
                let to_id = col_to.id;

                let result = ctx.move_cards(vec![card.id, invalid_id], to_id);
                assert!(
                    result.is_err(),
                    "move_cards with any invalid ID must error after KAN-428"
                );

                let fetched = backend.get_card(card.id).unwrap().unwrap();
                assert_eq!(
                    fetched.column_id, from_id,
                    "valid card must not have moved (snapshot rollback)"
                );
            }

            // KAN-428 followup: the service-level batch WIP pre-check produces a
            // single WipLimitExceeded error before any per-card MoveCard runs,
            // restoring the original batch-level error semantics.
            #[tokio::test(flavor = "multi_thread")]
            async fn test_move_cards_exceeding_wip_limit_returns_single_batch_error() {
                let (mut ctx, _dir) = $open_ctx.await;

                let board = ctx.create_board("B".into(), Some("TST".into())).unwrap();
                let src_col = ctx.create_column(board.id, "Src".into(), None).unwrap();
                let dst_col = ctx.create_column(board.id, "Dst".into(), None).unwrap();
                ctx.update_column(
                    dst_col.id,
                    kanban_domain::ColumnUpdate {
                        wip_limit: kanban_domain::FieldUpdate::Set(1),
                        ..Default::default()
                    },
                )
                .unwrap();
                let c1 = ctx
                    .create_card(board.id, src_col.id, "C1".into(), Default::default())
                    .unwrap();
                let c2 = ctx
                    .create_card(board.id, src_col.id, "C2".into(), Default::default())
                    .unwrap();
                let c3 = ctx
                    .create_card(board.id, src_col.id, "C3".into(), Default::default())
                    .unwrap();

                let err = ctx
                    .move_cards(vec![c1.id, c2.id, c3.id], dst_col.id)
                    .unwrap_err();
                assert!(
                    err.is_wip_limit_exceeded(),
                    "expected WipLimitExceeded, got {err:?}"
                );
            }

            // KAN-428 followup: when invalid ids would *also* push the count
            // past a WIP limit, the pre-existence check should surface
            // not_found first — the WipLimitExceeded that the batch WIP
            // pre-check would otherwise produce is misleading.
            #[tokio::test(flavor = "multi_thread")]
            async fn test_move_cards_invalid_id_into_wip_limited_column_returns_not_found() {
                let (mut ctx, _dir) = $open_ctx.await;

                let board = ctx.create_board("B".into(), Some("TST".into())).unwrap();
                let src_col = ctx.create_column(board.id, "Src".into(), None).unwrap();
                let dst_col = ctx.create_column(board.id, "Dst".into(), None).unwrap();
                // Limit the destination column to 1 so any oversized batch would trip WIP.
                ctx.update_column(
                    dst_col.id,
                    kanban_domain::ColumnUpdate {
                        wip_limit: kanban_domain::FieldUpdate::Set(1),
                        ..Default::default()
                    },
                )
                .unwrap();
                let valid = ctx
                    .create_card(board.id, src_col.id, "C1".into(), Default::default())
                    .unwrap();
                let invalid_a = uuid::Uuid::new_v4();
                let invalid_b = uuid::Uuid::new_v4();

                let err = ctx
                    .move_cards(vec![valid.id, invalid_a, invalid_b], dst_col.id)
                    .unwrap_err();
                assert!(
                    err.is_not_found(),
                    "expected NotFound (precedes WIP pre-check), got {err:?}"
                );
            }
        }
    };
}

move_cards_tests!(json_backend, open_json_ctx());
move_cards_tests!(sqlite_backend, open_sqlite_ctx());
