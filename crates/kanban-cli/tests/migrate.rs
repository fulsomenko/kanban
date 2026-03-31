use kanban_domain::KanbanOperations;
use kanban_persistence::PersistenceStore;
use kanban_persistence_json::JsonFileStore;
use kanban_persistence_sqlite::SqliteStore;
use kanban_service::KanbanContext;
use kanban_core::AppConfig;
use std::sync::Arc;
use tempfile::TempDir;

async fn create_populated_context(store: Arc<dyn PersistenceStore + Send + Sync>) -> KanbanContext {
    let mut ctx = KanbanContext::load(store, AppConfig::default()).await.unwrap();
    let board = ctx
        .create_board("Test Board".into(), Some("TB".into()))
        .unwrap();
    let col = ctx.create_column(board.id, "Todo".into(), None).unwrap();
    ctx.create_card(board.id, col.id, "Test Card".into(), Default::default())
        .unwrap();
    ctx.save().await.unwrap();
    ctx
}

#[tokio::test]
async fn test_migrate_json_to_sqlite_roundtrip() {
    let dir = TempDir::new().unwrap();
    let json_path = dir.path().join("source.json");
    let db_path = dir.path().join("dest.db");

    let json_store = Arc::new(JsonFileStore::new(&json_path));
    let original = create_populated_context(json_store.clone()).await;

    let (snapshot, _) = json_store.load().await.unwrap();
    let sqlite_store = Arc::new(SqliteStore::new(&db_path));
    sqlite_store.save(snapshot).await.unwrap();

    let loaded = KanbanContext::load(Arc::new(SqliteStore::new(&db_path)), AppConfig::default())
        .await
        .unwrap();

    assert_eq!(
        original.list_boards().unwrap().len(),
        loaded.list_boards().unwrap().len()
    );
    let orig_board = &original.list_boards().unwrap()[0];
    let loaded_board = &loaded.list_boards().unwrap()[0];
    assert_eq!(orig_board.name, loaded_board.name);
    assert_eq!(orig_board.card_prefix, loaded_board.card_prefix);
}

#[tokio::test]
async fn test_migrate_sqlite_to_json_roundtrip() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("source.db");
    let json_path = dir.path().join("dest.json");

    let sqlite_store = Arc::new(SqliteStore::new(&db_path));
    let original = create_populated_context(sqlite_store.clone()).await;

    let (snapshot, _) = sqlite_store.load().await.unwrap();
    let json_store = Arc::new(JsonFileStore::new(&json_path));
    json_store.save(snapshot).await.unwrap();

    let loaded = KanbanContext::load(Arc::new(JsonFileStore::new(&json_path)), AppConfig::default())
        .await
        .unwrap();

    assert_eq!(
        original.list_boards().unwrap().len(),
        loaded.list_boards().unwrap().len()
    );
    let orig_board = &original.list_boards().unwrap()[0];
    let loaded_board = &loaded.list_boards().unwrap()[0];
    assert_eq!(orig_board.name, loaded_board.name);
}

#[tokio::test]
async fn test_migrate_json_to_json_roundtrip() {
    let dir = TempDir::new().unwrap();
    let src_path = dir.path().join("source.json");
    let dst_path = dir.path().join("dest.json");

    let src_store = Arc::new(JsonFileStore::new(&src_path));
    create_populated_context(src_store.clone()).await;

    let (snapshot, _) = src_store.load().await.unwrap();
    let dst_store = Arc::new(JsonFileStore::new(&dst_path));
    dst_store.save(snapshot).await.unwrap();

    let loaded = KanbanContext::load(Arc::new(JsonFileStore::new(&dst_path)), AppConfig::default())
        .await
        .unwrap();

    assert_eq!(loaded.list_boards().unwrap().len(), 1);
    assert_eq!(loaded.list_boards().unwrap()[0].name, "Test Board");
}

#[tokio::test]
async fn test_migrate_sqlite_to_sqlite_roundtrip() {
    let dir = TempDir::new().unwrap();
    let src_path = dir.path().join("source.db");
    let dst_path = dir.path().join("dest.db");

    let src_store = Arc::new(SqliteStore::new(&src_path));
    create_populated_context(src_store.clone()).await;

    let (snapshot, _) = src_store.load().await.unwrap();
    let dst_store = Arc::new(SqliteStore::new(&dst_path));
    dst_store.save(snapshot).await.unwrap();

    let loaded = KanbanContext::load(Arc::new(SqliteStore::new(&dst_path)), AppConfig::default())
        .await
        .unwrap();

    assert_eq!(loaded.list_boards().unwrap().len(), 1);
    assert_eq!(loaded.list_boards().unwrap()[0].name, "Test Board");
}

#[tokio::test]
async fn test_migrate_rejects_missing_source() {
    use assert_cmd::cargo_bin_cmd;

    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("nonexistent.json");

    let output = cargo_bin_cmd!("kanban")
        .args(["migrate", missing.to_str().unwrap(), "sqlite"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Source file not found"), "stderr: {stderr}");
}

#[tokio::test]
async fn test_migrate_rejects_existing_target() {
    use assert_cmd::cargo_bin_cmd;

    let dir = TempDir::new().unwrap();
    let src_path = dir.path().join("source.json");
    let dst_path = dir.path().join("dest.db");

    let src_store = Arc::new(JsonFileStore::new(&src_path));
    create_populated_context(src_store).await;

    std::fs::write(&dst_path, "existing").unwrap();

    let output = cargo_bin_cmd!("kanban")
        .args([
            "migrate",
            src_path.to_str().unwrap(),
            "sqlite",
            "--output",
            dst_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Destination already exists"),
        "stderr: {stderr}"
    );
}

#[tokio::test]
async fn test_migrate_cli_with_explicit_output() {
    use assert_cmd::cargo_bin_cmd;

    let dir = TempDir::new().unwrap();
    let src_path = dir.path().join("source.json");
    let dst_path = dir.path().join("custom_output.db");

    let src_store = Arc::new(JsonFileStore::new(&src_path));
    create_populated_context(src_store).await;

    let output = cargo_bin_cmd!("kanban")
        .args([
            "migrate",
            src_path.to_str().unwrap(),
            "sqlite",
            "--output",
            dst_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dst_path.exists());

    let loaded = KanbanContext::load(Arc::new(SqliteStore::new(&dst_path)))
        .await
        .unwrap();
    assert_eq!(loaded.list_boards().unwrap().len(), 1);
}

#[tokio::test]
async fn test_migrate_cli_default_output_path() {
    use assert_cmd::cargo_bin_cmd;

    let dir = TempDir::new().unwrap();
    let src_path = dir.path().join("myboard.json");

    let src_store = Arc::new(JsonFileStore::new(&src_path));
    create_populated_context(src_store).await;

    let output = cargo_bin_cmd!("kanban")
        .args(["migrate", src_path.to_str().unwrap(), "sqlite"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected_output = dir.path().join("myboard.db");
    assert!(
        expected_output.exists(),
        "Expected default output at {}",
        expected_output.display()
    );

    let loaded = KanbanContext::load(Arc::new(SqliteStore::new(&expected_output)))
        .await
        .unwrap();
    assert_eq!(loaded.list_boards().unwrap().len(), 1);
}

#[tokio::test]
async fn test_migrate_rejects_unknown_backend() {
    use assert_cmd::cargo_bin_cmd;

    let dir = TempDir::new().unwrap();
    let src_path = dir.path().join("source.json");

    let src_store = Arc::new(JsonFileStore::new(&src_path));
    create_populated_context(src_store).await;

    let output = cargo_bin_cmd!("kanban")
        .args(["migrate", src_path.to_str().unwrap(), "postgres"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown backend"), "stderr: {stderr}");
}
