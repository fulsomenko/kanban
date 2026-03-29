use crate::helpers::{db_err, required_str};
use kanban_persistence::PersistenceResult;
use sqlx::{Row, Sqlite, Transaction};

pub(crate) async fn upsert_board(
    tx: &mut Transaction<'_, Sqlite>,
    board: &serde_json::Value,
) -> PersistenceResult<()> {
    let id = required_str(board, "id")?;
    let name = required_str(board, "name")?;
    let description = board["description"].as_str();
    let sprint_prefix = board["sprint_prefix"].as_str();
    let card_prefix = board["card_prefix"].as_str();
    let task_sort_field = required_str(board, "task_sort_field")?;
    let task_sort_order = required_str(board, "task_sort_order")?;
    let sprint_duration_days = board["sprint_duration_days"].as_i64().map(|v| v as i32);
    let sprint_name_used_count = board["sprint_name_used_count"].as_i64().unwrap_or(0) as i32;
    let next_sprint_number = board["next_sprint_number"].as_i64().unwrap_or(1) as i32;
    let active_sprint_id = board["active_sprint_id"].as_str();
    let task_list_view = required_str(board, "task_list_view")?;
    let completion_column_id = board["completion_column_id"].as_str();
    let created_at = required_str(board, "created_at")?;
    let updated_at = required_str(board, "updated_at")?;

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

pub(crate) async fn upsert_column(
    tx: &mut Transaction<'_, Sqlite>,
    column: &serde_json::Value,
) -> PersistenceResult<()> {
    let id = required_str(column, "id")?;
    let board_id = required_str(column, "board_id")?;
    let name = required_str(column, "name")?;
    let position = column["position"].as_i64().unwrap_or(0) as i32;
    let wip_limit = column["wip_limit"].as_i64().map(|v| v as i32);
    let created_at = required_str(column, "created_at")?;
    let updated_at = required_str(column, "updated_at")?;

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

pub(crate) async fn upsert_card(
    tx: &mut Transaction<'_, Sqlite>,
    card: &serde_json::Value,
) -> PersistenceResult<()> {
    let id = required_str(card, "id")?;
    let column_id = required_str(card, "column_id")?;
    let title = required_str(card, "title")?;
    let description = card["description"].as_str();
    let priority = required_str(card, "priority")?;
    let status = required_str(card, "status")?;
    let position = card["position"].as_i64().unwrap_or(0) as i32;
    let due_date = card["due_date"].as_str();
    let points = card["points"].as_i64().map(|v| v as i32);
    let card_number = card["card_number"].as_i64().unwrap_or(0) as i32;
    let sprint_id = card["sprint_id"].as_str();
    let assigned_prefix = card["assigned_prefix"].as_str();
    let card_prefix = card["card_prefix"].as_str();
    let created_at = required_str(card, "created_at")?;
    let updated_at = required_str(card, "updated_at")?;
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

pub(crate) async fn upsert_sprint(
    tx: &mut Transaction<'_, Sqlite>,
    sprint: &serde_json::Value,
) -> PersistenceResult<()> {
    let id = required_str(sprint, "id")?;
    let board_id = required_str(sprint, "board_id")?;
    let sprint_number = sprint["sprint_number"].as_i64().unwrap_or(0) as i32;
    let name_index = sprint["name_index"].as_i64().map(|v| v as i32);
    let prefix = sprint["prefix"].as_str();
    let card_prefix = sprint["card_prefix"].as_str();
    let status = required_str(sprint, "status")?;
    let start_date = sprint["start_date"].as_str();
    let end_date = sprint["end_date"].as_str();
    let created_at = required_str(sprint, "created_at")?;
    let updated_at = required_str(sprint, "updated_at")?;

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

pub(crate) async fn upsert_archived_card(
    tx: &mut Transaction<'_, Sqlite>,
    archived: &serde_json::Value,
) -> PersistenceResult<()> {
    let card = &archived["card"];
    upsert_card(tx, card).await?;

    let card_id = required_str(card, "id")?;
    let archived_at = required_str(archived, "archived_at")?;
    let original_column_id = required_str(archived, "original_column_id")?;
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

pub(crate) async fn upsert_edges(
    tx: &mut Transaction<'_, Sqlite>,
    graph: &serde_json::Value,
) -> PersistenceResult<()> {
    let incoming_edges: Vec<&serde_json::Value> = graph
        .get("cards")
        .and_then(|c| c.get("edges"))
        .and_then(|e| e.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default();

    let incoming_keys: std::collections::HashSet<(String, String, String)> = incoming_edges
        .iter()
        .filter_map(|e| {
            Some((
                e["source"].as_str()?.to_string(),
                e["target"].as_str()?.to_string(),
                e["edge_type"].as_str()?.to_string(),
            ))
        })
        .collect();

    let existing_rows =
        sqlx::query("SELECT source_id, target_id, edge_type FROM card_edges")
            .fetch_all(&mut **tx)
            .await
            .map_err(db_err)?;

    for row in &existing_rows {
        let key: (String, String, String) = (
            row.try_get("source_id").map_err(db_err)?,
            row.try_get("target_id").map_err(db_err)?,
            row.try_get("edge_type").map_err(db_err)?,
        );
        if !incoming_keys.contains(&key) {
            sqlx::query(
                "DELETE FROM card_edges WHERE source_id = ? AND target_id = ? AND edge_type = ?",
            )
            .bind(&key.0)
            .bind(&key.1)
            .bind(&key.2)
            .execute(&mut **tx)
            .await
            .map_err(db_err)?;
        }
    }

    for edge in &incoming_edges {
        let source_id = required_str(edge, "source")?;
        let target_id = required_str(edge, "target")?;

        sqlx::query(
            "INSERT INTO card_edges
                (source_id, target_id, edge_type, direction, weight, created_at, archived_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(source_id, target_id, edge_type) DO UPDATE SET
                direction = excluded.direction,
                weight = excluded.weight,
                archived_at = excluded.archived_at",
        )
        .bind(source_id)
        .bind(target_id)
        .bind(edge["edge_type"].as_str().unwrap_or_default())
        .bind(edge["direction"].as_str().unwrap_or_default())
        // f64→f32: acceptable precision loss for edge weights (see builders.rs)
        .bind(edge["weight"].as_f64().map(|w| w as f32))
        .bind(edge["created_at"].as_str().unwrap_or_default())
        .bind(edge["archived_at"].as_str())
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    }

    Ok(())
}
