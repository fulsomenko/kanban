use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        sprint_name: Option<impl Into<String>>,
        status: impl Into<String>,
    ) -> Self {
        Self {
            sprint_id,
            sprint_number,
            sprint_name: sprint_name.map(Into::into),
            started_at: Utc::now(),
            ended_at: None,
            status: status.into(),
        }
    }

    pub fn end_sprint(&mut self) {
        self.ended_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprint_log_new_accepts_str_args_without_to_string() {
        let sprint_id = uuid::Uuid::new_v4();
        let log = SprintLog::new(sprint_id, 1, Some("Sprint 1"), "Active");
        assert_eq!(log.sprint_name, Some("Sprint 1".to_string()));
        assert_eq!(log.status, "Active");
    }
}
