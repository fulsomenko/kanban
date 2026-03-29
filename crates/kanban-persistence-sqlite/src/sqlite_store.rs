use async_trait::async_trait;
use kanban_persistence::{
    PersistenceError, PersistenceMetadata, PersistenceResult, PersistenceStore, StoreSnapshot,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Row, Sqlite, Transaction};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::builders::{build_board, build_card, build_column, build_graph, build_sprint};
use crate::helpers::{db_err, parse_datetime, parse_uuid, ser_err, SnapshotData, Table};
use crate::upserts;

const SCHEMA: &str = include_str!("schema.sql");
const CURRENT_SCHEMA_VERSION: i32 = 1;

pub struct SqliteStore {
    path: PathBuf,
    instance_id: Uuid,
    pool: tokio::sync::OnceCell<Pool<Sqlite>>,
    last_known_metadata: tokio::sync::Mutex<Option<PersistenceMetadata>>,
}

impl SqliteStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            instance_id: Uuid::new_v4(),
            pool: tokio::sync::OnceCell::new(),
            last_known_metadata: tokio::sync::Mutex::new(None),
        }
    }

    pub fn with_instance_id(path: impl AsRef<Path>, instance_id: Uuid) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            instance_id,
            pool: tokio::sync::OnceCell::new(),
            last_known_metadata: tokio::sync::Mutex::new(None),
        }
    }

    pub fn instance_id(&self) -> Uuid {
        self.instance_id
    }

    async fn get_pool(&self) -> PersistenceResult<&Pool<Sqlite>> {
        self.pool
            .get_or_try_init(|| async {
                let options = SqliteConnectOptions::new()
                    .filename(&self.path)
                    .create_if_missing(true)
                    .foreign_keys(true)
                    .pragma("journal_mode", "wal");

                let pool = SqlitePoolOptions::new()
                    .max_connections(2)
                    .connect_with(options)
                    .await
                    .map_err(|e| PersistenceError::Database(e.to_string()))?;

                sqlx::raw_sql(SCHEMA)
                    .execute(&pool)
                    .await
                    .map_err(|e| PersistenceError::Database(e.to_string()))?;

                migrate(&pool).await?;

                Ok(pool)
            })
            .await
    }

    async fn load_current_state(&self, pool: &Pool<Sqlite>) -> PersistenceResult<SnapshotData> {
        let mut tx = pool.begin().await.map_err(db_err)?;

        let board_rows = sqlx::query(
            "SELECT id, name, description, sprint_prefix, card_prefix, task_sort_field,
                    task_sort_order, sprint_duration_days, sprint_name_used_count,
                    next_sprint_number, active_sprint_id, task_list_view,
                    completion_column_id, created_at, updated_at FROM boards",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;

        let sprint_name_rows = sqlx::query(
            "SELECT board_id, position, name FROM board_sprint_names ORDER BY board_id, position",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;

        let mut sprint_names_map: HashMap<String, Vec<String>> = HashMap::new();
        for row in &sprint_name_rows {
            let board_id: String = row.try_get("board_id").map_err(db_err)?;
            let name: String = row.try_get("name").map_err(db_err)?;
            sprint_names_map.entry(board_id).or_default().push(name);
        }

        let prefix_counter_rows =
            sqlx::query("SELECT board_id, prefix, counter FROM board_prefix_counters")
                .fetch_all(&mut *tx)
                .await
                .map_err(db_err)?;

        let mut prefix_counters_map: HashMap<String, HashMap<String, u32>> = HashMap::new();
        for row in &prefix_counter_rows {
            let board_id: String = row.try_get("board_id").map_err(db_err)?;
            let prefix: String = row.try_get("prefix").map_err(db_err)?;
            let counter: i32 = row.try_get("counter").map_err(db_err)?;
            prefix_counters_map
                .entry(board_id)
                .or_default()
                .insert(prefix, counter as u32);
        }

        let sprint_counter_rows =
            sqlx::query("SELECT board_id, prefix, counter FROM board_sprint_counters")
                .fetch_all(&mut *tx)
                .await
                .map_err(db_err)?;

        let mut sprint_counters_map: HashMap<String, HashMap<String, u32>> = HashMap::new();
        for row in &sprint_counter_rows {
            let board_id: String = row.try_get("board_id").map_err(db_err)?;
            let prefix: String = row.try_get("prefix").map_err(db_err)?;
            let counter: i32 = row.try_get("counter").map_err(db_err)?;
            sprint_counters_map
                .entry(board_id)
                .or_default()
                .insert(prefix, counter as u32);
        }

        let mut boards = Vec::with_capacity(board_rows.len());
        for row in &board_rows {
            let id_str: String = row.try_get("id").map_err(db_err)?;
            boards.push(build_board(
                row,
                sprint_names_map.remove(&id_str).unwrap_or_default(),
                prefix_counters_map.remove(&id_str).unwrap_or_default(),
                sprint_counters_map.remove(&id_str).unwrap_or_default(),
            )?);
        }

        let columns: Vec<serde_json::Value> = sqlx::query(
            "SELECT id, board_id, name, position, wip_limit, created_at, updated_at FROM columns",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?
        .iter()
        .map(build_column)
        .collect::<PersistenceResult<Vec<_>>>()?;

        let sprints: Vec<serde_json::Value> = sqlx::query(
            "SELECT id, board_id, sprint_number, name_index, prefix, card_prefix, status,
                    start_date, end_date, created_at, updated_at FROM sprints",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?
        .iter()
        .map(build_sprint)
        .collect::<PersistenceResult<Vec<_>>>()?;

        let card_rows = sqlx::query(
            "SELECT id, column_id, title, description, priority, status, position, due_date,
                    points, card_number, sprint_id, assigned_prefix, card_prefix, created_at,
                    updated_at, completed_at FROM cards
             ORDER BY position ASC, created_at ASC",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;

        let sprint_log_rows = sqlx::query(
            "SELECT card_id, sprint_id, sprint_number, sprint_name, started_at, ended_at, status
             FROM sprint_logs ORDER BY card_id, id",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;

        let mut sprint_logs_map: HashMap<String, Vec<kanban_domain::SprintLog>> = HashMap::new();
        for row in &sprint_log_rows {
            let card_id: String = row.try_get("card_id").map_err(db_err)?;
            let sprint_id_str: String = row.try_get("sprint_id").map_err(db_err)?;
            let sprint_number: i32 = row.try_get("sprint_number").map_err(db_err)?;
            let sprint_name: Option<String> = row.try_get("sprint_name").map_err(db_err)?;
            let started_at_str: String = row.try_get("started_at").map_err(db_err)?;
            let ended_at_str: Option<String> = row.try_get("ended_at").map_err(db_err)?;
            let status: String = row.try_get("status").map_err(db_err)?;

            sprint_logs_map
                .entry(card_id)
                .or_default()
                .push(kanban_domain::SprintLog {
                    sprint_id: parse_uuid(&sprint_id_str)?,
                    sprint_number: sprint_number as u32,
                    sprint_name,
                    started_at: parse_datetime(&started_at_str)?,
                    ended_at: ended_at_str.as_deref().map(parse_datetime).transpose()?,
                    status,
                });
        }

        let mut all_cards_map: HashMap<String, serde_json::Value> = HashMap::new();
        for row in &card_rows {
            let id_str: String = row.try_get("id").map_err(db_err)?;
            let logs = sprint_logs_map.remove(&id_str).unwrap_or_default();
            let card_value = build_card(row, logs)?;
            all_cards_map.insert(id_str, card_value);
        }

        let archived_rows = sqlx::query(
            "SELECT card_id, archived_at, original_column_id, original_position
             FROM archived_cards",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;

        let mut archived_card_ids = std::collections::HashSet::new();
        let mut archived_cards = Vec::with_capacity(archived_rows.len());
        for row in &archived_rows {
            let card_id: String = row.try_get("card_id").map_err(db_err)?;
            let archived_at: String = row.try_get("archived_at").map_err(db_err)?;
            let original_column_id: String = row.try_get("original_column_id").map_err(db_err)?;
            let original_position: i32 = row.try_get("original_position").map_err(db_err)?;

            if let Some(card_value) = all_cards_map.get(&card_id) {
                archived_cards.push(serde_json::json!({
                    "card": card_value,
                    "archived_at": archived_at,
                    "original_column_id": original_column_id,
                    "original_position": original_position,
                }));
                archived_card_ids.insert(card_id);
            }
        }

        let cards: Vec<serde_json::Value> = card_rows
            .iter()
            .filter_map(|row| {
                let id_str: String = row.try_get("id").ok()?;
                if archived_card_ids.contains(&id_str) {
                    None
                } else {
                    all_cards_map.remove(&id_str)
                }
            })
            .collect();

        let edge_rows = sqlx::query(
            "SELECT source_id, target_id, edge_type, direction, weight, created_at, archived_at
             FROM card_edges",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;

        let graph = build_graph(&edge_rows)?;

        Ok(SnapshotData {
            boards,
            columns,
            cards,
            archived_cards,
            sprints,
            graph,
        })
    }

    async fn sync_table<F>(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        table: Table,
        incoming: &[serde_json::Value],
        id_extractor: F,
    ) -> PersistenceResult<()>
    where
        F: Fn(&serde_json::Value) -> Option<String>,
    {
        let incoming_ids: Vec<String> = incoming.iter().filter_map(&id_extractor).collect();

        if incoming_ids.is_empty() {
            let sql = format!("DELETE FROM {}", table.table_name());
            sqlx::query(&sql).execute(&mut **tx).await.map_err(db_err)?;
        } else {
            let placeholders: String = incoming_ids
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "DELETE FROM {} WHERE {} NOT IN ({})",
                table.table_name(),
                table.id_column(),
                placeholders,
            );
            let mut query = sqlx::query(&sql);
            for id in &incoming_ids {
                query = query.bind(id);
            }
            query.execute(&mut **tx).await.map_err(db_err)?;
        }

        Ok(())
    }

    async fn lock_metadata(&self) -> tokio::sync::MutexGuard<'_, Option<PersistenceMetadata>> {
        self.last_known_metadata.lock().await
    }
}

async fn migrate(pool: &Pool<Sqlite>) -> PersistenceResult<()> {
    let version: i32 = sqlx::query_scalar("SELECT schema_version FROM metadata WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map_err(db_err)?
        .unwrap_or(0);

    if version > CURRENT_SCHEMA_VERSION {
        return Err(PersistenceError::Database(format!(
            "database schema version ({version}) is newer than supported ({CURRENT_SCHEMA_VERSION})"
        )));
    }

    // Version 0 → 1: initial schema (handled by CREATE IF NOT EXISTS above)
    // Future: if version < 2 { ALTER TABLE ... ; update schema_version }

    Ok(())
}

#[async_trait]
impl PersistenceStore for SqliteStore {
    async fn save(&self, mut snapshot: StoreSnapshot) -> PersistenceResult<PersistenceMetadata> {
        let pool = self.get_pool().await?;

        snapshot.metadata.instance_id = self.instance_id;
        snapshot.metadata.saved_at = chrono::Utc::now();

        let data: SnapshotData = serde_json::from_slice(&snapshot.data).map_err(ser_err)?;

        let mut tx = pool.begin().await.map_err(db_err)?;

        // Conflict detection inside transaction
        let existing_meta: Option<(String, String)> =
            sqlx::query_as("SELECT instance_id, saved_at FROM metadata WHERE id = 1")
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;

        if let Some((db_instance_id, db_saved_at)) = existing_meta {
            let last_known = {
                let guard = self.lock_metadata().await;
                guard.clone()
            };
            if let Some(ref last_known) = last_known {
                let db_instance = parse_uuid(&db_instance_id)?;
                let db_time = parse_datetime(&db_saved_at)?;

                if db_instance != last_known.instance_id || db_time != last_known.saved_at {
                    return Err(PersistenceError::ConflictDetected {
                        path: self.path.to_string_lossy().to_string(),
                        source: None,
                    });
                }
            }
        }

        // Sync deletions (reverse FK order)
        self.sync_table(&mut tx, Table::ArchivedCards, &data.archived_cards, |v| {
            v["card"]["id"].as_str().map(String::from)
        })
        .await?;

        let mut all_card_refs: Vec<serde_json::Value> = data
            .cards
            .iter()
            .map(|c| serde_json::json!({"id": c["id"]}))
            .collect();
        for archived in &data.archived_cards {
            if let Some(id) = archived.get("card").and_then(|c| c.get("id")) {
                all_card_refs.push(serde_json::json!({"id": id}));
            }
        }
        self.sync_table(&mut tx, Table::Cards, &all_card_refs, |v| {
            v["id"].as_str().map(String::from)
        })
        .await?;

        self.sync_table(&mut tx, Table::Sprints, &data.sprints, |v| {
            v["id"].as_str().map(String::from)
        })
        .await?;
        self.sync_table(&mut tx, Table::Columns, &data.columns, |v| {
            v["id"].as_str().map(String::from)
        })
        .await?;
        self.sync_table(&mut tx, Table::Boards, &data.boards, |v| {
            v["id"].as_str().map(String::from)
        })
        .await?;

        // Upserts (parent-first FK order)
        for board in &data.boards {
            upserts::upsert_board(&mut tx, board).await?;
        }
        for column in &data.columns {
            upserts::upsert_column(&mut tx, column).await?;
        }
        for sprint in &data.sprints {
            upserts::upsert_sprint(&mut tx, sprint).await?;
        }
        for card in &data.cards {
            upserts::upsert_card(&mut tx, card).await?;
        }
        for archived in &data.archived_cards {
            upserts::upsert_archived_card(&mut tx, archived).await?;
        }

        upserts::upsert_edges(&mut tx, &data.graph).await?;

        // Update metadata
        let saved_at_str = snapshot.metadata.saved_at.to_rfc3339();
        sqlx::query(
            "INSERT INTO metadata (id, instance_id, saved_at, schema_version)
             VALUES (1, ?, ?, 1)
             ON CONFLICT(id) DO UPDATE SET
                instance_id = excluded.instance_id,
                saved_at = excluded.saved_at",
        )
        .bind(self.instance_id.to_string())
        .bind(&saved_at_str)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

        tx.commit().await.map_err(db_err)?;

        // If the process dies between tx.commit() and this cache update,
        // the in-memory metadata goes stale. This self-heals on next app
        // start because load() re-reads metadata from the database.
        {
            let mut guard = self.lock_metadata().await;
            *guard = Some(snapshot.metadata.clone());
        }

        tracing::info!("Saved to SQLite database at {}", self.path.display());

        Ok(snapshot.metadata)
    }

    async fn load(&self) -> PersistenceResult<(StoreSnapshot, PersistenceMetadata)> {
        let pool = self.get_pool().await?;

        let meta_row: Option<(String, String)> =
            sqlx::query_as("SELECT instance_id, saved_at FROM metadata WHERE id = 1")
                .fetch_optional(pool)
                .await
                .map_err(db_err)?;

        let metadata = if let Some((instance_id_str, saved_at_str)) = meta_row {
            PersistenceMetadata {
                instance_id: parse_uuid(&instance_id_str)?,
                saved_at: parse_datetime(&saved_at_str)?,
            }
        } else {
            PersistenceMetadata::new(self.instance_id)
        };

        let data = self.load_current_state(pool).await?;
        let data_bytes = serde_json::to_vec(&data).map_err(ser_err)?;

        let snapshot = StoreSnapshot {
            data: data_bytes,
            metadata: metadata.clone(),
        };

        {
            let mut guard = self.lock_metadata().await;
            *guard = Some(metadata.clone());
        }

        tracing::info!("Loaded from SQLite database at {}", self.path.display());

        Ok((snapshot, metadata))
    }

    async fn exists(&self) -> bool {
        self.path.exists()
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn instance_id(&self) -> Uuid {
        self.instance_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = SqliteStore::new(&db_path);

        let data = serde_json::json!({
            "boards": [],
            "columns": [],
            "cards": [],
            "archived_cards": [],
            "sprints": []
        });
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };

        let _metadata = store.save(snapshot).await.unwrap();
        assert!(db_path.exists());

        let (loaded_snapshot, _) = store.load().await.unwrap();
        let loaded_data: serde_json::Value = serde_json::from_slice(&loaded_snapshot.data).unwrap();
        assert!(loaded_data["boards"].is_array());
    }

    #[tokio::test]
    async fn test_exists() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("nonexistent.db");
        let store = SqliteStore::new(&db_path);

        assert!(!store.exists().await);

        let data = serde_json::json!({
            "boards": [],
            "columns": [],
            "cards": [],
            "archived_cards": [],
            "sprints": []
        });
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };
        store.save(snapshot).await.unwrap();

        assert!(store.exists().await);
    }

    #[tokio::test]
    async fn test_path_with_spaces_roundtrip() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("path with spaces/test board.db");
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        let store = SqliteStore::new(&db_path);

        let data = serde_json::json!({
            "boards": [],
            "columns": [],
            "cards": [],
            "archived_cards": [],
            "sprints": []
        });
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };

        store.save(snapshot).await.unwrap();
        let (loaded, _) = store.load().await.unwrap();
        let loaded_data: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        assert!(loaded_data["boards"].is_array());
    }

    #[tokio::test]
    async fn test_cards_loaded_in_position_order() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("order.db");
        let store = SqliteStore::new(&db_path);

        let board_id = uuid::Uuid::new_v4().to_string();
        let col_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let cards: Vec<serde_json::Value> = [2, 0, 1]
            .iter()
            .map(|pos| {
                serde_json::json!({
                    "id": uuid::Uuid::new_v4().to_string(),
                    "column_id": col_id,
                    "title": format!("Card pos {pos}"),
                    "priority": "Medium",
                    "status": "Todo",
                    "position": pos,
                    "card_number": pos,
                    "created_at": now,
                    "updated_at": now,
                    "sprint_logs": []
                })
            })
            .collect();

        let data = serde_json::json!({
            "boards": [{
                "id": board_id,
                "name": "B",
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
            "cards": cards,
            "archived_cards": [],
            "sprints": []
        });

        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };
        store.save(snapshot).await.unwrap();

        let (loaded, _) = store.load().await.unwrap();
        let loaded_data: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        let loaded_cards = loaded_data["cards"].as_array().unwrap();
        let positions: Vec<i64> = loaded_cards
            .iter()
            .map(|c| c["position"].as_i64().unwrap())
            .collect();
        assert_eq!(positions, vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn test_upsert_board_missing_name_returns_error() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = SqliteStore::new(&db_path);

        let data = SnapshotData {
            boards: vec![serde_json::json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            })],
            columns: vec![],
            cards: vec![],
            archived_cards: vec![],
            sprints: vec![],
            graph: serde_json::json!({}),
        };
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };

        let result = store.save(snapshot).await;
        assert!(
            matches!(result, Err(PersistenceError::Serialization(ref msg)) if msg.contains("name")),
            "Expected Serialization error about 'name', got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_upsert_card_missing_title_returns_error() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = SqliteStore::new(&db_path);

        let board_id = uuid::Uuid::new_v4().to_string();
        let col_id = uuid::Uuid::new_v4().to_string();
        let now = "2024-01-01T00:00:00Z";
        let data = SnapshotData {
            boards: vec![serde_json::json!({
                "id": board_id, "name": "B",
                "task_sort_field": "Default", "task_sort_order": "Ascending",
                "sprint_name_used_count": 0, "next_sprint_number": 1,
                "task_list_view": "Flat", "prefix_counters": {}, "sprint_counters": {},
                "created_at": now, "updated_at": now
            })],
            columns: vec![serde_json::json!({
                "id": col_id, "board_id": board_id, "name": "Col", "position": 0,
                "created_at": now, "updated_at": now
            })],
            cards: vec![serde_json::json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "column_id": col_id,
                "position": 0, "card_number": 0,
                "created_at": now, "updated_at": now
            })],
            archived_cards: vec![],
            sprints: vec![],
            graph: serde_json::json!({}),
        };
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };

        let result = store.save(snapshot).await;
        assert!(
            matches!(result, Err(PersistenceError::Serialization(ref msg)) if msg.contains("title")),
            "Expected Serialization error about 'title', got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_upsert_card_missing_timestamps_returns_error() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = SqliteStore::new(&db_path);

        let board_id = uuid::Uuid::new_v4().to_string();
        let col_id = uuid::Uuid::new_v4().to_string();
        let now = "2024-01-01T00:00:00Z";
        let data = SnapshotData {
            boards: vec![serde_json::json!({
                "id": board_id, "name": "B",
                "task_sort_field": "Default", "task_sort_order": "Ascending",
                "sprint_name_used_count": 0, "next_sprint_number": 1,
                "task_list_view": "Flat", "prefix_counters": {}, "sprint_counters": {},
                "created_at": now, "updated_at": now
            })],
            columns: vec![serde_json::json!({
                "id": col_id, "board_id": board_id, "name": "Col", "position": 0,
                "created_at": now, "updated_at": now
            })],
            cards: vec![serde_json::json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "column_id": col_id,
                "title": "Test",
                "priority": "Medium",
                "status": "Todo",
                "position": 0, "card_number": 0
            })],
            archived_cards: vec![],
            sprints: vec![],
            graph: serde_json::json!({}),
        };
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };

        let result = store.save(snapshot).await;
        assert!(
            matches!(result, Err(PersistenceError::Serialization(ref msg)) if msg.contains("created_at")),
            "Expected Serialization error about 'created_at', got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_unknown_priority_returns_error() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = SqliteStore::new(&db_path);
        let pool = store.get_pool().await.unwrap();

        let bid = Uuid::new_v4().to_string();
        let cid = Uuid::new_v4().to_string();
        let kid = Uuid::new_v4().to_string();
        let ts = "2024-01-01T00:00:00Z";
        sqlx::query("INSERT INTO boards (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)")
            .bind(&bid)
            .bind("B")
            .bind(ts)
            .bind(ts)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO columns (id, board_id, name, position, created_at, updated_at) VALUES (?, ?, ?, 0, ?, ?)")
            .bind(&cid).bind(&bid).bind("Col").bind(ts).bind(ts)
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO cards (id, column_id, title, priority, status, position, card_number, created_at, updated_at) VALUES (?, ?, ?, 'InvalidPriority', 'Todo', 0, 0, ?, ?)")
            .bind(&kid).bind(&cid).bind("T").bind(ts).bind(ts)
            .execute(pool).await.unwrap();

        let result = store.load().await;
        assert!(
            matches!(result, Err(PersistenceError::Serialization(ref msg)) if msg.contains("priority")),
            "Expected Serialization error about 'priority', got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_unknown_sprint_status_returns_error() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = SqliteStore::new(&db_path);
        let pool = store.get_pool().await.unwrap();

        let bid = Uuid::new_v4().to_string();
        let sid = Uuid::new_v4().to_string();
        let ts = "2024-01-01T00:00:00Z";
        sqlx::query("INSERT INTO boards (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)")
            .bind(&bid)
            .bind("B")
            .bind(ts)
            .bind(ts)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO sprints (id, board_id, sprint_number, status, created_at, updated_at) VALUES (?, ?, 1, 'BadStatus', ?, ?)")
            .bind(&sid).bind(&bid).bind(ts).bind(ts)
            .execute(pool).await.unwrap();

        let result = store.load().await;
        assert!(
            matches!(result, Err(PersistenceError::Serialization(ref msg)) if msg.contains("sprint status")),
            "Expected Serialization error about 'sprint status', got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_upsert_edges_removes_stale_edges() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = SqliteStore::new(&db_path);

        let board_id = uuid::Uuid::new_v4().to_string();
        let col_id = uuid::Uuid::new_v4().to_string();
        let card1_id = uuid::Uuid::new_v4().to_string();
        let card2_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let make_data = |edges: serde_json::Value| -> serde_json::Value {
            serde_json::json!({
                "boards": [{"id": board_id, "name": "B", "task_sort_field": "Default",
                    "task_sort_order": "Ascending", "sprint_name_used_count": 0,
                    "next_sprint_number": 1, "task_list_view": "Flat",
                    "prefix_counters": {}, "sprint_counters": {},
                    "created_at": now, "updated_at": now}],
                "columns": [{"id": col_id, "board_id": board_id, "name": "C", "position": 0,
                    "created_at": now, "updated_at": now}],
                "cards": [
                    {"id": card1_id, "column_id": col_id, "title": "A", "priority": "Medium",
                     "status": "Todo", "position": 0, "card_number": 0,
                     "created_at": now, "updated_at": now, "sprint_logs": []},
                    {"id": card2_id, "column_id": col_id, "title": "B", "priority": "Medium",
                     "status": "Todo", "position": 1, "card_number": 1,
                     "created_at": now, "updated_at": now, "sprint_logs": []}
                ],
                "archived_cards": [],
                "sprints": [],
                "graph": edges
            })
        };

        let two_edges = serde_json::json!({"cards": {"edges": [
            {"source": card1_id, "target": card2_id, "edge_type": "Blocks",
             "direction": "Directed", "created_at": now},
            {"source": card1_id, "target": card2_id, "edge_type": "RelatesTo",
             "direction": "Bidirectional", "created_at": now}
        ]}});

        let data1 = make_data(two_edges);
        let snap1 = StoreSnapshot {
            data: serde_json::to_vec(&data1).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };
        store.save(snap1).await.unwrap();

        let one_edge = serde_json::json!({"cards": {"edges": [
            {"source": card1_id, "target": card2_id, "edge_type": "Blocks",
             "direction": "Directed", "created_at": now}
        ]}});

        let data2 = make_data(one_edge);
        let snap2 = StoreSnapshot {
            data: serde_json::to_vec(&data2).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };
        store.save(snap2).await.unwrap();

        let (loaded, _) = store.load().await.unwrap();
        let loaded_data: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        let edges = loaded_data["graph"]["cards"]["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1, "Stale edge should have been removed");
        assert_eq!(edges[0]["edge_type"], "Blocks");
    }

    #[tokio::test]
    async fn test_upsert_board_missing_id_returns_serialization_error() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = SqliteStore::new(&db_path);

        let data = SnapshotData {
            boards: vec![serde_json::json!({
                "name": "No ID Board",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            })],
            columns: vec![],
            cards: vec![],
            archived_cards: vec![],
            sprints: vec![],
            graph: serde_json::json!({}),
        };
        let snapshot = StoreSnapshot {
            data: serde_json::to_vec(&data).unwrap(),
            metadata: PersistenceMetadata::new(store.instance_id()),
        };

        let result = store.save(snapshot).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, PersistenceError::Serialization(ref msg) if msg.contains("missing required field")),
            "Expected Serialization error with 'missing required field', got: {err:?}"
        );
    }
}
