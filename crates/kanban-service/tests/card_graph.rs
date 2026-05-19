//! Integration tests for `KanbanContext`'s `GraphOperations` impl (KAN-504).
//!
//! Exercises the four primitive methods (`add_card_edge`, `remove_card_edge`,
//! `list_card_edges_from`, `list_card_edges_to`) keyed by `CardEdgeType::ParentOf`,
//! plus the convenience defaults inherited from the trait. Run against both
//! `JsonDataStore` and `SqliteBackend` via a macro to catch backend-specific
//! divergence; the underlying graph behavior (cycle detection, self-reference
//! rejection) is unit-tested in `kanban_domain::dependencies::card_graph`.

use kanban_domain::{Board, Card, CardEdgeType, Column, GraphOperations, KanbanOperations};
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

/// Seed a board with a single column and three cards. Returns the card ids
/// for use as graph nodes in tests.
fn seed_three_cards(backend: &Arc<dyn KanbanBackend>) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let mut board = Board::new("Test".to_string(), Some("TST".to_string()));
    let col = Column::new(board.id, "TODO".to_string(), 0);
    let col_id = col.id;
    let a = Card::new(&mut board, col_id, "A".to_string(), 0);
    let b = Card::new(&mut board, col_id, "B".to_string(), 1);
    let c = Card::new(&mut board, col_id, "C".to_string(), 2);
    let (a_id, b_id, c_id) = (a.id, b.id, c.id);
    backend.upsert_board(board).unwrap();
    backend.upsert_column(col).unwrap();
    backend.upsert_card(a).unwrap();
    backend.upsert_card(b).unwrap();
    backend.upsert_card(c).unwrap();
    (a_id, b_id, c_id)
}

macro_rules! card_graph_tests {
    ($mod_name:ident, $open_ctx:expr) => {
        mod $mod_name {
            use super::*;

            // --- Primitive API ---

            #[tokio::test(flavor = "multi_thread")]
            async fn test_add_card_edge_parentof_creates_directed_edge() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, child_id, _) = seed_three_cards(&ctx.backend());

                ctx.add_card_edge(parent_id, child_id, CardEdgeType::ParentOf)
                    .unwrap();

                let children = ctx
                    .list_card_edges_from(parent_id, CardEdgeType::ParentOf)
                    .unwrap();
                assert_eq!(children.len(), 1);
                assert_eq!(children[0].id, child_id);

                let parents = ctx
                    .list_card_edges_to(child_id, CardEdgeType::ParentOf)
                    .unwrap();
                assert_eq!(parents.len(), 1);
                assert_eq!(parents[0].id, parent_id);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_add_card_edge_parentof_self_reference_returns_error() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (a, _, _) = seed_three_cards(&ctx.backend());

                let err = ctx
                    .add_card_edge(a, a, CardEdgeType::ParentOf)
                    .unwrap_err();
                assert!(
                    err.is_self_reference(),
                    "expected SelfReference, got {:?}",
                    err
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_add_card_edge_parentof_cycle_returns_error() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (a, b, _) = seed_three_cards(&ctx.backend());

                ctx.add_card_edge(a, b, CardEdgeType::ParentOf).unwrap();
                let err = ctx
                    .add_card_edge(b, a, CardEdgeType::ParentOf)
                    .unwrap_err();
                assert!(
                    err.is_cycle_detected(),
                    "expected CycleDetected, got {:?}",
                    err
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_remove_card_edge_parentof_removes_edge() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, child_id, _) = seed_three_cards(&ctx.backend());

                ctx.add_card_edge(parent_id, child_id, CardEdgeType::ParentOf)
                    .unwrap();
                ctx.remove_card_edge(parent_id, child_id, CardEdgeType::ParentOf)
                    .unwrap();

                assert!(ctx
                    .list_card_edges_from(parent_id, CardEdgeType::ParentOf)
                    .unwrap()
                    .is_empty());
                assert!(ctx
                    .list_card_edges_to(child_id, CardEdgeType::ParentOf)
                    .unwrap()
                    .is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_remove_card_edge_parentof_nonexistent_edge_returns_error() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (a, b, _) = seed_three_cards(&ctx.backend());

                let err = ctx
                    .remove_card_edge(a, b, CardEdgeType::ParentOf)
                    .unwrap_err();
                assert!(err.is_edge_not_found(), "expected EdgeNotFound, got {:?}", err);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_list_card_edges_from_parentof_returns_children_summaries() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, c1, c2) = seed_three_cards(&ctx.backend());

                ctx.add_card_edge(parent_id, c1, CardEdgeType::ParentOf)
                    .unwrap();
                ctx.add_card_edge(parent_id, c2, CardEdgeType::ParentOf)
                    .unwrap();

                let children = ctx
                    .list_card_edges_from(parent_id, CardEdgeType::ParentOf)
                    .unwrap();
                let mut ids: Vec<uuid::Uuid> = children.iter().map(|s| s.id).collect();
                ids.sort();
                let mut expected = vec![c1, c2];
                expected.sort();
                assert_eq!(ids, expected);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_list_card_edges_to_parentof_returns_parents_summaries() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (p1, p2, child_id) = seed_three_cards(&ctx.backend());

                ctx.add_card_edge(p1, child_id, CardEdgeType::ParentOf)
                    .unwrap();
                ctx.add_card_edge(p2, child_id, CardEdgeType::ParentOf)
                    .unwrap();

                let parents = ctx
                    .list_card_edges_to(child_id, CardEdgeType::ParentOf)
                    .unwrap();
                let mut ids: Vec<uuid::Uuid> = parents.iter().map(|s| s.id).collect();
                ids.sort();
                let mut expected = vec![p1, p2];
                expected.sort();
                assert_eq!(ids, expected);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_add_card_edge_is_undoable_through_undo_stack() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, child_id, _) = seed_three_cards(&ctx.backend());

                ctx.add_card_edge(parent_id, child_id, CardEdgeType::ParentOf)
                    .unwrap();
                assert_eq!(
                    ctx.list_card_edges_from(parent_id, CardEdgeType::ParentOf)
                        .unwrap()
                        .len(),
                    1
                );

                assert!(ctx.can_undo());
                ctx.undo().unwrap();
                assert!(
                    ctx.list_card_edges_from(parent_id, CardEdgeType::ParentOf)
                        .unwrap()
                        .is_empty(),
                    "undo should remove the parent edge"
                );
            }

            // --- Convenience defaults ---

            #[tokio::test(flavor = "multi_thread")]
            async fn test_set_card_parent_creates_edge_visible_via_list_card_children() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, child_id, _) = seed_three_cards(&ctx.backend());

                ctx.set_card_parent(child_id, parent_id).unwrap();

                let children = ctx.list_card_children(parent_id).unwrap();
                assert_eq!(children.len(), 1);
                assert_eq!(children[0].id, child_id);

                let parents = ctx.list_card_parents(child_id).unwrap();
                assert_eq!(parents.len(), 1);
                assert_eq!(parents[0].id, parent_id);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_list_card_parents_matches_list_card_edges_to_parentof() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, child_id, _) = seed_three_cards(&ctx.backend());

                ctx.set_card_parent(child_id, parent_id).unwrap();

                let convenience: Vec<uuid::Uuid> = ctx
                    .list_card_parents(child_id)
                    .unwrap()
                    .into_iter()
                    .map(|s| s.id)
                    .collect();
                let primitive: Vec<uuid::Uuid> = ctx
                    .list_card_edges_to(child_id, CardEdgeType::ParentOf)
                    .unwrap()
                    .into_iter()
                    .map(|s| s.id)
                    .collect();
                assert_eq!(convenience, primitive);
            }
        }
    };
}

card_graph_tests!(json, open_json_ctx());
card_graph_tests!(sqlite, open_sqlite_ctx());
