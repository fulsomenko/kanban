use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::algorithms;
use super::edge::{Edge, EdgeDirection};

/// Edge-list container shared by every concrete graph kind.
///
/// Stores edges as a flat list for efficient serialization and
/// provides adjacency-list views for graph algorithms. The relation
/// kind lives at the outer [`super::DependencyGraph`] layer (one
/// sub-graph per kind), so this type carries no per-edge kind tag.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EdgeStore {
    edges: Vec<Edge>,
}

impl EdgeStore {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self { edges: Vec::new() }
    }

    /// Add an edge to the graph
    ///
    /// Note: Cycle checking must be done by caller if needed
    /// (see `would_create_cycle` method)
    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }

    /// Remove an edge between two nodes
    ///
    /// Returns true if an edge was removed, false if no matching edge found
    pub fn remove_edge(&mut self, source: Uuid, target: Uuid) -> bool {
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
    pub fn outgoing(&self, node_id: Uuid) -> impl Iterator<Item = &Edge> {
        self.edges.iter().filter(move |e| e.source == node_id)
    }

    /// Get all incoming edges to a node (where node is target)
    pub fn incoming(&self, node_id: Uuid) -> impl Iterator<Item = &Edge> {
        self.edges.iter().filter(move |e| e.target == node_id)
    }

    /// Get all active outgoing edges from a node
    pub fn outgoing_active(&self, node_id: Uuid) -> impl Iterator<Item = &Edge> {
        self.edges
            .iter()
            .filter(move |e| e.source == node_id && e.is_active())
    }

    /// Get all active incoming edges to a node
    pub fn incoming_active(&self, node_id: Uuid) -> impl Iterator<Item = &Edge> {
        self.edges
            .iter()
            .filter(move |e| e.target == node_id && e.is_active())
    }

    /// Get all neighbor node IDs (connected nodes, handling bidirectional edges)
    pub fn neighbors(&self, node_id: Uuid) -> Vec<Uuid> {
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
    pub fn neighbors_active(&self, node_id: Uuid) -> Vec<Uuid> {
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
    pub fn adjacency_list(&self) -> HashMap<Uuid, Vec<Uuid>> {
        let mut adj_list: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

        for edge in self.edges.iter().filter(|e| e.is_active()) {
            adj_list.entry(edge.source).or_default().push(edge.target);

            if edge.direction == EdgeDirection::Bidirectional {
                adj_list.entry(edge.target).or_default().push(edge.source);
            }
        }

        adj_list
    }

    /// Get all edges (for serialization/inspection)
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Get all active edges
    pub fn active_edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.iter().filter(|e| e.is_active())
    }

    /// Check if an edge exists between two nodes
    pub fn has_edge(&self, source: Uuid, target: Uuid) -> bool {
        self.edges.iter().any(|e| e.connects(source, target))
    }

    /// Check if adding an edge would create a cycle
    /// Only checks active edges
    pub fn would_create_cycle(&self, source: Uuid, target: Uuid) -> bool {
        let adj_list = self.adjacency_list();
        algorithms::would_create_cycle(&adj_list, source, target)
    }

    /// Check if the graph contains any cycles
    /// Only checks active edges
    pub fn has_cycle(&self) -> bool {
        let adj_list = self.adjacency_list();
        algorithms::has_cycle(&adj_list)
    }

    /// Get all nodes reachable from a given node
    /// Only considers active edges
    pub fn reachable_from(&self, start: Uuid) -> std::collections::HashSet<Uuid> {
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

    fn directed_edge(source: Uuid, target: Uuid) -> Edge {
        Edge::new(source, target, EdgeDirection::Directed)
    }

    fn bidi_edge(a: Uuid, b: Uuid) -> Edge {
        Edge::new(a, b, EdgeDirection::Bidirectional)
    }

    #[test]
    fn test_new_returns_empty_edge_store() {
        let graph = EdgeStore::new();
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_edge_increments_count_and_marks_present() {
        let mut graph = EdgeStore::new();
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();

        graph.add_edge(directed_edge(source, target));
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.has_edge(source, target));
    }

    #[test]
    fn test_remove_edge_drops_matching_entry() {
        let mut graph = EdgeStore::new();
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();

        graph.add_edge(directed_edge(source, target));
        assert!(graph.remove_edge(source, target));
        assert_eq!(graph.edge_count(), 0);
        assert!(!graph.has_edge(source, target));
    }

    #[test]
    fn test_remove_node_drops_every_incident_edge() {
        let mut graph = EdgeStore::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph.add_edge(directed_edge(node_a, node_b));
        graph.add_edge(directed_edge(node_b, node_c));
        graph.add_edge(directed_edge(node_c, node_a));

        assert_eq!(graph.edge_count(), 3);

        graph.remove_node(node_b);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.has_edge(node_c, node_a));
        assert!(!graph.has_edge(node_a, node_b));
        assert!(!graph.has_edge(node_b, node_c));
    }

    #[test]
    fn test_archive_node_hides_incident_edges_from_active_count() {
        let mut graph = EdgeStore::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph.add_edge(directed_edge(node_a, node_b));
        graph.add_edge(directed_edge(node_b, node_c));

        assert_eq!(graph.active_edge_count(), 2);

        graph.archive_node(node_b);
        assert_eq!(graph.edge_count(), 2);
        assert_eq!(graph.active_edge_count(), 0);
    }

    #[test]
    fn test_unarchive_node_restores_active_count() {
        let mut graph = EdgeStore::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();

        graph.add_edge(directed_edge(node_a, node_b));
        graph.archive_node(node_a);
        assert_eq!(graph.active_edge_count(), 0);

        graph.unarchive_node(node_a);
        assert_eq!(graph.active_edge_count(), 1);
    }

    #[test]
    fn test_outgoing_and_incoming_split_by_direction() {
        let mut graph = EdgeStore::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph.add_edge(directed_edge(node_a, node_b));
        graph.add_edge(directed_edge(node_a, node_c));
        graph.add_edge(directed_edge(node_c, node_a));

        assert_eq!(graph.outgoing(node_a).count(), 2);
        assert_eq!(graph.incoming(node_a).count(), 1);
        assert_eq!(graph.outgoing(node_b).count(), 0);
        assert_eq!(graph.incoming(node_b).count(), 1);
    }

    #[test]
    fn test_neighbors_directed_only_includes_outgoing_targets() {
        let mut graph = EdgeStore::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph.add_edge(directed_edge(node_a, node_b));
        graph.add_edge(directed_edge(node_a, node_c));

        let neighbors = graph.neighbors(node_a);
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&node_b));
        assert!(neighbors.contains(&node_c));

        assert_eq!(graph.neighbors(node_b).len(), 0);
    }

    #[test]
    fn test_neighbors_bidirectional_includes_both_endpoints() {
        let mut graph = EdgeStore::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();

        graph.add_edge(bidi_edge(node_a, node_b));

        let neighbors_a = graph.neighbors(node_a);
        let neighbors_b = graph.neighbors(node_b);

        assert_eq!(neighbors_a.len(), 1);
        assert_eq!(neighbors_b.len(), 1);
        assert!(neighbors_a.contains(&node_b));
        assert!(neighbors_b.contains(&node_a));
    }

    #[test]
    fn test_would_create_cycle_detects_directed_path_closing() {
        let mut graph = EdgeStore::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph.add_edge(directed_edge(node_a, node_b));
        graph.add_edge(directed_edge(node_b, node_c));

        // c -> a would create cycle: a -> b -> c -> a
        assert!(graph.would_create_cycle(node_c, node_a));

        // c -> b would also create cycle: b -> c -> b
        assert!(graph.would_create_cycle(node_c, node_b));
    }

    #[test]
    fn test_adjacency_list_counts_active_outgoing_per_node() {
        let mut graph = EdgeStore::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        graph.add_edge(directed_edge(node_a, node_b));
        graph.add_edge(directed_edge(node_b, node_c));

        let adj_list = graph.adjacency_list();
        assert_eq!(adj_list.len(), 2);
        assert_eq!(adj_list.get(&node_a).unwrap().len(), 1);
        assert_eq!(adj_list.get(&node_b).unwrap().len(), 1);
    }
}
