use kanban_domain::export::AllBoardsExport;
use kanban_persistence::StoreRegistry;
use kanban_persistence_json::JsonStoreFactory;
use kanban_persistence_sqlite::SqliteStoreFactory;
use kanban_service::StoreManager;

fn json_only_manager() -> StoreManager {
    let mut registry = StoreRegistry::new();
    registry.register(Box::new(JsonStoreFactory));
    StoreManager::new(registry)
}

fn full_manager() -> StoreManager {
    let mut registry = StoreRegistry::new();
    registry.register(Box::new(SqliteStoreFactory));
    registry.register(Box::new(JsonStoreFactory));
    StoreManager::new(registry)
}

#[tokio::test]
async fn test_export_to_sqlite_without_sqlite_backend_returns_error_with_context() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("out.sqlite").to_string_lossy().to_string();

    let err = json_only_manager()
        .export_to_sqlite(AllBoardsExport::empty(), &output)
        .await
        .unwrap_err();

    let msg = err.to_string();
    assert!(
        msg.contains("No backend") || msg.contains("Unsupported") || msg.contains("sqlite"),
        "expected root-cause to be present in error, got: {msg}"
    );
}

#[tokio::test]
async fn test_export_to_sqlite_with_sqlite_backend_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("out.sqlite").to_string_lossy().to_string();

    full_manager()
        .export_to_sqlite(AllBoardsExport::empty(), &output)
        .await
        .expect("export_to_sqlite must succeed when sqlite backend is registered");

    assert!(
        std::path::Path::new(&output).exists(),
        "exported sqlite file must be created on disk"
    );
}
