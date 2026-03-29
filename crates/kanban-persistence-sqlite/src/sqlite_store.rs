use async_trait::async_trait;
use kanban_persistence::{
    PersistenceError, PersistenceMetadata, PersistenceResult, PersistenceStore, StoreSnapshot,
};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Row, Sqlite, Transaction};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Mutex;
use uuid::Uuid;

const SCHEMA: &str = include_str!("schema.sql");

#[derive(Debug, Clone, Copy)]
enum Table {
    Boards,
    Columns,
    Cards,
    ArchivedCards,
    Sprints,
}

impl Table {
    const fn select_ids_sql(self) -> &'static str {
        match self {
            Table::Boards => "SELECT id FROM boards",
            Table::Columns => "SELECT id FROM columns",
            Table::Cards => "SELECT id FROM cards",
            Table::ArchivedCards => "SELECT id FROM archived_cards",
            Table::Sprints => "SELECT id FROM sprints",
        }
    }

    const fn delete_by_id_sql(self) -> &'static str {
        match self {
            Table::Boards => "DELETE FROM boards WHERE id = ?",
            Table::Columns => "DELETE FROM columns WHERE id = ?",
            Table::Cards => "DELETE FROM cards WHERE id = ?",
            Table::ArchivedCards => "DELETE FROM archived_cards WHERE id = ?",
            Table::Sprints => "DELETE FROM sprints WHERE id = ?",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SnapshotData {
    boards: Vec<serde_json::Value>,
    columns: Vec<serde_json::Value>,
    cards: Vec<serde_json::Value>,
    archived_cards: Vec<serde_json::Value>,
    sprints: Vec<serde_json::Value>,
}

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
                .foreign_keys(true);

                let pool = SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect_with(options)
                    .await
                    .map_err(|e| PersistenceError::Database(e.to_string()))?;

                // Initialize schema
                sqlx::raw_sql(SCHEMA)
                    .execute(&pool)
                    .await
                    .map_err(|e| PersistenceError::Database(e.to_string()))?;

                Ok(pool)
            })
            .await
    }

    async fn load_current_state(
        &self,
        pool: &Pool<Sqlite>,
    ) -> PersistenceResult<HashMap<String, serde_json::Value>> {
        let mut state = HashMap::new();

        // Load boards
        let boards: Vec<serde_json::Value> = sqlx::query(
            "SELECT id, name, description, sprint_prefix, card_prefix, task_sort_field,
                    task_sort_order, sprint_duration_days, sprint_names, sprint_name_used_count,
                    next_sprint_number, active_sprint_id, task_list_view, prefix_counters,
                    sprint_counters, completion_column_id, created_at, updated_at FROM boards",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| PersistenceError::Database(e.to_string()))?
        .into_iter()
        .map(|row| self.row_to_board(&row))
        .collect::<PersistenceResult<Vec<_>>>()?;

        // Load columns
        let columns: Vec<serde_json::Value> = sqlx::query(
            "SELECT id, board_id, name, position, wip_limit, created_at, updated_at FROM columns",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| PersistenceError::Database(e.to_string()))?
        .into_iter()
        .map(|row| self.row_to_column(&row))
        .collect::<PersistenceResult<Vec<_>>>()?;

        // Load cards
        let cards: Vec<serde_json::Value> = sqlx::query(
            "SELECT id, column_id, title, description, priority, status, position, due_date,
                    points, card_number, sprint_id, assigned_prefix, card_prefix, created_at,
                    updated_at, completed_at, sprint_logs FROM cards",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| PersistenceError::Database(e.to_string()))?
        .into_iter()
        .map(|row| self.row_to_card(&row))
        .collect::<PersistenceResult<Vec<_>>>()?;

        // Load sprints
        let sprints: Vec<serde_json::Value> = sqlx::query(
            "SELECT id, board_id, sprint_number, name_index, prefix, card_prefix, status,
                    start_date, end_date, created_at, updated_at FROM sprints",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| PersistenceError::Database(e.to_string()))?
        .into_iter()
        .map(|row| self.row_to_sprint(&row))
        .collect::<PersistenceResult<Vec<_>>>()?;

        // Load archived cards
        let archived_cards: Vec<serde_json::Value> = sqlx::query(
            "SELECT id, card_data, archived_at, original_column_id, original_position FROM archived_cards"
        )
        .fetch_all(pool)
        .await
        .map_err(|e| PersistenceError::Database(e.to_string()))?
        .into_iter()
        .map(|row| self.row_to_archived_card(&row))
        .collect::<PersistenceResult<Vec<_>>>()?;

        state.insert("boards".to_string(), serde_json::to_value(boards).unwrap());
        state.insert(
            "columns".to_string(),
            serde_json::to_value(columns).unwrap(),
        );
        state.insert("cards".to_string(), serde_json::to_value(cards).unwrap());
        state.insert(
            "sprints".to_string(),
            serde_json::to_value(sprints).unwrap(),
        );
        state.insert(
            "archived_cards".to_string(),
            serde_json::to_value(archived_cards).unwrap(),
        );

        Ok(state)
    }

    fn row_to_board(&self, row: &sqlx::sqlite::SqliteRow) -> PersistenceResult<serde_json::Value> {
        use kanban_domain::board::{Board, SortField, SortOrder};
        use std::collections::HashMap;

        let db_err = |e: sqlx::Error| PersistenceError::Database(e.to_string());
        let ser_err = |e: serde_json::Error| PersistenceError::Serialization(e.to_string());

        let id_str: String = row.try_get("id").map_err(db_err)?;
        let sprint_names_json: String = row.try_get("sprint_names").map_err(db_err)?;
        let prefix_counters_json: String = row.try_get("prefix_counters").map_err(db_err)?;
        let sprint_counters_json: String = row.try_get("sprint_counters").map_err(db_err)?;
        let task_sort_field_str: String = row.try_get("task_sort_field").map_err(db_err)?;
        let task_sort_order_str: String = row.try_get("task_sort_order").map_err(db_err)?;
        let task_list_view_str: String = row.try_get("task_list_view").map_err(db_err)?;
        let created_at_str: String = row.try_get("created_at").map_err(db_err)?;
        let updated_at_str: String = row.try_get("updated_at").map_err(db_err)?;
        let active_sprint_id_str: Option<String> = row.try_get("active_sprint_id").map_err(db_err)?;
        let completion_column_id_str: Option<String> = row.try_get("completion_column_id").map_err(db_err)?;
        let sprint_duration_days_raw: Option<i32> = row.try_get("sprint_duration_days").map_err(db_err)?;

        let board = Board {
            id: Uuid::parse_str(&id_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?,
            name: row.try_get("name").map_err(db_err)?,
            description: row.try_get("description").map_err(db_err)?,
            sprint_prefix: row.try_get("sprint_prefix").map_err(db_err)?,
            card_prefix: row.try_get("card_prefix").map_err(db_err)?,
            task_sort_field: serde_json::from_str(&format!("\"{}\"", task_sort_field_str))
                .unwrap_or(SortField::Default),
            task_sort_order: serde_json::from_str(&format!("\"{}\"", task_sort_order_str))
                .unwrap_or(SortOrder::Ascending),
            sprint_duration_days: sprint_duration_days_raw.map(|v| v as u32),
            sprint_names: serde_json::from_str(&sprint_names_json).unwrap_or_default(),
            sprint_name_used_count: row.try_get::<i32, _>("sprint_name_used_count").map_err(db_err)? as usize,
            next_sprint_number: row.try_get::<i32, _>("next_sprint_number").map_err(db_err)? as u32,
            active_sprint_id: active_sprint_id_str
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok()),
            task_list_view: serde_json::from_str(&format!("\"{}\"", task_list_view_str))
                .unwrap_or_default(),
            prefix_counters: serde_json::from_str::<HashMap<String, u32>>(&prefix_counters_json)
                .unwrap_or_default(),
            sprint_counters: serde_json::from_str::<HashMap<String, u32>>(&sprint_counters_json)
                .unwrap_or_default(),
            completion_column_id: completion_column_id_str
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok()),
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?
                .with_timezone(&chrono::Utc),
        };

        serde_json::to_value(&board).map_err(ser_err)
    }

    fn row_to_column(&self, row: &sqlx::sqlite::SqliteRow) -> PersistenceResult<serde_json::Value> {
        use kanban_domain::Column;

        let db_err = |e: sqlx::Error| PersistenceError::Database(e.to_string());
        let ser_err = |e: serde_json::Error| PersistenceError::Serialization(e.to_string());

        let id_str: String = row.try_get("id").map_err(db_err)?;
        let board_id_str: String = row.try_get("board_id").map_err(db_err)?;
        let created_at_str: String = row.try_get("created_at").map_err(db_err)?;
        let updated_at_str: String = row.try_get("updated_at").map_err(db_err)?;

        let column = Column {
            id: Uuid::parse_str(&id_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?,
            board_id: Uuid::parse_str(&board_id_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?,
            name: row.try_get("name").map_err(db_err)?,
            position: row.try_get("position").map_err(db_err)?,
            wip_limit: row.try_get("wip_limit").map_err(db_err)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?
                .with_timezone(&chrono::Utc),
        };

        serde_json::to_value(&column).map_err(ser_err)
    }

    fn row_to_card(&self, row: &sqlx::sqlite::SqliteRow) -> PersistenceResult<serde_json::Value> {
        use kanban_domain::card::{Card, CardPriority, CardStatus};
        use kanban_domain::SprintLog;

        let db_err = |e: sqlx::Error| PersistenceError::Database(e.to_string());
        let ser_err = |e: serde_json::Error| PersistenceError::Serialization(e.to_string());

        let id_str: String = row.try_get("id").map_err(db_err)?;
        let column_id_str: String = row.try_get("column_id").map_err(db_err)?;
        let sprint_id_str: Option<String> = row.try_get("sprint_id").map_err(db_err)?;
        let sprint_logs_json: String = row.try_get("sprint_logs").map_err(db_err)?;
        let created_at_str: String = row.try_get("created_at").map_err(db_err)?;
        let updated_at_str: String = row.try_get("updated_at").map_err(db_err)?;
        let completed_at_str: Option<String> = row.try_get("completed_at").map_err(db_err)?;
        let due_date_str: Option<String> = row.try_get("due_date").map_err(db_err)?;
        let priority_str: String = row.try_get("priority").map_err(db_err)?;
        let status_str: String = row.try_get("status").map_err(db_err)?;
        let points_raw: Option<i32> = row.try_get("points").map_err(db_err)?;

        let card = Card {
            id: Uuid::parse_str(&id_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?,
            column_id: Uuid::parse_str(&column_id_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?,
            title: row.try_get("title").map_err(db_err)?,
            description: row.try_get("description").map_err(db_err)?,
            priority: serde_json::from_str(&format!("\"{}\"", priority_str))
                .unwrap_or(CardPriority::Medium),
            status: serde_json::from_str(&format!("\"{}\"", status_str))
                .unwrap_or(CardStatus::Todo),
            position: row.try_get("position").map_err(db_err)?,
            due_date: due_date_str
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            points: points_raw.map(|v| v as u8),
            card_number: row.try_get::<i32, _>("card_number").map_err(db_err)? as u32,
            sprint_id: sprint_id_str.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
            assigned_prefix: row.try_get("assigned_prefix").map_err(db_err)?,
            card_prefix: row.try_get("card_prefix").map_err(db_err)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?
                .with_timezone(&chrono::Utc),
            completed_at: completed_at_str
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            sprint_logs: serde_json::from_str::<Vec<SprintLog>>(&sprint_logs_json)
                .unwrap_or_default(),
        };

        serde_json::to_value(&card).map_err(ser_err)
    }

    fn row_to_sprint(&self, row: &sqlx::sqlite::SqliteRow) -> PersistenceResult<serde_json::Value> {
        use kanban_domain::sprint::{Sprint, SprintStatus};

        let db_err = |e: sqlx::Error| PersistenceError::Database(e.to_string());
        let ser_err = |e: serde_json::Error| PersistenceError::Serialization(e.to_string());

        let id_str: String = row.try_get("id").map_err(db_err)?;
        let board_id_str: String = row.try_get("board_id").map_err(db_err)?;
        let created_at_str: String = row.try_get("created_at").map_err(db_err)?;
        let updated_at_str: String = row.try_get("updated_at").map_err(db_err)?;
        let status_str: String = row.try_get("status").map_err(db_err)?;
        let start_date_str: Option<String> = row.try_get("start_date").map_err(db_err)?;
        let end_date_str: Option<String> = row.try_get("end_date").map_err(db_err)?;
        let name_index_raw: Option<i32> = row.try_get("name_index").map_err(db_err)?;

        let sprint = Sprint {
            id: Uuid::parse_str(&id_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?,
            board_id: Uuid::parse_str(&board_id_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?,
            sprint_number: row.try_get::<i32, _>("sprint_number").map_err(db_err)? as u32,
            name_index: name_index_raw.map(|v| v as usize),
            prefix: row.try_get("prefix").map_err(db_err)?,
            card_prefix: row.try_get("card_prefix").map_err(db_err)?,
            status: serde_json::from_str(&format!("\"{}\"", status_str))
                .unwrap_or(SprintStatus::Planning),
            start_date: start_date_str
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            end_date: end_date_str
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?
                .with_timezone(&chrono::Utc),
        };

        serde_json::to_value(&sprint).map_err(ser_err)
    }

    fn row_to_archived_card(
        &self,
        row: &sqlx::sqlite::SqliteRow,
    ) -> PersistenceResult<serde_json::Value> {
        let card_data: String = row.get("card_data");
        let card: serde_json::Value = serde_json::from_str(&card_data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        Ok(serde_json::json!({
            "card": card,
            "archived_at": row.get::<String, _>("archived_at"),
            "original_column_id": row.get::<String, _>("original_column_id"),
            "original_position": row.get::<i32, _>("original_position"),
        }))
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

        // Get existing IDs using static SQL from Table enum
        let existing_ids: std::collections::HashSet<String> = sqlx::query(table.select_ids_sql())
            .fetch_all(&mut **tx)
            .await
            .map_err(|e| PersistenceError::Database(e.to_string()))?
            .into_iter()
            .map(|row| row.get::<String, _>("id"))
            .collect();

        // Delete removed items using static SQL from Table enum
        let to_delete: Vec<_> = existing_ids.difference(&incoming_ids).collect();
        for id in to_delete {
            sqlx::query(table.delete_by_id_sql())
                .bind(id)
                .execute(&mut **tx)
                .await
                .map_err(|e| PersistenceError::Database(e.to_string()))?;
        }

        Ok(())
    }

    async fn upsert_board(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        board: &serde_json::Value,
    ) -> PersistenceResult<()> {
        let id = board["id"].as_str().unwrap_or_default();
        let name = board["name"].as_str().unwrap_or_default();
        let description = board["description"].as_str();
        let sprint_prefix = board["sprint_prefix"].as_str();
        let card_prefix = board["card_prefix"].as_str();
        let task_sort_field = board["task_sort_field"].as_str().unwrap_or("Default");
        let task_sort_order = board["task_sort_order"].as_str().unwrap_or("Ascending");
        let sprint_duration_days = board["sprint_duration_days"].as_i64().map(|v| v as i32);
        let sprint_names =
            serde_json::to_string(&board["sprint_names"]).unwrap_or_else(|_| "[]".to_string());
        let sprint_name_used_count = board["sprint_name_used_count"].as_i64().unwrap_or(0) as i32;
        let next_sprint_number = board["next_sprint_number"].as_i64().unwrap_or(1) as i32;
        let active_sprint_id = board["active_sprint_id"].as_str();
        let task_list_view = board["task_list_view"].as_str().unwrap_or("Flat");
        let prefix_counters =
            serde_json::to_string(&board["prefix_counters"]).unwrap_or_else(|_| "{}".to_string());
        let sprint_counters =
            serde_json::to_string(&board["sprint_counters"]).unwrap_or_else(|_| "{}".to_string());
        let completion_column_id = board["completion_column_id"].as_str();
        let created_at = board["created_at"].as_str().unwrap_or_default();
        let updated_at = board["updated_at"].as_str().unwrap_or_default();

        sqlx::query(
            "INSERT INTO boards (id, name, description, sprint_prefix, card_prefix, task_sort_field,
                task_sort_order, sprint_duration_days, sprint_names, sprint_name_used_count,
                next_sprint_number, active_sprint_id, task_list_view, prefix_counters,
                sprint_counters, completion_column_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                sprint_prefix = excluded.sprint_prefix,
                card_prefix = excluded.card_prefix,
                task_sort_field = excluded.task_sort_field,
                task_sort_order = excluded.task_sort_order,
                sprint_duration_days = excluded.sprint_duration_days,
                sprint_names = excluded.sprint_names,
                sprint_name_used_count = excluded.sprint_name_used_count,
                next_sprint_number = excluded.next_sprint_number,
                active_sprint_id = excluded.active_sprint_id,
                task_list_view = excluded.task_list_view,
                prefix_counters = excluded.prefix_counters,
                sprint_counters = excluded.sprint_counters,
                completion_column_id = excluded.completion_column_id,
                updated_at = excluded.updated_at"
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(sprint_prefix)
        .bind(card_prefix)
        .bind(task_sort_field)
        .bind(task_sort_order)
        .bind(sprint_duration_days)
        .bind(&sprint_names)
        .bind(sprint_name_used_count)
        .bind(next_sprint_number)
        .bind(active_sprint_id)
        .bind(task_list_view)
        .bind(&prefix_counters)
        .bind(&sprint_counters)
        .bind(completion_column_id)
        .bind(created_at)
        .bind(updated_at)
        .execute(&mut **tx)
        .await
        .map_err(|e| PersistenceError::Database(e.to_string()))?;

        Ok(())
    }

    async fn upsert_column(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        column: &serde_json::Value,
    ) -> PersistenceResult<()> {
        let id = column["id"].as_str().unwrap_or_default();
        let board_id = column["board_id"].as_str().unwrap_or_default();
        let name = column["name"].as_str().unwrap_or_default();
        let position = column["position"].as_i64().unwrap_or(0) as i32;
        let wip_limit = column["wip_limit"].as_i64().map(|v| v as i32);
        let created_at = column["created_at"].as_str().unwrap_or_default();
        let updated_at = column["updated_at"].as_str().unwrap_or_default();

        sqlx::query(
            "INSERT INTO columns (id, board_id, name, position, wip_limit, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                board_id = excluded.board_id,
                name = excluded.name,
                position = excluded.position,
                wip_limit = excluded.wip_limit,
                updated_at = excluded.updated_at",
        )
        .bind(id)
        .bind(board_id)
        .bind(name)
        .bind(position)
        .bind(wip_limit)
        .bind(created_at)
        .bind(updated_at)
        .execute(&mut **tx)
        .await
        .map_err(|e| PersistenceError::Database(e.to_string()))?;

        Ok(())
    }

    async fn upsert_card(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        card: &serde_json::Value,
    ) -> PersistenceResult<()> {
        let id = card["id"].as_str().unwrap_or_default();
        let column_id = card["column_id"].as_str().unwrap_or_default();
        let title = card["title"].as_str().unwrap_or_default();
        let description = card["description"].as_str();
        let priority = card["priority"].as_str().unwrap_or("Medium");
        let status = card["status"].as_str().unwrap_or("Todo");
        let position = card["position"].as_i64().unwrap_or(0) as i32;
        let due_date = card["due_date"].as_str();
        let points = card["points"].as_i64().map(|v| v as i32);
        let card_number = card["card_number"].as_i64().unwrap_or(0) as i32;
        let sprint_id = card["sprint_id"].as_str();
        let assigned_prefix = card["assigned_prefix"].as_str();
        let card_prefix = card["card_prefix"].as_str();
        let created_at = card["created_at"].as_str().unwrap_or_default();
        let updated_at = card["updated_at"].as_str().unwrap_or_default();
        let completed_at = card["completed_at"].as_str();
        let sprint_logs =
            serde_json::to_string(&card["sprint_logs"]).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "INSERT INTO cards (id, column_id, title, description, priority, status, position,
                due_date, points, card_number, sprint_id, assigned_prefix, card_prefix,
                created_at, updated_at, completed_at, sprint_logs)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                column_id = excluded.column_id,
                title = excluded.title,
                description = excluded.description,
                priority = excluded.priority,
                status = excluded.status,
                position = excluded.position,
                due_date = excluded.due_date,
                points = excluded.points,
                card_number = excluded.card_number,
                sprint_id = excluded.sprint_id,
                assigned_prefix = excluded.assigned_prefix,
                card_prefix = excluded.card_prefix,
                updated_at = excluded.updated_at,
                completed_at = excluded.completed_at,
                sprint_logs = excluded.sprint_logs",
        )
        .bind(id)
        .bind(column_id)
        .bind(title)
        .bind(description)
        .bind(priority)
        .bind(status)
        .bind(position)
        .bind(due_date)
        .bind(points)
        .bind(card_number)
        .bind(sprint_id)
        .bind(assigned_prefix)
        .bind(card_prefix)
        .bind(created_at)
        .bind(updated_at)
        .bind(completed_at)
        .bind(&sprint_logs)
        .execute(&mut **tx)
        .await
        .map_err(|e| PersistenceError::Database(e.to_string()))?;

        Ok(())
    }

    async fn upsert_sprint(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        sprint: &serde_json::Value,
    ) -> PersistenceResult<()> {
        let id = sprint["id"].as_str().unwrap_or_default();
        let board_id = sprint["board_id"].as_str().unwrap_or_default();
        let sprint_number = sprint["sprint_number"].as_i64().unwrap_or(0) as i32;
        let name_index = sprint["name_index"].as_i64().map(|v| v as i32);
        let prefix = sprint["prefix"].as_str();
        let card_prefix = sprint["card_prefix"].as_str();
        let status = sprint["status"].as_str().unwrap_or("Planning");
        let start_date = sprint["start_date"].as_str();
        let end_date = sprint["end_date"].as_str();
        let created_at = sprint["created_at"].as_str().unwrap_or_default();
        let updated_at = sprint["updated_at"].as_str().unwrap_or_default();

        sqlx::query(
            "INSERT INTO sprints (id, board_id, sprint_number, name_index, prefix, card_prefix,
                status, start_date, end_date, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                board_id = excluded.board_id,
                sprint_number = excluded.sprint_number,
                name_index = excluded.name_index,
                prefix = excluded.prefix,
                card_prefix = excluded.card_prefix,
                status = excluded.status,
                start_date = excluded.start_date,
                end_date = excluded.end_date,
                updated_at = excluded.updated_at",
        )
        .bind(id)
        .bind(board_id)
        .bind(sprint_number)
        .bind(name_index)
        .bind(prefix)
        .bind(card_prefix)
        .bind(status)
        .bind(start_date)
        .bind(end_date)
        .bind(created_at)
        .bind(updated_at)
        .execute(&mut **tx)
        .await
        .map_err(|e| PersistenceError::Database(e.to_string()))?;

        Ok(())
    }

    async fn upsert_archived_card(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        archived: &serde_json::Value,
    ) -> PersistenceResult<()> {
        let card = &archived["card"];
        let id = card["id"].as_str().unwrap_or_default();
        let card_data = serde_json::to_string(card)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        let archived_at = archived["archived_at"].as_str().unwrap_or_default();
        let original_column_id = archived["original_column_id"].as_str().unwrap_or_default();
        let original_position = archived["original_position"].as_i64().unwrap_or(0) as i32;

        sqlx::query(
            "INSERT INTO archived_cards (id, card_data, archived_at, original_column_id, original_position)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                card_data = excluded.card_data,
                archived_at = excluded.archived_at,
                original_column_id = excluded.original_column_id,
                original_position = excluded.original_position"
        )
        .bind(id)
        .bind(&card_data)
        .bind(archived_at)
        .bind(original_column_id)
        .bind(original_position)
        .execute(&mut **tx)
        .await
        .map_err(|e| PersistenceError::Database(e.to_string()))?;

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

        // Check for conflicts using metadata table
        let existing_meta: Option<(String, String)> =
            sqlx::query_as("SELECT instance_id, saved_at FROM metadata WHERE id = 1")
                .fetch_optional(pool)
                .await
                .map_err(|e| PersistenceError::Database(e.to_string()))?;

        if let Some((db_instance_id, db_saved_at)) = existing_meta {
            let guard = self.lock_metadata();
            if let Some(ref last_known) = *guard {
                let db_instance = Uuid::parse_str(&db_instance_id)
                    .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
                let db_time = chrono::DateTime::parse_from_rfc3339(&db_saved_at)
                    .map_err(|e| PersistenceError::Serialization(e.to_string()))?
                    .with_timezone(&chrono::Utc);

                if db_instance != last_known.instance_id || db_time != last_known.saved_at {
                    return Err(PersistenceError::ConflictDetected {
                        path: self.path.to_string_lossy().to_string(),
                        source: None,
                    });
                }
            }
        }

        // Update metadata
        snapshot.metadata.instance_id = self.instance_id;
        snapshot.metadata.saved_at = chrono::Utc::now();

        // Parse incoming data
        let data: SnapshotData = serde_json::from_slice(&snapshot.data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        // Begin transaction
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| PersistenceError::Database(e.to_string()))?;

        // Sync deletions first (respecting foreign key order)
        self.sync_table(&mut tx, Table::ArchivedCards, &data.archived_cards, |v| {
            v["card"]["id"].as_str().map(String::from)
        })
        .await?;
        self.sync_table(&mut tx, Table::Cards, &data.cards, |v| {
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

        // Upsert in correct order (parents before children)
        for board in &data.boards {
            self.upsert_board(&mut tx, board).await?;
        }
        for column in &data.columns {
            self.upsert_column(&mut tx, column).await?;
        }
        for sprint in &data.sprints {
            self.upsert_sprint(&mut tx, sprint).await?;
        }
        for card in &data.cards {
            self.upsert_card(&mut tx, card).await?;
        }
        for archived in &data.archived_cards {
            self.upsert_archived_card(&mut tx, archived).await?;
        }

        // Update metadata table
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
        .map_err(|e| PersistenceError::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| PersistenceError::Database(e.to_string()))?;

        // Update last known metadata
        {
            let mut guard = self.lock_metadata();
            *guard = Some(snapshot.metadata.clone());
        }

        tracing::info!("Saved to SQLite database at {}", self.path.display());

        Ok(snapshot.metadata)
    }

    async fn load(&self) -> PersistenceResult<(StoreSnapshot, PersistenceMetadata)> {
        let pool = self.get_pool().await?;

        // Load metadata
        let meta_row: Option<(String, String)> =
            sqlx::query_as("SELECT instance_id, saved_at FROM metadata WHERE id = 1")
                .fetch_optional(pool)
                .await
                .map_err(|e| PersistenceError::Database(e.to_string()))?;

        let metadata = if let Some((instance_id_str, saved_at_str)) = meta_row {
            let instance_id = Uuid::parse_str(&instance_id_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
            let saved_at = chrono::DateTime::parse_from_rfc3339(&saved_at_str)
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?
                .with_timezone(&chrono::Utc);
            PersistenceMetadata {
                instance_id,
                saved_at,
            }
        } else {
            PersistenceMetadata::new(self.instance_id)
        };

        // Load all data
        let state = self.load_current_state(pool).await?;

        let data = SnapshotData {
            boards: serde_json::from_value(state.get("boards").cloned().unwrap_or_default())
                .unwrap_or_default(),
            columns: serde_json::from_value(state.get("columns").cloned().unwrap_or_default())
                .unwrap_or_default(),
            cards: serde_json::from_value(state.get("cards").cloned().unwrap_or_default())
                .unwrap_or_default(),
            archived_cards: serde_json::from_value(
                state.get("archived_cards").cloned().unwrap_or_default(),
            )
            .unwrap_or_default(),
            sprints: serde_json::from_value(state.get("sprints").cloned().unwrap_or_default())
                .unwrap_or_default(),
        };

        let data_bytes = serde_json::to_vec(&data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let snapshot = StoreSnapshot {
            data: data_bytes,
            metadata: metadata.clone(),
        };

        // Track metadata for conflict detection
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
}
