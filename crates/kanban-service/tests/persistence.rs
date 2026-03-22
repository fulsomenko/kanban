use kanban_domain::KanbanOperations;
use kanban_persistence_json::JsonFileStore;
use kanban_service::KanbanContext;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_load_save_reload_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.kanban");
    let path_str = path.to_string_lossy().to_string();

    let mut ctx = KanbanContext::load(Arc::new(JsonFileStore::new(&path_str)))
        .await
        .unwrap();
    ctx.create_board("My Board".into(), Some("MB".into()))
        .unwrap();
    ctx.save().await.unwrap();

    let reloaded = KanbanContext::load(Arc::new(JsonFileStore::new(&path_str)))
        .await
        .unwrap();
    let boards = reloaded.list_boards().unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "My Board");
}

#[tokio::test]
async fn test_save_overwrites_correctly() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.kanban");
    let path_str = path.to_string_lossy().to_string();

    let mut ctx = KanbanContext::load(Arc::new(JsonFileStore::new(&path_str)))
        .await
        .unwrap();
    ctx.create_board("Board One".into(), None).unwrap();
    ctx.save().await.unwrap();

    ctx.create_board("Board Two".into(), None).unwrap();
    ctx.save().await.unwrap();

    let reloaded = KanbanContext::load(Arc::new(JsonFileStore::new(&path_str)))
        .await
        .unwrap();
    let boards = reloaded.list_boards().unwrap();
    assert_eq!(boards.len(), 2);
    assert!(boards.iter().any(|b| b.name == "Board One"));
    assert!(boards.iter().any(|b| b.name == "Board Two"));
}

#[tokio::test]
async fn test_reload_picks_up_external_changes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.kanban");
    let path_str = path.to_string_lossy().to_string();

    let mut ctx_a = KanbanContext::load(Arc::new(JsonFileStore::new(&path_str)))
        .await
        .unwrap();
    ctx_a.create_board("Board A".into(), None).unwrap();
    ctx_a.save().await.unwrap();

    let mut ctx_b = KanbanContext::load(Arc::new(JsonFileStore::new(&path_str)))
        .await
        .unwrap();
    ctx_b.create_board("Board B".into(), None).unwrap();
    ctx_b.save().await.unwrap();

    ctx_a.reload().await.unwrap();
    let boards = ctx_a.list_boards().unwrap();
    assert_eq!(boards.len(), 2);
    assert!(boards.iter().any(|b| b.name == "Board B"));
}
