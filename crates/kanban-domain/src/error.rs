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

    #[error("column {column_id} has reached its WIP limit of {limit}")]
    WipLimitExceeded { column_id: Uuid, limit: u32 },
}

impl DomainError {
    pub fn board_not_found(id: Uuid) -> Self {
        Self::NotFound {
            entity: "board",
            id,
        }
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
    pub fn wip_limit_exceeded(column_id: Uuid, limit: u32) -> Self {
        Self::WipLimitExceeded { column_id, limit }
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

    #[error("database error: {0}")]
    Database(String),

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

    pub fn is_not_found(&self) -> bool {
        matches!(self, KanbanError::Domain(DomainError::NotFound { .. }))
    }

    pub fn is_validation(&self) -> bool {
        matches!(self, KanbanError::Domain(DomainError::Validation(_)))
    }

    pub fn is_cycle_detected(&self) -> bool {
        matches!(
            self,
            KanbanError::Domain(DomainError::Dependency(DependencyError::CycleDetected))
        )
    }

    pub fn is_self_reference(&self) -> bool {
        matches!(
            self,
            KanbanError::Domain(DomainError::Dependency(DependencyError::SelfReference))
        )
    }

    pub fn is_edge_not_found(&self) -> bool {
        matches!(
            self,
            KanbanError::Domain(DomainError::Dependency(DependencyError::EdgeNotFound))
        )
    }

    pub fn is_conflict_detected(&self) -> bool {
        matches!(self, KanbanError::ConflictDetected { .. })
    }

    pub fn is_wip_limit_exceeded(&self) -> bool {
        matches!(
            self,
            KanbanError::Domain(DomainError::WipLimitExceeded { .. })
        )
    }

    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
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
            kanban_core::CoreError::Config(msg) => KanbanError::Internal(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_is_not_found_returns_true_for_card_not_found() {
        let err = KanbanError::not_found("card", Uuid::new_v4());
        assert!(err.is_not_found());
    }

    #[test]
    fn test_is_not_found_returns_false_for_validation_error() {
        let err = KanbanError::validation("bad input");
        assert!(!err.is_not_found());
    }

    #[test]
    fn test_is_validation_returns_true_for_validation_error() {
        let err = KanbanError::validation("bad input");
        assert!(err.is_validation());
    }

    #[test]
    fn test_is_cycle_detected_returns_true() {
        let err = KanbanError::from(DependencyError::CycleDetected);
        assert!(err.is_cycle_detected());
    }

    #[test]
    fn test_is_self_reference_returns_true() {
        let err = KanbanError::from(DependencyError::SelfReference);
        assert!(err.is_self_reference());
    }

    #[test]
    fn test_is_edge_not_found_returns_true() {
        let err = KanbanError::from(DependencyError::EdgeNotFound);
        assert!(err.is_edge_not_found());
    }

    #[test]
    fn test_is_self_reference_returns_false_for_other_error() {
        let err = KanbanError::not_found("card", Uuid::new_v4());
        assert!(!err.is_self_reference());
    }

    #[test]
    fn test_is_edge_not_found_returns_false_for_other_error() {
        let err = KanbanError::not_found("card", Uuid::new_v4());
        assert!(!err.is_edge_not_found());
    }

    #[test]
    fn test_is_conflict_detected_returns_true() {
        let err = KanbanError::ConflictDetected {
            path: "test.json".to_string(),
            source: None,
        };
        assert!(err.is_conflict_detected());
    }

    #[test]
    fn test_is_wip_limit_exceeded_returns_true() {
        let id = Uuid::new_v4();
        let err = KanbanError::Domain(DomainError::wip_limit_exceeded(id, 3));
        assert!(err.is_wip_limit_exceeded());
    }

    #[test]
    fn test_not_found_display_includes_entity_and_id() {
        let id = Uuid::new_v4();
        let err = KanbanError::not_found("card", id);
        let msg = err.to_string();
        assert!(msg.contains("card"));
        assert!(msg.contains(&id.to_string()));
    }

    #[test]
    fn test_from_dependency_error_converts_to_kanban_domain() {
        let dep_err = DependencyError::CycleDetected;
        let kanban_err = KanbanError::from(dep_err);
        assert!(matches!(
            kanban_err,
            KanbanError::Domain(DomainError::Dependency(DependencyError::CycleDetected))
        ));
    }

    #[test]
    fn test_from_core_error_validation_converts_to_kanban_validation() {
        let core_err = kanban_core::CoreError::Validation("bad".to_string());
        let kanban_err = KanbanError::from(core_err);
        assert!(kanban_err.is_validation());
    }

    #[test]
    fn test_from_core_error_config_converts_to_internal() {
        let core_err = kanban_core::CoreError::Config("cfg error".to_string());
        let kanban_err = KanbanError::from(core_err);
        assert!(matches!(kanban_err, KanbanError::Internal(_)));
    }
}
