#![cfg(feature = "sqlite")]

//! SQLite audit log persists across sessions.

use kanban_domain::command_store::CommandStore;
use kanban_domain::commands::{BoardCommand, Command, CreateBoard};
use kanban_service::sqlite_backend::SqliteBackend;
use kanban_service::KanbanBackend;
use uuid::Uuid;

fn make_create(name: &str) -> Command {
    Command::Board(BoardCommand::Create(CreateBoard {
        id: Uuid::new_v4(),
        name: name.into(),
        card_prefix: None,
        position: 0,
    }))
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_audit_log_persists_across_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.sqlite3");
    let locator = path.to_str().unwrap();

    {
        let backend = SqliteBackend::open(locator).await.unwrap();
        backend.append_commands(&[make_create("first")]).unwrap();
        backend.append_commands(&[make_create("second")]).unwrap();
        assert_eq!(backend.command_count().unwrap(), 2);
        backend.flush().await.unwrap();
    }

    let backend = SqliteBackend::open(locator).await.unwrap();
    assert_eq!(
        backend.command_count().unwrap(),
        2,
        "audit log entries from the previous session must persist"
    );

    let batches = backend.load_commands(0, 2).unwrap();
    assert_eq!(batches.len(), 2);
    match &batches[0].commands[0] {
        Command::Board(BoardCommand::Create(cb)) => assert_eq!(cb.name, "first"),
        other => panic!("expected Create('first'), got {other:?}"),
    }
    match &batches[1].commands[0] {
        Command::Board(BoardCommand::Create(cb)) => assert_eq!(cb.name, "second"),
        other => panic!("expected Create('second'), got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_audit_log_append_returns_new_count() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("count.sqlite3");
    let backend = SqliteBackend::open(path.to_str().unwrap()).await.unwrap();

    assert_eq!(backend.append_commands(&[make_create("A")]).unwrap(), 1);
    assert_eq!(backend.append_commands(&[make_create("B")]).unwrap(), 2);
    assert_eq!(backend.append_commands(&[make_create("C")]).unwrap(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_audit_log_batch_preserves_multiple_commands() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("batch.sqlite3");
    let backend = SqliteBackend::open(path.to_str().unwrap()).await.unwrap();

    backend
        .append_commands(&[make_create("A"), make_create("B"), make_create("C")])
        .unwrap();
    let batches = backend.load_commands(0, 1).unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].len(), 3, "multi-command batch must stay grouped");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sqlite_audit_log_load_clamps_out_of_range() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("range.sqlite3");
    let backend = SqliteBackend::open(path.to_str().unwrap()).await.unwrap();
    backend.append_commands(&[make_create("A")]).unwrap();

    let batches = backend.load_commands(10, 20).unwrap();
    assert!(batches.is_empty());
    let batches = backend.load_commands(0, 99).unwrap();
    assert_eq!(batches.len(), 1);
}
