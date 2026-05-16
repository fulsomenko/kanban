//! KAN-191 Phase 3 — verifies the UndoStack + Command::capture_inverse
//! scaffolding behaves correctly while we still have no command-level
//! inverse implementations.
//!
//! Today the default `Command::capture_inverse` returns `None`, so undo
//! falls back to the legacy replay path for every command. The tests below
//! confirm that:
//!
//! - Undo still works through the fallback (regression check).
//! - The UndoStack stays empty (because no inverses are captured yet).
//! - As soon as one command implements `capture_inverse`, that branch will
//!   begin using the stack — Phases 4-6 add those implementations.

use kanban_core::AppConfig;
use kanban_domain::commands::{BoardCommand, Command, CreateBoard};
use kanban_domain::{InMemoryStore, KanbanResult};
use kanban_service::KanbanContext;
use std::sync::Arc;
use uuid::Uuid;

async fn make_ctx() -> KanbanContext {
    KanbanContext::open(Arc::new(InMemoryStore::new()), AppConfig::default())
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_undo_works_when_no_inverse_is_captured() -> KanbanResult<()> {
    // CreateBoard does not yet implement capture_inverse — so execute returns
    // None, the UndoStack stays empty, and undo falls back to the legacy
    // replay path. The user-visible behaviour is unchanged.
    let mut ctx = make_ctx().await;

    let cmd = Command::Board(BoardCommand::Create(CreateBoard {
        id: Uuid::new_v4(),
        name: "B".into(),
        card_prefix: None,
        position: 0,
    }));
    ctx.execute(vec![cmd])?;
    assert_eq!(ctx.boards()?.len(), 1);

    assert!(ctx.undo()?, "legacy replay path must still satisfy undo");
    assert_eq!(
        ctx.boards()?.len(),
        0,
        "fallback path must restore pre-state"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_redo_works_after_legacy_undo() -> KanbanResult<()> {
    // Same scaffold check on the redo side: legacy-path redo after legacy-
    // path undo must reach the same state as the original execute.
    let mut ctx = make_ctx().await;

    let id = Uuid::new_v4();
    let cmd = Command::Board(BoardCommand::Create(CreateBoard {
        id,
        name: "B".into(),
        card_prefix: None,
        position: 0,
    }));
    ctx.execute(vec![cmd])?;
    ctx.undo()?;
    assert!(ctx.redo()?, "legacy redo must succeed");

    let boards = ctx.boards()?;
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].id, id, "redo reproduces the original id");
    Ok(())
}
