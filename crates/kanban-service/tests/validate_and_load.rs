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

    let snapshot = validate_and_load_store(&path).await.unwrap();
    assert_eq!(snapshot.boards.len(), 1);
}

#[tokio::test]
async fn test_validate_and_load_nonexistent_file_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");

    let err = validate_and_load_store(path.to_str().unwrap())
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

    let err = validate_and_load_store(path.to_str().unwrap())
        .await
        .unwrap_err();
    assert!(err.to_string().len() > 0, "expected error, got: {}", err);
}

#[tokio::test]
async fn test_validate_and_load_empty_file_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.json");
    std::fs::write(&path, "").unwrap();

    let err = validate_and_load_store(path.to_str().unwrap())
        .await
        .unwrap_err();
    assert!(err.to_string().len() > 0, "expected error, got: {}", err);
}

#[tokio::test]
async fn test_validate_and_load_preserves_board_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_json(dir.path(), "board.json", &["MyBoard"]);

    let snapshot = validate_and_load_store(&path).await.unwrap();
    assert_eq!(snapshot.boards.len(), 1);
    assert_eq!(snapshot.boards[0].name, "MyBoard");
}
