use crate::store::JsonFileStore;
use crate::traits::PersistenceStore;
use kanban_core::KanbanResult;
use std::path::Path;

#[cfg(feature = "sqlite")]
use crate::store::SqliteStore;

#[cfg(feature = "sqlite")]
pub async fn migrate_json_to_sqlite(json_path: &Path, sqlite_path: &Path) -> KanbanResult<()> {
    if !json_path.exists() {
        return Err(kanban_core::KanbanError::NotFound(format!(
            "JSON file not found: {}",
            json_path.display()
        )));
    }

    if sqlite_path.exists() {
        return Err(kanban_core::KanbanError::Validation(format!(
            "SQLite database already exists: {}. Remove it first or use a different path.",
            sqlite_path.display()
        )));
    }

    tracing::info!(
        "Migrating from JSON ({}) to SQLite ({})",
        json_path.display(),
        sqlite_path.display()
    );

    // Load from JSON
    let json_store = JsonFileStore::new(json_path);
    let (snapshot, _metadata) = json_store.load().await?;

    // Save to SQLite
    let sqlite_store = SqliteStore::new(sqlite_path);
    sqlite_store.save(snapshot).await?;

    tracing::info!("Migration completed successfully");

    Ok(())
}

#[cfg(feature = "sqlite")]
pub async fn auto_migrate_if_needed(json_path: &Path, sqlite_path: &Path) -> KanbanResult<bool> {
    // If SQLite already exists, no migration needed
    if sqlite_path.exists() {
        return Ok(false);
    }

    // If JSON exists but SQLite doesn't, migrate
    if json_path.exists() {
        migrate_json_to_sqlite(json_path, sqlite_path).await?;
        return Ok(true);
    }

    // Neither exists, nothing to migrate
    Ok(false)
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::traits::{PersistenceMetadata, StoreSnapshot};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_migrate_json_to_sqlite() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("test.json");
        let sqlite_path = dir.path().join("test.db");

        // Create a JSON file with some data
        let json_store = JsonFileStore::new(&json_path);
        let data = serde_json::json!({
            "boards": [{
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "name": "Test Board",
                "description": null,
                "sprint_prefix": null,
                "card_prefix": null,
                "task_sort_field": "Default",
                "task_sort_order": "Ascending",
                "sprint_duration_days": null,
                "sprint_names": [],
                "sprint_name_used_count": 0,
                "next_sprint_number": 1,
                "active_sprint_id": null,
                "task_list_view": "Flat",
                "prefix_counters": {},
                "sprint_counters": {},
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            }],
            "columns": [],
            "cards": [],
            "archived_cards": [],
            "sprints": []
        });
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(json_store.instance_id()),
        };
        json_store.save(snapshot).await.unwrap();

        // Migrate
        migrate_json_to_sqlite(&json_path, &sqlite_path)
            .await
            .unwrap();

        // Verify SQLite has the data
        let sqlite_store = SqliteStore::new(&sqlite_path);
        let (loaded_snapshot, _) = sqlite_store.load().await.unwrap();
        let loaded_data: serde_json::Value = serde_json::from_slice(&loaded_snapshot.data).unwrap();

        assert_eq!(loaded_data["boards"][0]["name"], "Test Board");
    }

    #[tokio::test]
    async fn test_auto_migrate_when_json_exists() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("test.json");
        let sqlite_path = dir.path().join("test.db");

        // Create a JSON file
        let json_store = JsonFileStore::new(&json_path);
        let data = serde_json::json!({
            "boards": [],
            "columns": [],
            "cards": [],
            "archived_cards": [],
            "sprints": []
        });
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(json_store.instance_id()),
        };
        json_store.save(snapshot).await.unwrap();

        // Auto migrate
        let migrated = auto_migrate_if_needed(&json_path, &sqlite_path)
            .await
            .unwrap();
        assert!(migrated);
        assert!(sqlite_path.exists());
    }

    #[tokio::test]
    async fn test_auto_migrate_when_sqlite_exists() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("test.json");
        let sqlite_path = dir.path().join("test.db");

        // Create both files
        let json_store = JsonFileStore::new(&json_path);
        let data = serde_json::json!({
            "boards": [],
            "columns": [],
            "cards": [],
            "archived_cards": [],
            "sprints": []
        });
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(json_store.instance_id()),
        };
        json_store.save(snapshot.clone()).await.unwrap();

        let sqlite_store = SqliteStore::new(&sqlite_path);
        sqlite_store.save(snapshot).await.unwrap();

        // Auto migrate should return false (already exists)
        let migrated = auto_migrate_if_needed(&json_path, &sqlite_path)
            .await
            .unwrap();
        assert!(!migrated);
    }

    #[tokio::test]
    async fn test_auto_migrate_when_neither_exists() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("nonexistent.json");
        let sqlite_path = dir.path().join("nonexistent.db");

        let migrated = auto_migrate_if_needed(&json_path, &sqlite_path)
            .await
            .unwrap();
        assert!(!migrated);
    }
}
