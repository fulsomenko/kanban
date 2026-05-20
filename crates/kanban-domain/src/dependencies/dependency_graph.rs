use kanban_core::{DagGraph, Graph, GraphError, UndirectedGraph};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DependencyError;
use crate::{CardId, KanbanResult};

/// Top-level container for all entity dependency graphs.
///
/// Three discrete sub-graphs, each with its own structural rules:
///
/// - `parent_child`: directed acyclic, cycle + self-reference rejected
/// - `blocks`: directed acyclic, cycle + self-reference rejected
/// - `relates`: undirected, self-reference rejected (cycles permitted)
///
/// Cross-cutting operations (`archive_node`, `unarchive_node`,
/// `remove_node`) cascade across all three. Per-edge-type convenience
/// methods (`set_parent`, `add_blocks`, `add_relates`, etc.) delegate
/// to the matching sub-graph and convert [`GraphError`] into the
/// domain-level [`DependencyError`] / [`crate::KanbanError`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DependencyGraph {
    #[serde(default)]
    pub parent_child: DagGraph,
    #[serde(default)]
    pub blocks: DagGraph,
    #[serde(default)]
    pub relates: UndirectedGraph,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    // --- Cross-cutting node cascades ---

    /// Archive every edge involving `card` across all three sub-graphs.
    pub fn archive_node(&mut self, card: CardId) {
        self.parent_child.archive_node(card);
        self.blocks.archive_node(card);
        self.relates.archive_node(card);
    }

    /// Unarchive every edge involving `card` across all three sub-graphs.
    pub fn unarchive_node(&mut self, card: CardId) {
        self.parent_child.unarchive_node(card);
        self.blocks.unarchive_node(card);
        self.relates.unarchive_node(card);
    }

    /// Hard-remove every edge involving `card` across all three sub-graphs.
    pub fn remove_node(&mut self, card: CardId) {
        self.parent_child.remove_node(card);
        self.blocks.remove_node(card);
        self.relates.remove_node(card);
    }

    /// Total edge count across all three sub-graphs (active + archived).
    pub fn edge_count(&self) -> usize {
        self.parent_child.edge_count() + self.blocks.edge_count() + self.relates.edge_count()
    }

    /// Active edge count across all three sub-graphs.
    pub fn active_edge_count(&self) -> usize {
        self.parent_child.active_edge_count()
            + self.blocks.active_edge_count()
            + self.relates.active_edge_count()
    }

    // --- Parent / child ---

    /// Add a `parent -> child` parent-of edge.
    pub fn set_parent(&mut self, child: CardId, parent: CardId) -> KanbanResult<()> {
        self.parent_child.add_edge(parent, child).map_err(dep_err)
    }

    /// Remove the `parent -> child` parent-of edge.
    pub fn remove_parent(&mut self, child: CardId, parent: CardId) -> KanbanResult<()> {
        self.parent_child
            .remove_edge(parent, child)
            .map_err(dep_err)
    }

    /// Direct children of `parent`.
    pub fn children(&self, parent: CardId) -> Vec<CardId> {
        self.parent_child.outgoing(parent)
    }

    /// Direct parents of `child`.
    pub fn parents(&self, child: CardId) -> Vec<CardId> {
        self.parent_child.incoming(child)
    }

    /// Transitive ancestors of `child`.
    pub fn ancestors(&self, child: CardId) -> Vec<CardId> {
        self.parent_child.ancestors(child)
    }

    /// Transitive descendants of `parent`.
    pub fn descendants(&self, parent: CardId) -> Vec<CardId> {
        self.parent_child.descendants(parent)
    }

    /// Count of direct children (for the `[N]` badge).
    pub fn child_count(&self, parent: CardId) -> usize {
        self.parent_child.outgoing(parent).len()
    }

    // --- Blocks ---

    /// Add a `blocker -> blocked` blocks edge.
    pub fn add_blocks(&mut self, blocker: CardId, blocked: CardId) -> KanbanResult<()> {
        self.blocks.add_edge(blocker, blocked).map_err(dep_err)
    }

    /// Remove the `blocker -> blocked` blocks edge.
    pub fn remove_blocks(&mut self, blocker: CardId, blocked: CardId) -> KanbanResult<()> {
        self.blocks.remove_edge(blocker, blocked).map_err(dep_err)
    }

    /// Cards `card` blocks (outgoing blocks edges).
    pub fn blocks_targets(&self, card: CardId) -> Vec<CardId> {
        self.blocks.outgoing(card)
    }

    /// Cards that block `card` (incoming blocks edges).
    pub fn blockers(&self, card: CardId) -> Vec<CardId> {
        self.blocks.incoming(card)
    }

    /// True if every blocker of `card` is complete.
    pub fn can_start<F>(&self, card: CardId, is_complete: F) -> bool
    where
        F: Fn(CardId) -> bool,
    {
        self.blockers(card).into_iter().all(is_complete)
    }

    // --- Relates ---

    /// Add an undirected `a <-> b` relates edge.
    pub fn add_relates_to(&mut self, a: CardId, b: CardId) -> KanbanResult<()> {
        self.relates.add_edge(a, b).map_err(dep_err)
    }

    /// Remove the undirected `a <-> b` relates edge.
    pub fn remove_relates_to(&mut self, a: CardId, b: CardId) -> KanbanResult<()> {
        self.relates.remove_edge(a, b).map_err(dep_err)
    }

    /// Cards related to `card` via any active relates edge.
    pub fn related(&self, card: CardId) -> Vec<CardId> {
        self.relates.neighbors(card)
    }

    /// True iff any edge between `a` and `b` exists in any sub-graph
    /// (active or archived). Cross-cutting check across all three.
    pub fn has_edge(&self, a: Uuid, b: Uuid) -> bool {
        self.parent_child.has_edge(a, b)
            || self.blocks.has_edge(a, b)
            || self.relates.has_edge(a, b)
    }

    /// Tolerant cross-cutting edge removal: removes the `a -> b` edge
    /// from every sub-graph it appears in. Returns `true` iff at least
    /// one sub-graph held the edge — lets callers distinguish "the
    /// pair was disconnected" from "no edge was there to remove"
    /// without needing per-type knowledge.
    pub fn try_remove_edge(&mut self, a: Uuid, b: Uuid) -> bool {
        let removed_parent = self.parent_child.remove_edge(a, b).is_ok();
        let removed_blocks = self.blocks.remove_edge(a, b).is_ok();
        let removed_relates = self.relates.remove_edge(a, b).is_ok();
        removed_parent || removed_blocks || removed_relates
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
    fn test_dependency_graph_creation() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_dependency_graph_serialization() {
        let graph = DependencyGraph::new();
        let json = serde_json::to_string(&graph).unwrap();
        let deserialized: DependencyGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.edge_count(), 0);
    }

    // --- Parent/child convenience ---

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

    // --- Blocks convenience ---

    #[test]
    fn test_add_blocks_creates_directed_edge() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.add_blocks(a, b).unwrap();
        assert_eq!(g.blocks_targets(a), vec![b]);
        assert_eq!(g.blockers(b), vec![a]);
    }

    #[test]
    fn test_add_blocks_cycle_returns_error() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.add_blocks(a, b).unwrap();
        g.add_blocks(b, c).unwrap();
        assert!(g.add_blocks(c, a).unwrap_err().is_cycle_detected());
    }

    #[test]
    fn test_add_blocks_self_reference_returns_error() {
        let (a, _, _) = ids();
        let mut g = DependencyGraph::new();
        assert!(g.add_blocks(a, a).unwrap_err().is_self_reference());
    }

    #[test]
    fn test_can_start_returns_true_when_all_blockers_complete() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.add_blocks(a, c).unwrap();
        g.add_blocks(b, c).unwrap();
        assert!(!g.can_start(c, |id| id == a));
        assert!(g.can_start(c, |_| true));
    }

    // --- Relates convenience ---

    #[test]
    fn test_add_relates_to_creates_bidirectional_edge() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.add_relates_to(a, b).unwrap();
        assert_eq!(g.related(a), vec![b]);
        assert_eq!(g.related(b), vec![a]);
    }

    #[test]
    fn test_add_relates_to_self_reference_returns_error() {
        let (a, _, _) = ids();
        let mut g = DependencyGraph::new();
        assert!(g.add_relates_to(a, a).unwrap_err().is_self_reference());
    }

    #[test]
    fn test_add_relates_permits_cycle() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.add_relates_to(a, b).unwrap();
        g.add_relates_to(b, c).unwrap();
        assert!(g.add_relates_to(c, a).is_ok());
    }

    // --- Cross-cutting cascades ---

    #[test]
    fn test_archive_node_hides_edges_in_all_subgraphs() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(b, a).unwrap();
        g.add_blocks(a, c).unwrap();
        g.add_relates_to(a, c).unwrap();
        g.archive_node(a);
        assert!(g.children(a).is_empty());
        assert!(g.blocks_targets(a).is_empty());
        assert!(g.related(a).is_empty());
        assert_eq!(g.edge_count(), 3);
        assert_eq!(g.active_edge_count(), 0);
    }

    #[test]
    fn test_remove_node_removes_edges_in_all_subgraphs() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(b, a).unwrap();
        g.add_blocks(a, c).unwrap();
        g.add_relates_to(a, c).unwrap();
        g.remove_node(a);
        assert_eq!(g.edge_count(), 0);
    }

    // --- Cross-cutting tolerant removal ---

    #[test]
    fn test_try_remove_edge_returns_true_when_edge_existed_in_any_subgraph() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.add_blocks(a, b).unwrap();
        assert!(
            g.try_remove_edge(a, b),
            "edge existed in blocks; expected true"
        );
        assert!(
            !g.try_remove_edge(a, b),
            "edge already gone; expected false"
        );
    }

    #[test]
    fn test_try_remove_edge_returns_false_when_no_edge_exists() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        assert!(!g.try_remove_edge(a, b), "no edge present in any subgraph");
    }

    #[test]
    fn test_try_remove_edge_removes_from_every_subgraph_holding_pair() {
        let (a, b, _) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(b, a).unwrap();
        g.add_blocks(a, b).unwrap();
        assert!(g.try_remove_edge(a, b));
        assert!(g.children(a).is_empty());
        assert!(g.blocks_targets(a).is_empty());
    }

    #[test]
    fn test_parent_child_and_blocks_are_independent() {
        let (a, b, c) = ids();
        let mut g = DependencyGraph::new();
        g.set_parent(b, a).unwrap();
        g.add_blocks(b, c).unwrap();
        assert_eq!(g.children(a), vec![b]);
        assert_eq!(g.blocks_targets(b), vec![c]);
    }
}
