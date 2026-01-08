use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::algorithms;
use super::edge::{Edge, EdgeDirection};
use crate::KanbanResult;

/// Generic graph structure that can hold any edge type E
///
/// Stores edges as an edge list for efficient serialization.
/// Provides adjacency list views for graph algorithms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph<E> {
    edges: Vec<Edge<E>>,
}

impl<E> Default for Graph<E> {
    fn default() -> Self {
        Self { edges: Vec::new() }
    }
}

impl<E> Graph<E> {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self { edges: Vec::new() }
    }

    /// Add an edge to the graph
    ///
    /// Note: Cycle checking must be done by caller if needed
    /// (see `would_create_cycle` method)
    pub fn add_edge(&mut self, edge: Edge<E>) -> KanbanResult<()> {
        self.edges.push(edge);
        Ok(())
    }

    /// Remove an edge between two nodes
    ///
    /// Returns true if an edge was removed, false if no matching edge found
    pub fn remove_edge(&mut self, source: Uuid, target: Uuid) -> bool
    where
        E: Clone,
    {
        let initial_len = self.edges.len();
        self.edges.retain(|e| !e.connects(source, target));
        self.edges.len() < initial_len
    }

    /// Remove all edges involving a node (for deletion cascades)
    pub fn remove_node(&mut self, node_id: Uuid) {
        self.edges.retain(|e| !e.involves(node_id));
    }

    /// Archive all edges involving a node (for archive cascades)
    pub fn archive_node(&mut self, node_id: Uuid) {
        for edge in &mut self.edges {
            if edge.involves(node_id) {
                edge.archive();
            }
        }
    }

    /// Unarchive all edges involving a node (for unarchive cascades)
    pub fn unarchive_node(&mut self, node_id: Uuid) {
        for edge in &mut self.edges {
            if edge.involves(node_id) {
                edge.unarchive();
            }
        }
    }

    /// Get all outgoing edges from a node (where node is source)
    pub fn outgoing(&self, node_id: Uuid) -> Vec<&Edge<E>> {
        self.edges.iter().filter(|e| e.source == node_id).collect()
    }

    /// Get all incoming edges to a node (where node is target)
    pub fn incoming(&self, node_id: Uuid) -> Vec<&Edge<E>> {
        self.edges.iter().filter(|e| e.target == node_id).collect()
    }

    /// Get all active outgoing edges from a node
    pub fn outgoing_active(&self, node_id: Uuid) -> Vec<&Edge<E>> {
        self.edges
            .iter()
            .filter(|e| e.source == node_id && e.is_active())
            .collect()
    }

    /// Get all active incoming edges to a node
    pub fn incoming_active(&self, node_id: Uuid) -> Vec<&Edge<E>> {
        self.edges
            .iter()
            .filter(|e| e.target == node_id && e.is_active())
            .collect()
    }

    /// Get all neighbor node IDs (connected nodes, handling bidirectional edges)
    pub fn neighbors(&self, node_id: Uuid) -> Vec<Uuid>
    where
        E: Clone,
    {
        let mut neighbors = Vec::new();

        for edge in &self.edges {
            if edge.source == node_id {
                neighbors.push(edge.target);
            } else if edge.target == node_id {
                match edge.direction {
                    EdgeDirection::Bidirectional => neighbors.push(edge.source),
                    EdgeDirection::Directed => {}
                }
            }
        }

        neighbors
    }

    /// Get all active neighbor node IDs
    pub fn neighbors_active(&self, node_id: Uuid) -> Vec<Uuid>
    where
        E: Clone,
    {
        let mut neighbors = Vec::new();

        for edge in self.edges.iter().filter(|e| e.is_active()) {
            if edge.source == node_id {
                neighbors.push(edge.target);
            } else if edge.target == node_id {
                match edge.direction {
                    EdgeDirection::Bidirectional => neighbors.push(edge.source),
                    EdgeDirection::Directed => {}
                }
            }
        }

        neighbors
    }

    /// Build an adjacency list view of the graph (for algorithms)
    /// Only includes active edges
    pub fn adjacency_list(&self) -> HashMap<Uuid, Vec<Uuid>>
    where
        E: Clone,
    {
        let mut adj_list: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

        for edge in self.edges.iter().filter(|e| e.is_active()) {
            adj_list
                .entry(edge.source)
                .or_insert_with(Vec::new)
                .push(edge.target);

            if edge.direction == EdgeDirection::Bidirectional {
                adj_list
                    .entry(edge.target)
                    .or_insert_with(Vec::new)
                    .push(edge.source);
            }
        }

        adj_list
    }

    /// Get all edges (for serialization/inspection)
    pub fn edges(&self) -> &[Edge<E>] {
        &self.edges
    }

    /// Get all active edges
    pub fn active_edges(&self) -> Vec<&Edge<E>> {
        self.edges.iter().filter(|e| e.is_active()).collect()
    }

    /// Check if an edge exists between two nodes
    pub fn has_edge(&self, source: Uuid, target: Uuid) -> bool
    where
        E: Clone,
    {
        self.edges.iter().any(|e| e.connects(source, target))
    }

    /// Check if adding an edge would create a cycle
    /// Only checks active edges
    pub fn would_create_cycle(&self, source: Uuid, target: Uuid) -> bool
    where
        E: Clone,
    {
        let adj_list = self.adjacency_list();
        algorithms::would_create_cycle(&adj_list, source, target)
    }

    /// Check if the graph contains any cycles
    /// Only checks active edges
    pub fn has_cycle(&self) -> bool
    where
        E: Clone,
    {
        let adj_list = self.adjacency_list();
        algorithms::has_cycle(&adj_list)
    }

    /// Get all nodes reachable from a given node
    /// Only considers active edges
    pub fn reachable_from(&self, start: Uuid) -> std::collections::HashSet<Uuid>
    where
        E: Clone,
    {
        let adj_list = self.adjacency_list();
        algorithms::reachable_from(&adj_list, start)
    }

    /// Get the count of edges (total, including archived)
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get the count of active edges
    pub fn active_edge_count(&self) -> usize {
        self.edges.iter().filter(|e| e.is_active()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::edge::EdgeDirection;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    enum TestEdgeType {
        TypeA,
        TypeB,
    }

    #[test]
    fn test_graph_creation() {
        let graph: Graph<TestEdgeType> = Graph::new();
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = Graph::new();
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        let edge = Edge::new(source, target, TestEdgeType::TypeA, EdgeDirection::Directed);

        graph.add_edge(edge).unwrap();
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.has_edge(source, target));
    }

    #[test]
    fn test_remove_edge() {
        let mut graph = Graph::new();
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        let edge = Edge::new(source, target, TestEdgeType::TypeA, EdgeDirection::Directed);

        graph.add_edge(edge).unwrap();
        assert!(graph.remove_edge(source, target));
        assert_eq!(graph.edge_count(), 0);
        assert!(!graph.has_edge(source, target));
    }

    #[test]
    fn test_remove_node() {
        let mut graph = Graph::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph
            .add_edge(Edge::new(
                node_a,
                node_b,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();
        graph
            .add_edge(Edge::new(
                node_b,
                node_c,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();
        graph
            .add_edge(Edge::new(
                node_c,
                node_a,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();

        assert_eq!(graph.edge_count(), 3);

        graph.remove_node(node_b);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.has_edge(node_c, node_a));
        assert!(!graph.has_edge(node_a, node_b));
        assert!(!graph.has_edge(node_b, node_c));
    }

    #[test]
    fn test_archive_node() {
        let mut graph = Graph::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph
            .add_edge(Edge::new(
                node_a,
                node_b,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();
        graph
            .add_edge(Edge::new(
                node_b,
                node_c,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();

        assert_eq!(graph.active_edge_count(), 2);

        graph.archive_node(node_b);
        assert_eq!(graph.edge_count(), 2);
        assert_eq!(graph.active_edge_count(), 0);
    }

    #[test]
    fn test_unarchive_node() {
        let mut graph = Graph::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();

        graph
            .add_edge(Edge::new(
                node_a,
                node_b,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();
        graph.archive_node(node_a);
        assert_eq!(graph.active_edge_count(), 0);

        graph.unarchive_node(node_a);
        assert_eq!(graph.active_edge_count(), 1);
    }

    #[test]
    fn test_outgoing_incoming() {
        let mut graph = Graph::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph
            .add_edge(Edge::new(
                node_a,
                node_b,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();
        graph
            .add_edge(Edge::new(
                node_a,
                node_c,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();
        graph
            .add_edge(Edge::new(
                node_c,
                node_a,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();

        assert_eq!(graph.outgoing(node_a).len(), 2);
        assert_eq!(graph.incoming(node_a).len(), 1);
        assert_eq!(graph.outgoing(node_b).len(), 0);
        assert_eq!(graph.incoming(node_b).len(), 1);
    }

    #[test]
    fn test_neighbors_directed() {
        let mut graph = Graph::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph
            .add_edge(Edge::new(
                node_a,
                node_b,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();
        graph
            .add_edge(Edge::new(
                node_a,
                node_c,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();

        let neighbors = graph.neighbors(node_a);
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&node_b));
        assert!(neighbors.contains(&node_c));

        assert_eq!(graph.neighbors(node_b).len(), 0);
    }

    #[test]
    fn test_neighbors_bidirectional() {
        let mut graph = Graph::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();

        graph
            .add_edge(Edge::new(
                node_a,
                node_b,
                TestEdgeType::TypeA,
                EdgeDirection::Bidirectional,
            ))
            .unwrap();

        let neighbors_a = graph.neighbors(node_a);
        let neighbors_b = graph.neighbors(node_b);

        assert_eq!(neighbors_a.len(), 1);
        assert_eq!(neighbors_b.len(), 1);
        assert!(neighbors_a.contains(&node_b));
        assert!(neighbors_b.contains(&node_a));
    }

    #[test]
    fn test_would_create_cycle() {
        let mut graph = Graph::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph
            .add_edge(Edge::new(
                node_a,
                node_b,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();
        graph
            .add_edge(Edge::new(
                node_b,
                node_c,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();

        // c -> a would create cycle: a -> b -> c -> a
        assert!(graph.would_create_cycle(node_c, node_a));

        // c -> b would also create cycle: b -> c -> b
        assert!(graph.would_create_cycle(node_c, node_b));
    }

    #[test]
    fn test_adjacency_list() {
        let mut graph = Graph::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph
            .add_edge(Edge::new(
                node_a,
                node_b,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();
        graph
            .add_edge(Edge::new(
                node_b,
                node_c,
                TestEdgeType::TypeA,
                EdgeDirection::Directed,
            ))
            .unwrap();

        let adj_list = graph.adjacency_list();
        assert_eq!(adj_list.len(), 2);
        assert_eq!(adj_list.get(&node_a).unwrap().len(), 1);
        assert_eq!(adj_list.get(&node_b).unwrap().len(), 1);
    }
}
