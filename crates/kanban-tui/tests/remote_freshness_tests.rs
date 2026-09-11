mod helpers;

use helpers::{CountingBackend, ReadOpLog};
use kanban_core::ClientId;
use kanban_domain::{CreateCardOptions, KanbanOperations};
use kanban_service::api::{ChangeEventFrame, EntityIdsDto, InvalidationDto};
use kanban_tui::app::mode::{AppMode, DialogMode};
use kanban_tui::App;
use uuid::Uuid;

struct Seed {
    board: Uuid,
    card: Uuid,
}

fn seed_board_column_card(app: &mut App) -> Seed {
    let board = app.ctx.create_board("Board".to_string(), None).unwrap();
    let column = app
        .ctx
        .create_column(board.id, "Column".to_string(), Some(0))
        .unwrap();
    let card = app
        .ctx
        .create_card(
            board.id,
            column.id,
            "Card".to_string(),
            CreateCardOptions::default(),
        )
        .unwrap();
    Seed {
        board: board.id,
        card: card.id,
    }
}

async fn prime(app: &mut App) -> ReadOpLog {
    let (backend, _reads, ops) = CountingBackend::wrap(app.ctx.backend());
    app.ctx.replace_backend(backend);
    app.load_initial_state().await;
    ops.lock().unwrap().clear();
    ops
}

async fn prime_with_instance_id(app: &mut App, id: Uuid) -> ReadOpLog {
    let (backend, _reads, ops) = CountingBackend::wrap_with_instance_id(app.ctx.backend(), id);
    app.ctx.replace_backend(backend);
    app.load_initial_state().await;
    ops.lock().unwrap().clear();
    ops
}

fn has_op(ops: &ReadOpLog, method: &str) -> bool {
    ops.lock().unwrap().iter().any(|op| op.method == method)
}

fn foreign_frame(issued_by: ClientId, invalidation: Option<InvalidationDto>) -> ChangeEventFrame {
    let frame = ChangeEventFrame::now(Uuid::new_v4(), Uuid::new_v4(), issued_by);
    match invalidation {
        Some(inv) => frame.with_invalidation(inv),
        None => frame,
    }
}

fn rename_card_out_of_band(app: &App, card_id: Uuid, new_title: &str) {
    let store = app.ctx.backend();
    let mut card = store
        .as_data_store()
        .get_card(card_id)
        .unwrap()
        .expect("card exists");
    card.title = new_title.to_string();
    store.as_data_store().upsert_card(card).unwrap();
}

fn rename_board_out_of_band(app: &App, board_id: Uuid, new_name: &str) {
    let store = app.ctx.backend();
    let mut board = store
        .as_data_store()
        .get_board(board_id)
        .unwrap()
        .expect("board exists");
    board.name = new_name.to_string();
    store.as_data_store().upsert_board(board).unwrap();
}

#[tokio::test]
async fn test_foreign_frame_invalidation_refreshes_model_via_resolve_after_command() {
    let mut app = App::test_default();
    let seed = seed_board_column_card(&mut app);
    app.selection.active_board_id = Some(seed.board);
    let ops = prime(&mut app).await;

    rename_card_out_of_band(&app, seed.card, "Renamed");
    ops.lock().unwrap().clear();

    let frame = foreign_frame(
        ClientId::from(Uuid::new_v4()),
        Some(InvalidationDto::Entities(EntityIdsDto {
            cards: vec![seed.card],
            ..Default::default()
        })),
    );
    app.handle_remote_change_frame(frame);

    let card = app.model.card_by_id_state(seed.card).loaded().copied();
    assert_eq!(card.map(|c| c.title.clone()), Some("Renamed".to_string()));
    assert!(!has_op(&ops, "list_boards"));
}

#[tokio::test]
async fn test_frame_without_invalidation_resolves_as_all() {
    let mut app = App::test_default();
    let seed = seed_board_column_card(&mut app);
    app.selection.active_board_id = Some(seed.board);
    let ops = prime(&mut app).await;

    rename_board_out_of_band(&app, seed.board, "Renamed Board");
    ops.lock().unwrap().clear();

    let frame = foreign_frame(ClientId::from(Uuid::new_v4()), None);
    app.handle_remote_change_frame(frame);

    let board = app.model.board_by_id_state(seed.board).loaded().copied();
    assert_eq!(
        board.map(|b| b.name.clone()),
        Some("Renamed Board".to_string())
    );
    assert!(has_op(&ops, "list_boards"));
}

#[tokio::test]
async fn test_own_frame_matching_backend_instance_id_is_suppressed() {
    let mut app = App::test_default();
    let seed = seed_board_column_card(&mut app);
    app.selection.active_board_id = Some(seed.board);
    let own_id = Uuid::new_v4();
    let ops = prime_with_instance_id(&mut app, own_id).await;

    rename_board_out_of_band(&app, seed.board, "Renamed Board");
    ops.lock().unwrap().clear();
    app.needs_redraw = false;

    let frame = foreign_frame(ClientId::from(own_id), Some(InvalidationDto::All));
    app.handle_remote_change_frame(frame);

    let board = app.model.board_by_id_state(seed.board).loaded().copied();
    assert_eq!(board.map(|b| b.name.clone()), Some("Board".to_string()));
    assert!(ops.lock().unwrap().is_empty());
    assert!(!app.needs_redraw);
    assert_eq!(app.mode, AppMode::Normal);
}

#[tokio::test]
async fn test_nil_issued_by_frame_is_never_suppressed() {
    let mut app = App::test_default();
    let seed = seed_board_column_card(&mut app);
    app.selection.active_board_id = Some(seed.board);
    let ops = prime(&mut app).await;

    rename_board_out_of_band(&app, seed.board, "Renamed Board");
    ops.lock().unwrap().clear();

    let frame = foreign_frame(ClientId::nil(), None);
    app.handle_remote_change_frame(frame);

    let board = app.model.board_by_id_state(seed.board).loaded().copied();
    assert_eq!(
        board.map(|b| b.name.clone()),
        Some("Renamed Board".to_string())
    );
}

#[tokio::test]
async fn test_dirty_model_frame_opens_external_change_dialog() {
    let mut app = App::test_default();
    let seed = seed_board_column_card(&mut app);
    app.selection.active_board_id = Some(seed.board);
    let ops = prime(&mut app).await;

    app.ctx.mark_dirty();
    rename_board_out_of_band(&app, seed.board, "Renamed Board");
    ops.lock().unwrap().clear();

    let frame = foreign_frame(ClientId::from(Uuid::new_v4()), Some(InvalidationDto::All));
    app.handle_remote_change_frame(frame);

    assert_eq!(
        app.mode,
        AppMode::Dialog(DialogMode::ExternalChangeDetected)
    );
    let board = app.model.board_by_id_state(seed.board).loaded().copied();
    assert_eq!(board.map(|b| b.name.clone()), Some("Board".to_string()));
    assert!(ops.lock().unwrap().is_empty());
}
