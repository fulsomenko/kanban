use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use super::core::EdgeStore;
use super::edge::Edge;
use super::error::GraphError;
use super::traits::{Cascadable, EdgeSet, Graph, Undirected};

/// Undirected graph generic over any `E: Edge`.
///
/// Rejects self-references; cycles are permitted (the directed
/// concept does not apply). Implements [`Undirected`] exclusively —
/// there is no `outgoing` / `incoming` distinction to ask for, and
/// the type system will reject any caller that tries. Wraps an
/// [`EdgeStore<E>`] for archive parity with [`super::DagGraph`].
///
/// `Deserialize` validates the self-reference invariant on every
/// loaded edge so a corrupted file surfaces the breach at load time
/// instead of silently rehydrating it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UndirectedGraph<E> {
    #[serde(flatten)]
    store: EdgeStore<E>,
}

impl<E> Default for UndirectedGraph<E> {
    fn default() -> Self {
        Self {
            store: EdgeStore::new(),
        }
    }
}

impl<'de, E> Deserialize<'de> for UndirectedGraph<E>
where
    E: Edge + Clone + Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let store: EdgeStore<E> = EdgeStore::deserialize(deserializer)?;
        let mut graph: UndirectedGraph<E> = UndirectedGraph::default();
        for edge in store.edges() {
            graph
                .add_edge_with_metadata(edge.clone())
                .map_err(serde::de::Error::custom)?;
        }
        Ok(graph)
    }
}

impl<E: Edge> UndirectedGraph<E> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow the raw underlying edge list (active + archived).
    /// Persistence layers use this to serialise; size and membership
    /// queries go through the [`EdgeSet`] trait surface.
    pub fn edges(&self) -> &[E] {
        self.store.edges()
    }

    /// Push an edge while preserving caller-supplied metadata.
    /// Rejects self-references regardless of active/archived status.
    /// Load paths use this to rehydrate stored edges and surface
    /// corrupt self-loops as a hard load failure.
    pub fn add_edge_with_metadata(&mut self, edge: E) -> Result<(), GraphError> {
        if edge.source() == edge.target() {
            return Err(GraphError::SelfReference);
        }
        self.store.add_edge(edge);
        Ok(())
    }
}

impl<E: Edge> Cascadable for UndirectedGraph<E> {
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

impl<E: Edge> EdgeSet for UndirectedGraph<E> {
    fn len(&self) -> usize {
        self.store.edge_count()
    }
    fn active_len(&self) -> usize {
        self.store.active_edge_count()
    }
    /// Symmetric membership: any edge whose endpoints are `{a, b}`
    /// regardless of ordering. Considers both active and archived
    /// edges.
    fn contains(&self, a: Uuid, b: Uuid) -> bool {
        self.store
            .edges()
            .iter()
            .any(|e| (e.source() == a && e.target() == b) || (e.source() == b && e.target() == a))
    }
}

impl<E: Edge> Graph for UndirectedGraph<E> {
    type NodeId = Uuid;

    fn add_edge(&mut self, from: Uuid, to: Uuid) -> Result<(), GraphError> {
        self.add_edge_with_metadata(E::from_endpoints(from, to))
    }

    fn remove_edge(&mut self, from: Uuid, to: Uuid) -> Result<(), GraphError> {
        if self.store.remove_undirected_edge(from, to) {
            Ok(())
        } else {
            Err(GraphError::EdgeNotFound)
        }
    }

    /// Symmetric: an edge between `{from, to}` in either ordering.
    /// Considers active edges only.
    fn contains_edge(&self, from: Uuid, to: Uuid) -> bool {
        self.store.active_edges().any(|e| {
            (e.source() == from && e.target() == to) || (e.source() == to && e.target() == from)
        })
    }
}

impl<E: Edge> Undirected for UndirectedGraph<E> {
    /// Neighbours of `node` from any active edge (either endpoint).
    /// The `Undirected` trait is the only access path — callers must
    /// bring it into scope. Choosing this over an inherent method
    /// makes the trait load-bearing rather than decorative.
    fn neighbors(&self, node: Uuid) -> Vec<Uuid> {
        let mut out = Vec::new();
        for edge in self.store.active_edges() {
            if edge.source() == node {
                out.push(edge.target());
            } else if edge.target() == node {
                out.push(edge.source());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::edge::EdgeBase;

    fn ids() -> (Uuid, Uuid, Uuid) {
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
    }

    #[test]
    fn test_new_undirected_is_empty() {
        let g: UndirectedGraph<EdgeBase> = UndirectedGraph::new();
        assert_eq!(g.len(), 0);
    }

    #[test]
    fn test_add_edge_creates_bidirectional_neighbours() {
        let (a, b, _) = ids();
        let mut g: UndirectedGraph<EdgeBase> = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        assert_eq!(g.neighbors(a), vec![b]);
        assert_eq!(g.neighbors(b), vec![a]);
    }

    #[test]
    fn test_add_edge_self_reference_returns_error() {
        let (a, _, _) = ids();
        let mut g: UndirectedGraph<EdgeBase> = UndirectedGraph::new();
        assert_eq!(g.add_edge(a, a), Err(GraphError::SelfReference));
    }

    #[test]
    fn test_add_edge_permits_cycle() {
        let (a, b, c) = ids();
        let mut g: UndirectedGraph<EdgeBase> = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        g.add_edge(b, c).unwrap();
        assert_eq!(g.add_edge(c, a), Ok(()));
    }

    #[test]
    fn test_remove_edge_existing_succeeds() {
        let (a, b, _) = ids();
        let mut g: UndirectedGraph<EdgeBase> = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        assert_eq!(g.remove_edge(a, b), Ok(()));
        assert!(g.neighbors(a).is_empty());
    }

    #[test]
    fn test_remove_edge_works_in_either_direction() {
        let (a, b, _) = ids();
        let mut g: UndirectedGraph<EdgeBase> = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        assert_eq!(g.remove_edge(b, a), Ok(()));
        assert!(g.neighbors(a).is_empty());
    }

    #[test]
    fn test_remove_edge_missing_returns_edge_not_found() {
        let (a, b, _) = ids();
        let mut g: UndirectedGraph<EdgeBase> = UndirectedGraph::new();
        assert_eq!(g.remove_edge(a, b), Err(GraphError::EdgeNotFound));
    }

    #[test]
    fn test_contains_edge_is_symmetric() {
        let (a, b, _) = ids();
        let mut g: UndirectedGraph<EdgeBase> = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        assert!(g.contains_edge(a, b));
        assert!(g.contains_edge(b, a));
    }

    #[test]
    fn test_archive_node_removes_from_neighbours_view() {
        let (a, b, _) = ids();
        let mut g: UndirectedGraph<EdgeBase> = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        g.archive_node(a);
        assert!(g.neighbors(b).is_empty());
        assert_eq!(g.len(), 1);
        assert_eq!(g.active_len(), 0);
    }

    #[test]
    fn test_unarchive_node_restores_neighbours_view() {
        let (a, b, _) = ids();
        let mut g: UndirectedGraph<EdgeBase> = UndirectedGraph::new();
        g.add_edge(a, b).unwrap();
        g.archive_node(a);
        g.unarchive_node(a);
        assert_eq!(g.neighbors(b), vec![a]);
    }

    #[test]
    fn test_remove_node_deletes_all_involved_edges() {
        let (a, b, c) = ids();
        let mut g: UndirectedGraph<EdgeBase> = UndirectedGraph::new();
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
                {"source": a, "target": a, "created_at": now, "archived_at": null},
            ]
        });
        let result: Result<UndirectedGraph<EdgeBase>, _> = serde_json::from_value(json);
        let err = result.expect_err("self-reference must fail to deserialize");
        assert!(
            err.to_string().to_lowercase().contains("self"),
            "deserialize error should name the self-reference invariant: {err}"
        );
    }

    #[test]
    fn test_deserialize_accepts_cycle() {
        let (a, b, c) = ids();
        let now = chrono::Utc::now();
        let json = serde_json::json!({
            "edges": [
                {"source": a, "target": b, "created_at": now, "archived_at": null},
                {"source": b, "target": c, "created_at": now, "archived_at": null},
                {"source": c, "target": a, "created_at": now, "archived_at": null},
            ]
        });
        let graph: UndirectedGraph<EdgeBase> =
            serde_json::from_value(json).expect("undirected cycle must be loadable");
        assert_eq!(graph.len(), 3);
    }
}
