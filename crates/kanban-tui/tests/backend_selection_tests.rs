// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
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
        kanban_tui::App::new_with_store(sm, Some(path.to_str().unwrap().to_string()))
            .await
            .unwrap();

    assert!(
        !app.ctx.backend().needs_save_worker(),
        "SQLite backend must not require a background save worker"
    );
    assert!(
        save_rx.is_none(),
        "no save channel should be created for a write-through backend"
    );
}

#[tokio::test]
async fn test_new_with_store_json_path_yields_save_worker() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("board.json");

    let sm = kanban_service::StoreManager::new(kanban_service::default_registry());
    let (app, save_rx) =
        kanban_tui::App::new_with_store(sm, Some(path.to_str().unwrap().to_string()))
            .await
            .unwrap();

    assert!(
        app.ctx.backend().needs_save_worker(),
        "JSON backend must require a background save worker"
    );
    assert!(
        save_rx.is_some(),
        "a save channel should be created for a JSON backend"
    );
}

#[tokio::test]
async fn test_new_with_store_no_file_uses_in_memory_backend_and_has_no_save_file() {
    let dir = tempfile::TempDir::new().unwrap();

    let sm = kanban_service::StoreManager::new(kanban_service::default_registry());
    let (app, _save_rx) = kanban_tui::App::new_with_store(sm, None).await.unwrap();

    assert!(
        app.persistence.save_file.is_none(),
        "no-file mode must not associate a save path"
    );
    assert!(
        !app.has_data_file,
        "no-file mode must set has_data_file = false"
    );
    assert!(
        !dir.path().join("kanban.json").exists(),
        "must not create kanban.json when no file is given"
    );
}

#[tokio::test]
async fn test_no_file_tui_startup_pushes_choose_storage_dialog_prefilled_with_kanban_json() {
    let sm = kanban_service::StoreManager::new(kanban_service::default_registry());
    let (mut app, _save_rx) = kanban_tui::App::new_with_store(sm, None).await.unwrap();

    app.maybe_push_startup_file_dialog();

    assert_eq!(
        app.mode,
        kanban_tui::AppMode::Dialog(kanban_tui::DialogMode::ChooseStorageFile),
        "no-file startup must open the ChooseStorageFile dialog"
    );
    assert_eq!(
        app.input.as_str(),
        "kanban.json",
        "dialog must be pre-filled with kanban.json"
    );
}

#[tokio::test]
async fn test_no_file_tui_startup_dialog_cancel_stays_in_memory() {
    use crossterm::event::KeyCode;

    let sm = kanban_service::StoreManager::new(kanban_service::default_registry());
    let (mut app, _save_rx) = kanban_tui::App::new_with_store(sm, None).await.unwrap();
    app.maybe_push_startup_file_dialog();

    app.handle_choose_storage_file_dialog(KeyCode::Esc);

    assert_eq!(
        app.mode,
        kanban_tui::AppMode::Normal,
        "cancelling the dialog must return to Normal mode"
    );
    assert!(
        !app.has_data_file,
        "cancelling must leave the app in in-memory mode"
    );
    assert!(
        app.persistence.save_file.is_none(),
        "cancelling must not set a save file"
    );
}

// multi_thread required because make_backend for SQLite opens the DB asynchronously,
// and block_in_place (used in adopt_storage_file) needs multiple OS threads.
#[tokio::test(flavor = "multi_thread")]
async fn test_no_file_tui_startup_dialog_confirm_creates_file_and_adopts_backend() {
    use crossterm::event::KeyCode;

    let dir = tempfile::TempDir::new().unwrap();
    let target = dir.path().join("myboard.json");
    let target_str = target.to_str().unwrap().to_string();

    let sm = kanban_service::StoreManager::new(kanban_service::default_registry());
    let (mut app, _save_rx) = kanban_tui::App::new_with_store(sm, None).await.unwrap();
    app.maybe_push_startup_file_dialog();

    // Type the target filename
    app.input.clear();
    app.input.set(target_str.clone());

    app.handle_choose_storage_file_dialog(KeyCode::Enter);

    assert!(
        app.has_data_file,
        "confirming must mark the app as having a data file"
    );
    assert_eq!(
        app.persistence.save_file.as_deref(),
        Some(target_str.as_str()),
        "persistence.save_file must point to the chosen path"
    );
    assert_eq!(
        app.mode,
        kanban_tui::AppMode::Normal,
        "confirming must dismiss the dialog"
    );
}
