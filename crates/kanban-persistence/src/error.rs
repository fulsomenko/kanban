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

    #[error("unsupported operation: {0}")]
    Unsupported(String),

    #[error(
        "file format v{file_version} is newer than this binary's max v{binary_max}; \
         please upgrade kanban"
    )]
    UnsupportedFutureVersion { file_version: u32, binary_max: u32 },
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
                    "No backend registered for {:?}. Available backends: {}",
                    locator,
                    if supported.is_empty() {
                        "none".to_string()
                    } else {
                        supported.join(", ")
                    }
                ))
            }
            PersistenceError::ConflictDetected { path, source } => {
                kanban_domain::KanbanError::ConflictDetected { path, source }
            }
            PersistenceError::Unsupported(s) => kanban_domain::KanbanError::Internal(s),
            PersistenceError::UnsupportedFutureVersion {
                file_version,
                binary_max,
            } => kanban_domain::KanbanError::UnsupportedFutureVersion {
                file_version,
                binary_max,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_domain::KanbanError;

    #[test]
    fn test_unsupported_future_version_display_mentions_both_versions() {
        let err = PersistenceError::UnsupportedFutureVersion {
            file_version: 99,
            binary_max: 6,
        };
        let msg = err.to_string();
        assert!(msg.contains("99"), "msg: {msg}");
        assert!(msg.contains('6'), "msg: {msg}");
    }

    #[test]
    fn test_persistence_unsupported_future_version_maps_to_kanban_error() {
        let pe = PersistenceError::UnsupportedFutureVersion {
            file_version: 99,
            binary_max: 6,
        };
        let ke: KanbanError = pe.into();
        assert!(
            matches!(
                ke,
                KanbanError::UnsupportedFutureVersion {
                    file_version: 99,
                    binary_max: 6
                }
            ),
            "expected UnsupportedFutureVersion variant"
        );
    }
}
