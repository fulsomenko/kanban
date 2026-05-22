use thiserror::Error;

/// Errors that can arise from graph mutations.
///
/// Implementations of [`super::Graph`] choose which of these can occur:
/// a DAG rejects `Cycle`, `SelfReference`, and `Duplicate`; an
/// undirected graph rejects `SelfReference` and `Duplicate` (treated
/// as duplicate in either ordering). `EdgeNotFound` is universal.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum GraphError {
    #[error("operation would create a cycle")]
    Cycle,
    #[error("self-reference is not allowed")]
    SelfReference,
    #[error("edge not found")]
    EdgeNotFound,
    /// An active edge with the same endpoints already exists.
    /// Directed graphs reject the same `source -> target` ordering;
    /// undirected graphs reject either `{a, b}` ordering. Archived
    /// edges don't count against this check, so re-adding after
    /// archive succeeds.
    #[error("an active edge with the same endpoints already exists")]
    Duplicate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_error_cycle_displays_expected_message() {
        assert_eq!(
            GraphError::Cycle.to_string(),
            "operation would create a cycle"
        );
    }

    #[test]
    fn test_graph_error_self_reference_displays_expected_message() {
        assert_eq!(
            GraphError::SelfReference.to_string(),
            "self-reference is not allowed"
        );
    }

    #[test]
    fn test_graph_error_edge_not_found_displays_expected_message() {
        assert_eq!(GraphError::EdgeNotFound.to_string(), "edge not found");
    }

    #[test]
    fn test_graph_error_supports_equality() {
        assert_eq!(GraphError::Cycle, GraphError::Cycle);
        assert_ne!(GraphError::Cycle, GraphError::SelfReference);
    }
}
