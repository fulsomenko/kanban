use uuid::Uuid;

use super::error::GraphError;

/// Trait for entities that can participate in a graph
///
/// Implemented by domain entities like Card, Sprint, Board, etc.
/// Provides a unique identifier for graph node operations.
pub trait GraphNode {
    /// Get the unique identifier for this node
    fn node_id(&self) -> Uuid;
}

/// Common interface for graph data structures.
///
/// Implementors decide what `add_edge` enforces (acyclicity for DAGs,
/// no constraint beyond self-reference for undirected graphs). For
/// undirected graphs `outgoing` and `incoming` return the same neighbour
/// set; for directed graphs they return distinct sets.
///
/// Object-safe with an explicit `NodeId` binding, e.g.
/// `&dyn Graph<NodeId = Uuid>`.
pub trait Graph {
    type NodeId: Copy + Eq + std::hash::Hash;

    /// Add an edge `from -> to`. Returns [`GraphError::Cycle`] /
    /// [`GraphError::SelfReference`] when the implementation rejects.
    fn add_edge(&mut self, from: Self::NodeId, to: Self::NodeId) -> Result<(), GraphError>;

    /// Remove the edge `from -> to`. Returns [`GraphError::EdgeNotFound`]
    /// if no such edge exists.
    fn remove_edge(&mut self, from: Self::NodeId, to: Self::NodeId) -> Result<(), GraphError>;

    /// True iff an edge `from -> to` is present.
    fn contains_edge(&self, from: Self::NodeId, to: Self::NodeId) -> bool;

    /// Direct successors of `node`.
    fn outgoing(&self, node: Self::NodeId) -> Vec<Self::NodeId>;

    /// Direct predecessors of `node`. For undirected graphs this returns
    /// the same set as `outgoing`.
    fn incoming(&self, node: Self::NodeId) -> Vec<Self::NodeId>;
}

/// Soft-delete and aggregate operations that every node-keyed graph
/// container in this workspace supports, on top of the basic [`Graph`]
/// edge surface.
///
/// `SubGraph: Graph<NodeId = Uuid>` so a `&dyn SubGraph` automatically
/// provides the directed-graph vocabulary too. Both [`super::DagGraph`]
/// and [`super::UndirectedGraph`] implement this — composite types
/// (e.g. `kanban_domain::DependencyGraph`) can iterate their sub-graphs
/// uniformly without per-field hard-coding.
pub trait SubGraph: Graph<NodeId = Uuid> {
    /// Archive every edge involving `node` (soft delete).
    fn archive_node(&mut self, node: Uuid);
    /// Unarchive every edge involving `node`.
    fn unarchive_node(&mut self, node: Uuid);
    /// Remove every edge involving `node` (hard delete).
    fn remove_node(&mut self, node: Uuid);
    /// Total edges (active + archived).
    fn edge_count(&self) -> usize;
    /// Active edges only.
    fn active_edge_count(&self) -> usize;
    /// True iff any (active or archived) edge between `a` and `b` exists.
    fn has_edge(&self, a: Uuid, b: Uuid) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_trait_is_object_safe_for_uuid_nodes() {
        fn _accepts_dyn(_: &dyn Graph<NodeId = Uuid>) {}
    }

    #[test]
    fn test_subgraph_trait_is_object_safe() {
        fn _accepts_dyn(_: &dyn SubGraph) {}
        fn _accepts_dyn_mut(_: &mut dyn SubGraph) {}
    }
}
