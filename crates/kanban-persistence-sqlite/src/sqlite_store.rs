use async_trait::async_trait;
use kanban_core::graph::{Edge, EdgeDirection, Graph};
use kanban_domain::{CardEdgeType, DependencyGraph};
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
            Table::ArchivedCards => "SELECT card_id AS id FROM archived_cards",
            Table::Sprints => "SELECT id FROM sprints",
        }
    }

    const fn delete_by_id_sql(self) -> &'static str {
        match self {
            Table::Boards => "DELETE FROM boards WHERE id = ?",
            Table::Columns => "DELETE FROM columns WHERE id = ?",
            Table::Cards => "DELETE FROM cards WHERE id = ?",
            Table::ArchivedCards => "DELETE FROM archived_cards WHERE card_id = ?",
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
    #[serde(default)]
    graph: serde_json::Value,
}

fn db_err(e: sqlx::Error) -> PersistenceError {
    PersistenceError::Database(e.to_string())
}

fn ser_err(e: serde_json::Error) -> PersistenceError {
    PersistenceError::Serialization(e.to_string())
}

fn parse_uuid(s: &str) -> PersistenceResult<Uuid> {
    Uuid::parse_str(s).map_err(|e| PersistenceError::Serialization(e.to_string()))
}

fn parse_datetime(s: &str) -> PersistenceResult<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

fn build_board(
    row: &sqlx::sqlite::SqliteRow,
    sprint_names: Vec<String>,
    prefix_counters: HashMap<String, u32>,
    sprint_counters: HashMap<String, u32>,
) -> PersistenceResult<serde_json::Value> {
    use kanban_domain::board::{Board, SortField, SortOrder};

    let id_str: String = row.try_get("id").map_err(db_err)?;
    let task_sort_field_str: String = row.try_get("task_sort_field").map_err(db_err)?;
    let task_sort_order_str: String = row.try_get("task_sort_order").map_err(db_err)?;
    let task_list_view_str: String = row.try_get("task_list_view").map_err(db_err)?;
    let created_at_str: String = row.try_get("created_at").map_err(db_err)?;
    let updated_at_str: String = row.try_get("updated_at").map_err(db_err)?;
    let active_sprint_id_str: Option<String> =
        row.try_get("active_sprint_id").map_err(db_err)?;
    let completion_column_id_str: Option<String> =
        row.try_get("completion_column_id").map_err(db_err)?;
    let sprint_duration_days_raw: Option<i32> =
        row.try_get("sprint_duration_days").map_err(db_err)?;

    let board = Board {
        id: parse_uuid(&id_str)?,
        name: row.try_get("name").map_err(db_err)?,
        description: row.try_get("description").map_err(db_err)?,
        sprint_prefix: row.try_get("sprint_prefix").map_err(db_err)?,
        card_prefix: row.try_get("card_prefix").map_err(db_err)?,
        task_sort_field: serde_json::from_str(&format!("\"{}\"", task_sort_field_str))
            .unwrap_or_else(|_| {
                tracing::warn!(field = "task_sort_field", value = %task_sort_field_str, "Unknown enum variant, using default");
                SortField::Default
            }),
        task_sort_order: serde_json::from_str(&format!("\"{}\"", task_sort_order_str))
            .unwrap_or_else(|_| {
                tracing::warn!(field = "task_sort_order", value = %task_sort_order_str, "Unknown enum variant, using default");
                SortOrder::Ascending
            }),
        sprint_duration_days: sprint_duration_days_raw.map(|v| v as u32),
        sprint_names,
        sprint_name_used_count: row
            .try_get::<i32, _>("sprint_name_used_count")
            .map_err(db_err)? as usize,
        next_sprint_number: row
            .try_get::<i32, _>("next_sprint_number")
            .map_err(db_err)? as u32,
        active_sprint_id: active_sprint_id_str
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok()),
        task_list_view: serde_json::from_str(&format!("\"{}\"", task_list_view_str))
            .unwrap_or_else(|_| {
                tracing::warn!(field = "task_list_view", value = %task_list_view_str, "Unknown enum variant, using default");
                Default::default()
            }),
        prefix_counters,
        sprint_counters,
        completion_column_id: completion_column_id_str
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok()),
        created_at: parse_datetime(&created_at_str)?,
        updated_at: parse_datetime(&updated_at_str)?,
    };

    serde_json::to_value(&board).map_err(ser_err)
}

fn build_column(row: &sqlx::sqlite::SqliteRow) -> PersistenceResult<serde_json::Value> {
    use kanban_domain::Column;

    let id_str: String = row.try_get("id").map_err(db_err)?;
    let board_id_str: String = row.try_get("board_id").map_err(db_err)?;
    let created_at_str: String = row.try_get("created_at").map_err(db_err)?;
    let updated_at_str: String = row.try_get("updated_at").map_err(db_err)?;

    let column = Column {
        id: parse_uuid(&id_str)?,
        board_id: parse_uuid(&board_id_str)?,
        name: row.try_get("name").map_err(db_err)?,
        position: row.try_get("position").map_err(db_err)?,
        wip_limit: row.try_get("wip_limit").map_err(db_err)?,
        created_at: parse_datetime(&created_at_str)?,
        updated_at: parse_datetime(&updated_at_str)?,
    };

    serde_json::to_value(&column).map_err(ser_err)
}

fn build_card(
    row: &sqlx::sqlite::SqliteRow,
    sprint_logs: Vec<kanban_domain::SprintLog>,
) -> PersistenceResult<serde_json::Value> {
    use kanban_domain::card::{Card, CardPriority, CardStatus};

    let id_str: String = row.try_get("id").map_err(db_err)?;
    let column_id_str: String = row.try_get("column_id").map_err(db_err)?;
    let sprint_id_str: Option<String> = row.try_get("sprint_id").map_err(db_err)?;
    let created_at_str: String = row.try_get("created_at").map_err(db_err)?;
    let updated_at_str: String = row.try_get("updated_at").map_err(db_err)?;
    let completed_at_str: Option<String> = row.try_get("completed_at").map_err(db_err)?;
    let due_date_str: Option<String> = row.try_get("due_date").map_err(db_err)?;
    let priority_str: String = row.try_get("priority").map_err(db_err)?;
    let status_str: String = row.try_get("status").map_err(db_err)?;
    let points_raw: Option<i32> = row.try_get("points").map_err(db_err)?;

    let card = Card {
        id: parse_uuid(&id_str)?,
        column_id: parse_uuid(&column_id_str)?,
        title: row.try_get("title").map_err(db_err)?,
        description: row.try_get("description").map_err(db_err)?,
        priority: serde_json::from_str(&format!("\"{}\"", priority_str))
            .unwrap_or_else(|_| {
                tracing::warn!(field = "priority", value = %priority_str, "Unknown enum variant, using default");
                CardPriority::Medium
            }),
        status: serde_json::from_str(&format!("\"{}\"", status_str))
            .unwrap_or_else(|_| {
                tracing::warn!(field = "status", value = %status_str, "Unknown enum variant, using default");
                CardStatus::Todo
            }),
        position: row.try_get("position").map_err(db_err)?,
        due_date: due_date_str
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        points: points_raw.map(|v| v as u8),
        card_number: row.try_get::<i32, _>("card_number").map_err(db_err)? as u32,
        sprint_id: sprint_id_str
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok()),
        assigned_prefix: row.try_get("assigned_prefix").map_err(db_err)?,
        card_prefix: row.try_get("card_prefix").map_err(db_err)?,
        created_at: parse_datetime(&created_at_str)?,
        updated_at: parse_datetime(&updated_at_str)?,
        completed_at: completed_at_str
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        sprint_logs,
    };

    serde_json::to_value(&card).map_err(ser_err)
}

fn build_sprint(row: &sqlx::sqlite::SqliteRow) -> PersistenceResult<serde_json::Value> {
    use kanban_domain::sprint::{Sprint, SprintStatus};

    let id_str: String = row.try_get("id").map_err(db_err)?;
    let board_id_str: String = row.try_get("board_id").map_err(db_err)?;
    let created_at_str: String = row.try_get("created_at").map_err(db_err)?;
    let updated_at_str: String = row.try_get("updated_at").map_err(db_err)?;
    let status_str: String = row.try_get("status").map_err(db_err)?;
    let start_date_str: Option<String> = row.try_get("start_date").map_err(db_err)?;
    let end_date_str: Option<String> = row.try_get("end_date").map_err(db_err)?;
    let name_index_raw: Option<i32> = row.try_get("name_index").map_err(db_err)?;

    let sprint = Sprint {
        id: parse_uuid(&id_str)?,
        board_id: parse_uuid(&board_id_str)?,
        sprint_number: row.try_get::<i32, _>("sprint_number").map_err(db_err)? as u32,
        name_index: name_index_raw.map(|v| v as usize),
        prefix: row.try_get("prefix").map_err(db_err)?,
        card_prefix: row.try_get("card_prefix").map_err(db_err)?,
        status: serde_json::from_str(&format!("\"{}\"", status_str))
            .unwrap_or_else(|_| {
                tracing::warn!(field = "sprint_status", value = %status_str, "Unknown enum variant, using default");
                SprintStatus::Planning
            }),
        start_date: start_date_str
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        end_date: end_date_str
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        created_at: parse_datetime(&created_at_str)?,
        updated_at: parse_datetime(&updated_at_str)?,
    };

    serde_json::to_value(&sprint).map_err(ser_err)
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
    ) -> PersistenceResult<SnapshotData> {
        // --- Boards + child data ---
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

        let prefix_counter_rows = sqlx::query(
            "SELECT board_id, prefix, counter FROM board_prefix_counters",
        )
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

        let sprint_counter_rows = sqlx::query(
            "SELECT board_id, prefix, counter FROM board_sprint_counters",
        )
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

        // --- Columns ---
        let columns: Vec<serde_json::Value> = sqlx::query(
            "SELECT id, board_id, name, position, wip_limit, created_at, updated_at FROM columns",
        )
        .fetch_all(pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(build_column)
        .collect::<PersistenceResult<Vec<_>>>()?;

        // --- Sprints ---
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

        // --- Cards + sprint logs ---
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

        // --- Archived cards metadata ---
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
            let original_column_id: String =
                row.try_get("original_column_id").map_err(db_err)?;
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

        // --- Graph edges ---
        let edge_rows = sqlx::query(
            "SELECT source_id, target_id, edge_type, direction, weight, created_at, archived_at
             FROM card_edges",
        )
        .fetch_all(pool)
        .await
        .map_err(db_err)?;

        let mut card_graph: Graph<CardEdgeType> = Graph::new();
        for row in &edge_rows {
            let source_str: String = row.try_get("source_id").map_err(db_err)?;
            let target_str: String = row.try_get("target_id").map_err(db_err)?;
            let edge_type_str: String = row.try_get("edge_type").map_err(db_err)?;
            let direction_str: String = row.try_get("direction").map_err(db_err)?;
            let weight: Option<f64> = row.try_get("weight").map_err(db_err)?;
            let created_at_str: String = row.try_get("created_at").map_err(db_err)?;
            let archived_at_str: Option<String> =
                row.try_get("archived_at").map_err(db_err)?;

            let edge_type: CardEdgeType =
                serde_json::from_str(&format!("\"{}\"", edge_type_str)).map_err(ser_err)?;
            let direction: EdgeDirection =
                serde_json::from_str(&format!("\"{}\"", direction_str)).map_err(ser_err)?;

            card_graph.add_edge(Edge {
                source: parse_uuid(&source_str)?,
                target: parse_uuid(&target_str)?,
                edge_type,
                direction,
                weight: weight.map(|w| w as f32),
                created_at: parse_datetime(&created_at_str)?,
                archived_at: archived_at_str
                    .as_deref()
                    .map(parse_datetime)
                    .transpose()?,
            });
        }

        let dep_graph = DependencyGraph { cards: card_graph };
        let graph = serde_json::to_value(&dep_graph).map_err(ser_err)?;

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
        let sprint_name_used_count = board["sprint_name_used_count"].as_i64().unwrap_or(0) as i32;
        let next_sprint_number = board["next_sprint_number"].as_i64().unwrap_or(1) as i32;
        let active_sprint_id = board["active_sprint_id"].as_str();
        let task_list_view = board["task_list_view"].as_str().unwrap_or("Flat");
        let completion_column_id = board["completion_column_id"].as_str();
        let created_at = board["created_at"].as_str().unwrap_or_default();
        let updated_at = board["updated_at"].as_str().unwrap_or_default();

        sqlx::query(
            "INSERT INTO boards (id, name, description, sprint_prefix, card_prefix,
                task_sort_field, task_sort_order, sprint_duration_days,
                sprint_name_used_count, next_sprint_number, active_sprint_id,
                task_list_view, completion_column_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                sprint_prefix = excluded.sprint_prefix,
                card_prefix = excluded.card_prefix,
                task_sort_field = excluded.task_sort_field,
                task_sort_order = excluded.task_sort_order,
                sprint_duration_days = excluded.sprint_duration_days,
                sprint_name_used_count = excluded.sprint_name_used_count,
                next_sprint_number = excluded.next_sprint_number,
                active_sprint_id = excluded.active_sprint_id,
                task_list_view = excluded.task_list_view,
                completion_column_id = excluded.completion_column_id,
                updated_at = excluded.updated_at",
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(sprint_prefix)
        .bind(card_prefix)
        .bind(task_sort_field)
        .bind(task_sort_order)
        .bind(sprint_duration_days)
        .bind(sprint_name_used_count)
        .bind(next_sprint_number)
        .bind(active_sprint_id)
        .bind(task_list_view)
        .bind(completion_column_id)
        .bind(created_at)
        .bind(updated_at)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;

        // Sprint names: delete + reinsert
        sqlx::query("DELETE FROM board_sprint_names WHERE board_id = ?")
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(db_err)?;

        if let Some(names) = board["sprint_names"].as_array() {
            for (i, name_val) in names.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO board_sprint_names (board_id, position, name) VALUES (?, ?, ?)",
                )
                .bind(id)
                .bind(i as i32)
                .bind(name_val.as_str().unwrap_or_default())
                .execute(&mut **tx)
                .await
                .map_err(db_err)?;
            }
        }

        // Prefix counters: delete + reinsert
        sqlx::query("DELETE FROM board_prefix_counters WHERE board_id = ?")
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(db_err)?;

        if let Some(counters) = board["prefix_counters"].as_object() {
            for (prefix, counter) in counters {
                sqlx::query(
                    "INSERT INTO board_prefix_counters (board_id, prefix, counter)
                     VALUES (?, ?, ?)",
                )
                .bind(id)
                .bind(prefix)
                .bind(counter.as_i64().unwrap_or(0) as i32)
                .execute(&mut **tx)
                .await
                .map_err(db_err)?;
            }
        }

        // Sprint counters: delete + reinsert
        sqlx::query("DELETE FROM board_sprint_counters WHERE board_id = ?")
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(db_err)?;

        if let Some(counters) = board["sprint_counters"].as_object() {
            for (prefix, counter) in counters {
                sqlx::query(
                    "INSERT INTO board_sprint_counters (board_id, prefix, counter)
                     VALUES (?, ?, ?)",
                )
                .bind(id)
                .bind(prefix)
                .bind(counter.as_i64().unwrap_or(0) as i32)
                .execute(&mut **tx)
                .await
                .map_err(db_err)?;
            }
        }

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
        .map_err(db_err)?;

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

        sqlx::query(
            "INSERT INTO cards (id, column_id, title, description, priority, status, position,
                due_date, points, card_number, sprint_id, assigned_prefix, card_prefix,
                created_at, updated_at, completed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
                completed_at = excluded.completed_at",
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
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;

        // Sprint logs: delete + reinsert
        sqlx::query("DELETE FROM sprint_logs WHERE card_id = ?")
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(db_err)?;

        if let Some(logs) = card["sprint_logs"].as_array() {
            for log in logs {
                sqlx::query(
                    "INSERT INTO sprint_logs (card_id, sprint_id, sprint_number, sprint_name,
                        started_at, ended_at, status)
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(id)
                .bind(log["sprint_id"].as_str().unwrap_or_default())
                .bind(log["sprint_number"].as_i64().unwrap_or(0) as i32)
                .bind(log["sprint_name"].as_str())
                .bind(log["started_at"].as_str().unwrap_or_default())
                .bind(log["ended_at"].as_str())
                .bind(log["status"].as_str().unwrap_or_default())
                .execute(&mut **tx)
                .await
                .map_err(db_err)?;
            }
        }

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
        .map_err(db_err)?;

        Ok(())
    }

    async fn upsert_archived_card(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        archived: &serde_json::Value,
    ) -> PersistenceResult<()> {
        let card = &archived["card"];
        self.upsert_card(tx, card).await?;

        let card_id = card["id"].as_str().unwrap_or_default();
        let archived_at = archived["archived_at"].as_str().unwrap_or_default();
        let original_column_id = archived["original_column_id"].as_str();
        let original_position = archived["original_position"].as_i64().unwrap_or(0) as i32;

        sqlx::query(
            "INSERT INTO archived_cards (card_id, archived_at, original_column_id, original_position)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(card_id) DO UPDATE SET
                archived_at = excluded.archived_at,
                original_column_id = excluded.original_column_id,
                original_position = excluded.original_position",
        )
        .bind(card_id)
        .bind(archived_at)
        .bind(original_column_id)
        .bind(original_position)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;

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

        let existing_meta: Option<(String, String)> =
            sqlx::query_as("SELECT instance_id, saved_at FROM metadata WHERE id = 1")
                .fetch_optional(pool)
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

        snapshot.metadata.instance_id = self.instance_id;
        snapshot.metadata.saved_at = chrono::Utc::now();

        let data: SnapshotData =
            serde_json::from_slice(&snapshot.data).map_err(ser_err)?;

        let mut tx = pool.begin().await.map_err(db_err)?;

        // Sync deletions (reverse FK order)
        self.sync_table(&mut tx, Table::ArchivedCards, &data.archived_cards, |v| {
            v["card"]["id"].as_str().map(String::from)
        })
        .await?;

        // Cards sync must include both active and archived inner cards
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

        // Sync edges (full replace)
        sqlx::query("DELETE FROM card_edges")
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

        if let Some(edges) = data
            .graph
            .get("cards")
            .and_then(|c| c.get("edges"))
            .and_then(|e| e.as_array())
        {
            for edge in edges {
                sqlx::query(
                    "INSERT INTO card_edges
                        (source_id, target_id, edge_type, direction, weight, created_at, archived_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(edge["source"].as_str().unwrap_or_default())
                .bind(edge["target"].as_str().unwrap_or_default())
                .bind(edge["edge_type"].as_str().unwrap_or_default())
                .bind(edge["direction"].as_str().unwrap_or_default())
                .bind(edge["weight"].as_f64().map(|w| w as f32 as f64))
                .bind(edge["created_at"].as_str().unwrap_or_default())
                .bind(edge["archived_at"].as_str())
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            }
        }

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
        let loaded_data: serde_json::Value =
            serde_json::from_slice(&loaded_snapshot.data).unwrap();
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
