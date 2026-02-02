use kanban_domain::KanbanOperations;
use kanban_mcp::context::McpContext;
use tempfile::TempDir;

fn kanban_bin() -> String {
    if let Ok(path) = std::env::var("KANBAN_BIN") {
        return path;
    }
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let bin = workspace_root.join("target/debug/kanban");
    assert!(
        bin.exists(),
        "kanban binary not found at {:?}. Run `cargo build --bin kanban` first.",
        bin
    );
    bin.to_string_lossy().to_string()
}

fn setup() -> (McpContext, TempDir) {
    let dir = TempDir::new().expect("failed to create temp dir");
    let path = dir.path().join("test.kanban");
    let path_str = path.to_string_lossy().to_string();
    let ctx = McpContext::new(&path_str).with_kanban_path(&kanban_bin());
    (ctx, dir)
}

// Board round-trips

#[test]
fn board_create_list_get() {
    let (mut ctx, _tmp) = setup();
    let board = ctx
        .create_board("Test Board".into(), Some("TB".into()))
        .unwrap();
    assert_eq!(board.name, "Test Board");

    let boards = ctx.list_boards().unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].id, board.id);

    let fetched = ctx.get_board(board.id).unwrap().unwrap();
    assert_eq!(fetched.name, "Test Board");
}

#[test]
fn board_get_nonexistent() {
    let (ctx, _tmp) = setup();
    let id = uuid::Uuid::new_v4();
    let result = ctx.get_board(id).unwrap();
    assert!(result.is_none());
}

// Column round-trips

#[test]
fn column_create_list_update() {
    let (mut ctx, _tmp) = setup();
    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "To Do".into(), None).unwrap();
    assert_eq!(col.name, "To Do");

    let cols = ctx.list_columns(board.id).unwrap();
    assert!(cols.iter().any(|c| c.id == col.id));

    let updated = ctx
        .update_column(
            col.id,
            kanban_domain::ColumnUpdate {
                name: Some("Done".into()),
                position: None,
                wip_limit: kanban_domain::FieldUpdate::NoChange,
            },
        )
        .unwrap();
    assert_eq!(updated.name, "Done");
}

#[test]
fn column_reorder() {
    let (mut ctx, _tmp) = setup();
    let board = ctx.create_board("Board".into(), None).unwrap();
    let _c1 = ctx
        .create_column(board.id, "Col A".into(), Some(0))
        .unwrap();
    let c2 = ctx
        .create_column(board.id, "Col B".into(), Some(1))
        .unwrap();
    let reordered = ctx.reorder_column(c2.id, 0).unwrap();
    assert_eq!(reordered.position, 0);
}

// Card round-trips

#[test]
fn card_create_get_move_archive_restore() {
    let (mut ctx, _tmp) = setup();
    let board = ctx.create_board("Board".into(), None).unwrap();
    let col1 = ctx.create_column(board.id, "To Do".into(), None).unwrap();
    let col2 = ctx.create_column(board.id, "Done".into(), None).unwrap();

    let card = ctx
        .create_card(board.id, col1.id, "My Card".into(), Default::default())
        .unwrap();
    assert_eq!(card.title, "My Card");

    let fetched = ctx.get_card(card.id).unwrap().unwrap();
    assert_eq!(fetched.id, card.id);

    let moved = ctx.move_card(card.id, col2.id, None).unwrap();
    assert_eq!(moved.column_id, col2.id);

    ctx.archive_card(card.id).unwrap();
    let archived = ctx.list_archived_cards().unwrap();
    assert!(archived.iter().any(|c| c.card.id == card.id));

    let restored = ctx.restore_card(card.id, None).unwrap();
    assert_eq!(restored.id, card.id);
}

#[test]
fn create_card_then_update_with_all_fields() {
    let (mut ctx, _tmp) = setup();
    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "To Do".into(), None).unwrap();

    let card = ctx
        .create_card(board.id, col.id, "Full Card".into(), Default::default())
        .unwrap();
    assert_eq!(card.title, "Full Card");

    let updated = ctx
        .update_card(
            card.id,
            kanban_domain::CardUpdate {
                title: None,
                description: kanban_domain::FieldUpdate::Set("A description".into()),
                priority: Some(kanban_domain::CardPriority::High),
                status: None,
                position: None,
                column_id: None,
                points: kanban_domain::FieldUpdate::Set(5),
                due_date: kanban_domain::FieldUpdate::NoChange,
                sprint_id: kanban_domain::FieldUpdate::NoChange,
                assigned_prefix: kanban_domain::FieldUpdate::NoChange,
                card_prefix: kanban_domain::FieldUpdate::NoChange,
            },
        )
        .unwrap();
    assert_eq!(updated.title, "Full Card");
    assert_eq!(updated.description.as_deref(), Some("A description"));
}

// Sprint round-trips

#[test]
fn sprint_create_list_activate_complete() {
    let (mut ctx, _tmp) = setup();
    let board = ctx.create_board("Board".into(), None).unwrap();

    let sprint = ctx.create_sprint(board.id, None, None).unwrap();
    let sprints = ctx.list_sprints(board.id).unwrap();
    assert_eq!(sprints.len(), 1);
    assert_eq!(sprints[0].id, sprint.id);

    let activated = ctx.activate_sprint(sprint.id, Some(14)).unwrap();
    assert_eq!(activated.id, sprint.id);

    let completed = ctx.complete_sprint(sprint.id).unwrap();
    assert_eq!(completed.id, sprint.id);
}

#[test]
fn sprint_update_via_trait() {
    let (mut ctx, _tmp) = setup();
    let board = ctx.create_board("Board".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();

    let updated = ctx
        .update_sprint(
            sprint.id,
            kanban_domain::SprintUpdate {
                name: Some("Sprint Alpha".into()),
                name_index: kanban_domain::FieldUpdate::NoChange,
                prefix: kanban_domain::FieldUpdate::Set("SA".into()),
                card_prefix: kanban_domain::FieldUpdate::NoChange,
                status: None,
                start_date: kanban_domain::FieldUpdate::Set(
                    chrono::NaiveDate::from_ymd_opt(2025, 1, 1)
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_utc(),
                ),
                end_date: kanban_domain::FieldUpdate::Set(
                    chrono::NaiveDate::from_ymd_opt(2025, 1, 15)
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_utc(),
                ),
            },
        )
        .unwrap();
    assert_eq!(updated.id, sprint.id);
}

#[test]
fn sprint_cancel() {
    let (mut ctx, _tmp) = setup();
    let board = ctx.create_board("Board".into(), None).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();
    let _ = ctx.activate_sprint(sprint.id, None).unwrap();
    let cancelled = ctx.cancel_sprint(sprint.id).unwrap();
    assert_eq!(cancelled.id, sprint.id);
}

// Card-sprint assignment

#[test]
fn card_assign_unassign_sprint() {
    let (mut ctx, _tmp) = setup();
    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "To Do".into(), None).unwrap();
    let card = ctx.create_card(board.id, col.id, "Card".into(), Default::default()).unwrap();
    let sprint = ctx.create_sprint(board.id, None, None).unwrap();

    let assigned = ctx.assign_card_to_sprint(card.id, sprint.id).unwrap();
    assert_eq!(assigned.sprint_id, Some(sprint.id));

    let unassigned = ctx.unassign_card_from_sprint(card.id).unwrap();
    assert_eq!(unassigned.sprint_id, None);
}

// Bulk operations

#[test]
fn bulk_archive() {
    let (mut ctx, _tmp) = setup();
    let board = ctx.create_board("Board".into(), None).unwrap();
    let col = ctx.create_column(board.id, "Col".into(), None).unwrap();
    let c1 = ctx.create_card(board.id, col.id, "Card 1".into(), Default::default()).unwrap();
    let c2 = ctx.create_card(board.id, col.id, "Card 2".into(), Default::default()).unwrap();

    let count = ctx.bulk_archive_cards(vec![c1.id, c2.id]).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn bulk_move() {
    let (mut ctx, _tmp) = setup();
    let board = ctx.create_board("Board".into(), None).unwrap();
    let col1 = ctx.create_column(board.id, "From".into(), None).unwrap();
    let col2 = ctx.create_column(board.id, "To".into(), None).unwrap();
    let c1 = ctx.create_card(board.id, col1.id, "Card 1".into(), Default::default()).unwrap();
    let c2 = ctx.create_card(board.id, col1.id, "Card 2".into(), Default::default()).unwrap();

    let count = ctx.bulk_move_cards(vec![c1.id, c2.id], col2.id).unwrap();
    assert_eq!(count, 2);
}

// Export/Import round-trip

#[test]
fn export_import_roundtrip() {
    let (mut ctx, _tmp) = setup();
    let board = ctx.create_board("Export Board".into(), None).unwrap();
    let _col = ctx.create_column(board.id, "Col".into(), None).unwrap();

    let json = ctx.export_board(Some(board.id)).unwrap();
    assert!(json.contains("Export Board"));

    ctx.import_board(&json).unwrap();
}
