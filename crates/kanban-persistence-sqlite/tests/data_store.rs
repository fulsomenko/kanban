use kanban_domain::command_store::CommandStore;
use kanban_domain::commands::{BoardCommand, Command, CreateBoard};
use kanban_domain::data_store::DataStore;
use kanban_domain::*;
use kanban_persistence_sqlite::SqliteStore;
use tempfile::TempDir;
use uuid::Uuid;

async fn make_store() -> (SqliteStore, TempDir) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::open(&path).await.unwrap();
    (store, dir)
}

fn make_board(name: &str) -> Board {
    Board::new(name.to_string(), None)
}

fn make_column(board_id: Uuid, name: &str, pos: i32) -> Column {
    Column::new(board_id, name.to_string(), pos)
}

fn make_card(board: &mut Board, column_id: Uuid, title: &str, pos: i32) -> Card {
    Card::new(board, column_id, title.to_string(), pos)
}

// --- Board CRUD ---

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_upsert_and_get_board() {
    let (store, _dir) = make_store().await;
    let mut board = make_board("Test Board");
    board.sprint_names = vec!["Alpha".to_string(), "Beta".to_string()];
    board.sprint_counters.insert("SP".to_string(), 5);
    let id = board.id;
    store.upsert_board(board).unwrap();

    let fetched = store.get_board(id).unwrap().unwrap();
    assert_eq!(fetched.name, "Test Board");
    assert_eq!(fetched.sprint_names, vec!["Alpha", "Beta"]);
    assert_eq!(fetched.sprint_counters.get("SP"), Some(&5));
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_list_boards_empty() {
    let (store, _dir) = make_store().await;
    assert!(store.list_boards().unwrap().is_empty());
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_delete_board_removes_it() {
    let (store, _dir) = make_store().await;
    let board = make_board("To Delete");
    let id = board.id;
    store.upsert_board(board).unwrap();
    store.delete_board(id).unwrap();
    assert!(store.get_board(id).unwrap().is_none());
}

// --- Column CRUD ---

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_upsert_and_get_column() {
    let (store, _dir) = make_store().await;
    let board = make_board("B");
    store.upsert_board(board.clone()).unwrap();
    let col = make_column(board.id, "Col", 0);
    let col_id = col.id;
    store.upsert_column(col).unwrap();

    let fetched = store.get_column(col_id).unwrap().unwrap();
    assert_eq!(fetched.name, "Col");
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_list_columns_by_board_filters_correctly() {
    let (store, _dir) = make_store().await;
    let board1 = make_board("B1");
    let board2 = make_board("B2");
    store.upsert_board(board1.clone()).unwrap();
    store.upsert_board(board2.clone()).unwrap();

    store
        .upsert_column(make_column(board1.id, "C1", 0))
        .unwrap();
    store
        .upsert_column(make_column(board1.id, "C2", 1))
        .unwrap();
    store
        .upsert_column(make_column(board2.id, "C3", 0))
        .unwrap();

    let cols = store.list_columns_by_board(board1.id).unwrap();
    assert_eq!(cols.len(), 2);
    assert!(cols.iter().all(|c| c.board_id == board1.id));
}

// --- Card CRUD ---

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_upsert_and_get_card() {
    let (store, _dir) = make_store().await;
    let mut board = make_board("B");
    let col = make_column(board.id, "Col", 0);
    store.upsert_board(board.clone()).unwrap();
    store.upsert_column(col.clone()).unwrap();

    let card = make_card(&mut board, col.id, "Card", 0);
    let card_id = card.id;
    store.upsert_card(card).unwrap();

    let fetched = store.get_card(card_id).unwrap().unwrap();
    assert_eq!(fetched.title, "Card");
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_list_cards_by_column() {
    let (store, _dir) = make_store().await;
    let mut board = make_board("B");
    let col1 = make_column(board.id, "C1", 0);
    let col2 = make_column(board.id, "C2", 1);
    store.upsert_board(board.clone()).unwrap();
    store.upsert_column(col1.clone()).unwrap();
    store.upsert_column(col2.clone()).unwrap();

    store
        .upsert_card(make_card(&mut board, col1.id, "Card1", 0))
        .unwrap();
    store
        .upsert_card(make_card(&mut board, col1.id, "Card2", 1))
        .unwrap();
    store
        .upsert_card(make_card(&mut board, col2.id, "Card3", 0))
        .unwrap();

    let cards = store.list_cards_by_column(col1.id).unwrap();
    assert_eq!(cards.len(), 2);
    assert!(cards.iter().all(|c| c.column_id == col1.id));
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_list_cards_by_sprint() {
    let (store, _dir) = make_store().await;
    let mut board = make_board("B");
    let col = make_column(board.id, "C", 0);
    let sprint = Sprint::new(board.id, 1, None, None);
    store.upsert_board(board.clone()).unwrap();
    store.upsert_column(col.clone()).unwrap();
    store.upsert_sprint(sprint.clone()).unwrap();

    let mut card1 = make_card(&mut board, col.id, "Card1", 0);
    card1.sprint_id = Some(sprint.id);
    let card2 = make_card(&mut board, col.id, "Card2", 1);
    store.upsert_card(card1).unwrap();
    store.upsert_card(card2).unwrap();

    let cards = store.list_cards_by_sprint(sprint.id).unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].sprint_id, Some(sprint.id));
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_count_cards_in_column() {
    let (store, _dir) = make_store().await;
    let mut board = make_board("B");
    let col = make_column(board.id, "C", 0);
    store.upsert_board(board.clone()).unwrap();
    store.upsert_column(col.clone()).unwrap();

    store
        .upsert_card(make_card(&mut board, col.id, "C1", 0))
        .unwrap();
    store
        .upsert_card(make_card(&mut board, col.id, "C2", 1))
        .unwrap();

    assert_eq!(store.count_cards_in_column(col.id).unwrap(), 2);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_count_cards_in_column_excluding() {
    let (store, _dir) = make_store().await;
    let mut board = make_board("B");
    let col = make_column(board.id, "C", 0);
    store.upsert_board(board.clone()).unwrap();
    store.upsert_column(col.clone()).unwrap();

    let card1 = make_card(&mut board, col.id, "C1", 0);
    let card1_id = card1.id;
    store.upsert_card(card1).unwrap();
    store
        .upsert_card(make_card(&mut board, col.id, "C2", 1))
        .unwrap();

    let count = store
        .count_cards_in_column_excluding(col.id, &[card1_id])
        .unwrap();
    assert_eq!(count, 1);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_clear_sprint_from_cards() {
    let (store, _dir) = make_store().await;
    let mut board = make_board("B");
    let col = make_column(board.id, "C", 0);
    let sprint = Sprint::new(board.id, 1, None, None);
    store.upsert_board(board.clone()).unwrap();
    store.upsert_column(col.clone()).unwrap();
    store.upsert_sprint(sprint.clone()).unwrap();

    let mut card1 = make_card(&mut board, col.id, "C1", 0);
    card1.sprint_id = Some(sprint.id);
    let card1_id = card1.id;
    store.upsert_card(card1).unwrap();

    store.clear_sprint_from_cards(sprint.id).unwrap();
    assert!(store
        .get_card(card1_id)
        .unwrap()
        .unwrap()
        .sprint_id
        .is_none());
}

// --- Sprint CRUD ---

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_upsert_and_get_sprint() {
    let (store, _dir) = make_store().await;
    let board = make_board("B");
    store.upsert_board(board.clone()).unwrap();
    let sprint = Sprint::new(board.id, 1, None, None);
    let sprint_id = sprint.id;
    store.upsert_sprint(sprint).unwrap();

    let fetched = store.get_sprint(sprint_id).unwrap().unwrap();
    assert_eq!(fetched.sprint_number, 1);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_list_sprints_by_board() {
    let (store, _dir) = make_store().await;
    let board1 = make_board("B1");
    let board2 = make_board("B2");
    store.upsert_board(board1.clone()).unwrap();
    store.upsert_board(board2.clone()).unwrap();

    store
        .upsert_sprint(Sprint::new(board1.id, 1, None, None))
        .unwrap();
    store
        .upsert_sprint(Sprint::new(board1.id, 2, None, None))
        .unwrap();
    store
        .upsert_sprint(Sprint::new(board2.id, 1, None, None))
        .unwrap();

    let sprints = store.list_sprints_by_board(board1.id).unwrap();
    assert_eq!(sprints.len(), 2);
}

// --- Archived card ---

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_insert_and_get_archived_card() {
    let (store, _dir) = make_store().await;
    let mut board = make_board("B");
    let col = make_column(board.id, "C", 0);
    store.upsert_board(board.clone()).unwrap();
    store.upsert_column(col.clone()).unwrap();

    let card = make_card(&mut board, col.id, "Card", 0);
    let card_id = card.id;
    let ac = ArchivedCard::new(card, col.id, 0);
    store.insert_archived_card(ac).unwrap();

    let fetched = store.get_archived_card(card_id).unwrap().unwrap();
    assert_eq!(fetched.card.id, card_id);
    assert_eq!(fetched.original_column_id, col.id);

    // Archived card should NOT appear in active card queries
    assert!(store.get_card(card_id).unwrap().is_none());
    assert!(store.list_all_cards().unwrap().is_empty());
}

// --- Graph ---

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_set_and_get_graph() {
    let (store, _dir) = make_store().await;
    let graph = DependencyGraph::new();
    store.set_graph(graph.clone()).unwrap();
    let fetched = store.get_graph().unwrap();
    assert_eq!(fetched, graph);
}

// --- Snapshot ---

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_snapshot_roundtrip() {
    let (store, _dir) = make_store().await;
    let mut board = make_board("B");
    let col = make_column(board.id, "C", 0);
    let sprint = Sprint::new(board.id, 1, None, None);
    store.upsert_board(board.clone()).unwrap();
    store.upsert_column(col.clone()).unwrap();
    store.upsert_sprint(sprint.clone()).unwrap();
    let card = make_card(&mut board, col.id, "Card", 0);
    store.upsert_card(card).unwrap();

    let snap = store.snapshot().unwrap();
    assert_eq!(snap.boards.len(), 1);
    assert_eq!(snap.columns.len(), 1);
    assert_eq!(snap.cards.len(), 1);
    assert_eq!(snap.sprints.len(), 1);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_apply_snapshot_replaces_existing_data() {
    let (store, _dir) = make_store().await;
    let board_old = make_board("Old");
    store.upsert_board(board_old).unwrap();

    let board_new = make_board("New");
    let snap = Snapshot::from_data(
        vec![board_new],
        vec![],
        vec![],
        vec![],
        vec![],
        DependencyGraph::new(),
    );
    store.apply_snapshot(snap).unwrap();

    let boards = store.list_boards().unwrap();
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "New");
}

// --- CommandStore ---

fn make_board_cmd(name: &str) -> Command {
    Command::Board(BoardCommand::Create(CreateBoard {
        id: Uuid::new_v4(),
        name: name.into(),
        card_prefix: None,
        position: 0,
    }))
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_command_store_append_returns_count() {
    let (store, _dir) = make_store().await;
    let count = store.append_commands(&[make_board_cmd("B1")]).unwrap();
    assert_eq!(count, 1);
    let count = store.append_commands(&[make_board_cmd("B2")]).unwrap();
    assert_eq!(count, 2);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_command_store_count_starts_at_zero() {
    let (store, _dir) = make_store().await;
    assert_eq!(store.command_count().unwrap(), 0);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_command_store_load_returns_slice() {
    let (store, _dir) = make_store().await;
    store.append_commands(&[make_board_cmd("B1")]).unwrap();
    store.append_commands(&[make_board_cmd("B2")]).unwrap();
    store.append_commands(&[make_board_cmd("B3")]).unwrap();

    let batches = store.load_commands(0, 3).unwrap();
    assert_eq!(batches.len(), 3);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_command_store_load_range_exclusive_end() {
    let (store, _dir) = make_store().await;
    store.append_commands(&[make_board_cmd("B1")]).unwrap();
    store.append_commands(&[make_board_cmd("B2")]).unwrap();

    assert_eq!(store.load_commands(0, 1).unwrap().len(), 1);
    assert_eq!(store.load_commands(1, 2).unwrap().len(), 1);
    assert_eq!(store.load_commands(0, 2).unwrap().len(), 2);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_command_store_truncate_removes_tail() {
    let (store, _dir) = make_store().await;
    store.append_commands(&[make_board_cmd("B1")]).unwrap();
    store.append_commands(&[make_board_cmd("B2")]).unwrap();
    store.append_commands(&[make_board_cmd("B3")]).unwrap();

    store.truncate_commands_after(1).unwrap();
    assert_eq!(store.command_count().unwrap(), 1);
    assert_eq!(store.load_commands(0, 1).unwrap().len(), 1);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_command_store_batch_stores_multiple_commands() {
    let (store, _dir) = make_store().await;
    let batch = vec![make_board_cmd("B1"), make_board_cmd("B2")];
    store.append_commands(&batch).unwrap();

    let batches = store.load_commands(0, 1).unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].len(), 2);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_command_store_truncate_after_zero_clears_all() {
    let (store, _dir) = make_store().await;
    store.append_commands(&[make_board_cmd("B1")]).unwrap();
    store.append_commands(&[make_board_cmd("B2")]).unwrap();
    store.append_commands(&[make_board_cmd("B3")]).unwrap();
    assert_eq!(store.command_count().unwrap(), 3);

    store.truncate_commands_after(0).unwrap();
    assert_eq!(store.command_count().unwrap(), 0);
    assert!(store.load_commands(0, 10).unwrap().is_empty());
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_command_store_load_all_commands_consistent() {
    let (store, _dir) = make_store().await;
    store.append_commands(&[make_board_cmd("B1")]).unwrap();
    store.append_commands(&[make_board_cmd("B2")]).unwrap();

    let (batches, count) = store.load_all_commands().unwrap();
    assert_eq!(count, 2);
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].len(), 1);
    assert_eq!(batches[1].len(), 1);
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_command_store_load_from_beyond_end_returns_empty() {
    let (store, _dir) = make_store().await;
    store.append_commands(&[make_board_cmd("B1")]).unwrap();

    let batches = store.load_commands(5, 10).unwrap();
    assert!(batches.is_empty());
}

#[tokio::test(flavor = "current_thread")]
#[should_panic(expected = "SqliteStore requires a multi-threaded Tokio runtime")]
async fn test_sqlite_on_current_thread_runtime_gives_clear_error() {
    let (store, _dir) = make_store().await;
    let board = make_board("CurrentThread");
    store.upsert_board(board).unwrap();
}

// multi_thread: sqlx connection pool spawns background tasks that deadlock on single-threaded runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_concurrent_reads_and_writes_no_panic() {
    use std::sync::Arc;

    let (store, _dir) = make_store().await;
    let store = Arc::new(store);
    let mut handles = vec![];

    for i in 0..10 {
        let s = Arc::clone(&store);
        handles.push(tokio::spawn(async move {
            let board = Board::new(format!("Board-{i}"), None);
            s.upsert_board(board.clone()).unwrap();
            let col = Column::new(board.id, format!("Col-{i}"), i);
            s.upsert_column(col).unwrap();
        }));
    }

    for _ in 0..10 {
        let s = Arc::clone(&store);
        handles.push(tokio::spawn(async move {
            for _ in 0..10 {
                let _ = s.list_boards();
                let _ = s.list_all_columns();
                let _ = s.list_all_cards();
                let _ = s.snapshot();
            }
        }));
    }

    for h in handles {
        h.await.expect("task should not panic");
    }

    let boards = store.list_boards().unwrap();
    assert_eq!(boards.len(), 10);
}
