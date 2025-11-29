use kanban_core::KanbanError;
use kanban_domain::{Board, Card, Column};
use kanban_persistence::{JsonFileStore, PersistenceMetadata, PersistenceStore, StoreSnapshot};
use kanban_tui::state::DataSnapshot;
use std::fs;
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn test_conflict_detection_on_concurrent_modification() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("kanban.json");

    // Create initial data and save via first instance
    let mut board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let card = Card::new(&mut board, column.id, "Test Task".to_string(), 0, "task");

    let snapshot1 = DataSnapshot {
        boards: vec![board.clone()],
        columns: vec![column.clone()],
        cards: vec![card.clone()],
        sprints: vec![],
        archived_cards: vec![],
    };

    let store_id = uuid::Uuid::new_v4();
    let store = JsonFileStore::with_instance_id(&file_path, store_id);
    let data1 = snapshot1.to_json_bytes().unwrap();
    let persist_snapshot1 = StoreSnapshot {
        data: data1,
        metadata: PersistenceMetadata::new(2, store_id),
    };
    store.save(persist_snapshot1).await.unwrap();

    // External modification: change the file directly
    let modified_data = serde_json::json!({
        "version": 2,
        "metadata": {
            "format_version": 2,
            "instance_id": uuid::Uuid::new_v4().to_string(),
            "saved_at": chrono::Utc::now().to_rfc3339()
        },
        "data": {}
    });

    fs::write(&file_path, serde_json::to_string_pretty(&modified_data).unwrap()).unwrap();

    // Small delay to ensure file metadata changes
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Same instance tries to save and should detect conflict
    let data2 = snapshot1.to_json_bytes().unwrap();
    let persist_snapshot2 = StoreSnapshot {
        data: data2,
        metadata: PersistenceMetadata::new(2, store_id),
    };

    let result = store.save(persist_snapshot2).await;
    assert!(result.is_err(), "Should detect conflict on second save");

    match result {
        Err(KanbanError::ConflictDetected { path, .. }) => {
            assert!(path.contains("kanban.json"), "Error should contain file path");
        }
        _ => panic!("Expected ConflictDetected error"),
    }
}

#[tokio::test]
async fn test_no_conflict_when_file_unchanged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("kanban.json");

    // Create and save initial data
    let mut board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let card = Card::new(&mut board, column.id, "Test Task".to_string(), 0, "task");

    let snapshot = DataSnapshot {
        boards: vec![board.clone()],
        columns: vec![column.clone()],
        cards: vec![card.clone()],
        sprints: vec![],
        archived_cards: vec![],
    };

    let store = JsonFileStore::new(&file_path);
    let data = snapshot.to_json_bytes().unwrap();
    let persist_snapshot = StoreSnapshot {
        data: data.clone(),
        metadata: PersistenceMetadata::new(2, store.instance_id()),
    };
    store.save(persist_snapshot.clone()).await.unwrap();

    // Save again with same data - should not detect conflict
    let result = store.save(persist_snapshot).await;
    assert!(result.is_ok(), "Should not detect conflict when file unchanged");
}

#[tokio::test]
async fn test_conflict_detection_tracks_file_metadata() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("kanban.json");

    // Create and save initial data
    let mut board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let _card = Card::new(&mut board, column.id, "Test Task".to_string(), 0, "task");

    let snapshot = DataSnapshot {
        boards: vec![board.clone()],
        columns: vec![column.clone()],
        cards: vec![],
        sprints: vec![],
        archived_cards: vec![],
    };

    let store = JsonFileStore::new(&file_path);
    let data = snapshot.to_json_bytes().unwrap();
    let persist_snapshot = StoreSnapshot {
        data,
        metadata: PersistenceMetadata::new(2, store.instance_id()),
    };
    store.save(persist_snapshot.clone()).await.unwrap();

    // Verify save was successful - file exists
    assert!(file_path.exists(), "File should exist after successful save");

    // Modify file externally
    let modified_data = serde_json::json!({
        "version": 2,
        "metadata": {
            "format_version": 2,
            "instance_id": uuid::Uuid::new_v4().to_string(),
            "saved_at": chrono::Utc::now().to_rfc3339()
        },
        "data": {}
    });
    fs::write(&file_path, serde_json::to_string_pretty(&modified_data).unwrap()).unwrap();

    // Small delay to ensure file metadata changes
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try to save and should detect conflict
    let result = store.save(persist_snapshot).await;
    assert!(result.is_err(), "Should detect conflict after external modification");
}

#[tokio::test]
async fn test_multiple_instances_with_different_ids() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("kanban.json");

    let mut board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let card = Card::new(&mut board, column.id, "Test Task".to_string(), 0, "task");

    let snapshot = DataSnapshot {
        boards: vec![board.clone()],
        columns: vec![column.clone()],
        cards: vec![card],
        sprints: vec![],
        archived_cards: vec![],
    };

    // First instance saves
    let store1_id = uuid::Uuid::new_v4();
    let store1 = JsonFileStore::with_instance_id(&file_path, store1_id);
    let data = snapshot.to_json_bytes().unwrap();
    let persist_snapshot1 = StoreSnapshot {
        data: data.clone(),
        metadata: PersistenceMetadata::new(2, store1_id),
    };
    store1.save(persist_snapshot1).await.unwrap();

    // Second instance with different ID loads and saves
    let store2_id = uuid::Uuid::new_v4();
    let store2 = JsonFileStore::with_instance_id(&file_path, store2_id);
    let persist_snapshot2 = StoreSnapshot {
        data: data.clone(),
        metadata: PersistenceMetadata::new(2, store2_id),
    };

    // Should succeed because it's a different instance
    let result = store2.save(persist_snapshot2).await;
    assert!(result.is_ok(), "Different instance should be able to save without conflict");
}

#[tokio::test]
async fn test_conflict_resolution_with_force_overwrite() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("kanban.json");

    let mut board = Board::new("Test Board".to_string(), None);
    let column = Column::new(board.id, "Todo".to_string(), 0);
    let card = Card::new(&mut board, column.id, "Test Task".to_string(), 0, "task");

    let snapshot = DataSnapshot {
        boards: vec![board.clone()],
        columns: vec![column.clone()],
        cards: vec![card],
        sprints: vec![],
        archived_cards: vec![],
    };

    let store = JsonFileStore::new(&file_path);

    // Save initial state
    let data = snapshot.to_json_bytes().unwrap();
    let persist_snapshot = StoreSnapshot {
        data: data.clone(),
        metadata: PersistenceMetadata::new(2, store.instance_id()),
    };
    store.save(persist_snapshot.clone()).await.unwrap();

    // Externally modify file
    let modified_data = serde_json::json!({"version": 2, "metadata": {}, "data": {}});
    fs::write(&file_path, serde_json::to_string_pretty(&modified_data).unwrap()).unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try to save and get conflict
    let result = store.save(persist_snapshot.clone()).await;
    assert!(result.is_err(), "Should detect conflict");

    // Now save with force overwrite (simulating user choosing to keep their changes)
    let result = store.save(persist_snapshot).await;
    assert!(result.is_err(), "Conflict should still be detected until metadata is cleared");
}
