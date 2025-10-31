use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintLog {
    pub sprint_id: Uuid,
    pub sprint_number: u32,
    pub sprint_name: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub status: String,
}

impl SprintLog {
    pub fn new(
        sprint_id: Uuid,
        sprint_number: u32,
        sprint_name: Option<String>,
        status: String,
    ) -> Self {
        Self {
            sprint_id,
            sprint_number,
            sprint_name,
            started_at: Utc::now(),
            ended_at: None,
            status,
        }
    }

    pub fn end_sprint(&mut self) {
        self.ended_at = Some(Utc::now());
    }
}
