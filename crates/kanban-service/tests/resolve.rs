//! Integration tests for the `KanbanOperations` default resolver methods
//! (`resolve_board_id`, `resolve_column_id`, `resolve_sprint_id`, `resolve_card_id`,
//! and their `_global` / batch variants). Covered:
//!   - UUID fast path
//!   - name fast path (case-insensitive)
//!   - sprint number fast path
//!   - miss → message lists alternatives
//!   - ambiguity → message lists conflicts
//!   - batch resolvers aggregate failures
//!   - `require_same_board` accepts homogeneous batches and rejects cross-board ones

use kanban_persistence_json::JsonFileStore;
use kanban_service::{
    json_backend::JsonDataStore, AppConfig, KanbanBackend, KanbanContext, KanbanOperations,
};
use std::sync::Arc;
use tempfile::tempdir;
use uuid::Uuid;

async fn open_ctx() -> (KanbanContext, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.json");
    let backend: Arc<dyn KanbanBackend> =
        Arc::new(JsonDataStore::new(Arc::new(JsonFileStore::new(&path))));
    let ctx = KanbanContext::open(backend, AppConfig::default())
        .await
        .unwrap();
    (ctx, dir)
}

// ---------- resolve_board_id ----------

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_board_id_by_name_case_insensitive() {
    let (mut ctx, _dir) = open_ctx().await;
    let board = ctx
        .create_board("Kanban".into(), Some("KAN".into()))
        .unwrap();
    assert_eq!(ctx.resolve_board_id("Kanban").unwrap(), board.id);
    assert_eq!(ctx.resolve_board_id("kanban").unwrap(), board.id);
    assert_eq!(ctx.resolve_board_id("KANBAN").unwrap(), board.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_board_id_by_uuid_fast_path() {
    let (mut ctx, _dir) = open_ctx().await;
    let board = ctx
        .create_board("Kanban".into(), Some("KAN".into()))
        .unwrap();
    assert_eq!(
        ctx.resolve_board_id(&board.id.to_string()).unwrap(),
        board.id
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_board_id_not_found_lists_available() {
    let (mut ctx, _dir) = open_ctx().await;
    ctx.create_board("Kanban".into(), Some("KAN".into()))
        .unwrap();
    ctx.create_board("Personal".into(), Some("PER".into()))
        .unwrap();
    let err = ctx.resolve_board_id("missing").unwrap_err().to_string();
    assert!(err.contains("'missing' not found"), "got: {err}");
    assert!(err.contains("Kanban"), "got: {err}");
    assert!(err.contains("Personal"), "got: {err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_board_id_ambiguous_returns_uuid_hint() {
    let (mut ctx, _dir) = open_ctx().await;
    ctx.create_board("Shared".into(), None).unwrap();
    ctx.create_board("Shared".into(), None).unwrap();
    let err = ctx.resolve_board_id("shared").unwrap_err().to_string();
    assert!(err.contains("ambiguous"), "got: {err}");
    assert!(err.contains("UUID"), "got: {err}");
}

// ---------- resolve_column_id ----------

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_column_id_by_name_within_board() {
    let (mut ctx, _dir) = open_ctx().await;
    let board = ctx.create_board("B".into(), None).unwrap();
    let col = ctx.create_column(board.id, "TODO".into(), None).unwrap();
    assert_eq!(ctx.resolve_column_id("todo", board.id).unwrap(), col.id);
    assert_eq!(ctx.resolve_column_id("TODO", board.id).unwrap(), col.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_column_id_not_found_lists_columns_on_board() {
    let (mut ctx, _dir) = open_ctx().await;
    let board = ctx.create_board("B".into(), None).unwrap();
    ctx.create_column(board.id, "TODO".into(), None).unwrap();
    ctx.create_column(board.id, "Doing".into(), None).unwrap();
    let err = ctx
        .resolve_column_id("nope", board.id)
        .unwrap_err()
        .to_string();
    assert!(err.contains("not found"), "got: {err}");
    assert!(err.contains("TODO"), "got: {err}");
    assert!(err.contains("Doing"), "got: {err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_column_id_global_finds_unique_across_boards() {
    let (mut ctx, _dir) = open_ctx().await;
    let board_a = ctx.create_board("A".into(), None).unwrap();
    let board_b = ctx.create_board("B".into(), None).unwrap();
    let col_a = ctx
        .create_column(board_a.id, "Backlog".into(), None)
        .unwrap();
    ctx.create_column(board_b.id, "Doing".into(), None).unwrap();
    assert_eq!(ctx.resolve_column_id_global("backlog").unwrap(), col_a.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_column_id_global_ambiguous_lists_board_names() {
    let (mut ctx, _dir) = open_ctx().await;
    let board_a = ctx.create_board("A".into(), None).unwrap();
    let board_b = ctx.create_board("B".into(), None).unwrap();
    ctx.create_column(board_a.id, "TODO".into(), None).unwrap();
    ctx.create_column(board_b.id, "TODO".into(), None).unwrap();
    let err = ctx
        .resolve_column_id_global("todo")
        .unwrap_err()
        .to_string();
    assert!(err.contains("ambiguous"), "got: {err}");
    assert!(err.contains("'A'"), "got: {err}");
    assert!(err.contains("'B'"), "got: {err}");
}

// ---------- resolve_sprint_id ----------

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_sprint_id_by_name() {
    let (mut ctx, _dir) = open_ctx().await;
    let board = ctx.create_board("B".into(), None).unwrap();
    let sprint = ctx
        .create_sprint(board.id, None, Some("yarara-release".into()))
        .unwrap();
    assert_eq!(
        ctx.resolve_sprint_id("yarara-release", board.id).unwrap(),
        sprint.id
    );
    assert_eq!(
        ctx.resolve_sprint_id("YARARA-RELEASE", board.id).unwrap(),
        sprint.id
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_sprint_id_by_number() {
    let (mut ctx, _dir) = open_ctx().await;
    let board = ctx.create_board("B".into(), None).unwrap();
    let sprint = ctx
        .create_sprint(board.id, None, Some("alpha".into()))
        .unwrap();
    let n = sprint.sprint_number;
    assert_eq!(
        ctx.resolve_sprint_id(&n.to_string(), board.id).unwrap(),
        sprint.id
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_sprint_id_global_finds_unique() {
    let (mut ctx, _dir) = open_ctx().await;
    let board_a = ctx.create_board("A".into(), None).unwrap();
    let board_b = ctx.create_board("B".into(), None).unwrap();
    let _s_a = ctx
        .create_sprint(board_a.id, None, Some("alpha".into()))
        .unwrap();
    let s_b = ctx
        .create_sprint(board_b.id, None, Some("beta".into()))
        .unwrap();
    assert_eq!(ctx.resolve_sprint_id_global("beta").unwrap(), s_b.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_sprint_id_global_ambiguous_number_lists_boards() {
    let (mut ctx, _dir) = open_ctx().await;
    let board_a = ctx.create_board("A".into(), None).unwrap();
    let board_b = ctx.create_board("B".into(), None).unwrap();
    let _ = ctx.create_sprint(board_a.id, None, None).unwrap();
    let _ = ctx.create_sprint(board_b.id, None, None).unwrap();
    let err = ctx.resolve_sprint_id_global("1").unwrap_err().to_string();
    assert!(err.contains("ambiguous"), "got: {err}");
    assert!(err.contains("'A'"), "got: {err}");
    assert!(err.contains("'B'"), "got: {err}");
}

// ---------- resolve_card_id / resolve_card_ids ----------

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_card_id_by_identifier() {
    let (mut ctx, _dir) = open_ctx().await;
    let board = ctx.create_board("B".into(), Some("KAN".into())).unwrap();
    let col = ctx.create_column(board.id, "TODO".into(), None).unwrap();
    let card = ctx
        .create_card(board.id, col.id, "Hello".into(), Default::default())
        .unwrap();
    let ident = format!("KAN-{}", card.card_number);
    assert_eq!(ctx.resolve_card_id(&ident).unwrap(), card.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_card_id_not_found_returns_validation_error() {
    let (ctx, _dir) = open_ctx().await;
    let err = ctx.resolve_card_id("KAN-999").unwrap_err();
    assert!(err.is_validation(), "got: {err}");
    assert!(err.to_string().contains("Card not found"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_card_ids_aggregates_failures() {
    let (mut ctx, _dir) = open_ctx().await;
    let board = ctx.create_board("B".into(), Some("KAN".into())).unwrap();
    let col = ctx.create_column(board.id, "TODO".into(), None).unwrap();
    let card = ctx
        .create_card(board.id, col.id, "ok".into(), Default::default())
        .unwrap();
    let ident_ok = format!("KAN-{}", card.card_number);
    let raws: Vec<String> = vec![ident_ok.clone(), "KAN-999".into(), "KAN-998".into()];
    let err = ctx.resolve_card_ids(&raws).unwrap_err().to_string();
    assert!(err.contains("2 card"), "got: {err}");
    assert!(err.contains("KAN-999"), "got: {err}");
    assert!(err.contains("KAN-998"), "got: {err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resolve_card_ids_success_returns_all_uuids() {
    let (mut ctx, _dir) = open_ctx().await;
    let board = ctx.create_board("B".into(), Some("KAN".into())).unwrap();
    let col = ctx.create_column(board.id, "TODO".into(), None).unwrap();
    let c1 = ctx
        .create_card(board.id, col.id, "one".into(), Default::default())
        .unwrap();
    let c2 = ctx
        .create_card(board.id, col.id, "two".into(), Default::default())
        .unwrap();
    let raws: Vec<String> = vec![format!("KAN-{}", c1.card_number), c2.id.to_string()];
    let ids = ctx.resolve_card_ids(&raws).unwrap();
    assert_eq!(ids, vec![c1.id, c2.id]);
}

// ---------- require_same_board ----------

#[tokio::test(flavor = "multi_thread")]
async fn test_require_same_board_accepts_homogeneous_batch() {
    let (mut ctx, _dir) = open_ctx().await;
    let board = ctx.create_board("B".into(), Some("KAN".into())).unwrap();
    let col = ctx.create_column(board.id, "TODO".into(), None).unwrap();
    let c1 = ctx
        .create_card(board.id, col.id, "one".into(), Default::default())
        .unwrap();
    let c2 = ctx
        .create_card(board.id, col.id, "two".into(), Default::default())
        .unwrap();
    let shared = ctx.require_same_board(&[c1.id, c2.id]).unwrap();
    assert_eq!(shared, board.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_require_same_board_rejects_cross_board_batch() {
    let (mut ctx, _dir) = open_ctx().await;
    let board_a = ctx
        .create_board("Kanban".into(), Some("KAN".into()))
        .unwrap();
    let board_b = ctx
        .create_board("Personal".into(), Some("PER".into()))
        .unwrap();
    let col_a = ctx.create_column(board_a.id, "TODO".into(), None).unwrap();
    let col_b = ctx.create_column(board_b.id, "TODO".into(), None).unwrap();
    let c_a = ctx
        .create_card(board_a.id, col_a.id, "a".into(), Default::default())
        .unwrap();
    let c_b = ctx
        .create_card(board_b.id, col_b.id, "b".into(), Default::default())
        .unwrap();
    let err = ctx
        .require_same_board(&[c_a.id, c_b.id])
        .unwrap_err()
        .to_string();
    assert!(err.contains("same board"), "got: {err}");
    assert!(err.contains("'Kanban'"), "got: {err}");
    assert!(err.contains("'Personal'"), "got: {err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_require_same_board_empty_batch_errors() {
    let (ctx, _dir) = open_ctx().await;
    let err = ctx.require_same_board(&[]).unwrap_err().to_string();
    assert!(err.contains("No cards"), "got: {err}");
}

// ---------- UUID fast path doesn't accidentally do lookup ----------

#[tokio::test(flavor = "multi_thread")]
async fn test_uuid_fast_path_returns_uuid_even_when_nothing_exists() {
    let (ctx, _dir) = open_ctx().await;
    let random = Uuid::new_v4();
    // No entities exist; resolver returns the UUID; downstream get_* would be NotFound.
    assert_eq!(ctx.resolve_board_id(&random.to_string()).unwrap(), random);
}
