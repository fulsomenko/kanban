//! Integration tests for `KanbanContext::delete_board` cascade orchestration (KAN-427).
//!
//! The atomic `BoardCommand::Delete(DeleteBoard)` now only removes the board record.
//! The cascade (graph edges → cards → archived cards → columns → sprints → board) is
//! orchestrated by `KanbanOperations::delete_board` in the service layer using a
//! single `execute(commands)` batch, which provides snapshot-based rollback if any
//! step fails.

use kanban_domain::{dependencies::CardGraphExt, Board, Card, Column, Sprint};
use kanban_persistence_json::JsonFileStore;
use kanban_service::{
    json_backend::JsonDataStore, AppConfig, KanbanBackend, KanbanContext, KanbanOperations,
};
use std::sync::Arc;
use tempfile::tempdir;

fn make_backend(path: &std::path::Path) -> Arc<dyn KanbanBackend> {
    Arc::new(JsonDataStore::new(Arc::new(JsonFileStore::new(path))))
}

async fn open_context() -> (KanbanContext, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.json");
    let ctx = KanbanContext::open(make_backend(&path), AppConfig::default())
        .await
        .unwrap();
    (ctx, dir)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_delete_board_cascades_to_columns_cards_and_sprints() {
    let (mut ctx, _dir) = open_context().await;
    let backend = ctx.backend();

    let mut board = Board::new("B".to_string(), Some("TST".to_string()));
    let board_id = board.id;
    let col1 = Column::new(board_id, "Col1".to_string(), 0);
    let col2 = Column::new(board_id, "Col2".to_string(), 1);
    let card1 = Card::new(&mut board, col1.id, "C1".to_string(), 0);
    let card2 = Card::new(&mut board, col2.id, "C2".to_string(), 0);
    let sprint = Sprint::new(board_id, 1, None, None);
    backend.upsert_board(board).unwrap();
    backend.upsert_column(col1).unwrap();
    backend.upsert_column(col2).unwrap();
    backend.upsert_card(card1).unwrap();
    backend.upsert_card(card2).unwrap();
    backend.upsert_sprint(sprint).unwrap();

    ctx.delete_board(board_id).unwrap();

    assert!(backend.list_boards().unwrap().is_empty());
    assert!(backend.list_all_columns().unwrap().is_empty());
    assert!(backend.list_all_cards().unwrap().is_empty());
    assert!(backend.list_all_sprints().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_delete_board_cleans_dependency_graph_edges_for_all_cards() {
    let (mut ctx, _dir) = open_context().await;
    let backend = ctx.backend();

    let mut board = Board::new("B".to_string(), Some("TST".to_string()));
    let board_id = board.id;
    let col = Column::new(board_id, "Col".to_string(), 0);
    let card_a = Card::new(&mut board, col.id, "A".to_string(), 0);
    let card_b = Card::new(&mut board, col.id, "B".to_string(), 1);
    let card_a_id = card_a.id;
    let card_b_id = card_b.id;
    backend.upsert_board(board).unwrap();
    backend.upsert_column(col).unwrap();
    backend.upsert_card(card_a).unwrap();
    backend.upsert_card(card_b).unwrap();

    let mut graph = backend.get_graph().unwrap();
    graph.cards.add_blocks(card_a_id, card_b_id).unwrap();
    backend.set_graph(graph).unwrap();
    assert_eq!(backend.get_graph().unwrap().cards.edges().len(), 1);

    ctx.delete_board(board_id).unwrap();

    assert_eq!(
        backend.get_graph().unwrap().cards.edges().len(),
        0,
        "service delete_board must clean dependency-graph edges for all deleted cards"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_delete_board_removes_archived_cards() {
    let (mut ctx, _dir) = open_context().await;
    let backend = ctx.backend();

    let mut board = Board::new("B".to_string(), Some("TST".to_string()));
    let board_id = board.id;
    let col = Column::new(board_id, "Col".to_string(), 0);
    let col_id = col.id;
    let card = Card::new(&mut board, col_id, "C".to_string(), 0);
    let archived = kanban_domain::ArchivedCard::new(card, col_id, 0);
    backend.upsert_board(board).unwrap();
    backend.upsert_column(col).unwrap();
    backend.insert_archived_card(archived).unwrap();

    ctx.delete_board(board_id).unwrap();

    assert!(backend.list_archived_cards().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_delete_board_undo_restores_full_cascade_state() {
    let (mut ctx, _dir) = open_context().await;
    let backend = ctx.backend();

    let board = ctx.create_board("B".into(), Some("TST".into())).unwrap();
    let board_id = board.id;
    let column = ctx.create_column(board_id, "Col".into(), None).unwrap();
    ctx.create_card(board_id, column.id, "C".into(), Default::default())
        .unwrap();

    assert_eq!(backend.list_boards().unwrap().len(), 1);
    assert_eq!(backend.list_all_cards().unwrap().len(), 1);

    ctx.delete_board(board_id).unwrap();
    assert!(backend.list_boards().unwrap().is_empty());

    let undone = ctx.undo().unwrap();
    assert!(undone, "undo should report success");

    assert_eq!(
        backend.list_boards().unwrap().len(),
        1,
        "undo of cascade delete must restore the board"
    );
    assert_eq!(
        backend.list_all_columns().unwrap().len(),
        1,
        "undo must restore the column"
    );
    assert_eq!(
        backend.list_all_cards().unwrap().len(),
        1,
        "undo must restore the card"
    );
}
