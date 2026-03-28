use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum DependencyError {
    #[error("cycle detected: adding this edge would create a circular dependency")]
    CycleDetected,
    #[error("self-reference not allowed")]
    SelfReference,
    #[error("edge not found")]
    EdgeNotFound,
}

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("{entity} {id} not found")]
    NotFound { entity: &'static str, id: Uuid },

    #[error("validation error: {0}")]
    Validation(String),

    #[error(transparent)]
    Dependency(#[from] DependencyError),
}

impl DomainError {
    pub fn board_not_found(id: Uuid) -> Self {
        Self::NotFound { entity: "board", id }
    }
    pub fn card_not_found(id: Uuid) -> Self {
        Self::NotFound { entity: "card", id }
    }
    pub fn column_not_found(id: Uuid) -> Self {
        Self::NotFound {
            entity: "column",
            id,
        }
    }
    pub fn sprint_not_found(id: Uuid) -> Self {
        Self::NotFound {
            entity: "sprint",
            id,
        }
    }
    pub fn archived_card_not_found(id: Uuid) -> Self {
        Self::NotFound {
            entity: "archived card",
            id,
        }
    }
    pub fn tag_not_found(id: Uuid) -> Self {
        Self::NotFound { entity: "tag", id }
    }
}

#[derive(Error, Debug)]
pub enum KanbanError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("file conflict: {path} was modified by another instance")]
    ConflictDetected {
        path: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("internal error: {0}")]
    Internal(String),
}

pub type KanbanResult<T> = Result<T, KanbanError>;

impl KanbanError {
    pub fn not_found(entity: &'static str, id: Uuid) -> Self {
        Self::Domain(DomainError::NotFound { entity, id })
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Domain(DomainError::Validation(msg.into()))
    }
}

impl From<DependencyError> for KanbanError {
    fn from(e: DependencyError) -> Self {
        KanbanError::Domain(DomainError::Dependency(e))
    }
}

impl From<kanban_core::CoreError> for KanbanError {
    fn from(e: kanban_core::CoreError) -> Self {
        match e {
            kanban_core::CoreError::Validation(msg) => KanbanError::validation(msg),
        }
    }
}
