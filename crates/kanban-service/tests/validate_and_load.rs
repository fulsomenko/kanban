use kanban_service::validate_and_load_store;

fn create_test_json(dir: &std::path::Path, name: &str, boards: &[&str]) -> String {
    let path = dir.join(name);
    let board_objects: Vec<serde_json::Value> = boards
        .iter()
        .map(|name| {
            serde_json::json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "name": name,
                "column_ids": [],
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z",
                "task_sort_field": "Default",
                "task_sort_order": "Ascending"
            })
        })
        .collect();
    let data = serde_json::json!({
        "boards": board_objects,
        "columns": [],
        "cards": [],
        "archived_cards": [],
        "sprints": [],
        "graph": { "cards": { "edges": [] } }
    });
    std::fs::write(&path, serde_json::to_string_pretty(&data).unwrap()).unwrap();
    path.to_str().unwrap().to_string()
}

#[tokio::test]
async fn test_validate_and_load_valid_json_returns_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_json(dir.path(), "board.json", &["Board1"]);

    let snapshot = validate_and_load_store("json", &path).await.unwrap();
    assert_eq!(snapshot.boards.len(), 1);
}

#[tokio::test]
async fn test_validate_and_load_nonexistent_file_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");

    let err = validate_and_load_store("json", path.to_str().unwrap())
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("does not exist") || err.to_string().contains("not found"),
        "got: {}",
        err
    );
}

#[tokio::test]
async fn test_validate_and_load_invalid_json_content_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.json");
    std::fs::write(&path, "hello world").unwrap();

    let err = validate_and_load_store("json", path.to_str().unwrap())
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("serialization") || msg.contains("parse") || msg.contains("invalid"),
        "expected a parse/serialization error, got: {}",
        msg
    );
}

#[tokio::test]
async fn test_validate_and_load_empty_file_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.json");
    std::fs::write(&path, "").unwrap();

    let err = validate_and_load_store("json", path.to_str().unwrap())
        .await
        .unwrap_err();
    assert!(!err.to_string().is_empty(), "expected error, got: {}", err);
}

#[tokio::test]
async fn test_validate_and_load_preserves_board_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_json(dir.path(), "board.json", &["MyBoard"]);

    let snapshot = validate_and_load_store("json", &path).await.unwrap();
    assert_eq!(snapshot.boards.len(), 1);
    assert_eq!(snapshot.boards[0].name, "MyBoard");
}

#[test]
fn test_storage_location_with_dotdot_fails_validation() {
    let config = kanban_core::AppConfig {
        storage_location: Some("../../foo".to_string()),
        ..Default::default()
    };
    let result = kanban_service::config::validate(&config);
    assert!(
        result.is_err(),
        "expected validation error for '..' in path"
    );
    let err = result.unwrap_err().to_string();
    assert!(err.contains(".."), "error should mention '..': {}", err);
}

#[test]
fn test_storage_location_with_dotdot_in_filename_is_accepted() {
    let config = kanban_core::AppConfig {
        storage_location: Some("/tmp/my..file.json".to_string()),
        ..Default::default()
    };
    let result = kanban_service::config::validate(&config);
    assert!(
        result.is_ok(),
        "expected no error for '..' in filename (not a path component): {:?}",
        result
    );
}

#[test]
fn test_storage_location_with_nested_dotdot_fails_validation() {
    let config = kanban_core::AppConfig {
        storage_location: Some("data/../../../etc".to_string()),
        ..Default::default()
    };
    let result = kanban_service::config::validate(&config);
    assert!(
        result.is_err(),
        "expected validation error for '..' in path"
    );
    let err = result.unwrap_err().to_string();
    assert!(err.contains(".."), "error should mention '..': {}", err);
}

#[cfg(feature = "sqlite-storage")]
async fn create_test_sqlite(dir: &std::path::Path, name: &str, boards: &[&str]) -> String {
    use kanban_persistence::{PersistenceMetadata, StoreSnapshot};

    let path = dir.join(name);
    let path_str = path.to_str().unwrap().to_string();
    let store = kanban_service::make_store("sqlite", &path_str).unwrap();

    let domain_boards: Vec<kanban_domain::Board> = boards
        .iter()
        .map(|name| kanban_domain::Board::new(name.to_string(), None))
        .collect();
    let snapshot = kanban_domain::Snapshot {
        boards: domain_boards,
        columns: vec![],
        cards: vec![],
        archived_cards: vec![],
        sprints: vec![],
        graph: Default::default(),
    };

    let store_snapshot = StoreSnapshot {
        data: serde_json::to_vec(&snapshot).unwrap(),
        metadata: PersistenceMetadata::new(store.instance_id()),
    };
    store.save(store_snapshot).await.unwrap();

    path_str
}

#[cfg(feature = "sqlite-storage")]
#[tokio::test]
async fn test_validate_and_load_valid_sqlite_returns_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_sqlite(dir.path(), "board.sqlite", &["Board1"]).await;

    let snapshot = validate_and_load_store("sqlite", &path).await.unwrap();
    assert_eq!(snapshot.boards.len(), 1);
}

#[cfg(feature = "sqlite-storage")]
#[tokio::test]
async fn test_validate_and_load_sqlite_preserves_board_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_sqlite(dir.path(), "board.sqlite", &["SQLiteBoard"]).await;

    let snapshot = validate_and_load_store("sqlite", &path).await.unwrap();
    assert_eq!(snapshot.boards.len(), 1);
    assert_eq!(snapshot.boards[0].name, "SQLiteBoard");
}
