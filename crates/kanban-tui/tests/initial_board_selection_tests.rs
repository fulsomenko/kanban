mod helpers;

#[tokio::test]
async fn test_load_initial_state_with_boards_selects_first_board() {
    let dir = tempfile::tempdir().unwrap();
    let path = helpers::create_test_json_file(dir.path(), "test.json", &["Alpha", "Beta"]).await;
    let (mut app, _rx) = kanban_tui::App::new(Some(path)).unwrap();
    app.load_initial_state().await;

    assert_eq!(
        app.selection.board.get(),
        Some(0),
        "first board should be selected after startup"
    );
}

#[tokio::test]
async fn test_load_initial_state_with_boards_refreshes_card_view() {
    use kanban_domain::{Board, Card, Column, Snapshot};
    use kanban_persistence::{PersistenceMetadata, PersistenceStore, StoreSnapshot};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("with_cards.json");
    let path_str = path.to_str().unwrap().to_string();

    let mut board = Board::new("Alpha".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let card = Card::new(&mut board, column.id, "Task One".to_string(), 0, "TST");
    let snapshot = Snapshot {
        boards: vec![board],
        columns: vec![column],
        cards: vec![card],
        archived_cards: vec![],
        sprints: vec![],
        graph: Default::default(),
    };
    let store = kanban_persistence_json::JsonFileStore::new(&path_str);
    let store_snapshot = StoreSnapshot {
        data: serde_json::to_vec(&snapshot).unwrap(),
        metadata: PersistenceMetadata::new(store.instance_id()),
    };
    store.save(store_snapshot).await.unwrap();

    let (mut app, _rx) = kanban_tui::App::new(Some(path_str)).unwrap();
    app.load_initial_state().await;

    let task_list = app.view.strategy.get_active_task_list();
    assert!(
        task_list.is_some_and(|l| !l.is_empty()),
        "card view should be populated after startup without user interaction"
    );
}

#[tokio::test]
async fn test_load_initial_state_with_no_boards_leaves_selection_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = helpers::create_test_json_file(dir.path(), "empty.json", &[]).await;
    let (mut app, _rx) = kanban_tui::App::new(Some(path)).unwrap();
    app.load_initial_state().await;

    assert_eq!(
        app.selection.board.get(),
        None,
        "selection should remain None when there are no boards"
    );
}

#[tokio::test]
async fn test_load_initial_state_with_no_file_leaves_selection_none() {
    let (mut app, _rx) = kanban_tui::App::new(None).unwrap();
    app.load_initial_state().await;

    assert_eq!(
        app.selection.board.get(),
        None,
        "selection should remain None when no file is provided"
    );
}
