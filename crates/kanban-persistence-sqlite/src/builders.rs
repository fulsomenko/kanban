use crate::helpers::{db_err, parse_datetime, parse_enum, parse_uuid, ser_err};
use kanban_core::graph::{Edge, EdgeDirection, Graph};
use kanban_domain::{CardEdgeType, DependencyGraph};
use kanban_persistence::PersistenceResult;
use sqlx::Row;
use std::collections::HashMap;

pub(crate) fn build_board(
    row: &sqlx::sqlite::SqliteRow,
    sprint_names: Vec<String>,
    prefix_counters: HashMap<String, u32>,
    sprint_counters: HashMap<String, u32>,
) -> PersistenceResult<serde_json::Value> {
    use kanban_domain::board::Board;

    let id_str: String = row.try_get("id").map_err(db_err)?;
    let task_sort_field_str: String = row.try_get("task_sort_field").map_err(db_err)?;
    let task_sort_order_str: String = row.try_get("task_sort_order").map_err(db_err)?;
    let task_list_view_str: String = row.try_get("task_list_view").map_err(db_err)?;
    let created_at_str: String = row.try_get("created_at").map_err(db_err)?;
    let updated_at_str: String = row.try_get("updated_at").map_err(db_err)?;
    let active_sprint_id_str: Option<String> = row.try_get("active_sprint_id").map_err(db_err)?;
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
        task_sort_field: parse_enum(&task_sort_field_str, "task_sort_field")?,
        task_sort_order: parse_enum(&task_sort_order_str, "task_sort_order")?,
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
            .map(parse_uuid)
            .transpose()?,
        task_list_view: parse_enum(&task_list_view_str, "task_list_view")?,
        prefix_counters,
        sprint_counters,
        completion_column_id: completion_column_id_str
            .as_deref()
            .map(parse_uuid)
            .transpose()?,
        created_at: parse_datetime(&created_at_str)?,
        updated_at: parse_datetime(&updated_at_str)?,
    };

    serde_json::to_value(&board).map_err(ser_err)
}

pub(crate) fn build_column(row: &sqlx::sqlite::SqliteRow) -> PersistenceResult<serde_json::Value> {
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

pub(crate) fn build_card(
    row: &sqlx::sqlite::SqliteRow,
    sprint_logs: Vec<kanban_domain::SprintLog>,
) -> PersistenceResult<serde_json::Value> {
    use kanban_domain::card::Card;

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
        priority: parse_enum(&priority_str, "priority")?,
        status: parse_enum(&status_str, "status")?,
        position: row.try_get("position").map_err(db_err)?,
        due_date: due_date_str.as_deref().map(parse_datetime).transpose()?,
        points: points_raw
            .map(|v| {
                u8::try_from(v).map_err(|_| ser_err(format!("points value {v} out of u8 range")))
            })
            .transpose()?,
        card_number: row.try_get::<i32, _>("card_number").map_err(db_err)? as u32,
        sprint_id: sprint_id_str.as_deref().map(parse_uuid).transpose()?,
        assigned_prefix: row.try_get("assigned_prefix").map_err(db_err)?,
        card_prefix: row.try_get("card_prefix").map_err(db_err)?,
        created_at: parse_datetime(&created_at_str)?,
        updated_at: parse_datetime(&updated_at_str)?,
        completed_at: completed_at_str
            .as_deref()
            .map(parse_datetime)
            .transpose()?,
        sprint_logs,
    };

    serde_json::to_value(&card).map_err(ser_err)
}

pub(crate) fn build_sprint(row: &sqlx::sqlite::SqliteRow) -> PersistenceResult<serde_json::Value> {
    use kanban_domain::sprint::Sprint;

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
        status: parse_enum(&status_str, "sprint status")?,
        start_date: start_date_str.as_deref().map(parse_datetime).transpose()?,
        end_date: end_date_str.as_deref().map(parse_datetime).transpose()?,
        created_at: parse_datetime(&created_at_str)?,
        updated_at: parse_datetime(&updated_at_str)?,
    };

    serde_json::to_value(&sprint).map_err(ser_err)
}

pub(crate) fn build_graph(
    edge_rows: &[sqlx::sqlite::SqliteRow],
) -> PersistenceResult<serde_json::Value> {
    let mut card_graph: Graph<CardEdgeType> = Graph::new();
    for row in edge_rows {
        let source_str: String = row.try_get("source_id").map_err(db_err)?;
        let target_str: String = row.try_get("target_id").map_err(db_err)?;
        let edge_type_str: String = row.try_get("edge_type").map_err(db_err)?;
        let direction_str: String = row.try_get("direction").map_err(db_err)?;
        let weight: Option<f64> = row.try_get("weight").map_err(db_err)?;
        let created_at_str: String = row.try_get("created_at").map_err(db_err)?;
        let archived_at_str: Option<String> = row.try_get("archived_at").map_err(db_err)?;

        let edge_type: CardEdgeType = parse_enum(&edge_type_str, "edge_type")?;
        let direction: EdgeDirection = parse_enum(&direction_str, "edge direction")?;

        card_graph.add_edge(Edge {
            source: parse_uuid(&source_str)?,
            target: parse_uuid(&target_str)?,
            edge_type,
            direction,
            // SQLite stores REAL as f64; domain uses f32. Precision loss is acceptable
            // for edge weights (relative ordering hint, not exact measurement).
            weight: weight.map(|w| w as f32),
            created_at: parse_datetime(&created_at_str)?,
            archived_at: archived_at_str.as_deref().map(parse_datetime).transpose()?,
        });
    }

    let dep_graph = DependencyGraph { cards: card_graph };
    serde_json::to_value(&dep_graph).map_err(ser_err)
}
