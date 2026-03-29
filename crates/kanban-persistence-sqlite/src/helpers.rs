use kanban_persistence::{PersistenceError, PersistenceResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Table {
    Boards,
    Columns,
    Cards,
    ArchivedCards,
    Sprints,
}

impl Table {
    pub(crate) const fn table_name(self) -> &'static str {
        match self {
            Table::Boards => "boards",
            Table::Columns => "columns",
            Table::Cards => "cards",
            Table::ArchivedCards => "archived_cards",
            Table::Sprints => "sprints",
        }
    }

    pub(crate) const fn id_column(self) -> &'static str {
        match self {
            Table::ArchivedCards => "card_id",
            _ => "id",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SnapshotData {
    pub boards: Vec<serde_json::Value>,
    pub columns: Vec<serde_json::Value>,
    pub cards: Vec<serde_json::Value>,
    pub archived_cards: Vec<serde_json::Value>,
    pub sprints: Vec<serde_json::Value>,
    #[serde(default)]
    pub graph: serde_json::Value,
}

pub(crate) fn db_err(e: sqlx::Error) -> PersistenceError {
    PersistenceError::Database(e.to_string())
}

pub(crate) fn ser_err(e: impl std::fmt::Display) -> PersistenceError {
    PersistenceError::Serialization(e.to_string())
}

pub(crate) fn required_str<'a>(
    val: &'a serde_json::Value,
    field: &str,
) -> PersistenceResult<&'a str> {
    val[field]
        .as_str()
        .ok_or_else(|| ser_err(format!("missing required field: {field}")))
}

pub(crate) fn parse_uuid(s: &str) -> PersistenceResult<Uuid> {
    Uuid::parse_str(s).map_err(|e| PersistenceError::Serialization(e.to_string()))
}

pub(crate) fn parse_enum<T: serde::de::DeserializeOwned>(
    s: &str,
    label: &str,
) -> PersistenceResult<T> {
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .map_err(|_| ser_err(format!("unknown {label} variant: {s}")))
}

pub(crate) fn parse_datetime(s: &str) -> PersistenceResult<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))
        .map(|dt| dt.with_timezone(&chrono::Utc))
}
