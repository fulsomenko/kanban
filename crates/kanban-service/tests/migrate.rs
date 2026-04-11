use kanban_persistence::StoreRegistry;
use kanban_service::StoreManager;

fn manager() -> StoreManager {
    let mut registry = StoreRegistry::new();
    registry.register(Box::new(kanban_persistence_sqlite::SqliteStoreFactory));
    registry.register(Box::new(kanban_persistence_json::JsonStoreFactory));
    StoreManager::new(registry)
}

fn create_test_json(dir: &std::path::Path, name: &str) -> String {
    let path = dir.join(name);
    let data = serde_json::json!({
        "boards": [],
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
async fn test_migrate_store_json_to_json_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let from = create_test_json(dir.path(), "source.json");
    let to = dir.path().join("target.json");
    let to_str = to.to_str().unwrap();

    manager()
        .migrate_store("json", &from, "json", to_str)
        .await
        .unwrap();
    assert!(to.exists());
}

#[tokio::test]
async fn test_migrate_store_json_to_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let from = create_test_json(dir.path(), "source.json");
    let to = dir.path().join("target.sqlite");
    let to_str = to.to_str().unwrap();

    manager()
        .migrate_store("json", &from, "sqlite", to_str)
        .await
        .unwrap();
    assert!(to.exists());
}

#[tokio::test]
async fn test_migrate_store_fails_if_target_exists() {
    let dir = tempfile::tempdir().unwrap();
    let from = create_test_json(dir.path(), "source.json");
    let to = create_test_json(dir.path(), "target.json");

    let err = manager()
        .migrate_store("json", &from, "json", &to)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[tokio::test]
async fn test_migrate_store_fails_if_source_missing() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("nonexistent.json");
    let to = dir.path().join("target.json");

    let err = manager()
        .migrate_store("json", from.to_str().unwrap(), "json", to.to_str().unwrap())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not found"));
}
