use serde::{Deserialize, Serialize};

use super::card_graph::CardDependencyGraph;

/// Top-level container for all entity dependency graphs
///
/// Currently contains only card dependencies (MVP).
/// Designed to be extended with sprint and board graphs in the future.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependencyGraph {
    /// Card dependency graph
    #[serde(default)]
    pub cards: CardDependencyGraph,
    // Future expansion:
    // #[serde(default)]
    // pub sprints: SprintDependencyGraph,
    // #[serde(default)]
    // pub boards: BoardDependencyGraph,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph_creation() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.cards.edge_count(), 0);
    }

    #[test]
    fn test_dependency_graph_serialization() {
        let graph = DependencyGraph::new();
        let json = serde_json::to_string(&graph).unwrap();
        let deserialized: DependencyGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cards.edge_count(), 0);
    }
}
