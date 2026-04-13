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
/// When the same card_number appears under multiple `assigned_prefix` values
/// within a board (possible when a board prefix was changed after cards were
/// created), the conflicting cards are renumbered above the current maximum
/// rather than aborting. Cards whose `assigned_prefix` matches the board's
/// canonical `card_prefix` always keep their original number.
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

    // Collect the canonical prefix for each board (board.card_prefix or "task")
    let mut board_canonical_prefix: HashMap<String, String> = HashMap::new();
    if let Some(boards) = data.get("boards").and_then(|v| v.as_array()) {
        for board in boards {
            if let Some(board_id) = board.get("id").and_then(|v| v.as_str()) {
                let prefix = board
                    .get("card_prefix")
                    .and_then(|v| v.as_str())
                    .unwrap_or("task")
                    .to_string();
                board_canonical_prefix.insert(board_id.to_string(), prefix);
            }
        }
    }

    // Collect all cards (active + archived) for each board:
    //   board_id → Vec<(card_id, card_number, assigned_prefix)>
    let mut board_cards: HashMap<String, Vec<(String, u32, String)>> = HashMap::new();
    for (source, is_archived) in &[("cards", false), ("archived_cards", true)] {
        if let Some(cards) = data.get(*source).and_then(|v| v.as_array()) {
            for card_val in cards {
                let card = if *is_archived {
                    card_val.get("card").unwrap_or(card_val)
                } else {
                    card_val
                };
                let col_id = card.get("column_id").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(board_id) = column_to_board.get(col_id) {
                    let card_id = card
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let card_number = card
                        .get("card_number")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    let prefix = card
                        .get("assigned_prefix")
                        .and_then(|v| v.as_str())
                        .unwrap_or("task")
                        .to_string();
                    board_cards
                        .entry(board_id.clone())
                        .or_default()
                        .push((card_id, card_number, prefix));
                }
            }
        }
    }

    // Detect collisions and build a renumber map for non-canonical cards.
    // renumber_map: card_id → new_card_number
    let mut renumber_map: HashMap<String, u32> = HashMap::new();
    // board_max_number: board_id → highest card_number (including any renumbered cards)
    let mut board_max_number: HashMap<String, u32> = HashMap::new();

    for (board_id, cards) in &board_cards {
        let canonical = board_canonical_prefix
            .get(board_id)
            .map(|s| s.as_str())
            .unwrap_or("task");

        let mut running_max: u32 = cards.iter().map(|(_, n, _)| *n).max().unwrap_or(0);

        // Group cards by card_number to find which numbers are contested
        let mut number_to_entries: HashMap<u32, Vec<(&str, &str)>> = HashMap::new();
        for (card_id, card_number, prefix) in cards {
            number_to_entries
                .entry(*card_number)
                .or_default()
                .push((card_id.as_str(), prefix.as_str()));
        }

        for (_, entries) in &number_to_entries {
            let prefixes: HashSet<&str> = entries.iter().map(|(_, p)| *p).collect();
            if prefixes.len() > 1 {
                // Collision: renumber every card whose prefix differs from canonical
                for (card_id, prefix) in entries {
                    if *prefix != canonical {
                        running_max += 1;
                        tracing::warn!(
                            "Migration: renumbering card {} (prefix='{}') to {} \
                             to resolve collision on board {}",
                            card_id,
                            prefix,
                            running_max,
                            board_id
                        );
                        renumber_map.insert(card_id.to_string(), running_max);
                    }
                }
            }
        }

        board_max_number.insert(board_id.clone(), running_max);
    }

    // Apply renumbering to active cards
    if let Some(cards) = data.get_mut("cards").and_then(|v| v.as_array_mut()) {
        for card in cards.iter_mut() {
            if let Some(obj) = card.as_object_mut() {
                if let Some(card_id) = obj.get("id").and_then(|v| v.as_str()).map(str::to_string) {
                    if let Some(&new_number) = renumber_map.get(&card_id) {
                        obj.insert("card_number".to_string(), Value::Number(new_number.into()));
                    }
                }
            }
        }
    }

    // Apply renumbering to archived cards
    if let Some(archived) = data.get_mut("archived_cards").and_then(|v| v.as_array_mut()) {
        for archived_card in archived.iter_mut() {
            if let Some(card) = archived_card.get_mut("card").and_then(|v| v.as_object_mut()) {
                if let Some(card_id) = card.get("id").and_then(|v| v.as_str()).map(str::to_string) {
                    if let Some(&new_number) = renumber_map.get(&card_id) {
                        card.insert("card_number".to_string(), Value::Number(new_number.into()));
                    }
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

                let canonical = board_canonical_prefix
                    .get(&board_id)
                    .map(|s| s.as_str())
                    .unwrap_or("task");

                // card_counter = max(prefix_counters[canonical], board_max + 1, 1)
                let from_prefix_counters = board_obj
                    .get("prefix_counters")
                    .and_then(|v| v.as_object())
                    .and_then(|m| m.get(canonical).and_then(|v| v.as_u64()));

                let from_max = board_max_number
                    .get(&board_id)
                    .map(|&max| (max + 1) as u64);

                let card_counter = match (from_prefix_counters, from_max) {
                    (Some(a), Some(b)) => a.max(b),
                    (Some(a), None) => a,
                    (None, Some(b)) => b,
                    (None, None) => 1,
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
    async fn test_migrate_v2_to_v3_collision_renumbers_non_canonical_cards() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        let board_id = "board-1";
        let col_id = "col-1";
        // board.card_prefix = "TASK" is canonical
        // card-1 (TASK prefix, number 1) → keeps number 1
        // card-2 (task prefix, number 1) → non-canonical, renumbered to 2
        let data = json!({
            "boards": [{
                "id": board_id,
                "name": "Test",
                "card_prefix": "TASK",
                "prefix_counters": { "TASK": 2, "task": 2 },
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
                    "assigned_prefix": "task"
                }
            ],
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

        let cards = migrated["data"]["cards"].as_array().unwrap();
        let card1 = cards.iter().find(|c| c["id"] == "card-1").unwrap();
        let card2 = cards.iter().find(|c| c["id"] == "card-2").unwrap();

        assert_eq!(card1["card_number"], 1, "canonical card keeps its number");
        assert_eq!(card2["card_number"], 2, "non-canonical card is renumbered above max");

        let board = &migrated["data"]["boards"][0];
        assert_eq!(board["card_counter"], 3, "card_counter is renumbered_max + 1");
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
