use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::algorithms::would_create_cycle;
use super::core::EdgeStore;
use super::edge::{Edge, EdgeDirection};
use super::error::GraphError;
use super::traits::{Graph, SubGraph};

/// Directed acyclic graph keyed by `Uuid` node identifiers.
///
/// Rejects self-references and any edge whose insertion would create a
/// cycle in the active-edge subgraph. Wraps an [`EdgeStore`] to inherit
/// archive / unarchive semantics for soft-delete cascades.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DagGraph {
    #[serde(flatten)]
    store: EdgeStore<()>,
}

impl DagGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Archive all edges involving `node` (soft-delete cascade).
    pub fn archive_node(&mut self, node: Uuid) {
        self.store.archive_node(node);
    }

    /// Unarchive all edges involving `node`.
    pub fn unarchive_node(&mut self, node: Uuid) {
        self.store.unarchive_node(node);
    }

    /// Remove all edges involving `node` (hard-delete cascade).
    pub fn remove_node(&mut self, node: Uuid) {
        self.store.remove_node(node);
    }

    /// Count of edges including archived.
    pub fn edge_count(&self) -> usize {
        self.store.edge_count()
    }

    /// Count of active edges.
    pub fn active_edge_count(&self) -> usize {
        self.store.active_edge_count()
    }

    /// Borrow the raw underlying edge list (active + archived).
    pub fn edges(&self) -> &[Edge<()>] {
        self.store.edges()
    }

    /// True if a (possibly archived) edge `source -> target` exists.
    pub fn has_edge(&self, source: Uuid, target: Uuid) -> bool {
        self.store
            .edges()
            .iter()
            .any(|e| e.connects(source, target))
    }

    /// Insert a raw edge without DAG validation. Use only for
    /// migrations and test fixtures.
    pub fn insert_raw_edge(&mut self, edge: Edge<()>) {
        self.store.add_edge(edge);
    }

    /// Transitive successors of `node` (descendants).
    pub fn descendants(&self, node: Uuid) -> Vec<Uuid> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        let mut seen = std::collections::HashSet::new();
        seen.insert(node);
        while let Some(n) = stack.pop() {
            for next in self.outgoing(n) {
                if seen.insert(next) {
                    out.push(next);
                    stack.push(next);
                }
            }
        }
        out
    }

    /// Transitive predecessors of `node` (ancestors).
    pub fn ancestors(&self, node: Uuid) -> Vec<Uuid> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        let mut seen = std::collections::HashSet::new();
        seen.insert(node);
        while let Some(n) = stack.pop() {
            for prev in self.incoming(n) {
                if seen.insert(prev) {
                    out.push(prev);
                    stack.push(prev);
                }
            }
        }
        out
    }

    fn active_adjacency(&self) -> std::collections::HashMap<Uuid, Vec<Uuid>> {
        let mut adj: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
        for edge in self.store.active_edges() {
            adj.entry(edge.source).or_default().push(edge.target);
        }
        adj
    }
}

impl SubGraph for DagGraph {
    fn archive_node(&mut self, node: Uuid) {
        DagGraph::archive_node(self, node);
    }
    fn unarchive_node(&mut self, node: Uuid) {
        DagGraph::unarchive_node(self, node);
    }
    fn remove_node(&mut self, node: Uuid) {
        DagGraph::remove_node(self, node);
    }
    fn edge_count(&self) -> usize {
        DagGraph::edge_count(self)
    }
    fn active_edge_count(&self) -> usize {
        DagGraph::active_edge_count(self)
    }
    fn has_edge(&self, a: Uuid, b: Uuid) -> bool {
        DagGraph::has_edge(self, a, b)
    }
}

impl Graph for DagGraph {
    type NodeId = Uuid;

    fn add_edge(&mut self, from: Uuid, to: Uuid) -> Result<(), GraphError> {
        if from == to {
            return Err(GraphError::SelfReference);
        }
        let adj = self.active_adjacency();
        if would_create_cycle(&adj, from, to) {
            return Err(GraphError::Cycle);
        }
        self.store
            .add_edge(Edge::new(from, to, (), EdgeDirection::Directed));
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
            .outgoing_active(from)
            .iter()
            .any(|e| e.target == to)
    }

    fn outgoing(&self, node: Uuid) -> Vec<Uuid> {
        self.store
            .outgoing_active(node)
            .into_iter()
            .map(|e| e.target)
            .collect()
    }

    fn incoming(&self, node: Uuid) -> Vec<Uuid> {
        self.store
            .incoming_active(node)
            .into_iter()
            .map(|e| e.source)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (Uuid, Uuid, Uuid) {
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
    }

    #[test]
    fn test_new_dag_is_empty() {
        let g = DagGraph::new();
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.active_edge_count(), 0);
    }

    #[test]
    fn test_add_edge_directed_creates_outgoing_and_incoming() {
        let (a, b, _) = ids();
        let mut g = DagGraph::new();
        g.add_edge(a, b).unwrap();
        assert_eq!(g.outgoing(a), vec![b]);
        assert_eq!(g.incoming(b), vec![a]);
        assert!(g.outgoing(b).is_empty());
        assert!(g.incoming(a).is_empty());
    }

    #[test]
    fn test_add_edge_self_reference_returns_error() {
        let (a, _, _) = ids();
        let mut g = DagGraph::new();
        assert_eq!(g.add_edge(a, a), Err(GraphError::SelfReference));
    }

    #[test]
    fn test_add_edge_creating_cycle_returns_error() {
        let (a, b, c) = ids();
        let mut g = DagGraph::new();
        g.add_edge(a, b).unwrap();
        g.add_edge(b, c).unwrap();
        assert_eq!(g.add_edge(c, a), Err(GraphError::Cycle));
    }

    #[test]
    fn test_add_edge_direct_cycle_returns_error() {
        let (a, b, _) = ids();
        let mut g = DagGraph::new();
        g.add_edge(a, b).unwrap();
        assert_eq!(g.add_edge(b, a), Err(GraphError::Cycle));
    }

    #[test]
    fn test_remove_edge_existing_succeeds() {
        let (a, b, _) = ids();
        let mut g = DagGraph::new();
        g.add_edge(a, b).unwrap();
        assert_eq!(g.remove_edge(a, b), Ok(()));
        assert!(g.outgoing(a).is_empty());
    }

    #[test]
    fn test_remove_edge_missing_returns_edge_not_found() {
        let (a, b, _) = ids();
        let mut g = DagGraph::new();
        assert_eq!(g.remove_edge(a, b), Err(GraphError::EdgeNotFound));
    }

    #[test]
    fn test_contains_edge_distinguishes_direction() {
        let (a, b, _) = ids();
        let mut g = DagGraph::new();
        g.add_edge(a, b).unwrap();
        assert!(g.contains_edge(a, b));
        assert!(!g.contains_edge(b, a));
    }

    #[test]
    fn test_archive_node_removes_from_active_view() {
        let (a, b, _) = ids();
        let mut g = DagGraph::new();
        g.add_edge(a, b).unwrap();
        g.archive_node(b);
        assert!(g.outgoing(a).is_empty());
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.active_edge_count(), 0);
    }

    #[test]
    fn test_unarchive_node_restores_active_view() {
        let (a, b, _) = ids();
        let mut g = DagGraph::new();
        g.add_edge(a, b).unwrap();
        g.archive_node(b);
        g.unarchive_node(b);
        assert_eq!(g.outgoing(a), vec![b]);
    }

    #[test]
    fn test_remove_node_deletes_all_involved_edges() {
        let (a, b, c) = ids();
        let mut g = DagGraph::new();
        g.add_edge(a, b).unwrap();
        g.add_edge(b, c).unwrap();
        g.remove_node(b);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn test_descendants_returns_transitive_successors() {
        let (a, b, c) = ids();
        let d = Uuid::new_v4();
        let mut g = DagGraph::new();
        g.add_edge(a, b).unwrap();
        g.add_edge(b, c).unwrap();
        g.add_edge(c, d).unwrap();
        let mut got = g.descendants(a);
        got.sort();
        let mut expected = vec![b, c, d];
        expected.sort();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_ancestors_returns_transitive_predecessors() {
        let (a, b, c) = ids();
        let mut g = DagGraph::new();
        g.add_edge(a, b).unwrap();
        g.add_edge(b, c).unwrap();
        let mut got = g.ancestors(c);
        got.sort();
        let mut expected = vec![a, b];
        expected.sort();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_archived_edge_is_ignored_for_cycle_check() {
        let (a, b, _) = ids();
        let mut g = DagGraph::new();
        g.add_edge(a, b).unwrap();
        g.archive_node(a);
        assert_eq!(g.add_edge(b, a), Ok(()));
    }
}
