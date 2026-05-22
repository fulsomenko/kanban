use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use super::core::EdgeStore;
use super::edge::Edge;
use super::error::GraphError;
use super::traits::{Cascadable, Directed, EdgeSet, Graph};

/// Directed acyclic graph generic over any `E: Edge`.
///
/// Rejects self-references and any edge whose insertion would
/// create a cycle in the active-edge subgraph. Wraps an
/// [`EdgeStore<E>`] to inherit archive / unarchive semantics for
/// soft-delete cascades.
///
/// `Deserialize` runs the DAG invariants against the active subset
/// of loaded edges; a corrupted file with a cycle or self-reference
/// fails to load up front rather than silently rehydrating into an
/// invariant-violating state.
///
/// The graph machinery is fully generic: external code can
/// instantiate `DagGraph<MyEdge>` for any `MyEdge: Edge` without
/// needing to modify this crate. The kanban-domain `DependencyGraph`
/// uses three concrete instantiations
/// (`DagGraph<SpawnsEdge>` / `DagGraph<BlocksEdge>` / etc.) but the
/// machinery itself is open for extension.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DagGraph<E> {
    #[serde(flatten)]
    store: EdgeStore<E>,
}

impl<E> Default for DagGraph<E> {
    fn default() -> Self {
        Self {
            store: EdgeStore::new(),
        }
    }
}

impl<'de, E> Deserialize<'de> for DagGraph<E>
where
    E: Edge + Clone + Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let store: EdgeStore<E> = EdgeStore::deserialize(deserializer)?;
        let mut graph: DagGraph<E> = DagGraph::default();
        for edge in store.edges() {
            graph
                .add_edge_with_metadata(edge.clone())
                .map_err(serde::de::Error::custom)?;
        }
        Ok(graph)
    }
}

impl<E: Edge> DagGraph<E> {
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
    ///
    /// Runs the same self-reference and cycle invariants as the
    /// trait's [`Graph::add_edge`]: self-references are rejected
    /// always; cycles are rejected when the edge is active, ignored
    /// when it is archived (archived edges are not part of the
    /// active DAG and don't constrain new mutations). Load paths
    /// use this to rehydrate stored edges and surface corrupt-DAG
    /// state as a hard load failure.
    pub fn add_edge_with_metadata(&mut self, edge: E) -> Result<(), GraphError> {
        if edge.source() == edge.target() {
            return Err(GraphError::SelfReference);
        }
        if edge.is_active() {
            let adj = self.store.adjacency_list();
            if super::algorithms::would_create_cycle(&adj, edge.source(), edge.target()) {
                return Err(GraphError::Cycle);
            }
        }
        self.store.add_edge(edge);
        Ok(())
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
}

impl<E: Edge> Cascadable for DagGraph<E> {
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

impl<E: Edge> EdgeSet for DagGraph<E> {
    fn len(&self) -> usize {
        self.store.edge_count()
    }
    fn active_len(&self) -> usize {
        self.store.active_edge_count()
    }
    /// Directed membership: source-to-target ordering matters.
    /// Considers both active and archived edges.
    fn contains(&self, a: Uuid, b: Uuid) -> bool {
        self.store
            .edges()
            .iter()
            .any(|e| e.source() == a && e.target() == b)
    }
}

impl<E: Edge> Graph for DagGraph<E> {
    type NodeId = Uuid;

    fn add_edge(&mut self, from: Uuid, to: Uuid) -> Result<(), GraphError> {
        // Synthesise an E with per-kind default metadata via the
        // trait constructor. Callers that need to set non-default
        // metadata construct the concrete struct (e.g.
        // `BlocksEdge::new(..., Severity::High)`) and push it via
        // `add_edge_with_metadata`.
        self.add_edge_with_metadata(E::from_endpoints(from, to))
    }

    fn remove_edge(&mut self, from: Uuid, to: Uuid) -> Result<(), GraphError> {
        if self.store.remove_directed_edge(from, to) {
            Ok(())
        } else {
            Err(GraphError::EdgeNotFound)
        }
    }

    fn contains_edge(&self, from: Uuid, to: Uuid) -> bool {
        self.store.outgoing_active(from).any(|e| e.target() == to)
    }
}

impl<E: Edge> Directed for DagGraph<E> {
    fn outgoing(&self, node: Uuid) -> Vec<Uuid> {
        self.store
            .outgoing_active(node)
            .map(|e| e.target())
            .collect()
    }

    fn incoming(&self, node: Uuid) -> Vec<Uuid> {
        self.store
            .incoming_active(node)
            .map(|e| e.source())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::edge::EdgeBase;

    fn ids() -> (Uuid, Uuid, Uuid) {
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
    }

    fn add(g: &mut DagGraph<EdgeBase>, a: Uuid, b: Uuid) -> Result<(), GraphError> {
        g.add_edge_with_metadata(EdgeBase::new(a, b))
    }

    #[test]
    fn test_new_dag_is_empty() {
        let g: DagGraph<EdgeBase> = DagGraph::new();
        assert_eq!(g.len(), 0);
        assert_eq!(g.active_len(), 0);
    }

    #[test]
    fn test_add_edge_with_metadata_creates_outgoing_and_incoming() {
        let (a, b, _) = ids();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        add(&mut g, a, b).unwrap();
        assert_eq!(g.outgoing(a), vec![b]);
        assert_eq!(g.incoming(b), vec![a]);
    }

    #[test]
    fn test_add_edge_with_metadata_self_reference_returns_error() {
        let (a, _, _) = ids();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        assert_eq!(add(&mut g, a, a), Err(GraphError::SelfReference));
    }

    #[test]
    fn test_add_edge_with_metadata_creating_cycle_returns_error() {
        let (a, b, c) = ids();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        add(&mut g, a, b).unwrap();
        add(&mut g, b, c).unwrap();
        assert_eq!(add(&mut g, c, a), Err(GraphError::Cycle));
    }

    #[test]
    fn test_remove_edge_existing_succeeds() {
        let (a, b, _) = ids();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        add(&mut g, a, b).unwrap();
        assert_eq!(g.remove_edge(a, b), Ok(()));
        assert!(g.outgoing(a).is_empty());
    }

    #[test]
    fn test_remove_edge_missing_returns_edge_not_found() {
        let (a, b, _) = ids();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        assert_eq!(g.remove_edge(a, b), Err(GraphError::EdgeNotFound));
    }

    #[test]
    fn test_contains_edge_distinguishes_direction() {
        let (a, b, _) = ids();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        add(&mut g, a, b).unwrap();
        assert!(g.contains_edge(a, b));
        assert!(!g.contains_edge(b, a));
    }

    #[test]
    fn test_archive_node_removes_from_active_view() {
        let (a, b, _) = ids();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        add(&mut g, a, b).unwrap();
        g.archive_node(b);
        assert!(g.outgoing(a).is_empty());
        assert_eq!(g.len(), 1);
        assert_eq!(g.active_len(), 0);
    }

    #[test]
    fn test_unarchive_node_restores_active_view() {
        let (a, b, _) = ids();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        add(&mut g, a, b).unwrap();
        g.archive_node(b);
        g.unarchive_node(b);
        assert_eq!(g.outgoing(a), vec![b]);
    }

    #[test]
    fn test_remove_node_deletes_all_involved_edges() {
        let (a, b, c) = ids();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        add(&mut g, a, b).unwrap();
        add(&mut g, b, c).unwrap();
        g.remove_node(b);
        assert_eq!(g.len(), 0);
    }

    #[test]
    fn test_descendants_returns_transitive_successors() {
        let (a, b, c) = ids();
        let d = Uuid::new_v4();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        add(&mut g, a, b).unwrap();
        add(&mut g, b, c).unwrap();
        add(&mut g, c, d).unwrap();
        let mut got = g.descendants(a);
        got.sort();
        let mut expected = vec![b, c, d];
        expected.sort();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_ancestors_returns_transitive_predecessors() {
        let (a, b, c) = ids();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        add(&mut g, a, b).unwrap();
        add(&mut g, b, c).unwrap();
        let mut got = g.ancestors(c);
        got.sort();
        let mut expected = vec![a, b];
        expected.sort();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_archived_edge_is_ignored_for_cycle_check() {
        let (a, b, _) = ids();
        let mut g: DagGraph<EdgeBase> = DagGraph::new();
        add(&mut g, a, b).unwrap();
        g.archive_node(a);
        assert_eq!(add(&mut g, b, a), Ok(()));
    }

    #[test]
    fn test_deserialize_rejects_cycle_in_active_edges() {
        let (a, b, c) = ids();
        let now = chrono::Utc::now();
        let json = serde_json::json!({
            "edges": [
                {"source": a, "target": b, "created_at": now, "archived_at": null},
                {"source": b, "target": c, "created_at": now, "archived_at": null},
                {"source": c, "target": a, "created_at": now, "archived_at": null},
            ]
        });
        let result: Result<DagGraph<EdgeBase>, _> = serde_json::from_value(json);
        let err = result.expect_err("a 3-cycle must fail to deserialize");
        assert!(
            err.to_string().to_lowercase().contains("cycle"),
            "deserialize error should name the cycle invariant: {err}"
        );
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
        let result: Result<DagGraph<EdgeBase>, _> = serde_json::from_value(json);
        let err = result.expect_err("self-reference must fail to deserialize");
        assert!(
            err.to_string().to_lowercase().contains("self"),
            "deserialize error should name the self-reference invariant: {err}"
        );
    }

    #[test]
    fn test_deserialize_accepts_archived_edge_completing_cycle() {
        let (a, b, c) = ids();
        let now = chrono::Utc::now();
        let json = serde_json::json!({
            "edges": [
                {"source": a, "target": b, "created_at": now, "archived_at": null},
                {"source": b, "target": c, "created_at": now, "archived_at": null},
                {"source": c, "target": a, "created_at": now, "archived_at": now},
            ]
        });
        let graph: DagGraph<EdgeBase> =
            serde_json::from_value(json).expect("cycle through archived edge must be loadable");
        assert_eq!(graph.len(), 3);
        assert_eq!(graph.active_len(), 2);
    }
}
