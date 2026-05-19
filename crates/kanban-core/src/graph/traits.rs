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
    fn add_edge(
        &mut self,
        from: Self::NodeId,
        to: Self::NodeId,
    ) -> Result<(), GraphError>;

    /// Remove the edge `from -> to`. Returns [`GraphError::EdgeNotFound`]
    /// if no such edge exists.
    fn remove_edge(
        &mut self,
        from: Self::NodeId,
        to: Self::NodeId,
    ) -> Result<(), GraphError>;

    /// True iff an edge `from -> to` is present.
    fn contains_edge(&self, from: Self::NodeId, to: Self::NodeId) -> bool;

    /// Direct successors of `node`.
    fn outgoing(&self, node: Self::NodeId) -> Vec<Self::NodeId>;

    /// Direct predecessors of `node`. For undirected graphs this returns
    /// the same set as `outgoing`.
    fn incoming(&self, node: Self::NodeId) -> Vec<Self::NodeId>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_trait_is_object_safe_for_uuid_nodes() {
        fn _accepts_dyn(_: &dyn Graph<NodeId = Uuid>) {}
    }
}
