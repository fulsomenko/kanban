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

/// Minimal contract every graph data structure satisfies.
///
/// Direction-agnostic: only methods that make sense for both directed
/// and undirected graphs live here. Directional access (`outgoing` /
/// `incoming`) lives on [`Directed`]; undirected neighbour access lives
/// on [`Undirected`]. A type that needs both must pick the right
/// subtype — there is no way to ask an [`Undirected`] graph for
/// directed semantics and silently get a wrong answer.
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
}

/// Directed-graph vocabulary: distinct outgoing and incoming neighbour
/// sets. Implementations preserve edge direction; calling
/// `outgoing(node)` returns successors, `incoming(node)` returns
/// predecessors.
///
/// Only types that genuinely encode direction implement this. An
/// [`Undirected`] graph cannot accidentally satisfy this trait by
/// returning the same set for both methods.
pub trait Directed: Graph {
    fn outgoing(&self, node: Self::NodeId) -> Vec<Self::NodeId>;
    fn incoming(&self, node: Self::NodeId) -> Vec<Self::NodeId>;
}

/// Undirected-graph vocabulary: a single neighbour set per node.
/// Calling `neighbors(node)` returns every node connected to `node` by
/// any edge, ignoring orientation. Implementations that genuinely lack
/// edge direction expose only this surface — directed callers must
/// require [`Directed`] explicitly.
pub trait Undirected: Graph {
    fn neighbors(&self, node: Self::NodeId) -> Vec<Self::NodeId>;
}

/// Soft-delete and aggregate operations every node-keyed graph
/// container in this workspace supports, on top of the basic [`Graph`]
/// edge surface.
///
/// `SubGraph: Graph<NodeId = Uuid>` so a `&dyn SubGraph` lets callers
/// add/remove edges and inspect existence without committing to either
/// directed or undirected semantics. Both [`super::DagGraph`] and
/// [`super::UndirectedGraph`] implement this — composite types (e.g.
/// `kanban_domain::DependencyGraph`) iterate their sub-graphs
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
    fn test_directed_trait_is_object_safe() {
        fn _accepts_dyn(_: &dyn Directed<NodeId = Uuid>) {}
    }

    #[test]
    fn test_undirected_trait_is_object_safe() {
        fn _accepts_dyn(_: &dyn Undirected<NodeId = Uuid>) {}
    }

    #[test]
    fn test_subgraph_trait_is_object_safe() {
        fn _accepts_dyn(_: &dyn SubGraph) {}
        fn _accepts_dyn_mut(_: &mut dyn SubGraph) {}
    }
}
