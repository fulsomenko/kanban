use kanban_persistence::{PersistenceMetadata, PersistenceStore, StoreSnapshot};
use kanban_persistence_sqlite::SqliteBlobStore;
use std::sync::Arc;
use tempfile::tempdir;

fn make_snapshot(store: &SqliteBlobStore) -> StoreSnapshot {
    let board_id = uuid::Uuid::new_v4().to_string();
    let col_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let data = serde_json::json!({
        "boards": [{
            "id": board_id,
            "name": "Board",
            "task_sort_field": "Default",
            "task_sort_order": "Ascending",
            "sprint_name_used_count": 0,
            "next_sprint_number": 1,
            "task_list_view": "Flat",
            "prefix_counters": {},
            "sprint_counters": {},
            "created_at": now,
            "updated_at": now
        }],
        "columns": [{
            "id": col_id,
            "board_id": board_id,
            "name": "Col",
            "position": 0,
            "created_at": now,
            "updated_at": now
        }],
        "cards": [],
        "archived_cards": [],
        "sprints": []
    });

    StoreSnapshot {
        data: serde_json::to_vec(&data).unwrap(),
        metadata: PersistenceMetadata::new(store.instance_id()),
    }
}

#[tokio::test]
async fn test_concurrent_saves_no_corruption() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("concurrent.db");
    let store = Arc::new(SqliteBlobStore::new(&db_path));

    let initial = make_snapshot(&store);
    store.save(initial).await.unwrap();

    let mut handles = Vec::new();
    for _ in 0..10 {
        let store = Arc::clone(&store);
        handles.push(tokio::spawn(async move {
            let snapshot = make_snapshot(&store);
            // Concurrent saves may conflict, which is acceptable.
            // We only verify no panics or data corruption.
            let _ = store.save(snapshot).await;
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let (loaded, _meta) = store.load().await.unwrap();
    let data: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
    assert!(!data["boards"].as_array().unwrap().is_empty());
    assert!(!data["columns"].as_array().unwrap().is_empty());
}
