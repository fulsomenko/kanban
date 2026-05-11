//! KAN-394: status ↔ completion-column auto-sync orchestrated at the service layer.

use kanban_domain::{
    BoardUpdate, CardStatus, CardUpdate, FieldUpdate, InMemoryStore, KanbanOperations, KanbanResult,
};
use kanban_service::KanbanContext;
use std::sync::Arc;
use uuid::Uuid;

async fn make_ctx() -> KanbanContext {
    KanbanContext::open(
        Arc::new(InMemoryStore::new()),
        kanban_core::AppConfig::default(),
    )
    .await
    .unwrap()
}

struct Fixture {
    backlog_id: Uuid,
    progress_id: Uuid,
    done_id: Uuid,
    card_id: Uuid,
}

async fn build_fixture(ctx: &mut KanbanContext, set_completion_column: bool) -> Fixture {
    let board = ctx.create_board("Test".into(), Some("TST".into())).unwrap();
    let backlog = ctx.create_column(board.id, "Backlog".into(), None).unwrap();
    let progress = ctx
        .create_column(board.id, "InProgress".into(), None)
        .unwrap();
    let done = ctx.create_column(board.id, "Done".into(), None).unwrap();
    if set_completion_column {
        ctx.update_board(
            board.id,
            BoardUpdate {
                completion_column_id: FieldUpdate::Set(done.id),
                ..Default::default()
            },
        )
        .unwrap();
    }
    let card = ctx
        .create_card(board.id, backlog.id, "Card".into(), Default::default())
        .unwrap();
    Fixture {
        backlog_id: backlog.id,
        progress_id: progress.id,
        done_id: done.id,
        card_id: card.id,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_update_card_status_to_done_moves_to_completion_column() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let fx = build_fixture(&mut ctx, true).await;

    let updated = ctx.update_card(
        fx.card_id,
        CardUpdate {
            status: Some(CardStatus::Done),
            ..Default::default()
        },
    )?;

    assert_eq!(updated.status, CardStatus::Done);
    assert_eq!(
        updated.column_id, fx.done_id,
        "status=Done should auto-move card to completion column"
    );
    assert!(updated.completed_at.is_some(), "completed_at must be set");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_update_card_status_to_done_uses_last_column_when_no_completion_column_set(
) -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let fx = build_fixture(&mut ctx, false).await;

    let updated = ctx.update_card(
        fx.card_id,
        CardUpdate {
            status: Some(CardStatus::Done),
            ..Default::default()
        },
    )?;

    assert_eq!(updated.status, CardStatus::Done);
    assert_eq!(
        updated.column_id, fx.done_id,
        "with no explicit completion_column_id, last column should be used"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_update_card_status_done_to_todo_in_completion_column_moves_to_second_to_last(
) -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let fx = build_fixture(&mut ctx, true).await;

    // Pre-state: card at Done with status=Done
    ctx.update_card(
        fx.card_id,
        CardUpdate {
            status: Some(CardStatus::Done),
            ..Default::default()
        },
    )?;

    let reverted = ctx.update_card(
        fx.card_id,
        CardUpdate {
            status: Some(CardStatus::Todo),
            ..Default::default()
        },
    )?;

    assert_eq!(reverted.status, CardStatus::Todo);
    assert_eq!(
        reverted.column_id, fx.progress_id,
        "Done→Todo in completion column should move to second-to-last column"
    );
    assert!(
        reverted.completed_at.is_none(),
        "completed_at must be cleared"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_move_card_to_completion_column_sets_status_done_and_completed_at() -> KanbanResult<()>
{
    let mut ctx = make_ctx().await;
    let fx = build_fixture(&mut ctx, true).await;

    let moved = ctx.move_card(fx.card_id, fx.done_id, None)?;

    assert_eq!(moved.column_id, fx.done_id);
    assert_eq!(
        moved.status,
        CardStatus::Done,
        "move to completion column should set status=Done"
    );
    assert!(
        moved.completed_at.is_some(),
        "completed_at must be populated"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_move_card_away_from_completion_column_clears_done_status() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let fx = build_fixture(&mut ctx, true).await;

    ctx.move_card(fx.card_id, fx.done_id, None)?;
    let moved_back = ctx.move_card(fx.card_id, fx.backlog_id, None)?;

    assert_eq!(moved_back.column_id, fx.backlog_id);
    assert_eq!(
        moved_back.status,
        CardStatus::Todo,
        "moving away from completion column should clear Done status"
    );
    assert!(moved_back.completed_at.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_update_card_with_explicit_column_id_and_status_respects_both() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let fx = build_fixture(&mut ctx, true).await;

    // TUI-style atomic update: caller pins both column and status
    let updated = ctx.update_card(
        fx.card_id,
        CardUpdate {
            status: Some(CardStatus::Done),
            column_id: Some(fx.progress_id),
            position: Some(0),
            ..Default::default()
        },
    )?;

    assert_eq!(updated.status, CardStatus::Done);
    assert_eq!(
        updated.column_id, fx.progress_id,
        "explicit column_id in update must not be overridden by auto-sync"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_move_cards_batch_to_completion_column_sets_status_done_on_all() -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let fx = build_fixture(&mut ctx, true).await;
    let board_id = ctx.boards()?[0].id;
    let card2 = ctx.create_card(board_id, fx.backlog_id, "Card 2".into(), Default::default())?;

    let moved = ctx.move_cards(vec![fx.card_id, card2.id], fx.done_id)?;
    assert_eq!(moved, 2);

    for id in [fx.card_id, card2.id] {
        let card = ctx.get_card(id)?.unwrap();
        assert_eq!(card.column_id, fx.done_id);
        assert_eq!(
            card.status,
            CardStatus::Done,
            "move_cards batch must set status=Done on every card moved to completion column"
        );
        assert!(card.completed_at.is_some());
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_move_cards_batch_away_from_completion_column_clears_done_status() -> KanbanResult<()>
{
    let mut ctx = make_ctx().await;
    let fx = build_fixture(&mut ctx, true).await;
    let board_id = ctx.boards()?[0].id;
    let card2 = ctx.create_card(board_id, fx.backlog_id, "Card 2".into(), Default::default())?;

    ctx.move_cards(vec![fx.card_id, card2.id], fx.done_id)?;
    ctx.move_cards(vec![fx.card_id, card2.id], fx.backlog_id)?;
    for id in [fx.card_id, card2.id] {
        let card = ctx.get_card(id)?.unwrap();
        assert_eq!(card.column_id, fx.backlog_id);
        assert_eq!(
            card.status,
            CardStatus::Todo,
            "move_cards batch away from completion column must clear Done"
        );
        assert!(card.completed_at.is_none());
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_undo_after_update_card_status_done_reverses_both_status_and_column_move(
) -> KanbanResult<()> {
    let mut ctx = make_ctx().await;
    let fx = build_fixture(&mut ctx, true).await;

    let card_before = ctx.get_card(fx.card_id)?.unwrap();
    assert_eq!(card_before.column_id, fx.backlog_id);
    assert_eq!(card_before.status, CardStatus::Todo);

    ctx.update_card(
        fx.card_id,
        CardUpdate {
            status: Some(CardStatus::Done),
            ..Default::default()
        },
    )?;
    let card_done = ctx.get_card(fx.card_id)?.unwrap();
    assert_eq!(card_done.column_id, fx.done_id);
    assert_eq!(card_done.status, CardStatus::Done);

    assert!(ctx.undo()?, "undo should report success");
    let card_after_undo = ctx.get_card(fx.card_id)?.unwrap();
    assert_eq!(
        card_after_undo.column_id, fx.backlog_id,
        "single undo must revert the column move chained behind status=Done"
    );
    assert_eq!(
        card_after_undo.status,
        CardStatus::Todo,
        "single undo must revert the status change"
    );
    assert!(card_after_undo.completed_at.is_none());
    Ok(())
}
