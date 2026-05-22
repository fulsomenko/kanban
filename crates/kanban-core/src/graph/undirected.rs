use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use super::core::EdgeStore;
use super::edge::{EdgeDirection, LegacyEdge};
use super::error::GraphError;
use super::traits::{Cascadable, EdgeSet, Graph, Undirected};

/// Undirected graph keyed by `Uuid` node identifiers.
///
/// Rejects self-references; cycles are permitted (the directed concept
/// does not apply). Implements [`Undirected`] exclusively — there is no
/// `outgoing` / `incoming` distinction to ask for, and the type system
/// will reject any caller that tries. Wraps an [`EdgeStore`] for archive
/// parity with [`super::DagGraph`].
///
/// `Deserialize` validates the self-reference invariant on every loaded
/// edge so a corrupted file surfaces the breach at load time instead
/// of silently rehydrating it.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct UndirectedGraph {
    #[serde(flatten)]
    store: EdgeStore,
}

impl<'de> Deserialize<'de> for UndirectedGraph {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let store = EdgeStore::deserialize(deserializer)?;
        let mut graph = UndirectedGraph::default();
        for edge in store.edges() {
            graph
                .add_edge_with_metadata(edge.clone())
                .map_err(serde::de::Error::custom)?;
        }
        Ok(graph)
    }
}

impl UndirectedGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow the raw underlying edge list (active + archived).
    /// Used by persistence layers that need to serialize the storage
    /// shape directly. For size and membership queries, use the
    /// [`EdgeSet`] trait surface instead.
    pub fn edges(&self) -> &[LegacyEdge] {
        self.store.edges()
    }

    /// Add an edge while preserving caller-supplied metadata
    /// (`created_at`, `weight`, `archived_at`). Rejects self-references
    /// regardless of active/archived status. Load paths use this to
    /// rehydrate stored edges and surface corrupt self-loops as a
    /// hard load failure.
    pub fn add_edge_with_metadata(&mut self, edge: LegacyEdge) -> Result<(), GraphError> {
        if edge.source == edge.target {
            return Err(GraphError::SelfReference);
        }
        self.store.add_edge(edge);
        Ok(())
    }
}

impl Cascadable for UndirectedGraph {
    fn archive_node(&mut self, node: Uuid) {
        self.store.archive_node(node);
    }
    fn unarchive_node(&mut self, node: Uuid) {
        self.store.unarchive_node(node);
    }
    fn remove_node(&mut self, node: Uuid) {
        self.store.remove_node(node);
    }
}

impl EdgeSet for UndirectedGraph {
    fn len(&self) -> usize {
        self.store.edge_count()
    }
    fn active_len(&self) -> usize {
        self.store.active_edge_count()
    }
    fn contains(&self, a: Uuid, b: Uuid) -> bool {
        self.store.edges().iter().any(|e| e.connects(a, b))
    }
}

impl Graph for UndirectedGraph {
    type NodeId = Uuid;

    fn add_edge(&mut self, from: Uuid, to: Uuid) -> Result<(), GraphError> {
        if from == to {
            return Err(GraphError::SelfReference);
        }
        self.store
            .add_edge(LegacyEdge::new(from, to, EdgeDirection::Bidirectional));
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
        self.store.active_edges().any(|e| e.connects(from, to))
    }
}

impl Undirected for UndirectedGraph {
    /// Neighbours of `node` from any active edge (either endpoint).
    /// The `Undirected` trait is the only access path — callers must
    /// bring it into scope. Choosing this over an inherent method
    /// makes the trait load-bearing rather than decorative.
    ///
    /// Delegates to [`EdgeStore::neighbors_active`], which checks
    /// `EdgeDirection::Bidirectional` per edge. `UndirectedGraph` only
    /// inserts bidirectional edges (see [`Graph::add_edge`]) so the
    /// delegation observes identical semantics to a hand-rolled scan.
    fn neighbors(&self, node: Uuid) -> Vec<Uuid> {
        self.store.neighbors_active(node)
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
        assert_eq!(g.len(), 0);
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
        assert_eq!(g.len(), 1);
        assert_eq!(g.active_len(), 0);
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
        assert_eq!(g.len(), 0);
    }

    #[test]
    fn test_deserialize_rejects_self_reference() {
        let a = Uuid::new_v4();
        let now = chrono::Utc::now();
        let json = serde_json::json!({
            "edges": [
                {"source": a, "target": a, "direction": "Bidirectional", "weight": null, "created_at": now, "archived_at": null},
            ]
        });
        let result: Result<UndirectedGraph, _> = serde_json::from_value(json);
        let err = result.expect_err("self-reference must fail to deserialize");
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("self"),
            "deserialize error should name the self-reference invariant: {msg}"
        );
    }

    #[test]
    fn test_deserialize_accepts_cycle() {
        // Undirected graphs permit cycles by definition.
        let (a, b, c) = ids();
        let now = chrono::Utc::now();
        let json = serde_json::json!({
            "edges": [
                {"source": a, "target": b, "direction": "Bidirectional", "weight": null, "created_at": now, "archived_at": null},
                {"source": b, "target": c, "direction": "Bidirectional", "weight": null, "created_at": now, "archived_at": null},
                {"source": c, "target": a, "direction": "Bidirectional", "weight": null, "created_at": now, "archived_at": null},
            ]
        });
        let graph: UndirectedGraph =
            serde_json::from_value(json).expect("undirected cycle must be loadable");
        assert_eq!(graph.len(), 3);
    }
}
