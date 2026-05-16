//! Cross-session undo for SQLite backend.
//!
//! KAN-191 step 5: SQLite persists the command log to disk so closing and
//! reopening a SQLite file preserves the undo history (which was lost in
//! KAN-405). JSON deliberately stays in-memory per the per-backend policy.

use kanban_domain::{KanbanOperations, KanbanResult};
use kanban_service::{open_context, AppConfig};
use tempfile::TempDir;

#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_undo_survives_session_close() -> KanbanResult<()> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.sqlite3");
    let locator = path.to_str().unwrap();

    {
        let mut ctx = open_context(locator, AppConfig::default()).await?;
        ctx.create_board("Persistent Board".into(), None)?;
        assert!(ctx.can_undo(), "fresh board should be undoable in-session");
        ctx.save().await?;
    }

    {
        let mut ctx = open_context(locator, AppConfig::default()).await?;
        assert_eq!(
            ctx.boards()?.len(),
            1,
            "board is persisted in entity tables"
        );
        assert!(
            ctx.can_undo(),
            "command log must survive session close — KAN-191 cross-session undo"
        );
        assert!(ctx.undo()?, "undo across sessions rewinds the create");
        assert_eq!(
            ctx.boards()?.len(),
            0,
            "after cross-session undo the board is gone"
        );
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_json_undo_does_not_survive_session_close() -> KanbanResult<()> {
    // Contrast test: JSON keeps the per-backend in-memory policy.
    // This guards against accidentally adding cross-session undo to JSON.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.json");
    let locator = path.to_str().unwrap();

    {
        let mut ctx = open_context(locator, AppConfig::default()).await?;
        ctx.create_board("Ephemeral Board".into(), None)?;
        ctx.save().await?;
    }

    {
        let ctx = open_context(locator, AppConfig::default()).await?;
        assert_eq!(ctx.boards()?.len(), 1, "board is persisted (entity state)");
        assert!(
            !ctx.can_undo(),
            "JSON undo is per-session — reopening must start clean"
        );
    }

    Ok(())
}
