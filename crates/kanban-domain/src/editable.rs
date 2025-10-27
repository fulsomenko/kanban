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
    pub branch_prefix: Option<String>,
    pub sprint_duration_days: Option<u32>,
    pub sprint_prefix: Option<String>,
    pub sprint_names: Vec<String>,
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
            branch_prefix: board.branch_prefix.clone(),
            sprint_duration_days: board.sprint_duration_days,
            sprint_prefix: board.sprint_prefix.clone(),
            sprint_names: board.sprint_names.clone(),
        }
    }

    fn apply_to(self, board: &mut Board) {
        board.branch_prefix = self.branch_prefix;
        board.sprint_duration_days = self.sprint_duration_days;
        board.sprint_prefix = self.sprint_prefix;
        board.sprint_names = self.sprint_names;
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
