use kanban_persistence::{PersistenceError, PersistenceResult};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Migrate a V2 JSON file to V3 format in-place (atomic write).
///
/// Changes:
/// - Removes `prefix_counters` from each board; sets `card_counter` from it.
/// - Removes `assigned_prefix` and `card_prefix` fields from each card.
/// - Sets `version` to 3.
///
/// Aborts (returns Err) without modifying the file if any card_number appears
/// in multiple `assigned_prefix` buckets within the same board (collision).
pub async fn migrate_v2_to_v3(path: &Path) -> PersistenceResult<()> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut envelope: Value = serde_json::from_str(&content)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

    let data = envelope
        .get_mut("data")
        .ok_or_else(|| PersistenceError::Serialization("missing 'data' field".into()))?;

    // Collect all columns → board_id mappings
    let mut column_to_board: HashMap<String, String> = HashMap::new();
    if let Some(columns) = data.get("columns").and_then(|v| v.as_array()) {
        for col in columns {
            if let (Some(col_id), Some(board_id)) = (
                col.get("id").and_then(|v| v.as_str()),
                col.get("board_id").and_then(|v| v.as_str()),
            ) {
                column_to_board.insert(col_id.to_string(), board_id.to_string());
            }
        }
    }

    // Determine card_counter for each board from active cards
    let mut board_card_max: HashMap<String, u32> = HashMap::new();
    for card_source in &["cards", "archived_cards"] {
        if let Some(cards) = data.get(*card_source).and_then(|v| v.as_array()) {
            for card_val in cards {
                let card = if *card_source == "archived_cards" {
                    card_val.get("card").unwrap_or(card_val)
                } else {
                    card_val
                };
                let col_id = card.get("column_id").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(board_id) = column_to_board.get(col_id) {
                    let card_number = card
                        .get("card_number")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    let entry = board_card_max.entry(board_id.clone()).or_insert(0);
                    if card_number > *entry {
                        *entry = card_number;
                    }
                }
            }
        }
    }

    // Collision detection: check that within each board, no card_number appears
    // in 2+ different assigned_prefix buckets among active cards
    if let Some(cards) = data.get("cards").and_then(|v| v.as_array()) {
        // board_id → (assigned_prefix → set of card_numbers)
        let mut board_prefix_numbers: HashMap<String, HashMap<String, HashSet<u32>>> =
            HashMap::new();
        for card in cards {
            let col_id = card.get("column_id").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(board_id) = column_to_board.get(col_id) {
                let prefix = card
                    .get("assigned_prefix")
                    .and_then(|v| v.as_str())
                    .unwrap_or("task")
                    .to_string();
                let card_number = card
                    .get("card_number")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                board_prefix_numbers
                    .entry(board_id.clone())
                    .or_default()
                    .entry(prefix)
                    .or_default()
                    .insert(card_number);
            }
        }

        for (board_id, prefix_map) in &board_prefix_numbers {
            // Collect all (card_number, prefix) pairs and find duplicates
            let mut number_to_prefixes: HashMap<u32, Vec<&str>> = HashMap::new();
            for (prefix, numbers) in prefix_map {
                for &n in numbers {
                    number_to_prefixes
                        .entry(n)
                        .or_default()
                        .push(prefix.as_str());
                }
            }
            for (number, prefixes) in &number_to_prefixes {
                if prefixes.len() > 1 {
                    return Err(PersistenceError::Serialization(format!(
                        "Migration aborted: card_number {} appears in multiple prefix buckets \
                         ({}) for board {}. File left unmodified.",
                        number,
                        prefixes.join(", "),
                        board_id
                    )));
                }
            }
        }
    }

    // Update each board: remove prefix_counters, set card_counter
    if let Some(boards) = data.get_mut("boards").and_then(|v| v.as_array_mut()) {
        for board in boards.iter_mut() {
            if let Some(board_obj) = board.as_object_mut() {
                let board_id = board_obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Determine card_counter from prefix_counters or card max
                let card_counter = {
                    let from_prefix_counters = board_obj
                        .get("prefix_counters")
                        .and_then(|v| v.as_object())
                        .and_then(|m| {
                            let card_prefix = board_obj
                                .get("card_prefix")
                                .and_then(|v| v.as_str())
                                .unwrap_or("task");
                            // Use counter for board.card_prefix if available
                            m.get(card_prefix)
                                .and_then(|v| v.as_u64())
                                .or_else(|| {
                                    // Fall back to max counter
                                    m.values()
                                        .filter_map(|v| v.as_u64())
                                        .max()
                                })
                        });

                    from_prefix_counters
                        .or_else(|| {
                            board_card_max
                                .get(&board_id)
                                .map(|&max| (max + 1) as u64)
                        })
                        .unwrap_or(1)
                };

                board_obj.remove("prefix_counters");
                board_obj.insert(
                    "card_counter".to_string(),
                    Value::Number(card_counter.into()),
                );
            }
        }
    }

    // Strip assigned_prefix and card_prefix from all cards (active + archived)
    strip_card_prefix_fields(data, "cards");
    strip_card_prefix_fields_archived(data, "archived_cards");

    // Bump version to 3
    envelope["version"] = Value::Number(3.into());

    // Write atomically via temp file + rename
    let json_str = serde_json::to_string_pretty(&envelope)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

    let tmp_path = path.with_extension("tmp");
    tokio::fs::write(&tmp_path, json_str.as_bytes()).await?;
    tokio::fs::rename(&tmp_path, path).await?;

    tracing::info!("Migrated {} from V2 to V3 format", path.display());
    Ok(())
}

fn strip_card_prefix_fields(data: &mut Value, key: &str) {
    if let Some(cards) = data.get_mut(key).and_then(|v| v.as_array_mut()) {
        for card in cards.iter_mut() {
            if let Some(obj) = card.as_object_mut() {
                obj.remove("assigned_prefix");
                obj.remove("card_prefix");
            }
        }
    }
}

fn strip_card_prefix_fields_archived(data: &mut Value, key: &str) {
    if let Some(archived) = data.get_mut(key).and_then(|v| v.as_array_mut()) {
        for archived_card in archived.iter_mut() {
            if let Some(card) = archived_card.get_mut("card").and_then(|v| v.as_object_mut()) {
                card.remove("assigned_prefix");
                card.remove("card_prefix");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_persistence::FormatVersion;
    use serde_json::json;
    use tempfile::tempdir;

    fn make_v2_envelope(data: Value) -> Value {
        json!({
            "version": 2,
            "metadata": {
                "instance_id": "00000000-0000-0000-0000-000000000001",
                "saved_at": "2024-01-01T00:00:00Z"
            },
            "data": data
        })
    }

    #[tokio::test]
    async fn test_migrate_v2_to_v3_converts_prefix_counters_to_card_counter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        let board_id = "board-1";
        let col_id = "col-1";
        let data = json!({
            "boards": [{
                "id": board_id,
                "name": "Test",
                "card_prefix": "TASK",
                "prefix_counters": { "TASK": 5, "OTHER": 2 },
                "sprint_counters": {}
            }],
            "columns": [{ "id": col_id, "board_id": board_id }],
            "cards": [],
            "archived_cards": [],
            "sprints": []
        });
        let envelope = make_v2_envelope(data);
        tokio::fs::write(&path, serde_json::to_string_pretty(&envelope).unwrap())
            .await
            .unwrap();

        migrate_v2_to_v3(&path).await.unwrap();

        let migrated: Value =
            serde_json::from_str(&tokio::fs::read_to_string(&path).await.unwrap()).unwrap();
        assert_eq!(migrated["version"], 3);
        let board = &migrated["data"]["boards"][0];
        assert_eq!(board["card_counter"], 5);
        assert!(board.get("prefix_counters").is_none());
    }

    #[tokio::test]
    async fn test_migrate_v2_to_v3_drops_assigned_prefix_and_card_prefix_from_cards() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        let board_id = "board-1";
        let col_id = "col-1";
        let data = json!({
            "boards": [{
                "id": board_id,
                "name": "Test",
                "card_prefix": null,
                "prefix_counters": {},
                "sprint_counters": {}
            }],
            "columns": [{ "id": col_id, "board_id": board_id }],
            "cards": [{
                "id": "card-1",
                "column_id": col_id,
                "card_number": 1,
                "assigned_prefix": "task",
                "card_prefix": null
            }],
            "archived_cards": [],
            "sprints": []
        });
        let envelope = make_v2_envelope(data);
        tokio::fs::write(&path, serde_json::to_string_pretty(&envelope).unwrap())
            .await
            .unwrap();

        migrate_v2_to_v3(&path).await.unwrap();

        let migrated: Value =
            serde_json::from_str(&tokio::fs::read_to_string(&path).await.unwrap()).unwrap();
        let card = &migrated["data"]["cards"][0];
        assert!(card.get("assigned_prefix").is_none());
        assert!(card.get("card_prefix").is_none());
    }

    #[tokio::test]
    async fn test_migrate_v2_to_v3_drops_assigned_prefix_from_archived_card_cards() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        let board_id = "board-1";
        let col_id = "col-1";
        let data = json!({
            "boards": [{
                "id": board_id,
                "name": "Test",
                "card_prefix": null,
                "prefix_counters": {},
                "sprint_counters": {}
            }],
            "columns": [{ "id": col_id, "board_id": board_id }],
            "cards": [],
            "archived_cards": [{
                "card": {
                    "id": "card-2",
                    "column_id": col_id,
                    "card_number": 2,
                    "assigned_prefix": "task",
                    "card_prefix": null
                },
                "archived_at": "2024-01-01T00:00:00Z",
                "original_column_id": col_id,
                "original_position": 0
            }],
            "sprints": []
        });
        let envelope = make_v2_envelope(data);
        tokio::fs::write(&path, serde_json::to_string_pretty(&envelope).unwrap())
            .await
            .unwrap();

        migrate_v2_to_v3(&path).await.unwrap();

        let migrated: Value =
            serde_json::from_str(&tokio::fs::read_to_string(&path).await.unwrap()).unwrap();
        let card = &migrated["data"]["archived_cards"][0]["card"];
        assert!(card.get("assigned_prefix").is_none());
        assert!(card.get("card_prefix").is_none());
    }

    #[tokio::test]
    async fn test_migrate_v2_to_v3_collision_aborts_and_leaves_file_unmodified() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        let board_id = "board-1";
        let col_id = "col-1";
        let data = json!({
            "boards": [{
                "id": board_id,
                "name": "Test",
                "card_prefix": "TASK",
                "prefix_counters": { "TASK": 3, "FEAT": 3 },
                "sprint_counters": {}
            }],
            "columns": [{ "id": col_id, "board_id": board_id }],
            "cards": [
                {
                    "id": "card-1",
                    "column_id": col_id,
                    "card_number": 1,
                    "assigned_prefix": "TASK"
                },
                {
                    "id": "card-2",
                    "column_id": col_id,
                    "card_number": 1,
                    "assigned_prefix": "FEAT"
                }
            ],
            "archived_cards": [],
            "sprints": []
        });
        let envelope = make_v2_envelope(data);
        let original_content = serde_json::to_string_pretty(&envelope).unwrap();
        tokio::fs::write(&path, &original_content).await.unwrap();

        let result = migrate_v2_to_v3(&path).await;
        assert!(result.is_err(), "collision should abort migration");

        let after = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(after, original_content, "file should be unmodified on abort");
    }

    #[tokio::test]
    async fn test_detect_v3_format_returns_v3() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        let v3_data = json!({
            "version": 3,
            "metadata": {},
            "data": {}
        });
        tokio::fs::write(&path, v3_data.to_string()).await.unwrap();

        let version = crate::migration::Migrator::detect_version(&path)
            .await
            .unwrap();
        assert_eq!(version, FormatVersion::V3);
    }
}
