//! These integration tests require `flavor = "multi_thread"` because
//! `JsonDataStore::ensure_loaded` uses `tokio::task::block_in_place`.

#[tokio::test(flavor = "multi_thread")]
async fn test_new_with_store_sqlite_path_yields_no_save_worker() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("board.sqlite");

    // Pre-create the SQLite file so content-sniffing works.
    kanban_persistence_sqlite::SqliteStore::open(path.to_str().unwrap())
        .await
        .unwrap();

    let sm = kanban_service::StoreManager::new(kanban_service::default_registry());
    let (app, save_rx) =
        kanban_tui::App::new_with_store(sm, Some(path.to_str().unwrap().to_string())).unwrap();

    assert!(
        !app.ctx.backend().needs_save_worker(),
        "SQLite backend must not require a background save worker"
    );
    assert!(
        save_rx.is_none(),
        "no save channel should be created for a write-through backend"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_new_with_store_json_path_yields_save_worker() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("board.json");

    let sm = kanban_service::StoreManager::new(kanban_service::default_registry());
    let (app, save_rx) =
        kanban_tui::App::new_with_store(sm, Some(path.to_str().unwrap().to_string())).unwrap();

    assert!(
        app.ctx.backend().needs_save_worker(),
        "JSON backend must require a background save worker"
    );
    assert!(
        save_rx.is_some(),
        "a save channel should be created for a JSON backend"
    );
}
