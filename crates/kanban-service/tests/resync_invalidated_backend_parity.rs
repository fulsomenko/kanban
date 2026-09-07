use kanban_backend_memory::InMemoryStore;
use kanban_domain::{Board, Card, Column, FieldUpdate, Invalidation, NoProjections, Sprint};
use kanban_persistence_json::{JsonDataStore, JsonFileStore};
use kanban_service::{
    requestable, AppConfig, CardUpdate, FetchPlan, FetchRound, KanbanBackend, KanbanContext,
    KanbanOperations, LoadedEntities, SprintUpdate,
};
use std::sync::Arc;
use tempfile::tempdir;
use uuid::Uuid;

struct WarmPlan {
    card_id: Uuid,
    sprint_id: Uuid,
}

impl FetchPlan for WarmPlan {
    fn next_round(&self, loaded: &dyn LoadedEntities) -> FetchRound {
        FetchRound {
            board_list: requestable(loaded.board_list()),
            column_list: requestable(loaded.column_list()),
            card_list: requestable(loaded.card_list()),
            sprint_list: requestable(loaded.sprint_list()),
            graph: requestable(loaded.graph()),
            cards: if requestable(loaded.card(self.card_id)) {
                vec![self.card_id]
            } else {
                vec![]
            },
            sprints: if requestable(loaded.sprint(self.sprint_id)) {
                vec![self.sprint_id]
            } else {
                vec![]
            },
            archived_card_list: requestable(loaded.archived_card_list()),
            archived_board_list: requestable(loaded.archived_board_list()),
            ..Default::default()
        }
    }
}

struct NothingPlan;
impl FetchPlan for NothingPlan {
    fn next_round(&self, _loaded: &dyn LoadedEntities) -> FetchRound {
        FetchRound::default()
    }
}

fn seed(ctx: &mut KanbanContext) -> (Board, Column, Card, Sprint) {
    let board = ctx
        .create_board("Board".into(), Some("BRD".into()))
        .unwrap();
    let column = ctx.create_column(board.id, "Col".into(), None).unwrap();
    let card = ctx
        .create_card(
            board.id,
            column.id,
            "before".into(),
            kanban_domain::CreateCardOptions::default(),
        )
        .unwrap();
    let sprint = ctx
        .create_sprint(board.id, Some("SPR".into()), Some("Sprint".into()))
        .unwrap();
    (board, column, card, sprint)
}

fn assert_resync_invalidated_returns_the_updated_graph(ctx: &mut KanbanContext) {
    let (board, column, card, sprint) = seed(ctx);

    let mut model = kanban_domain::Model::default();
    ctx.sync(
        &WarmPlan {
            card_id: card.id,
            sprint_id: sprint.id,
        },
        &mut model,
        &mut NoProjections,
    );
    assert!(model.boards_state().is_loaded());
    assert!(model.columns_state().is_loaded());
    assert!(model.cards_state().is_loaded());
    assert!(model.sprints_state().is_loaded());
    assert!(model.graph_state().is_loaded());
    assert!(model.card_by_id_state(card.id).loaded().is_some());
    assert!(model.sprint_by_id_state(sprint.id).loaded().is_some());

    let (_card, inv_card) = ctx
        .update_card_impl(
            card.id,
            CardUpdate {
                title: Some("after".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let (_sprint, inv_sprint) = ctx
        .update_sprint_impl(
            sprint.id,
            SprintUpdate {
                prefix: FieldUpdate::Set("REN".into()),
                ..Default::default()
            },
        )
        .unwrap();

    let combined = match (inv_card, inv_sprint) {
        (Invalidation::Entities(mut ids), Invalidation::Entities(other)) => {
            ids.merge(other);
            Invalidation::Entities(ids)
        }
        _ => Invalidation::All,
    };
    ctx.resync_invalidated(combined, &NothingPlan, &mut model, &mut NoProjections);

    assert_eq!(
        model.card_by_id_state(card.id).loaded().unwrap().title,
        "after"
    );
    assert_eq!(
        model.sprint_by_id_state(sprint.id).loaded().unwrap().prefix,
        Some("REN".into())
    );
    assert!(model.cards_state().is_loaded());
    assert!(model.sprints_state().is_loaded());

    assert!(model.boards_state().is_loaded());
    assert!(model
        .boards_state()
        .loaded_or_empty()
        .iter()
        .any(|b| b.id == board.id));
    assert!(model.columns_state().is_loaded());
    assert!(model
        .columns_state()
        .loaded_or_empty()
        .iter()
        .any(|c| c.id == column.id));
    assert!(model.graph_state().is_loaded());
}

fn assert_resync_invalidated_classifies_a_restored_card_as_live(ctx: &mut KanbanContext) {
    let (_board, _column, card, sprint) = seed(ctx);

    let _ = ctx.archive_card_impl(card.id).unwrap();

    let mut model = kanban_domain::Model::default();
    ctx.sync(
        &WarmPlan {
            card_id: card.id,
            sprint_id: sprint.id,
        },
        &mut model,
        &mut NoProjections,
    );
    assert!(model.archived_cards_state().is_loaded());
    assert!(model.archived_card_ids().contains(&card.id));

    let (_card, inv) = ctx.restore_card_impl(card.id, None).unwrap();
    assert!(matches!(inv, Invalidation::Entities(_)));

    ctx.resync_invalidated(inv, &NothingPlan, &mut model, &mut NoProjections);

    assert!(!model.archived_card_ids().contains(&card.id));
    assert!(model.archived_cards_state().is_loaded());
}

fn assert_resync_invalidated_classifies_an_archived_board_as_archived(ctx: &mut KanbanContext) {
    let (board, _column, card, sprint) = seed(ctx);

    let mut model = kanban_domain::Model::default();
    ctx.sync(
        &WarmPlan {
            card_id: card.id,
            sprint_id: sprint.id,
        },
        &mut model,
        &mut NoProjections,
    );
    assert!(model.archived_boards_state().is_loaded());
    assert!(!model.archived_board_ids().contains(&board.id));

    let inv = ctx.archive_board_impl(board.id).unwrap();
    assert!(matches!(inv, Invalidation::Entities(_)));

    ctx.resync_invalidated(inv, &NothingPlan, &mut model, &mut NoProjections);

    assert!(model.archived_board_ids().contains(&board.id));
    assert!(model.archived_boards_state().is_loaded());
}

#[test]
fn test_resync_invalidated_returns_the_updated_card_on_the_in_memory_backend() {
    let mut ctx =
        KanbanContext::open_deferred(Arc::new(InMemoryStore::new()), AppConfig::default());
    assert_resync_invalidated_returns_the_updated_graph(&mut ctx);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resync_invalidated_returns_the_updated_card_on_the_json_backend() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.json");
    let backend: Arc<dyn KanbanBackend> =
        Arc::new(JsonDataStore::new(Arc::new(JsonFileStore::new(&path))));
    let mut ctx = KanbanContext::open(backend, AppConfig::default())
        .await
        .unwrap();
    assert_resync_invalidated_returns_the_updated_graph(&mut ctx);
}

async fn open_sqlite_context(locator: &str, config: AppConfig) -> KanbanContext {
    let mut config = config;
    let mut stores = kanban_persistence::StoreRegistry::new();
    let mut backends = kanban_backend::KanbanBackendRegistry::new();
    backends.register(Box::new(kanban_persistence_sqlite::SqliteBackendFactory));
    stores.register(Box::new(kanban_persistence_json::JsonStoreFactory));
    backends.register(Box::new(kanban_persistence_json::JsonBackendFactory));
    let sm = kanban_service::StoreManager::new(stores, backends);
    sm.sync_backend_with_file(locator, &mut config);
    let backend = sm.make_backend(locator, &config).await.unwrap();
    KanbanContext::open(backend, config).await.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resync_invalidated_returns_the_updated_card_on_the_sqlite_backend() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.sqlite3");
    let mut ctx = open_sqlite_context(path.to_str().unwrap(), AppConfig::default()).await;
    assert_resync_invalidated_returns_the_updated_graph(&mut ctx);
}

#[test]
fn test_resync_invalidated_classifies_restored_card_as_live_on_the_in_memory_backend() {
    let mut ctx =
        KanbanContext::open_deferred(Arc::new(InMemoryStore::new()), AppConfig::default());
    assert_resync_invalidated_classifies_a_restored_card_as_live(&mut ctx);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resync_invalidated_classifies_restored_card_as_live_on_the_json_backend() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.json");
    let backend: Arc<dyn KanbanBackend> =
        Arc::new(JsonDataStore::new(Arc::new(JsonFileStore::new(&path))));
    let mut ctx = KanbanContext::open(backend, AppConfig::default())
        .await
        .unwrap();
    assert_resync_invalidated_classifies_a_restored_card_as_live(&mut ctx);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resync_invalidated_classifies_restored_card_as_live_on_the_sqlite_backend() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.sqlite3");
    let mut ctx = open_sqlite_context(path.to_str().unwrap(), AppConfig::default()).await;
    assert_resync_invalidated_classifies_a_restored_card_as_live(&mut ctx);
}

#[test]
fn test_resync_invalidated_classifies_archived_board_as_archived_on_the_in_memory_backend() {
    let mut ctx =
        KanbanContext::open_deferred(Arc::new(InMemoryStore::new()), AppConfig::default());
    assert_resync_invalidated_classifies_an_archived_board_as_archived(&mut ctx);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resync_invalidated_classifies_archived_board_as_archived_on_the_json_backend() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.json");
    let backend: Arc<dyn KanbanBackend> =
        Arc::new(JsonDataStore::new(Arc::new(JsonFileStore::new(&path))));
    let mut ctx = KanbanContext::open(backend, AppConfig::default())
        .await
        .unwrap();
    assert_resync_invalidated_classifies_an_archived_board_as_archived(&mut ctx);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resync_invalidated_classifies_archived_board_as_archived_on_the_sqlite_backend() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.sqlite3");
    let mut ctx = open_sqlite_context(path.to_str().unwrap(), AppConfig::default()).await;
    assert_resync_invalidated_classifies_an_archived_board_as_archived(&mut ctx);
}
