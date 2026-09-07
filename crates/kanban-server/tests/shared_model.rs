#![cfg(feature = "test-helpers")]

use kanban_domain::{LoadState, NoProjections};
use kanban_persistence_json::{JsonDataStore, JsonFileStore};
use kanban_server::state::{AppState, Session};
use kanban_server::test_helpers::make_sqlite_state;
use kanban_service::{
    requestable, AppConfig, FetchPlan, FetchRound, KanbanBackend, KanbanContext, KanbanOperations,
    LoadedEntities,
};
use std::sync::Arc;

struct BoardListPlan;

impl FetchPlan for BoardListPlan {
    fn next_round(&self, loaded: &dyn LoadedEntities) -> FetchRound {
        FetchRound {
            board_list: requestable(loaded.board_list()),
            ..Default::default()
        }
    }
}

fn json_state(path: &std::path::Path) -> AppState {
    let backend: Arc<dyn KanbanBackend> =
        Arc::new(JsonDataStore::new(Arc::new(JsonFileStore::new(path))));
    let ctx = KanbanContext::open_deferred(backend, AppConfig::default());
    AppState::new(ctx)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_lock_session_resets_the_model_for_a_sqlite_locator() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("board.sqlite");
    let state = make_sqlite_state(&db_path).await;

    {
        let mut guard = state.lock_session().await;
        let Session { ctx, model } = &mut *guard;
        ctx.create_board("B".into(), None).unwrap();
        ctx.sync(&BoardListPlan, model, &mut NoProjections);
        assert!(matches!(model.boards_state(), LoadState::Loaded(_)));
    }

    let guard = state.lock_session().await;
    assert!(matches!(guard.model.boards_state(), LoadState::NotLoaded));
}

#[tokio::test]
async fn test_lock_session_keeps_the_model_for_a_json_locator() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board.json");
    let state = json_state(&path);

    {
        let mut guard = state.lock_session().await;
        let Session { ctx, model } = &mut *guard;
        ctx.create_board("B".into(), None).unwrap();
        ctx.sync(&BoardListPlan, model, &mut NoProjections);
        assert!(matches!(model.boards_state(), LoadState::Loaded(_)));
    }

    let guard = state.lock_session().await;
    match guard.model.boards_state() {
        LoadState::Loaded(boards) => assert_eq!(boards.len(), 1),
        other => panic!("expected Loaded, got {other:?}"),
    }
}

#[tokio::test]
async fn test_app_state_new_does_not_reset_the_model_per_request() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board.json");
    let state = json_state(&path);

    assert!(!state.reset_model_per_request);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_external_file_change_invalidates_the_shared_model() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board.json");
    let backend: Arc<dyn KanbanBackend> =
        Arc::new(JsonDataStore::new(Arc::new(JsonFileStore::new(&path))));
    let ctx = KanbanContext::open(backend, AppConfig::default())
        .await
        .unwrap();
    let state = AppState::new(ctx);

    kanban_server::watch::watch_for_external_changes(state.clone(), path.to_str().unwrap(), false)
        .await
        .unwrap();

    {
        let mut guard = state.ctx.lock().await;
        let Session { ctx, model } = &mut *guard;
        ctx.sync(&BoardListPlan, model, &mut NoProjections);
        assert!(matches!(model.boards_state(), LoadState::Loaded(_)));
    }

    {
        let backend: Arc<dyn KanbanBackend> =
            Arc::new(JsonDataStore::new(Arc::new(JsonFileStore::new(&path))));
        let mut other = KanbanContext::open(backend, AppConfig::default())
            .await
            .unwrap();
        other.create_board("External".into(), None).unwrap();
        other.save().await.unwrap();
    }

    let mut became_not_loaded = false;
    for _ in 0..50 {
        {
            let guard = state.ctx.lock().await;
            if matches!(guard.model.boards_state(), LoadState::NotLoaded) {
                became_not_loaded = true;
            }
        }
        if became_not_loaded {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    assert!(became_not_loaded);

    let guard = state.ctx.lock().await;
    assert_eq!(guard.ctx.list_boards().unwrap().len(), 1);
}
