/// End-to-end tests for the `open_context()` free function (Step 5 of the
/// "Unified Backends via True Deferred Reads" architecture).
///
/// All tests call `kanban_service::open_context(locator, cfg)` and exercise
/// the full detection + backend-creation pipeline with real TempDir files.
use kanban_service::{open_context, AppConfig, KanbanOperations, KanbanResult};
use tempfile::tempdir;

/// JSON round-trip: create a board, save, reopen, board is still there.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_json_end_to_end() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("board.json");

    {
        let mut ctx = open_context(path.to_str().unwrap(), AppConfig::default()).await?;
        ctx.create_board("Board1".into(), None)?;
        ctx.save().await?;
    }

    let ctx = open_context(path.to_str().unwrap(), AppConfig::default()).await?;
    let boards = ctx.boards()?;
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "Board1");
    Ok(())
}

/// SQLite round-trip: create a board (write-through), reopen, board persists.
#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use super::*;
    use kanban_persistence_sqlite::SqliteStore;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_open_context_sqlite_end_to_end() -> KanbanResult<()> {
        let dir = tempdir().unwrap();
        let path = dir.path().join("board.sqlite");

        {
            let mut ctx = open_context(path.to_str().unwrap(), AppConfig::default()).await?;
            ctx.create_board("Board1".into(), None)?;
            // SQLite is write-through — no explicit save() needed.
        }

        let ctx = open_context(path.to_str().unwrap(), AppConfig::default()).await?;
        let boards = ctx.boards()?;
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].name, "Board1");
        Ok(())
    }

    /// `open_context` detects SQLite from magic bytes when the file has no
    /// recognised extension.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_open_context_auto_detects_backend_from_magic_bytes() -> KanbanResult<()> {
        let dir = tempdir().unwrap();
        let path = dir.path().join("noext");

        // Create a SQLite file with no extension so magic-byte detection kicks in.
        SqliteStore::open(path.to_str().unwrap()).await.unwrap();

        let mut ctx = open_context(path.to_str().unwrap(), AppConfig::default()).await?;
        ctx.create_board("B".into(), None)?;
        let boards = ctx.boards()?;
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].name, "B");
        Ok(())
    }
}

/// `open_initialized` populates the undo cursor from persisted commands so
/// that `can_undo()` returns true on a fresh context without any mutation.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_initialized_populates_undo_cursor_from_prior_commands() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("board.json");

    {
        let mut ctx = open_context(path.to_str().unwrap(), AppConfig::default()).await?;
        ctx.create_board("Board1".into(), None)?;
        ctx.save().await?;
    }

    let sm = kanban_service::StoreManager::new(kanban_service::default_registry());
    let backend = sm
        .make_backend(path.to_str().unwrap(), &AppConfig::default())
        .await?;
    let ctx =
        kanban_service::KanbanContext::open_initialized(backend, AppConfig::default()).await?;
    assert!(
        ctx.can_undo(),
        "open_initialized must restore undo cursor from persisted command log"
    );
    assert_eq!(ctx.undo_depth(), 1);
    Ok(())
}

/// A non-existent path produces an empty context (no boards).
#[tokio::test(flavor = "multi_thread")]
async fn test_open_context_new_file_starts_empty() -> KanbanResult<()> {
    let dir = tempdir().unwrap();
    let path = dir.path().join("new.json");

    let ctx = open_context(path.to_str().unwrap(), AppConfig::default()).await?;
    assert!(ctx.boards()?.is_empty());
    Ok(())
}
