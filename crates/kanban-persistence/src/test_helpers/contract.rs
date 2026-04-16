use super::helpers::fully_populated_snapshot;
use super::StoreFactory;
use crate::{PersistenceError, PersistenceMetadata, StoreSnapshot};
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

