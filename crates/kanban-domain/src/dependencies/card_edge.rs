use serde::{Deserialize, Serialize};

/// Types of relationships between cards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardEdgeType {
    /// This card blocks the target (must complete before target can start)
    /// Enforces DAG - no cycles allowed
    Blocks,

    /// General relationship (informational, allows cycles)
    RelatesTo,
    // Future edge types can be added here:
    // Duplicates,
    // ParentOf,
    // ChildOf,
}

impl CardEdgeType {
    /// Whether this edge type enforces DAG (no cycles)
    pub fn requires_dag(&self) -> bool {
        matches!(self, CardEdgeType::Blocks)
    }

    /// Whether this edge type allows cycles
    pub fn allows_cycles(&self) -> bool {
        !self.requires_dag()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_requires_dag() {
        assert!(CardEdgeType::Blocks.requires_dag());
        assert!(!CardEdgeType::Blocks.allows_cycles());
    }

    #[test]
    fn test_relates_to_allows_cycles() {
        assert!(!CardEdgeType::RelatesTo.requires_dag());
        assert!(CardEdgeType::RelatesTo.allows_cycles());
    }
}
