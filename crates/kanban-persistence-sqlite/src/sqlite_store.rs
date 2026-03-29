use async_trait::async_trait;
use kanban_persistence::{
    PersistenceError, PersistenceMetadata, PersistenceResult, PersistenceStore, StoreSnapshot,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Row, Sqlite, Transaction};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Mutex;
use uuid::Uuid;

use crate::builders::{build_board, build_card, build_column, build_graph, build_sprint};
use crate::helpers::{db_err, parse_datetime, parse_uuid, ser_err, SnapshotData, Table};
use crate::upserts;

const SCHEMA: &str = include_str!("schema.sql");

pub struct SqliteStore {
    path: PathBuf,
    instance_id: Uuid,
    pool: tokio::sync::OnceCell<Pool<Sqlite>>,
    last_known_metadata: Mutex<Option<PersistenceMetadata>>,
}

impl SqliteStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            instance_id: Uuid::new_v4(),
            pool: tokio::sync::OnceCell::new(),
            last_known_metadata: Mutex::new(None),
        }
    }

    pub fn with_instance_id(path: impl AsRef<Path>, instance_id: Uuid) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            instance_id,
            pool: tokio::sync::OnceCell::new(),
            last_known_metadata: Mutex::new(None),
        }
    }

    pub fn instance_id(&self) -> Uuid {
        self.instance_id
    }

    async fn get_pool(&self) -> PersistenceResult<&Pool<Sqlite>> {
        self.pool
            .get_or_try_init(|| async {
                let options = SqliteConnectOptions::from_str(&format!(
                    "sqlite://{}?mode=rwc",
                    self.path.display()
                ))
                .map_err(|e| PersistenceError::Database(e.to_string()))?
                .create_if_missing(true)
                .foreign_keys(true)
                .pragma("journal_mode", "wal");

                let pool = SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect_with(options)
                    .await
                    .map_err(|e| PersistenceError::Database(e.to_string()))?;

                sqlx::raw_sql(SCHEMA)
                    .execute(&pool)
                    .await
                    .map_err(|e| PersistenceError::Database(e.to_string()))?;

                Ok(pool)
            })
            .await
    }

    async fn load_current_state(&self, pool: &Pool<Sqlite>) -> PersistenceResult<SnapshotData> {
        let board_rows = sqlx::query(
            "SELECT id, name, description, sprint_prefix, card_prefix, task_sort_field,
                    task_sort_order, sprint_duration_days, sprint_name_used_count,
                    next_sprint_number, active_sprint_id, task_list_view,
                    completion_column_id, created_at, updated_at FROM boards",
        )
        .fetch_all(pool)
        .await
        .map_err(db_err)?;

        let sprint_name_rows = sqlx::query(
            "SELECT board_id, position, name FROM board_sprint_names ORDER BY board_id, position",
        )
        .fetch_all(pool)
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
                .fetch_all(pool)
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
                .fetch_all(pool)
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
        .fetch_all(pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(build_column)
        .collect::<PersistenceResult<Vec<_>>>()?;

        let sprints: Vec<serde_json::Value> = sqlx::query(
            "SELECT id, board_id, sprint_number, name_index, prefix, card_prefix, status,
                    start_date, end_date, created_at, updated_at FROM sprints",
        )
        .fetch_all(pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(build_sprint)
        .collect::<PersistenceResult<Vec<_>>>()?;

        let card_rows = sqlx::query(
            "SELECT id, column_id, title, description, priority, status, position, due_date,
                    points, card_number, sprint_id, assigned_prefix, card_prefix, created_at,
                    updated_at, completed_at FROM cards",
        )
        .fetch_all(pool)
        .await
        .map_err(db_err)?;

        let sprint_log_rows = sqlx::query(
            "SELECT card_id, sprint_id, sprint_number, sprint_name, started_at, ended_at, status
             FROM sprint_logs ORDER BY card_id, id",
        )
        .fetch_all(pool)
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
        .fetch_all(pool)
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

        let cards: Vec<serde_json::Value> = all_cards_map
            .into_iter()
            .filter(|(id, _)| !archived_card_ids.contains(id))
            .map(|(_, v)| v)
            .collect();

        let edge_rows = sqlx::query(
            "SELECT source_id, target_id, edge_type, direction, weight, created_at, archived_at
             FROM card_edges",
        )
        .fetch_all(pool)
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
        let incoming_ids: std::collections::HashSet<String> =
            incoming.iter().filter_map(&id_extractor).collect();

        let existing_ids: std::collections::HashSet<String> = sqlx::query(table.select_ids_sql())
            .fetch_all(&mut **tx)
            .await
            .map_err(db_err)?
            .into_iter()
            .map(|row| row.get::<String, _>("id"))
            .collect();

        let to_delete: Vec<_> = existing_ids.difference(&incoming_ids).collect();
        for id in to_delete {
            sqlx::query(table.delete_by_id_sql())
                .bind(id)
                .execute(&mut **tx)
                .await
                .map_err(db_err)?;
        }

        Ok(())
    }

    fn lock_metadata(&self) -> std::sync::MutexGuard<'_, Option<PersistenceMetadata>> {
        self.last_known_metadata.lock().expect(
            "Metadata mutex poisoned - a panic occurred while holding the lock. \
             Application state may be corrupted and recovery is not safe.",
        )
    }
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
            let guard = self.lock_metadata();
            if let Some(ref last_known) = *guard {
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

        {
            let mut guard = self.lock_metadata();
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
            let mut guard = self.lock_metadata();
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
