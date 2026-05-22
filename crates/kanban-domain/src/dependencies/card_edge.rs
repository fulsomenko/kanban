use serde::{Deserialize, Serialize};

/// Types of relationships between cards
///
/// `Default` is `ParentOf`, the primary user-facing edge kind. The
/// `#[default]` attribute is required because [`Edge<E>`]'s derived
/// `Deserialize` carries an `E: Default` bound (its `edge_type` field
/// is `#[serde(default)]`). On the V6 disk format edges are
/// `Edge<()>` and the kind comes from the sub-graph they live in, so
/// the variant choice here never affects production deserialisation.
/// It only fires for test helpers that construct `Edge<CardEdgeType>`
/// directly and round-trip it through serde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CardEdgeType {
    /// This card blocks the target (must complete before target can start)
    /// Enforces DAG - no cycles allowed
    Blocks,

    /// General relationship (informational, allows cycles)
    RelatesTo,

    /// Organizational grouping - parent contains child (source is parent, target is child)
    /// Enforces DAG - no cycles allowed (can't be own ancestor)
    #[default]
    ParentOf,
    // Future edge types can be added here:
    // Duplicates,
}

impl CardEdgeType {
    /// Whether this edge type enforces DAG (no cycles)
    pub fn requires_dag(&self) -> bool {
        matches!(self, CardEdgeType::Blocks | CardEdgeType::ParentOf)
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

    #[test]
    fn test_parent_of_requires_dag() {
        assert!(CardEdgeType::ParentOf.requires_dag());
        assert!(!CardEdgeType::ParentOf.allows_cycles());
    }
}
