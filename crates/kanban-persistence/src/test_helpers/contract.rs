use super::helpers::fully_populated_snapshot;
use super::StoreFactory;
use crate::{PersistenceError, PersistenceMetadata, StoreSnapshot};
use kanban_domain::commands::{BoardCommand, Command, CreateBoard};
use kanban_domain::Snapshot;
use tempfile::TempDir;

fn to_snapshot_bytes(snapshot: &Snapshot) -> Vec<u8> {
    serde_json::to_vec(snapshot).expect("snapshot must serialize")
}

pub async fn test_roundtrip_empty_snapshot(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let store = factory(&path);

    let original = Snapshot::default();
    store
        .save(StoreSnapshot {
            data: to_snapshot_bytes(&original),
            metadata: PersistenceMetadata::new(store.instance_id()),
        })
        .await
        .unwrap();

    let (loaded_snap, _) = store.load().await.unwrap();
    let loaded: Snapshot = serde_json::from_slice(&loaded_snap.data).unwrap();
    assert_eq!(original, loaded);
}

pub async fn test_roundtrip_fully_populated_snapshot(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let store = factory(&path);

    let original = fully_populated_snapshot();
    store
        .save(StoreSnapshot {
            data: to_snapshot_bytes(&original),
            metadata: PersistenceMetadata::new(store.instance_id()),
        })
        .await
        .unwrap();

    let (loaded_snap, _) = store.load().await.unwrap();
    let loaded: Snapshot = serde_json::from_slice(&loaded_snap.data).unwrap();
    assert_eq!(original, loaded);
}

pub async fn test_save_then_exists_returns_true(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let store = factory(&path);

    store
        .save(StoreSnapshot {
            data: to_snapshot_bytes(&Snapshot::default()),
            metadata: PersistenceMetadata::new(store.instance_id()),
        })
        .await
        .unwrap();
    assert!(store.exists().await);
}

pub async fn test_exists_is_false_before_first_save(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let store = factory(&path);
    assert!(!store.exists().await);
}

pub async fn test_load_returns_metadata_increment_after_save(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let store = factory(&path);

    let before = chrono::Utc::now();
    // Some filesystems have coarse mtime granularity; sleep to force a
    // measurable gap between `before` and the saved_at timestamp.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    store
        .save(StoreSnapshot {
            data: to_snapshot_bytes(&Snapshot::default()),
            metadata: PersistenceMetadata::new(store.instance_id()),
        })
        .await
        .unwrap();

    let (_, metadata) = store.load().await.unwrap();
    assert!(
        metadata.saved_at >= before,
        "expected saved_at ({}) >= before ({})",
        metadata.saved_at,
        before
    );
}

pub async fn test_save_with_stale_metadata_returns_conflict(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");

    let store_a = factory(&path);
    store_a
        .save(StoreSnapshot {
            data: to_snapshot_bytes(&Snapshot::default()),
            metadata: PersistenceMetadata::new(store_a.instance_id()),
        })
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    // Separate handle loads, then saves — becomes the new authoritative writer.
    let store_b = factory(&path);
    let (snap_b, _) = store_b.load().await.unwrap();
    store_b.save(snap_b).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    // Store A's last-known metadata is now stale; save must conflict.
    let result = store_a
        .save(StoreSnapshot {
            data: to_snapshot_bytes(&Snapshot::default()),
            metadata: PersistenceMetadata::new(store_a.instance_id()),
        })
        .await;

    match result {
        Err(PersistenceError::ConflictDetected { .. }) => {}
        other => panic!("expected ConflictDetected, got: {other:?}"),
    }
}

pub async fn test_instance_id_is_idempotent_within_handle(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let store = factory(&path);
    let id1 = store.instance_id();
    let id2 = store.instance_id();
    assert_eq!(
        id1, id2,
        "instance_id must return the same value on repeated calls within the same handle"
    );
}

pub async fn test_path_matches_locator(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let store = factory(&path);
    assert_eq!(store.path(), path.as_path());
}

pub async fn test_command_log_append_and_load(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let store = factory(&path);

    // Save initial snapshot so store exists
    store
        .save(StoreSnapshot {
            data: to_snapshot_bytes(&Snapshot::default()),
            metadata: PersistenceMetadata::new(store.instance_id()),
        })
        .await
        .unwrap();

    let cmd = Command::Board(BoardCommand::Create(CreateBoard {
        id: uuid::Uuid::new_v4(),
        name: "Test".into(),
        card_prefix: None,
    }));

    let idx = store.append_command(&cmd).await.unwrap();
    assert_eq!(idx, 1);
    assert_eq!(store.command_count().await.unwrap(), 1);

    let cmds = store.load_commands(0, 1).await.unwrap();
    assert_eq!(cmds.len(), 1);
}

pub async fn test_command_log_cursor_persistence(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let store = factory(&path);

    store
        .save(StoreSnapshot {
            data: to_snapshot_bytes(&Snapshot::default()),
            metadata: PersistenceMetadata::new(store.instance_id()),
        })
        .await
        .unwrap();

    store.set_undo_cursor(5).await.unwrap();
    assert_eq!(store.undo_cursor().await.unwrap(), 5);
}

pub async fn test_command_log_truncate_after(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let store = factory(&path);

    store
        .save(StoreSnapshot {
            data: to_snapshot_bytes(&Snapshot::default()),
            metadata: PersistenceMetadata::new(store.instance_id()),
        })
        .await
        .unwrap();

    for i in 0..3 {
        let cmd = Command::Board(BoardCommand::Create(CreateBoard {
            id: uuid::Uuid::new_v4(),
            name: format!("Board{}", i),
            card_prefix: None,
        }));
        store.append_command(&cmd).await.unwrap();
    }
    assert_eq!(store.command_count().await.unwrap(), 3);

    store.truncate_commands_after(1).await.unwrap();
    assert_eq!(store.command_count().await.unwrap(), 1);
}

pub async fn test_command_count_starts_at_zero(factory: &StoreFactory) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.store");
    let store = factory(&path);

    store
        .save(StoreSnapshot {
            data: to_snapshot_bytes(&Snapshot::default()),
            metadata: PersistenceMetadata::new(store.instance_id()),
        })
        .await
        .unwrap();

    assert_eq!(store.command_count().await.unwrap(), 0);
}
