//! Integration tests for `KanbanContext`'s `GraphOperations` impl (KAN-504).
//!
//! Exercises the four primitive methods (`add_card_edge`, `remove_card_edge`,
//! `list_card_edges_from`, `list_card_edges_to`) keyed by `CardEdgeType::Spawns`,
//! plus the convenience defaults inherited from the trait. Run against both
//! `JsonDataStore` and `SqliteBackend` via a macro to catch backend-specific
//! divergence; the underlying graph behavior (cycle detection, self-reference
//! rejection) is unit-tested in `kanban_domain::dependencies::card_graph`.

use kanban_domain::{Board, Card, CardEdgeType, Column, GraphOperations};
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

/// LegacyEdge kinds exercised by every kind-agnostic shape test.
/// Adding a fourth variant here causes every parameterised test to
/// pick it up automatically — that's the maintenance win this layout
/// is designed for.
const ALL_KINDS: [CardEdgeType; 3] = [
    CardEdgeType::Spawns,
    CardEdgeType::Blocks,
    CardEdgeType::RelatesTo,
];

/// Kind-discriminated adapters over the per-kind GraphOperations
/// methods so the parameterised tests below can loop over kinds
/// while the underlying API is per-kind typed. Argument order
/// (source, target, kind) matches the prior `ctx.add_card_edge(a, b,
/// kind)` shape so the sed swap stays a name-only rename.
fn add_by_kind(
    ctx: &mut KanbanContext,
    a: uuid::Uuid,
    b: uuid::Uuid,
    kind: CardEdgeType,
) -> kanban_domain::KanbanResult<()> {
    match kind {
        CardEdgeType::Spawns => ctx.add_spawns_edge(a, b),
        CardEdgeType::Blocks => ctx.add_blocks_edge(a, b, kanban_domain::Severity::default()),
        CardEdgeType::RelatesTo => {
            ctx.add_relates_edge(a, b, kanban_domain::RelatesKind::default())
        }
    }
}
fn remove_by_kind(
    ctx: &mut KanbanContext,
    a: uuid::Uuid,
    b: uuid::Uuid,
    kind: CardEdgeType,
) -> kanban_domain::KanbanResult<()> {
    match kind {
        CardEdgeType::Spawns => ctx.remove_spawns_edge(a, b),
        CardEdgeType::Blocks => ctx.remove_blocks_edge(a, b),
        CardEdgeType::RelatesTo => ctx.remove_relates_edge(a, b),
    }
}
fn list_from_by_kind(
    ctx: &KanbanContext,
    node: uuid::Uuid,
    kind: CardEdgeType,
) -> kanban_domain::KanbanResult<Vec<uuid::Uuid>> {
    match kind {
        CardEdgeType::Spawns => ctx.list_spawns_children(node),
        CardEdgeType::Blocks => ctx.list_blocked(node),
        CardEdgeType::RelatesTo => ctx.list_related(node),
    }
}
fn list_to_by_kind(
    ctx: &KanbanContext,
    node: uuid::Uuid,
    kind: CardEdgeType,
) -> kanban_domain::KanbanResult<Vec<uuid::Uuid>> {
    match kind {
        CardEdgeType::Spawns => ctx.list_spawns_parents(node),
        CardEdgeType::Blocks => ctx.list_blockers(node),
        CardEdgeType::RelatesTo => ctx.list_related(node),
    }
}

// --- Parameterised shape assertions ---
//
// Each helper exercises one piece of behaviour every edge kind must
// support. Tests pass `kind` in via a loop; assertion failures include
// the kind in the panic message so a single broken variant is easy to
// pinpoint.

fn assert_add_creates_visible_edge(
    ctx: &mut KanbanContext,
    kind: CardEdgeType,
    a: uuid::Uuid,
    b: uuid::Uuid,
) {
    add_by_kind(ctx, a, b, kind).unwrap();
    let from_a = list_from_by_kind(ctx, a, kind).unwrap();
    assert!(
        from_a.contains(&b),
        "{kind:?}: list_card_edges_from({a:?}) should contain {b:?}; got {from_a:?}"
    );
    let to_b = list_to_by_kind(ctx, b, kind).unwrap();
    assert!(
        to_b.contains(&a),
        "{kind:?}: list_card_edges_to({b:?}) should contain {a:?}; got {to_b:?}"
    );
}

fn assert_self_reference_rejected(ctx: &mut KanbanContext, kind: CardEdgeType, a: uuid::Uuid) {
    let err = add_by_kind(ctx, a, a, kind).unwrap_err();
    assert!(
        err.is_self_reference(),
        "{kind:?}: expected SelfReference, got {err:?}"
    );
}

fn assert_remove_clears_edge(
    ctx: &mut KanbanContext,
    kind: CardEdgeType,
    a: uuid::Uuid,
    b: uuid::Uuid,
) {
    add_by_kind(ctx, a, b, kind).unwrap();
    remove_by_kind(ctx, a, b, kind).unwrap();
    assert!(
        list_from_by_kind(ctx, a, kind).unwrap().is_empty(),
        "{kind:?}: list_from({a:?}) should be empty after remove"
    );
    assert!(
        list_to_by_kind(ctx, b, kind).unwrap().is_empty(),
        "{kind:?}: list_to({b:?}) should be empty after remove"
    );
}

fn assert_remove_nonexistent_errors(
    ctx: &mut KanbanContext,
    kind: CardEdgeType,
    a: uuid::Uuid,
    b: uuid::Uuid,
) {
    let err = remove_by_kind(ctx, a, b, kind).unwrap_err();
    assert!(
        err.is_edge_not_found(),
        "{kind:?}: expected EdgeNotFound, got {err:?}"
    );
}

fn assert_add_is_undoable(
    ctx: &mut KanbanContext,
    kind: CardEdgeType,
    a: uuid::Uuid,
    b: uuid::Uuid,
) {
    add_by_kind(ctx, a, b, kind).unwrap();
    assert!(ctx.can_undo(), "{kind:?}: can_undo() after add");
    ctx.undo().unwrap();
    assert!(
        list_from_by_kind(ctx, a, kind).unwrap().is_empty(),
        "{kind:?}: undo should remove the edge"
    );
}

/// Seed two distinct boards, one card on each. Returns the card ids for
/// use as cross-board graph nodes. The changeset advertises cross-board
/// parent/child as permitted; this helper backs the tests that exercise
/// it.
fn seed_two_boards_one_card_each(backend: &Arc<dyn KanbanBackend>) -> (uuid::Uuid, uuid::Uuid) {
    let mut board_a = Board::new("Board A".to_string(), Some("AAA".to_string()));
    let col_a = Column::new(board_a.id, "TODO".to_string(), 0);
    let card_a = Card::new(&mut board_a, col_a.id, "Card A".to_string(), 0);
    let card_a_id = card_a.id;

    let mut board_b = Board::new("Board B".to_string(), Some("BBB".to_string()));
    let col_b = Column::new(board_b.id, "TODO".to_string(), 0);
    let card_b = Card::new(&mut board_b, col_b.id, "Card B".to_string(), 0);
    let card_b_id = card_b.id;

    backend.upsert_board(board_a).unwrap();
    backend.upsert_column(col_a).unwrap();
    backend.upsert_card(card_a).unwrap();
    backend.upsert_board(board_b).unwrap();
    backend.upsert_column(col_b).unwrap();
    backend.upsert_card(card_b).unwrap();

    (card_a_id, card_b_id)
}

macro_rules! card_graph_tests {
    ($mod_name:ident, $open_ctx:expr) => {
        mod $mod_name {
            use super::*;

            // --- Parameterised shape contract ---
            //
            // One test per behaviour, looping every kind. Each iteration
            // gets a fresh context so state from one kind doesn't leak
            // into another. Adding a new edge kind to ALL_KINDS picks
            // up automatically below.

            #[tokio::test(flavor = "multi_thread")]
            async fn test_add_creates_visible_edge_for_all_kinds() {
                for kind in ALL_KINDS {
                    let (mut ctx, _dir) = $open_ctx.await;
                    let (a, b, _) = seed_three_cards(&ctx.backend());
                    assert_add_creates_visible_edge(&mut ctx, kind, a, b);
                }
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_self_reference_rejected_for_all_kinds() {
                for kind in ALL_KINDS {
                    let (mut ctx, _dir) = $open_ctx.await;
                    let (a, _, _) = seed_three_cards(&ctx.backend());
                    assert_self_reference_rejected(&mut ctx, kind, a);
                }
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_remove_clears_edge_for_all_kinds() {
                for kind in ALL_KINDS {
                    let (mut ctx, _dir) = $open_ctx.await;
                    let (a, b, _) = seed_three_cards(&ctx.backend());
                    assert_remove_clears_edge(&mut ctx, kind, a, b);
                }
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_remove_nonexistent_errors_for_all_kinds() {
                for kind in ALL_KINDS {
                    let (mut ctx, _dir) = $open_ctx.await;
                    let (a, b, _) = seed_three_cards(&ctx.backend());
                    assert_remove_nonexistent_errors(&mut ctx, kind, a, b);
                }
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_add_is_undoable_for_all_kinds() {
                for kind in ALL_KINDS {
                    let (mut ctx, _dir) = $open_ctx.await;
                    let (a, b, _) = seed_three_cards(&ctx.backend());
                    assert_add_is_undoable(&mut ctx, kind, a, b);
                }
            }

            // --- Multi-edge fanout (ParentOf-shaped: directional) ---

            #[tokio::test(flavor = "multi_thread")]
            async fn test_list_card_edges_from_parentof_returns_all_children() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, c1, c2) = seed_three_cards(&ctx.backend());

                add_by_kind(&mut ctx, parent_id, c1, CardEdgeType::Spawns).unwrap();
                add_by_kind(&mut ctx, parent_id, c2, CardEdgeType::Spawns).unwrap();

                let mut ids = list_from_by_kind(&ctx, parent_id, CardEdgeType::Spawns).unwrap();
                ids.sort();
                let mut expected = vec![c1, c2];
                expected.sort();
                assert_eq!(ids, expected);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_list_card_edges_to_parentof_returns_all_parents() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (p1, p2, child_id) = seed_three_cards(&ctx.backend());

                add_by_kind(&mut ctx, p1, child_id, CardEdgeType::Spawns).unwrap();
                add_by_kind(&mut ctx, p2, child_id, CardEdgeType::Spawns).unwrap();

                let mut ids = list_to_by_kind(&ctx, child_id, CardEdgeType::Spawns).unwrap();
                ids.sort();
                let mut expected = vec![p1, p2];
                expected.sort();
                assert_eq!(ids, expected);
            }

            // --- Kind-specific: DAG vs undirected semantics ---

            #[tokio::test(flavor = "multi_thread")]
            async fn test_parentof_cycle_rejected() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (a, b, _) = seed_three_cards(&ctx.backend());

                add_by_kind(&mut ctx, a, b, CardEdgeType::Spawns).unwrap();
                let err = add_by_kind(&mut ctx, b, a, CardEdgeType::Spawns).unwrap_err();
                assert!(
                    err.is_cycle_detected(),
                    "expected CycleDetected, got {err:?}"
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_blocks_cycle_rejected() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (a, b, c) = seed_three_cards(&ctx.backend());

                add_by_kind(&mut ctx, a, b, CardEdgeType::Blocks).unwrap();
                add_by_kind(&mut ctx, b, c, CardEdgeType::Blocks).unwrap();
                let err = add_by_kind(&mut ctx, c, a, CardEdgeType::Blocks).unwrap_err();
                assert!(
                    err.is_cycle_detected(),
                    "expected CycleDetected, got {err:?}"
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_relates_to_cycle_permitted() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (a, b, c) = seed_three_cards(&ctx.backend());

                add_by_kind(&mut ctx, a, b, CardEdgeType::RelatesTo).unwrap();
                add_by_kind(&mut ctx, b, c, CardEdgeType::RelatesTo).unwrap();
                // Undirected — closing the triangle is not a cycle violation.
                add_by_kind(&mut ctx, c, a, CardEdgeType::RelatesTo).unwrap();
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_relates_to_is_bidirectional_from_both_endpoints() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (a, b, _) = seed_three_cards(&ctx.backend());

                add_by_kind(&mut ctx, a, b, CardEdgeType::RelatesTo).unwrap();

                // Undirected: list_from and list_to both return the neighbour
                // from either endpoint. This is the defining property that
                // distinguishes RelatesTo from the two DAG kinds.
                assert_eq!(
                    list_from_by_kind(&ctx, a, CardEdgeType::RelatesTo).unwrap(),
                    vec![b]
                );
                assert_eq!(
                    list_to_by_kind(&ctx, a, CardEdgeType::RelatesTo).unwrap(),
                    vec![b]
                );
                assert_eq!(
                    list_from_by_kind(&ctx, b, CardEdgeType::RelatesTo).unwrap(),
                    vec![a]
                );
                assert_eq!(
                    list_to_by_kind(&ctx, b, CardEdgeType::RelatesTo).unwrap(),
                    vec![a]
                );
            }

            // --- Convenience defaults ---

            #[tokio::test(flavor = "multi_thread")]
            async fn test_set_parent_creates_edge_visible_via_list_card_children() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, child_id, _) = seed_three_cards(&ctx.backend());

                ctx.set_parent(child_id, parent_id).unwrap();

                let children = ctx.list_card_children(parent_id).unwrap();
                assert_eq!(children.len(), 1);
                assert_eq!(children[0], child_id);

                let parents = ctx.list_card_parents(child_id).unwrap();
                assert_eq!(parents.len(), 1);
                assert_eq!(parents[0], parent_id);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_list_card_parents_matches_list_card_edges_to_parentof() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, child_id, _) = seed_three_cards(&ctx.backend());

                ctx.set_parent(child_id, parent_id).unwrap();

                let convenience: Vec<uuid::Uuid> = ctx.list_card_parents(child_id).unwrap();
                let primitive: Vec<uuid::Uuid> =
                    list_to_by_kind(&ctx, child_id, CardEdgeType::Spawns).unwrap();
                assert_eq!(convenience, primitive);
            }

            // --- Cross-board parent/child ---
            //
            // Pins the changeset claim that cross-board parent/child is
            // permitted. The graph is keyed by card UUIDs only — board
            // identity is not consulted, and the edge persists in both
            // sub-graphs equally.

            #[tokio::test(flavor = "multi_thread")]
            async fn test_set_parent_across_boards_is_permitted_and_visible_from_both_sides() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_on_a, child_on_b) = seed_two_boards_one_card_each(&ctx.backend());

                ctx.set_parent(child_on_b, parent_on_a).unwrap();

                let children = ctx.list_card_children(parent_on_a).unwrap();
                assert_eq!(children, vec![child_on_b], "child visible from parent side");

                let parents = ctx.list_card_parents(child_on_b).unwrap();
                assert_eq!(parents, vec![parent_on_a], "parent visible from child side");
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_set_parent_across_boards_persists_to_backend_round_trip() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_on_a, child_on_b) = seed_two_boards_one_card_each(&ctx.backend());

                ctx.set_parent(child_on_b, parent_on_a).unwrap();
                ctx.save().await.unwrap();

                // Re-read graph from the backend rather than the in-memory
                // KanbanContext to confirm the edge survives serialization.
                let graph = ctx.backend().get_graph().unwrap();
                assert_eq!(graph.parents(child_on_b), vec![parent_on_a]);
                assert_eq!(graph.children(parent_on_a), vec![child_on_b]);
            }

            // --- Card-existence validation ---
            //
            // The service layer rejects edge mutations against unknown
            // card ids before mutating the graph. Without this guard
            // a stale or fabricated UUID would silently add a dangling
            // edge whose endpoints reference no live card — a data
            // integrity hole the CLI's identifier-resolution layer
            // doesn't close (raw UUIDs are parsed but not looked up).

            #[tokio::test(flavor = "multi_thread")]
            async fn test_add_card_edge_with_unknown_source_returns_card_not_found() {
                for kind in ALL_KINDS {
                    let (mut ctx, _dir) = $open_ctx.await;
                    let (b, _, _) = seed_three_cards(&ctx.backend());
                    let phantom = uuid::Uuid::new_v4();
                    let err = add_by_kind(&mut ctx, phantom, b, kind).unwrap_err();
                    assert!(
                        err.is_not_found(),
                        "{kind:?}: expected NotFound for phantom source; got {err:?}"
                    );
                    assert!(
                        err.to_string().contains(&phantom.to_string()),
                        "{kind:?}: error must name the missing id; got {err:?}"
                    );
                }
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_add_card_edge_with_unknown_target_returns_card_not_found() {
                for kind in ALL_KINDS {
                    let (mut ctx, _dir) = $open_ctx.await;
                    let (a, _, _) = seed_three_cards(&ctx.backend());
                    let phantom = uuid::Uuid::new_v4();
                    let err = add_by_kind(&mut ctx, a, phantom, kind).unwrap_err();
                    assert!(
                        err.is_not_found(),
                        "{kind:?}: expected NotFound for phantom target; got {err:?}"
                    );
                    assert!(
                        err.to_string().contains(&phantom.to_string()),
                        "{kind:?}: error must name the missing id; got {err:?}"
                    );
                }
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_remove_card_edge_with_unknown_source_returns_card_not_found() {
                for kind in ALL_KINDS {
                    let (mut ctx, _dir) = $open_ctx.await;
                    let (b, _, _) = seed_three_cards(&ctx.backend());
                    let phantom = uuid::Uuid::new_v4();
                    let err = remove_by_kind(&mut ctx, phantom, b, kind).unwrap_err();
                    assert!(
                        err.is_not_found(),
                        "{kind:?}: expected NotFound for phantom source; got {err:?}"
                    );
                }
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_remove_card_edge_with_unknown_target_returns_card_not_found() {
                for kind in ALL_KINDS {
                    let (mut ctx, _dir) = $open_ctx.await;
                    let (a, _, _) = seed_three_cards(&ctx.backend());
                    let phantom = uuid::Uuid::new_v4();
                    let err = remove_by_kind(&mut ctx, a, phantom, kind).unwrap_err();
                    assert!(
                        err.is_not_found(),
                        "{kind:?}: expected NotFound for phantom target; got {err:?}"
                    );
                }
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_list_card_edges_from_with_unknown_node_returns_card_not_found() {
                // Listing edges of a phantom UUID must surface NotFound,
                // symmetric with add/remove. Previously the list paths
                // silently returned an empty Vec, which hides graph
                // corruption from any caller that resolved a stale UUID.
                for kind in ALL_KINDS {
                    let (ctx, _dir) = $open_ctx.await;
                    let phantom = uuid::Uuid::new_v4();
                    let err = list_from_by_kind(&ctx, phantom, kind).unwrap_err();
                    assert!(
                        err.is_not_found(),
                        "{kind:?}: expected NotFound; got {err:?}"
                    );
                    assert!(
                        err.to_string().contains(&phantom.to_string()),
                        "{kind:?}: error must name the missing id; got {err:?}"
                    );
                }
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_list_card_edges_to_with_unknown_node_returns_card_not_found() {
                for kind in ALL_KINDS {
                    let (ctx, _dir) = $open_ctx.await;
                    let phantom = uuid::Uuid::new_v4();
                    let err = list_to_by_kind(&ctx, phantom, kind).unwrap_err();
                    assert!(
                        err.is_not_found(),
                        "{kind:?}: expected NotFound; got {err:?}"
                    );
                }
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_remove_parent_across_boards_clears_edge() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_on_a, child_on_b) = seed_two_boards_one_card_each(&ctx.backend());

                ctx.set_parent(child_on_b, parent_on_a).unwrap();
                ctx.remove_parent(child_on_b, parent_on_a).unwrap();

                assert!(ctx.list_card_children(parent_on_a).unwrap().is_empty());
                assert!(ctx.list_card_parents(child_on_b).unwrap().is_empty());
            }

            // --- Atomic multi-child batch (add_children / remove_children) ---
            //
            // The CLI's `relation add P C1 C2 C3` invocation must
            // commit either every child or none, so a mid-list failure
            // does not leak a partial state into in-memory or on-disk
            // form. These tests pin the all-or-nothing contract at the
            // service layer for both backends.

            #[tokio::test(flavor = "multi_thread")]
            async fn test_add_children_attaches_every_child_atomically() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, c1, c2) = seed_three_cards(&ctx.backend());

                ctx.add_children(parent_id, vec![c1, c2]).unwrap();

                let mut children = ctx.list_card_children(parent_id).unwrap();
                children.sort();
                let mut expected = vec![c1, c2];
                expected.sort();
                assert_eq!(children, expected);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_add_children_rolls_back_when_a_later_child_creates_a_cycle() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, c1, c2) = seed_three_cards(&ctx.backend());

                // Seed parent->c1->c2 so closing parent into c2 cycles.
                ctx.add_child(parent_id, c1).unwrap();
                ctx.add_child(c1, c2).unwrap();

                // Batch: attach c1 AND parent as children of c2.
                // First would succeed in isolation (c1 already child of c2);
                // second would create cycle parent->c1->c2->parent.
                let err = ctx
                    .add_children(c2, vec![c1, parent_id])
                    .expect_err("batch must fail on cycle");
                assert!(err.is_cycle_detected(), "expected cycle, got {err:?}");

                // Nothing new attached: c2 must still have only its prior children.
                let children = ctx.list_card_children(c2).unwrap();
                assert!(
                    !children.contains(&c1),
                    "c1 must not be re-attached as a child of c2 after rollback"
                );
                assert!(
                    !children.contains(&parent_id),
                    "parent must not be attached as a child of c2 after rollback"
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_add_children_rejects_unknown_child_without_partial_attach() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, c1, _c2) = seed_three_cards(&ctx.backend());
                let phantom = uuid::Uuid::new_v4();

                let err = ctx
                    .add_children(parent_id, vec![c1, phantom])
                    .expect_err("batch must fail when any child is unknown");
                assert!(err.is_not_found(), "expected NotFound, got {err:?}");
                assert!(
                    err.to_string().contains(&phantom.to_string()),
                    "error must name the missing id; got {err:?}"
                );

                let children = ctx.list_card_children(parent_id).unwrap();
                assert!(
                    children.is_empty(),
                    "no children should be attached when validation fails; got {children:?}"
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_remove_children_detaches_every_child_atomically() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, c1, c2) = seed_three_cards(&ctx.backend());

                ctx.add_children(parent_id, vec![c1, c2]).unwrap();
                ctx.remove_children(parent_id, vec![c1, c2]).unwrap();

                assert!(ctx.list_card_children(parent_id).unwrap().is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_remove_children_rolls_back_when_an_edge_is_missing() {
                let (mut ctx, _dir) = $open_ctx.await;
                let (parent_id, c1, c2) = seed_three_cards(&ctx.backend());

                // Only c1 is attached; c2 is not.
                ctx.add_child(parent_id, c1).unwrap();

                let err = ctx
                    .remove_children(parent_id, vec![c1, c2])
                    .expect_err("batch must fail when any edge is missing");
                assert!(
                    err.is_edge_not_found(),
                    "expected EdgeNotFound, got {err:?}"
                );

                // c1 must still be attached: the partial remove was rolled back.
                let children = ctx.list_card_children(parent_id).unwrap();
                assert_eq!(
                    children,
                    vec![c1],
                    "rollback must restore the pre-batch state"
                );
            }
        }
    };
}

card_graph_tests!(json, open_json_ctx());
card_graph_tests!(sqlite, open_sqlite_ctx());
