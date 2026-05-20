use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::core::EdgeStore;
use super::edge::{Edge, EdgeDirection};
use super::error::GraphError;
use super::traits::{Graph, SubGraph, Undirected};

/// Undirected graph keyed by `Uuid` node identifiers.
///
/// Rejects self-references; cycles are permitted (the directed concept
/// does not apply). `outgoing` and `incoming` return the same neighbour
/// set. Wraps an [`EdgeStore`] for archive parity with [`super::DagGraph`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UndirectedGraph {
    #[serde(flatten)]
    store: EdgeStore<()>,
}

impl UndirectedGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn archive_node(&mut self, node: Uuid) {
        self.store.archive_node(node);
    }

    pub fn unarchive_node(&mut self, node: Uuid) {
        self.store.unarchive_node(node);
    }

    pub fn remove_node(&mut self, node: Uuid) {
        self.store.remove_node(node);
    }

    pub fn edge_count(&self) -> usize {
        self.store.edge_count()
    }

    pub fn active_edge_count(&self) -> usize {
        self.store.active_edge_count()
    }

    /// Borrow the raw underlying edge list (active + archived).
    pub fn edges(&self) -> &[Edge<()>] {
        self.store.edges()
    }

    /// True if a (possibly archived) edge between `a` and `b` exists.
    pub fn has_edge(&self, a: Uuid, b: Uuid) -> bool {
        self.store.edges().iter().any(|e| e.connects(a, b))
    }

    /// Insert a raw edge without validation. Use only for migrations
    /// and test fixtures.
    pub fn insert_raw_edge(&mut self, edge: Edge<()>) {
        self.store.add_edge(edge);
    }
}

impl SubGraph for UndirectedGraph {
    fn archive_node(&mut self, node: Uuid) {
        UndirectedGraph::archive_node(self, node);
    }
    fn unarchive_node(&mut self, node: Uuid) {
        UndirectedGraph::unarchive_node(self, node);
    }
    fn remove_node(&mut self, node: Uuid) {
        UndirectedGraph::remove_node(self, node);
    }
    fn edge_count(&self) -> usize {
        UndirectedGraph::edge_count(self)
    }
    fn active_edge_count(&self) -> usize {
        UndirectedGraph::active_edge_count(self)
    }
    fn has_edge(&self, a: Uuid, b: Uuid) -> bool {
        UndirectedGraph::has_edge(self, a, b)
    }
}

impl Graph for UndirectedGraph {
    type NodeId = Uuid;

    fn add_edge(&mut self, from: Uuid, to: Uuid) -> Result<(), GraphError> {
        if from == to {
            return Err(GraphError::SelfReference);
        }
        self.store
            .add_edge(Edge::new(from, to, (), EdgeDirection::Bidirectional));
        Ok(())
    }

    fn remove_edge(&mut self, from: Uuid, to: Uuid) -> Result<(), GraphError> {
        if self.store.remove_edge(from, to) {
            Ok(())
        } else {
            Err(GraphError::EdgeNotFound)
        }
    }

    fn contains_edge(&self, from: Uuid, to: Uuid) -> bool {
        self.store
            .active_edges()
            .iter()
            .any(|e| e.connects(from, to))
    }
}

impl Undirected for UndirectedGraph {
    /// Neighbours of `node` from any active edge (either endpoint).
    /// The `Undirected` trait is the only access path — callers must
    /// bring it into scope. Choosing this over an inherent method
    /// makes the trait load-bearing rather than decorative.
    fn neighbors(&self, node: Uuid) -> Vec<Uuid> {
        let mut out = Vec::new();
        for edge in self.store.active_edges() {
            if edge.source == node {
                out.push(edge.target);
            } else if edge.target == node {
                out.push(edge.source);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (Uuid, Uuid, Uuid) {
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
    }

    #[test]
    fn test_new_undirected_is_empty() {
        let g = UndirectedGraph::new();
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn test_add_edge_creates_bidirectional_neighbours() {
        let (a, b, _) = ids();
        let mut g = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        // Undirected vocabulary: both endpoints see the other as a
        // neighbour. There is no outgoing/incoming distinction to ask
        // for — the type system enforces this.
        assert_eq!(g.neighbors(a), vec![b]);
        assert_eq!(g.neighbors(b), vec![a]);
    }

    #[test]
    fn test_add_edge_self_reference_returns_error() {
        let (a, _, _) = ids();
        let mut g = UndirectedGraph::new();
        assert_eq!(g.add_edge(a, a), Err(GraphError::SelfReference));
    }

    #[test]
    fn test_add_edge_permits_cycle() {
        let (a, b, c) = ids();
        let mut g = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        g.add_edge(b, c).unwrap();
        assert_eq!(g.add_edge(c, a), Ok(()));
    }

    #[test]
    fn test_remove_edge_existing_succeeds() {
        let (a, b, _) = ids();
        let mut g = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        assert_eq!(g.remove_edge(a, b), Ok(()));
        assert!(g.neighbors(a).is_empty());
    }

    #[test]
    fn test_remove_edge_works_in_either_direction() {
        let (a, b, _) = ids();
        let mut g = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        assert_eq!(g.remove_edge(b, a), Ok(()));
        assert!(g.neighbors(a).is_empty());
    }

    #[test]
    fn test_remove_edge_missing_returns_edge_not_found() {
        let (a, b, _) = ids();
        let mut g = UndirectedGraph::new();
        assert_eq!(g.remove_edge(a, b), Err(GraphError::EdgeNotFound));
    }

    #[test]
    fn test_contains_edge_is_symmetric() {
        let (a, b, _) = ids();
        let mut g = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        assert!(g.contains_edge(a, b));
        assert!(g.contains_edge(b, a));
    }

    #[test]
    fn test_archive_node_removes_from_neighbours_view() {
        let (a, b, _) = ids();
        let mut g = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        g.archive_node(a);
        assert!(g.neighbors(b).is_empty());
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.active_edge_count(), 0);
    }

    #[test]
    fn test_unarchive_node_restores_neighbours_view() {
        let (a, b, _) = ids();
        let mut g = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        g.archive_node(a);
        g.unarchive_node(a);
        assert_eq!(g.neighbors(b), vec![a]);
    }

    #[test]
    fn test_remove_node_deletes_all_involved_edges() {
        let (a, b, c) = ids();
        let mut g = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        g.add_edge(b, c).unwrap();
        g.remove_node(b);
        assert_eq!(g.edge_count(), 0);
    }
}
