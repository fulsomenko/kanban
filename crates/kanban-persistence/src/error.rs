use thiserror::Error;

#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("unsupported storage locator {locator:?}; supported: {}", supported.join(", "))]
    UnsupportedLocator {
        locator: String,
        supported: Vec<String>,
    },

    #[error("file conflict: {path} was modified by another instance")]
    ConflictDetected {
        path: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

pub type PersistenceResult<T> = Result<T, PersistenceError>;

impl From<PersistenceError> for kanban_domain::KanbanError {
    fn from(e: PersistenceError) -> Self {
        match e {
            PersistenceError::Io(io) => kanban_domain::KanbanError::Io(io),
            PersistenceError::Serialization(s) => kanban_domain::KanbanError::Serialization(s),
            PersistenceError::Database(s) => kanban_domain::KanbanError::Database(s),
            PersistenceError::UnsupportedLocator { locator, supported } => {
                kanban_domain::KanbanError::Internal(format!(
                    "unsupported storage locator {:?}; supported: {}",
                    locator,
                    supported.join(", ")
                ))
            }
            PersistenceError::ConflictDetected { path, source } => {
                kanban_domain::KanbanError::ConflictDetected { path, source }
            }
        }
    }
}
