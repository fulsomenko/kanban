use serde::{Deserialize, Serialize};

/// Types of relationships between cards.
///
/// Grammar is "source [variant] target": A `Blocks` B, A `RelatesTo`
/// B, A `Spawns` B. Verb-form throughout; the previous `ParentOf`
/// (noun-of-noun) was inconsistent with the other variants.
///
/// `Default` is `Spawns`, the primary user-facing hierarchy edge.
/// The kind lives at the sub-graph layer on disk (parent_child /
/// blocks / relates), not on the edge itself, so this default is
/// informational only — it never fires during production
/// deserialisation. Kept so `CardEdgeType::default()` resolves to a
/// sensible value if any caller (today: test helpers) needs one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CardEdgeType {
    /// This card blocks the target (must complete before target can start)
    /// Enforces DAG - no cycles allowed
    Blocks,

    /// General relationship (informational, allows cycles)
    RelatesTo,

    /// Hierarchy edge: source spawns target as a sub-item.
    /// Transitive: if A spawns B and B spawns C, then A is an
    /// ancestor of C. Enforces DAG: no cycles allowed (a card can't
    /// be its own ancestor). The user-facing API still uses
    /// parent/child language (`set_parent`, `parents`, `children`) —
    /// that's how the relationship reads from either side; this
    /// variant names the directed edge itself.
    #[default]
    Spawns,
    // Future edge types can be added here:
    // Duplicates,
}

impl CardEdgeType {
    /// Whether this edge type enforces DAG (no cycles)
    pub fn requires_dag(&self) -> bool {
        matches!(self, CardEdgeType::Blocks | CardEdgeType::Spawns)
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
    fn test_spawns_requires_dag() {
        assert!(CardEdgeType::Spawns.requires_dag());
        assert!(!CardEdgeType::Spawns.allows_cycles());
    }
}
