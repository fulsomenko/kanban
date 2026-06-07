//! Integration tests for KanbanContext data integrity invariants (KAN-175).
//!
//! Six scenarios covering: blocker-edge cleanup on card delete, column-delete
//! guard when cards exist, sprint-unassign on sprint delete, not-found on
//! missing card update, import validation for invalid column references, and
//! rapid-save queue completeness.

use kanban_domain::{CreateCardOptions, GraphOperations, KanbanOperations, KanbanResult, Severity};
use kanban_persistence_json::JsonFileStore;
use kanban_service::{json_backend::JsonDataStore, AppConfig, KanbanBackend, KanbanContext};
use std::sync::Arc;
use tempfile::tempdir;
use uuid::Uuid;

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

/// 1. Delete card A which blocks card B; the blocks edge must be removed
///    from the graph once card A is deleted.
#[tokio::test(flavor = "multi_thread")]
async fn test_delete_card_cleans_dependencies() -> KanbanResult<()> {
    let (mut ctx, _dir) = open_json_ctx().await;

    let board = ctx.create_board("B".to_string(), None)?;
    let col = ctx.create_column(board.id, "Col".to_string(), None)?;
    let card_a = ctx.create_card(
        board.id,
        col.id,
        "A".to_string(),
        CreateCardOptions::default(),
    )?;
    let card_b = ctx.create_card(
        board.id,
        col.id,
        "B".to_string(),
        CreateCardOptions::default(),
    )?;

    ctx.block(card_a.id, card_b.id, Severity::default())?;
    assert_eq!(
        ctx.list_blocked_by(card_a.id)?,
        vec![card_b.id],
        "blocker edge must exist before delete"
    );

    ctx.delete_card(card_a.id)?;

    let graph = ctx.backend().get_graph()?;
    assert_eq!(
        graph.len(),
        0,
        "deleting the blocker card must remove the blocks edge"
    );
    Ok(())
}

/// 2. A column that still contains live cards must reject deletion with an
///    error; the column must remain present after the failed attempt.
#[tokio::test(flavor = "multi_thread")]
async fn test_delete_column_with_cards_returns_error() -> KanbanResult<()> {
    let (mut ctx, _dir) = open_json_ctx().await;

    let board = ctx.create_board("B".to_string(), None)?;
    let col = ctx.create_column(board.id, "Col".to_string(), None)?;
    let col_id = col.id;
    ctx.create_card(
        board.id,
        col_id,
        "C".to_string(),
        CreateCardOptions::default(),
    )?;

    let result = ctx.delete_column(col_id);
    assert!(
        result.is_err(),
        "delete_column must return an error when the column contains cards"
    );

    let column_still_there = ctx
        .get_column(col_id)
        .expect("get_column should not error after failed delete");
    assert!(
        column_still_there.is_some(),
        "the column must still exist after the failed delete"
    );
    Ok(())
}

/// 3. Deleting a sprint must unassign all cards that were assigned to it;
///    those cards must have no sprint_id afterwards.
#[tokio::test(flavor = "multi_thread")]
async fn test_delete_sprint_unassigns_cards() -> KanbanResult<()> {
    let (mut ctx, _dir) = open_json_ctx().await;

    let board = ctx.create_board("B".to_string(), None)?;
    let col = ctx.create_column(board.id, "Col".to_string(), None)?;
    let sprint = ctx.create_sprint(board.id, None, None)?;
    let sprint_id = sprint.id;

    let card_a = ctx.create_card(
        board.id,
        col.id,
        "A".to_string(),
        CreateCardOptions::default(),
    )?;
    let card_b = ctx.create_card(
        board.id,
        col.id,
        "B".to_string(),
        CreateCardOptions::default(),
    )?;
    ctx.assign_card_to_sprint(card_a.id, sprint_id)?;
    ctx.assign_card_to_sprint(card_b.id, sprint_id)?;

    let assigned = ctx.backend().list_cards_by_sprint(sprint_id)?;
    assert_eq!(
        assigned.len(),
        2,
        "both cards must be assigned before delete"
    );

    ctx.delete_sprint(sprint_id)?;

    let card_a_after = ctx.get_card(card_a.id)?.expect("card A must still exist");
    let card_b_after = ctx.get_card(card_b.id)?.expect("card B must still exist");
    assert_eq!(
        card_a_after.sprint_id, None,
        "card A must have no sprint_id after sprint delete"
    );
    assert_eq!(
        card_b_after.sprint_id, None,
        "card B must have no sprint_id after sprint delete"
    );
    Ok(())
}

/// 4. Updating a card that does not exist must return a NotFound error.
#[tokio::test(flavor = "multi_thread")]
async fn test_update_nonexistent_card_returns_not_found() -> KanbanResult<()> {
    let (mut ctx, _dir) = open_json_ctx().await;

    let nonexistent_id = Uuid::new_v4();
    let result = ctx.update_card(nonexistent_id, kanban_domain::CardUpdate::default());
    assert!(
        result.is_err(),
        "updating a non-existent card must return an error"
    );
    let err = result.unwrap_err();
    assert!(err.is_not_found(), "error must be NotFound, got: {err:?}");
    Ok(())
}

/// 5. Importing a Snapshot where a card references a column_id that is not
///    present in the snapshot's columns list must return a validation error.
#[tokio::test(flavor = "multi_thread")]
async fn test_import_with_invalid_column_reference_fails() -> KanbanResult<()> {
    let (mut ctx, _dir) = open_json_ctx().await;

    let board = kanban_domain::Board::new("Imported", None::<String>);
    let board_id = board.id;
    let nonexistent_column_id = Uuid::new_v4();

    let mut orphan_board = board.clone();
    let orphan_card =
        kanban_domain::Card::new(&mut orphan_board, nonexistent_column_id, "Orphan", 0);

    let snapshot = kanban_domain::Snapshot {
        boards: vec![board],
        columns: vec![],
        cards: vec![orphan_card],
        archived_cards: vec![],
        sprints: vec![],
        graph: kanban_domain::DependencyGraph::default(),
    };

    let json = serde_json::to_string(&snapshot).unwrap();
    let result = ctx.import_board(&json);
    assert!(
        result.is_err(),
        "importing a card with an invalid column reference must return an error"
    );
    let err_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        err_msg.contains("column") || err_msg.contains("invalid") || err_msg.contains("not found"),
        "error must mention column or invalid reference, got: {err_msg}"
    );

    // The board must not have been partially imported.
    let boards = ctx.boards()?;
    assert!(
        boards.iter().all(|b| b.id != board_id),
        "the board must not be present after a failed import"
    );
    Ok(())
}

/// 6. Queue 200+ rapid mutations, flush, then verify all updates persisted.
///    This exercises the save path under load and asserts no updates are dropped.
#[tokio::test(flavor = "multi_thread")]
async fn test_rapid_save_queue_completes_all() -> KanbanResult<()> {
    let (mut ctx, _dir) = open_json_ctx().await;

    let board = ctx.create_board("Board".to_string(), None)?;
    let col = ctx.create_column(board.id, "Col".to_string(), None)?;

    let mut card_ids = Vec::new();
    for i in 0..220 {
        let card = ctx.create_card(
            board.id,
            col.id,
            format!("Card {i}"),
            CreateCardOptions::default(),
        )?;
        card_ids.push(card.id);
    }

    ctx.save().await?;

    let all_cards = ctx.list_all_cards()?;
    assert_eq!(
        all_cards.len(),
        220,
        "all 220 cards must persist after rapid creation and flush"
    );

    for id in &card_ids {
        assert!(
            ctx.get_card(*id)?.is_some(),
            "card {id} must be retrievable after rapid save"
        );
    }
    Ok(())
}
