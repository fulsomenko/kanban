use kanban_domain::{Board, Card, Column};
use kanban_tui::{App, state::DataSnapshot};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_import_failure_prevents_empty_state_save() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("kanban.json");

    // Create a real board and then save it in V2 format
    let board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);

    // Create snapshot with one board
    let snapshot = kanban_tui::state::DataSnapshot {
        boards: vec![board.clone()],
        columns: vec![column.clone()],
        cards: vec![],
        archived_cards: vec![],
        sprints: vec![],
    };

    // Manually create V2 format JSON
    let snapshot_json = serde_json::to_value(&snapshot).unwrap();
    let v2_content = serde_json::json!({
        "version": 2,
        "metadata": {
            "format_version": 2,
            "instance_id": "test-instance-id",
            "saved_at": chrono::Utc::now().to_rfc3339()
        },
        "data": snapshot_json
    });

    fs::write(&file_path, serde_json::to_string_pretty(&v2_content).unwrap()).unwrap();

    // Create app with the V2 format file - should handle it gracefully now
    let app = App::new(Some(file_path.to_str().unwrap().to_string()));

    // App should load the board from V2 format
    assert_eq!(app.boards.len(), 1, "V2 format should be imported successfully");
    assert_eq!(app.boards[0].name, "Test Board");
    assert!(
        app.save_file.is_some(),
        "save_file should still be enabled after successful V2 import"
    );
}

#[test]
fn test_import_failure_disables_save_file() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("kanban.json");

    // Create an invalid JSON file
    fs::write(&file_path, "{ invalid json }").unwrap();

    // Create app with invalid file
    let app = App::new(Some(file_path.to_str().unwrap().to_string()));

    // save_file should be None due to import failure
    assert!(
        app.save_file.is_none(),
        "save_file should be None when import fails"
    );

    // App should have empty state
    assert_eq!(app.boards.len(), 0);
}

#[test]
fn test_v2_format_is_imported_correctly() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("kanban.json");

    // Create a real board
    let board = Board::new("My Project".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let mut board_mut = board.clone();
    let card = Card::new(&mut board_mut, column.id, "Important Task".to_string(), 0, "task");

    // Create snapshot with board, column, and card
    let snapshot = DataSnapshot {
        boards: vec![board],
        columns: vec![column],
        cards: vec![card],
        archived_cards: vec![],
        sprints: vec![],
    };

    // Manually create V2 format JSON
    let snapshot_json = serde_json::to_value(&snapshot).unwrap();
    let v2_content = serde_json::json!({
        "version": 2,
        "metadata": {
            "format_version": 2,
            "instance_id": "test-instance",
            "saved_at": chrono::Utc::now().to_rfc3339()
        },
        "data": snapshot_json
    });

    fs::write(&file_path, serde_json::to_string_pretty(&v2_content).unwrap()).unwrap();

    // Create app with V2 format file
    let app = App::new(Some(file_path.to_str().unwrap().to_string()));

    // Should successfully import the board with its column and card
    assert_eq!(app.boards.len(), 1);
    assert_eq!(app.boards[0].name, "My Project");
    assert_eq!(app.columns.len(), 1);
    assert_eq!(app.cards.len(), 1);
    assert_eq!(app.cards[0].title, "Important Task");
    assert!(
        app.save_file.is_some(),
        "save_file should remain enabled after successful V2 import"
    );
}
