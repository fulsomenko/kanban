use crate::{Board, Card};
use kanban_core::Editable;
use serde::{Deserialize, Serialize};

// Case-insensitive enum parsers that normalize to canonical format
fn parse_card_priority_case_insensitive(s: &str) -> Option<String> {
    match s.to_lowercase().as_str() {
        "low" => Some("Low".to_string()),
        "medium" => Some("Medium".to_string()),
        "high" => Some("High".to_string()),
        "critical" => Some("Critical".to_string()),
        _ => None,
    }
}

fn parse_card_status_case_insensitive(s: &str) -> Option<String> {
    match s.to_lowercase().replace('_', "").as_str() {
        "todo" => Some("Todo".to_string()),
        "inprogress" => Some("InProgress".to_string()),
        "blocked" => Some("Blocked".to_string()),
        "done" => Some("Done".to_string()),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSettingsDto {
    #[serde(alias = "branch_prefix")]
    pub sprint_prefix: Option<String>,
    pub card_prefix: Option<String>,
    pub sprint_duration_days: Option<u32>,
    pub sprint_names: Vec<String>,
    #[serde(default)]
    pub completion_column_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardMetadataDto {
    pub priority: String,
    pub status: String,
    pub points: Option<u8>,
    pub due_date: Option<String>,
}

impl Editable<Board> for BoardSettingsDto {
    fn from_entity(board: &Board) -> Self {
        Self {
            sprint_prefix: board.sprint_prefix.clone(),
            card_prefix: board.card_prefix.clone(),
            sprint_duration_days: board.sprint_duration_days,
            sprint_names: board.sprint_names.clone(),
            completion_column_id: board.completion_column_id,
        }
    }

    fn apply_to(self, board: &mut Board) {
        board.sprint_prefix = self.sprint_prefix;
        board.card_prefix = self.card_prefix;
        board.sprint_duration_days = self.sprint_duration_days;
        board.sprint_names = self.sprint_names;
        board.completion_column_id = self.completion_column_id;
        board.updated_at = chrono::Utc::now();
    }
}

impl Editable<Card> for CardMetadataDto {
    fn from_entity(card: &Card) -> Self {
        Self {
            priority: format!("{:?}", card.priority),
            status: format!("{:?}", card.status),
            points: card.points,
            due_date: card.due_date.map(|dt| dt.to_rfc3339()),
        }
    }

    fn apply_to(self, card: &mut Card) {
        if let Some(canonical_priority) = parse_card_priority_case_insensitive(&self.priority) {
            if let Ok(priority) = serde_json::from_value::<crate::CardPriority>(
                serde_json::Value::String(canonical_priority),
            ) {
                card.priority = priority;
            }
        }

        if let Some(canonical_status) = parse_card_status_case_insensitive(&self.status) {
            if let Ok(status) = serde_json::from_value::<crate::CardStatus>(
                serde_json::Value::String(canonical_status),
            ) {
                card.status = status;
            }
        }

        if let Some(points) = self.points {
            card.points = Some(points);
        }

        if let Some(due_date_str) = self.due_date {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&due_date_str) {
                card.due_date = Some(dt.with_timezone(&chrono::Utc));
            }
        }

        card.updated_at = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_old_branch_prefix_format() {
        // Test that old JSON with branch_prefix field is correctly deserialized
        let old_format = r#"{
            "branch_prefix": "FEAT",
            "sprint_duration_days": null,
            "sprint_names": []
        }"#;

        let settings: BoardSettingsDto = serde_json::from_str(old_format)
            .expect("Failed to deserialize BoardSettingsDto with old branch_prefix field");

        // Via the serde alias, branch_prefix should map to sprint_prefix
        assert_eq!(settings.sprint_prefix, Some("FEAT".to_string()));
        // card_prefix should be None since it wasn't in the old format
        assert_eq!(settings.card_prefix, None);
    }

    #[test]
    fn test_deserialize_new_separate_prefixes() {
        // Test that new JSON with both sprint_prefix and card_prefix works
        let new_format = r#"{
            "sprint_prefix": "SPRINT",
            "card_prefix": "TASK",
            "sprint_duration_days": 14,
            "sprint_names": []
        }"#;

        let settings: BoardSettingsDto = serde_json::from_str(new_format)
            .expect("Failed to deserialize BoardSettingsDto with separate prefixes");

        assert_eq!(settings.sprint_prefix, Some("SPRINT".to_string()));
        assert_eq!(settings.card_prefix, Some("TASK".to_string()));
        assert_eq!(settings.sprint_duration_days, Some(14));
    }

    #[test]
    fn test_sprint_deserialization_without_card_prefix() {
        // Test that old Sprint JSON without card_prefix field deserializes correctly
        let old_sprint_json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "board_id": "550e8400-e29b-41d4-a716-446655440001",
            "sprint_number": 1,
            "name_index": null,
            "prefix": null,
            "status": "Planning",
            "start_date": null,
            "end_date": null,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let sprint: crate::Sprint = serde_json::from_str(old_sprint_json)
            .expect("Failed to deserialize Sprint without card_prefix field");

        // card_prefix should default to None via #[serde(default)]
        assert_eq!(sprint.card_prefix, None);
        assert_eq!(sprint.sprint_number, 1);
    }

    #[test]
    fn test_card_deserialization_without_card_prefix() {
        // Test that old Card JSON without card_prefix field deserializes correctly
        let old_card_json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "column_id": "550e8400-e29b-41d4-a716-446655440002",
            "title": "Test Card",
            "description": null,
            "priority": "Medium",
            "status": "Todo",
            "position": 0,
            "due_date": null,
            "points": null,
            "card_number": 1,
            "sprint_id": null,
            "assigned_prefix": "task",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "completed_at": null,
            "sprint_logs": []
        }"#;

        let card: Card = serde_json::from_str(old_card_json)
            .expect("Failed to deserialize Card without card_prefix field");

        // card_prefix should default to None via #[serde(default)]
        assert_eq!(card.card_prefix, None);
        assert_eq!(card.title, "Test Card");
        assert_eq!(card.card_number, 1);
    }
}
