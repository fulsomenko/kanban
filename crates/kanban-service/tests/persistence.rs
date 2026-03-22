use kanban_domain::KanbanOperations;
use kanban_service::KanbanContext;
use tempfile::TempDir;

#[tokio::test]
async fn test_load_save_reload_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.kanban");
    let path_str = path.to_string_lossy().to_string();

    let mut ctx = KanbanContext::load_json(&path_str).await.unwrap();
    ctx.create_board("My Board".into(), Some("MB".into()))
        .unwrap();
    ctx.save().await.unwrap();

    let reloaded = KanbanContext::load_json(&path_str).await.unwrap();
    let boards = reloaded.list_boards().unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "My Board");
}

#[tokio::test]
async fn test_save_overwrites_correctly() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.kanban");
    let path_str = path.to_string_lossy().to_string();

    let mut ctx = KanbanContext::load_json(&path_str).await.unwrap();
    ctx.create_board("Board One".into(), None).unwrap();
    ctx.save().await.unwrap();

    ctx.create_board("Board Two".into(), None).unwrap();
    ctx.save().await.unwrap();

    let reloaded = KanbanContext::load_json(&path_str).await.unwrap();
    let boards = reloaded.list_boards().unwrap();
    assert_eq!(boards.len(), 2);
    assert!(boards.iter().any(|b| b.name == "Board One"));
    assert!(boards.iter().any(|b| b.name == "Board Two"));
}
