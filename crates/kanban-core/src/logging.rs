use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub message: String,
}

impl LogEntry {
    pub fn new(message: String) -> Self {
        Self {
            timestamp: Utc::now(),
            message,
        }
    }
}

pub trait Loggable {
    fn add_log(&mut self, message: String);
    fn get_logs(&self) -> &[LogEntry];
}
