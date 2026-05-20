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

/// Node-keyed cascade operations: archive, unarchive, and hard-remove
/// every edge involving a given node.
///
/// Strictly mutating. Distinct from [`EdgeStats`] (which is read-only)
/// because the two surfaces are independent — a caller that just
/// counts edges doesn't need cascade authority, and a caller doing
/// soft-deletes doesn't need to ask for counts. Splitting them keeps
/// each trait's name accurate to its single purpose.
pub trait Cascadable: Graph<NodeId = Uuid> {
    /// Archive every edge involving `node` (soft delete).
    fn archive_node(&mut self, node: Uuid);
    /// Unarchive every edge involving `node`.
    fn unarchive_node(&mut self, node: Uuid);
    /// Remove every edge involving `node` (hard delete).
    fn remove_node(&mut self, node: Uuid);
}

/// Edge-level aggregate and existence queries: counts and
/// presence-checks across both active and archived edges.
///
/// Strictly read-only. Lives separately from [`Cascadable`] so a
/// generic consumer can require only the surface it actually uses —
/// no implicit mutation authority leaking into inspection code.
pub trait EdgeStats: Graph<NodeId = Uuid> {
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
    fn test_cascadable_trait_is_object_safe() {
        fn _accepts_dyn(_: &dyn Cascadable) {}
        fn _accepts_dyn_mut(_: &mut dyn Cascadable) {}
    }

    #[test]
    fn test_edge_stats_trait_is_object_safe() {
        fn _accepts_dyn(_: &dyn EdgeStats) {}
    }
}
