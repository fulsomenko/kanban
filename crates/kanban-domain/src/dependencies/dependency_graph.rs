use kanban_core::{
    Cascadable, DagGraph, Directed, EdgeBase, EdgeSet, GraphError, Undirected, UndirectedGraph,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::edges::{BlocksEdge, RelatesEdge, SpawnsEdge};
use super::CardEdgeType;
use crate::error::DependencyError;
use crate::{CardId, KanbanResult};

/// Top-level container for all entity dependency graphs.
///
/// Three discrete sub-graphs, each with its own structural rules
/// and its own concrete edge kind (carrying any per-kind metadata):
///
/// - `parent_child: DagGraph<SpawnsEdge>` (no extra metadata today)
/// - `blocks: DagGraph<BlocksEdge>` (carries [`Severity`])
/// - `relates: UndirectedGraph<RelatesEdge>` (carries [`RelatesKind`])
///
/// Cross-cutting operations (`archive_node`, `unarchive_node`,
/// `remove_node`) cascade across all three. Per-edge-type convenience
/// methods delegate to the matching sub-graph and convert
/// [`GraphError`] into the domain-level [`DependencyError`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DependencyGraph {
    #[serde(default)]
    pub(crate) parent_child: DagGraph<SpawnsEdge>,
    #[serde(default)]
    pub(crate) blocks: DagGraph<BlocksEdge>,
    #[serde(default)]
    pub(crate) relates: UndirectedGraph<RelatesEdge>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow each component graph as `&mut dyn Cascadable` for
    /// node-level cascade operations. Order is `parent_child`,
    /// `blocks`, `relates`. A new component sub-graph only needs to
    /// be added to this helper, not to every cascade method.
    fn cascadable_parts_mut(&mut self) -> [&mut dyn Cascadable; 3] {
        [&mut self.parent_child, &mut self.blocks, &mut self.relates]
    }

    /// Borrow each component graph as `&dyn EdgeSet` for read-only
    /// edge-level queries.
    fn edge_sets(&self) -> [&dyn EdgeSet; 3] {
        [&self.parent_child, &self.blocks, &self.relates]
    }

    // --- Cross-cutting node cascades (Cascadable surface) ---

    pub fn archive_node(&mut self, card: CardId) {
        for sg in self.cascadable_parts_mut() {
            sg.archive_node(card);
        }
    }

    pub fn unarchive_node(&mut self, card: CardId) {
        for sg in self.cascadable_parts_mut() {
            sg.unarchive_node(card);
        }
    }

    pub fn remove_node(&mut self, card: CardId) {
        for sg in self.cascadable_parts_mut() {
            sg.remove_node(card);
        }
    }

    // --- Cross-cutting edge aggregates (EdgeSet surface) ---

    pub fn len(&self) -> usize {
        self.edge_sets().iter().map(|g| g.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn active_len(&self) -> usize {
        self.edge_sets().iter().map(|g| g.active_len()).sum()
    }

    // --- Parent / child (Spawns) ---

    pub fn set_parent(&mut self, child: CardId, parent: CardId) -> KanbanResult<()> {
        self.parent_child
            .add_edge_with_metadata(SpawnsEdge::new(parent, child))
            .map_err(dep_err)
    }

    pub fn remove_parent(&mut self, child: CardId, parent: CardId) -> KanbanResult<()> {
        // Use the structural `Graph::remove_edge` via the trait so the
        // sub-graph picks the right matching semantics (directed for
        // DAG sub-graphs).
        use kanban_core::Graph as _;
        self.parent_child
            .remove_edge(parent, child)
            .map_err(dep_err)
    }

    pub fn children(&self, parent: CardId) -> Vec<CardId> {
        self.parent_child.outgoing(parent)
    }

    pub fn parents(&self, child: CardId) -> Vec<CardId> {
        self.parent_child.incoming(child)
    }

    pub fn ancestors(&self, child: CardId) -> Vec<CardId> {
        self.parent_child.ancestors(child)
    }

    pub fn descendants(&self, parent: CardId) -> Vec<CardId> {
        self.parent_child.descendants(parent)
    }

    pub fn child_count(&self, parent: CardId) -> usize {
        self.parent_child.outgoing(parent).len()
    }

    // --- Blocks ---

    /// Add a `blocker -> blocked` edge with default
    /// ([`Severity::Medium`]) severity.
    pub fn set_block(&mut self, blocker: CardId, blocked: CardId) -> KanbanResult<()> {
        self.set_block_with_severity(blocker, blocked, super::Severity::default())
    }

    /// Add a `blocker -> blocked` edge with an explicit severity.
    pub fn set_block_with_severity(
        &mut self,
        blocker: CardId,
        blocked: CardId,
        severity: super::Severity,
    ) -> KanbanResult<()> {
        self.blocks
            .add_edge_with_metadata(BlocksEdge::new(blocker, blocked, severity))
            .map_err(dep_err)
    }

    pub fn unblock(&mut self, blocker: CardId, blocked: CardId) -> KanbanResult<()> {
        use kanban_core::Graph as _;
        self.blocks.remove_edge(blocker, blocked).map_err(dep_err)
    }

    pub fn blocked(&self, card: CardId) -> Vec<CardId> {
        self.blocks.outgoing(card)
    }

    pub fn blockers(&self, card: CardId) -> Vec<CardId> {
        self.blocks.incoming(card)
    }

    pub fn can_start<F>(&self, card: CardId, is_complete: F) -> bool
    where
        F: Fn(CardId) -> bool,
    {
        self.blockers(card).into_iter().all(is_complete)
    }

    // --- Relates ---

    /// Add an undirected `a <-> b` relates edge with default
    /// ([`RelatesKind::General`]) kind.
    pub fn relate(&mut self, a: CardId, b: CardId) -> KanbanResult<()> {
        self.relate_with_kind(a, b, super::RelatesKind::default())
    }

    /// Add an undirected `a <-> b` relates edge with an explicit
    /// sub-kind.
    pub fn relate_with_kind(
        &mut self,
        a: CardId,
        b: CardId,
        kind: super::RelatesKind,
    ) -> KanbanResult<()> {
        self.relates
            .add_edge_with_metadata(RelatesEdge::new(a, b, kind))
            .map_err(dep_err)
    }

    pub fn unrelate(&mut self, a: CardId, b: CardId) -> KanbanResult<()> {
        use kanban_core::Graph as _;
        self.relates.remove_edge(a, b).map_err(dep_err)
    }

    pub fn related(&self, card: CardId) -> Vec<CardId> {
        self.relates.neighbors(card)
    }

    /// True iff any edge between `a` and `b` exists in any sub-graph
    /// (active or archived).
    pub fn contains(&self, a: Uuid, b: Uuid) -> bool {
        self.edge_sets().iter().any(|g| g.contains(a, b))
    }

    /// Sever the single specific edge between `a` and `b` across all
    /// three sub-graphs. For the two directed sub-graphs only the
    /// exact `a -> b` orientation is removed; for the undirected
    /// sub-graph either ordering removes the edge.
    pub fn disconnect(&mut self, a: Uuid, b: Uuid) -> bool {
        let mut any_removed = false;
        for sg in self.cascadable_parts_mut() {
            if sg.remove_edge(a, b).is_ok() {
                any_removed = true;
            }
        }
        any_removed
    }

    // --- Persistence helpers ---

    /// Borrow the raw [`EdgeBase`] list for a given sub-graph as a
    /// uniform read view. Callers needing per-kind metadata (severity,
    /// kind) access the typed sub-graph directly via
    /// `parent_child_edges` / `blocks_edges` / `relates_edges`.
    pub fn edge_bases_of(&self, kind: CardEdgeType) -> Vec<EdgeBase> {
        match kind {
            CardEdgeType::Spawns => self
                .parent_child
                .edges()
                .iter()
                .map(|e| e.base.clone())
                .collect(),
            CardEdgeType::Blocks => self.blocks.edges().iter().map(|e| e.base.clone()).collect(),
            CardEdgeType::RelatesTo => self
                .relates
                .edges()
                .iter()
                .map(|e| e.base.clone())
                .collect(),
        }
    }

    /// Per-kind raw edge accessors. Persistence backends and test
    /// fixtures use these to round-trip metadata-bearing edges.
    pub fn spawns_edges(&self) -> &[SpawnsEdge] {
        self.parent_child.edges()
    }
    pub fn blocks_edges(&self) -> &[BlocksEdge] {
        self.blocks.edges()
    }
    pub fn relates_edges(&self) -> &[RelatesEdge] {
        self.relates.edges()
    }

    /// Construct a graph from per-kind edge vectors with structural
    /// validation. Each sub-graph independently checks its
    /// self-reference / cycle invariants on the way in; a corrupted
    /// load fails up front instead of silently rehydrating an
    /// invariant-violating graph.
    pub fn from_validated_per_kind_edges(
        spawns: Vec<SpawnsEdge>,
        blocks: Vec<BlocksEdge>,
        relates: Vec<RelatesEdge>,
    ) -> KanbanResult<Self> {
        let mut graph = Self::new();
        for edge in spawns {
            graph
                .parent_child
                .add_edge_with_metadata(edge)
                .map_err(dep_err)?;
        }
        for edge in blocks {
            graph.blocks.add_edge_with_metadata(edge).map_err(dep_err)?;
        }
        for edge in relates {
            graph
                .relates
                .add_edge_with_metadata(edge)
                .map_err(dep_err)?;
        }
        Ok(graph)
    }
}

fn dep_err(e: GraphError) -> crate::KanbanError {
    let d: DependencyError = e.into();
    d.into()
}

impl From<GraphError> for DependencyError {
    fn from(e: GraphError) -> Self {
        match e {
            GraphError::Cycle => DependencyError::CycleDetected,
            GraphError::SelfReference => DependencyError::SelfReference,
            GraphError::EdgeNotFound => DependencyError::EdgeNotFound,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn ids() -> (Uuid, Uuid, Uuid) {
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
    }

    #[test]
    fn test_new_returns_empty_graph() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.len(), 0);
    }

    #[test]
    fn test_default_graph_round_trips_through_serde() {
        let graph = DependencyGraph::new();
        let json = serde_json::to_string(&graph).unwrap();
        let deserialized: DependencyGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 0);
    }

    // --- Parent/child (Spawns) ---

    #[test]
    fn test_set_parent_creates_parent_child_edge() {
        let (parent, child, _) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(child, parent).unwrap();
        assert_eq!(g.children(parent), vec![child]);
        assert_eq!(g.parents(child), vec![parent]);
    }

    #[test]
    fn test_set_parent_self_reference_returns_error() {
        let (a, _, _) = ids();
        let mut g = DependencyGraph::new();
        assert!(g.set_parent(a, a).unwrap_err().is_self_reference());
    }

    #[test]
    fn test_set_parent_cycle_returns_error() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(b, a).unwrap();
        g.set_parent(c, b).unwrap();
        assert!(g.set_parent(a, c).unwrap_err().is_cycle_detected());
    }

    #[test]
    fn test_remove_parent_existing_succeeds() {
        let (parent, child, _) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(child, parent).unwrap();
        g.remove_parent(child, parent).unwrap();
        assert!(g.children(parent).is_empty());
    }

    #[test]
    fn test_remove_parent_missing_returns_edge_not_found() {
        let (parent, child, _) = ids();
        let mut g = DependencyGraph::new();
        assert!(g
            .remove_parent(child, parent)
            .unwrap_err()
            .is_edge_not_found());
    }

    #[test]
    fn test_ancestors_returns_transitive_parents() {
        let (gp, p, c) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(p, gp).unwrap();
        g.set_parent(c, p).unwrap();
        let mut anc = g.ancestors(c);
        anc.sort();
        let mut expected = vec![gp, p];
        expected.sort();
        assert_eq!(anc, expected);
    }

    #[test]
    fn test_descendants_returns_transitive_children() {
        let (parent, child, grandchild) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(child, parent).unwrap();
        g.set_parent(grandchild, child).unwrap();
        let mut desc = g.descendants(parent);
        desc.sort();
        let mut expected = vec![child, grandchild];
        expected.sort();
        assert_eq!(desc, expected);
    }

    #[test]
    fn test_child_count_matches_children_len() {
        let (parent, a, b) = ids();
        let mut g = DependencyGraph::new();
        assert_eq!(g.child_count(parent), 0);
        g.set_parent(a, parent).unwrap();
        g.set_parent(b, parent).unwrap();
        assert_eq!(g.child_count(parent), 2);
    }

    // --- Blocks ---

    #[test]
    fn test_add_blocks_creates_directed_edge() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.set_block(a, b).unwrap();
        assert_eq!(g.blocked(a), vec![b]);
        assert_eq!(g.blockers(b), vec![a]);
    }

    #[test]
    fn test_set_block_with_severity_preserves_metadata() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.set_block_with_severity(a, b, super::super::Severity::Critical)
            .unwrap();
        assert_eq!(
            g.blocks_edges()[0].severity,
            super::super::Severity::Critical
        );
    }

    #[test]
    fn test_set_block_default_is_medium_severity() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.set_block(a, b).unwrap();
        assert_eq!(g.blocks_edges()[0].severity, super::super::Severity::Medium);
    }

    #[test]
    fn test_add_blocks_cycle_returns_error() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.set_block(a, b).unwrap();
        g.set_block(b, c).unwrap();
        assert!(g.set_block(c, a).unwrap_err().is_cycle_detected());
    }

    #[test]
    fn test_add_blocks_self_reference_returns_error() {
        let (a, _, _) = ids();
        let mut g = DependencyGraph::new();
        assert!(g.set_block(a, a).unwrap_err().is_self_reference());
    }

    #[test]
    fn test_can_start_returns_true_when_all_blockers_complete() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.set_block(a, c).unwrap();
        g.set_block(b, c).unwrap();
        assert!(!g.can_start(c, |id| id == a));
        assert!(g.can_start(c, |_| true));
    }

    // --- Relates ---

    #[test]
    fn test_add_relates_to_creates_bidirectional_edge() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.relate(a, b).unwrap();
        assert_eq!(g.related(a), vec![b]);
        assert_eq!(g.related(b), vec![a]);
    }

    #[test]
    fn test_relate_with_kind_preserves_metadata() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.relate_with_kind(a, b, super::super::RelatesKind::Duplicates)
            .unwrap();
        assert_eq!(
            g.relates_edges()[0].kind,
            super::super::RelatesKind::Duplicates
        );
    }

    #[test]
    fn test_relate_default_is_general_kind() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.relate(a, b).unwrap();
        assert_eq!(
            g.relates_edges()[0].kind,
            super::super::RelatesKind::General
        );
    }

    #[test]
    fn test_add_relates_to_self_reference_returns_error() {
        let (a, _, _) = ids();
        let mut g = DependencyGraph::new();
        assert!(g.relate(a, a).unwrap_err().is_self_reference());
    }

    #[test]
    fn test_add_relates_permits_cycle() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.relate(a, b).unwrap();
        g.relate(b, c).unwrap();
        assert!(g.relate(c, a).is_ok());
    }

    // --- Cross-cutting cascades ---

    #[test]
    fn test_archive_node_hides_edges_in_all_subgraphs() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(b, a).unwrap();
        g.set_block(a, c).unwrap();
        g.relate(a, c).unwrap();
        g.archive_node(a);
        assert!(g.children(a).is_empty());
        assert!(g.blocked(a).is_empty());
        assert!(g.related(a).is_empty());
        assert_eq!(g.len(), 3);
        assert_eq!(g.active_len(), 0);
    }

    #[test]
    fn test_remove_node_removes_edges_in_all_subgraphs() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(b, a).unwrap();
        g.set_block(a, c).unwrap();
        g.relate(a, c).unwrap();
        g.remove_node(a);
        assert_eq!(g.len(), 0);
    }

    // --- Cross-cutting tolerant removal ---

    #[test]
    fn test_disconnect_returns_true_when_edge_existed_in_any_subgraph() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.set_block(a, b).unwrap();
        assert!(g.disconnect(a, b), "edge existed in blocks; expected true");
        assert!(!g.disconnect(a, b), "edge already gone; expected false");
    }

    #[test]
    fn test_disconnect_returns_false_when_no_edge_exists() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        assert!(!g.disconnect(a, b), "no edge present in any subgraph");
    }

    #[test]
    fn test_disconnect_removes_from_every_subgraph_holding_pair() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(b, a).unwrap();
        g.set_block(a, b).unwrap();
        assert!(g.disconnect(a, b));
        assert!(g.children(a).is_empty());
        assert!(g.blocked(a).is_empty());
    }

    #[test]
    fn test_disconnect_preserves_opposite_orientation_edges_in_dag_subgraphs() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(a, b).unwrap();
        assert!(
            !g.disconnect(a, b),
            "no a->b edge exists; nothing should be removed"
        );
        assert_eq!(g.parents(a), vec![b]);
    }

    #[test]
    fn test_disconnect_removes_undirected_edge_in_either_query_orientation() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.relate(a, b).unwrap();
        assert!(g.disconnect(b, a), "undirected edges are symmetric");
        assert!(g.related(a).is_empty());
    }

    #[test]
    fn test_parent_child_and_blocks_are_independent() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(b, a).unwrap();
        g.set_block(b, c).unwrap();
        assert_eq!(g.children(a), vec![b]);
        assert_eq!(g.blocked(b), vec![c]);
    }
}
