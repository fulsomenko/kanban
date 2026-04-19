use kanban_core::AppConfig;
use kanban_persistence::StoreRegistry;
use kanban_service::{KanbanContext, StoreManager};

fn manager() -> StoreManager {
    let mut registry = StoreRegistry::new();
    registry.register(Box::new(kanban_persistence_json::JsonStoreFactory));
    StoreManager::new(registry)
}

fn now() -> &'static str {
    "2024-01-01T00:00:00Z"
}

fn write_json(dir: &std::path::Path, name: &str, data: serde_json::Value) -> String {
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_string_pretty(&data).unwrap()).unwrap();
    path.to_str().unwrap().to_string()
}

fn create_test_json(dir: &std::path::Path, name: &str) -> String {
    write_json(
        dir,
        name,
        serde_json::json!({
            "boards": [],
            "columns": [],
            "cards": [],
            "archived_cards": [],
            "sprints": [],
            "graph": { "cards": { "edges": [] } }
        }),
    )
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

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
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

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_migrate_store_repairs_dangling_sprint_id() {
    let dir = tempfile::tempdir().unwrap();
    let board_id = uuid::Uuid::new_v4().to_string();
    let col_id = uuid::Uuid::new_v4().to_string();
    let card_id = uuid::Uuid::new_v4().to_string();
    let ghost_sprint_id = uuid::Uuid::new_v4().to_string();

    let from = write_json(
        dir.path(),
        "source.json",
        serde_json::json!({
            "boards": [{ "id": board_id, "name": "B",
                "task_sort_field": "Default", "task_sort_order": "Ascending",
                "sprint_name_used_count": 0, "next_sprint_number": 1,
                "task_list_view": "Flat", "prefix_counters": {}, "sprint_counters": {},
                "created_at": now(), "updated_at": now() }],
            "columns": [{ "id": col_id, "board_id": board_id, "name": "TODO",
                "position": 0, "created_at": now(), "updated_at": now() }],
            "sprints": [],
            "cards": [{ "id": card_id, "column_id": col_id, "title": "Orphaned",
                "priority": "Medium", "status": "Todo", "position": 0, "card_number": 1,
                "sprint_id": ghost_sprint_id,
                "sprint_logs": [], "created_at": now(), "updated_at": now() }],
            "archived_cards": [],
            "graph": { "cards": { "edges": [] } }
        }),
    );
    let to = dir.path().join("out.sqlite");

    manager()
        .migrate_store("json", &from, "sqlite", to.to_str().unwrap())
        .await
        .unwrap();

    let ctx = KanbanContext::open_sqlite(to.to_str().unwrap(), AppConfig::default())
        .await
        .unwrap();
    let cards = ctx.cards().unwrap();
    assert_eq!(cards.len(), 1, "card should be present");
    assert!(
        cards[0].sprint_id.is_none(),
        "dangling sprint_id should be nulled out"
    );
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_migrate_store_repairs_orphaned_column_id() {
    let dir = tempfile::tempdir().unwrap();
    let board_id = uuid::Uuid::new_v4().to_string();
    let valid_col_id = uuid::Uuid::new_v4().to_string();
    let ghost_col_id = uuid::Uuid::new_v4().to_string();
    let card_id = uuid::Uuid::new_v4().to_string();

    let from = write_json(
        dir.path(),
        "source.json",
        serde_json::json!({
            "boards": [{ "id": board_id, "name": "B",
                "task_sort_field": "Default", "task_sort_order": "Ascending",
                "sprint_name_used_count": 0, "next_sprint_number": 1,
                "task_list_view": "Flat", "prefix_counters": {}, "sprint_counters": {},
                "created_at": now(), "updated_at": now() }],
            "columns": [{ "id": valid_col_id, "board_id": board_id, "name": "TODO",
                "position": 0, "created_at": now(), "updated_at": now() }],
            "sprints": [],
            "cards": [{ "id": card_id, "column_id": ghost_col_id, "title": "Orphaned",
                "priority": "Medium", "status": "Todo", "position": 0, "card_number": 1,
                "sprint_logs": [], "created_at": now(), "updated_at": now() }],
            "archived_cards": [],
            "graph": { "cards": { "edges": [] } }
        }),
    );
    let to = dir.path().join("out.sqlite");

    manager()
        .migrate_store("json", &from, "sqlite", to.to_str().unwrap())
        .await
        .unwrap();

    let ctx = KanbanContext::open_sqlite(to.to_str().unwrap(), AppConfig::default())
        .await
        .unwrap();
    let cards = ctx.cards().unwrap();
    assert_eq!(cards.len(), 1, "card should be present");
    let expected_col_id = uuid::Uuid::parse_str(&valid_col_id).unwrap();
    assert_eq!(
        cards[0].column_id, expected_col_id,
        "orphaned card should be moved to the first valid column"
    );
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_migrate_json_to_sqlite_preserves_command_log() {
    use kanban_domain::commands::{
        BoardCommand, ColumnCommand, Command, CreateBoard, CreateColumn,
    };

    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("source.json");
    let source_str = source_path.to_str().unwrap();

    // Create a JSON context, execute two commands (board + column), and save
    let mut ctx = KanbanContext::open_json(source_str, AppConfig::default())
        .await
        .unwrap();
    let board_id = uuid::Uuid::new_v4();
    ctx.execute(vec![Command::Board(BoardCommand::Create(CreateBoard {
        id: board_id,
        name: "Test Board".into(),
        card_prefix: None,
        position: 0,
    }))])
    .unwrap();
    let col_id = uuid::Uuid::new_v4();
    ctx.execute(vec![Command::Column(ColumnCommand::Create(CreateColumn {
        id: col_id,
        board_id,
        name: "TODO".into(),
        position: 0,
    }))])
    .unwrap();
    assert_eq!(ctx.undo_depth(), 2, "should have 2 undo steps");
    ctx.save().await.unwrap();

    // Migrate JSON → SQLite
    let target_path = dir.path().join("target.sqlite");
    let target_str = target_path.to_str().unwrap();
    manager()
        .migrate_store("json", source_str, "sqlite", target_str)
        .await
        .unwrap();

    // Open migrated SQLite and verify undo history is preserved
    let mut ctx2 = KanbanContext::open_sqlite(target_str, AppConfig::default())
        .await
        .unwrap();
    assert_eq!(ctx2.undo_depth(), 2, "undo depth should survive migration");

    // Verify undo restores previous state (undoes column creation)
    assert!(ctx2.undo().unwrap(), "undo should succeed");
    let columns = ctx2.columns().unwrap();
    assert!(
        columns.is_empty(),
        "undo should remove the column created by the second command"
    );
    let boards = ctx2.boards().unwrap();
    assert_eq!(
        boards.len(),
        1,
        "board from first command should still exist"
    );
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_migrate_sqlite_to_json_preserves_command_log() {
    use kanban_domain::commands::{BoardCommand, Command, CreateBoard};

    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("source.sqlite");
    let source_str = source_path.to_str().unwrap();

    // Create a SQLite context, execute commands
    let mut ctx = KanbanContext::open_sqlite(source_str, AppConfig::default())
        .await
        .unwrap();
    let board_id = uuid::Uuid::new_v4();
    ctx.execute(vec![Command::Board(BoardCommand::Create(CreateBoard {
        id: board_id,
        name: "Test Board".into(),
        card_prefix: None,
        position: 0,
    }))])
    .unwrap();
    assert!(ctx.can_undo(), "should have undo history");
    drop(ctx);

    // Migrate SQLite → JSON
    let target_path = dir.path().join("target.json");
    let target_str = target_path.to_str().unwrap();
    manager()
        .migrate_store("sqlite", source_str, "json", target_str)
        .await
        .unwrap();

    // Open migrated JSON and verify undo history is preserved
    let mut ctx2 = KanbanContext::open_json(target_str, AppConfig::default())
        .await
        .unwrap();
    assert!(ctx2.can_undo(), "undo history should survive migration");

    // Verify undo actually restores previous state
    let boards_before_undo = ctx2.boards().unwrap();
    assert_eq!(boards_before_undo.len(), 1);
    assert!(ctx2.undo().unwrap(), "undo should succeed");
    let boards_after_undo = ctx2.boards().unwrap();
    assert!(
        boards_after_undo.is_empty(),
        "undo should restore to empty state"
    );
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_migrate_store_cleans_up_destination_on_failure() {
    let dir = tempfile::tempdir().unwrap();
    let ghost_col_id = uuid::Uuid::new_v4().to_string();
    let card_id = uuid::Uuid::new_v4().to_string();

    // Write JSON that is valid JSON but would produce a non-repairable save error
    // by having a card whose column_id references nothing and there are NO valid columns
    // (so the repair fallback has nothing to fall back to — save will fail)
    let from = write_json(
        dir.path(),
        "source.json",
        serde_json::json!({
            "boards": [],
            "columns": [],
            "sprints": [],
            "cards": [{ "id": card_id, "column_id": ghost_col_id, "title": "T",
                "priority": "Medium", "status": "Todo", "position": 0, "card_number": 1,
                "sprint_logs": [], "created_at": now(), "updated_at": now() }],
            "archived_cards": [],
            "graph": { "cards": { "edges": [] } }
        }),
    );
    let to = dir.path().join("out.sqlite");

    let result = manager()
        .migrate_store("json", &from, "sqlite", to.to_str().unwrap())
        .await;

    assert!(result.is_err(), "migration should fail");
    assert!(
        !to.exists(),
        "destination file should be cleaned up after failure"
    );
}
