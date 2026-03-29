use kanban_persistence::{PersistenceMetadata, PersistenceStore, StoreSnapshot};
use kanban_persistence_json::JsonFileStore;
use kanban_service::test_helpers::helpers::fully_populated_snapshot;
use kanban_service::DataSnapshot;
use tempfile::TempDir;

#[tokio::test]
async fn full_roundtrip_preserves_all_fields() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.json");
    let store = JsonFileStore::new(&path);

    let original = fully_populated_snapshot();
    let data = serde_json::to_vec(&original).unwrap();
    store
        .save(StoreSnapshot {
            data,
            metadata: PersistenceMetadata::new(store.instance_id()),
        })
        .await
        .unwrap();

    let (loaded_snap, _) = store.load().await.unwrap();
    let loaded: DataSnapshot = serde_json::from_slice(&loaded_snap.data).unwrap();

    assert_eq!(original, loaded);
}
