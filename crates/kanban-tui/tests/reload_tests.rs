use kanban_domain::KanbanOperations;
use kanban_service::{AppConfig, KanbanContext};
use kanban_tui::tui_context::TuiContext;
use tempfile::TempDir;

async fn make_tui_ctx(path: &std::path::Path) -> TuiContext {
    let sm = kanban_service::StoreManager::new(kanban_service::default_registry());
    let backend = sm
        .make_backend(path.to_str().unwrap(), &AppConfig::default())
        .await
        .unwrap();
    let ctx = KanbanContext::open(backend, AppConfig::default())
        .await
        .unwrap();
    let (tui_ctx, _, _) = TuiContext::new(ctx).unwrap();
    tui_ctx
}

/// After `reload()`, `can_undo()` must return `false` and `create_board` must
/// return an error until `initialize_undo_state()` is called again.
#[tokio::test(flavor = "multi_thread")]
async fn test_tui_reload_invalidates_undo_state() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("reload.json");
    let mut tui_ctx = make_tui_ctx(&path).await;

    tui_ctx.create_board("Before".to_string(), None).unwrap();
    assert!(tui_ctx.can_undo(), "must be undoable before reload");

    tui_ctx.save().await.unwrap();
    tui_ctx.reload().await.unwrap();

    assert!(
        !tui_ctx.can_undo(),
        "undo history must be invalidated after reload"
    );

    // Mutations must fail until undo state is re-initialized.
    let err = tui_ctx
        .create_board("After-no-init".to_string(), None)
        .unwrap_err();
    assert!(
        err.to_string().contains("undo state not initialized"),
        "expected 'undo state not initialized' error, got: {err}"
    );

    tui_ctx.initialize_undo_state().unwrap();
    tui_ctx
        .create_board("After-with-init".to_string(), None)
        .unwrap();
    assert!(tui_ctx.can_undo(), "must be undoable after re-initialization");
}

/// `reload()` followed by `initialize_undo_state()` must expose the data that
/// was present on disk at the time of the reload.
#[tokio::test(flavor = "multi_thread")]
async fn test_tui_reload_then_save_preserves_data() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("preserve.json");
    let mut tui_ctx = make_tui_ctx(&path).await;

    tui_ctx.create_board("Persisted".to_string(), None).unwrap();
    tui_ctx.save().await.unwrap();

    tui_ctx.reload().await.unwrap();
    tui_ctx.initialize_undo_state().unwrap();

    let boards = tui_ctx.list_boards().unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "Persisted");
}
