use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Direction of an edge in the graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeDirection {
    /// A -> B (source points to target, one-way relationship)
    Directed,
    /// A <-> B (bidirectional relationship)
    Bidirectional,
}

/// Common edge data: endpoints plus creation / archival timestamps.
///
/// Per-kind edge structs embed this and add their own kind-specific
/// fields. Concrete kinds live in domain crates (kanban-domain holds
/// `SpawnsEdge` / `BlocksEdge` / `RelatesEdge`); external crates can
/// define their own structs the same way. The graph machinery is
/// generic over [`Edge`], so any struct that implements the trait
/// works in [`crate::graph::DagGraph`] / [`crate::graph::UndirectedGraph`].
///
/// Directionality is *not* on `EdgeBase`: it's encoded by the
/// sub-graph type the edge lives in. `DagGraph` carries directed
/// edges, `UndirectedGraph` carries undirected edges. The previous
/// `EdgeDirection` field was dead data after the V6 sub-graph split.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeBase {
    pub source: Uuid,
    pub target: Uuid,
    pub created_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

impl EdgeBase {
    pub fn new(source: Uuid, target: Uuid) -> Self {
        Self {
            source,
            target,
            created_at: Utc::now(),
            archived_at: None,
        }
    }
}

/// Read+archive trait satisfied by every edge type the graph
/// machinery handles. Concrete edges (`SpawnsEdge`, `BlocksEdge`,
/// `RelatesEdge`, and any future external kinds) implement this so
/// `DagGraph<E: Edge>` and `UndirectedGraph<E: Edge>` can operate on
/// them uniformly without knowing kind-specific metadata.
///
/// Direction is encoded by the sub-graph type, so `connects` is not
/// on this trait: `DagGraph` checks strict `source == a && target ==
/// b`, while `UndirectedGraph` checks either ordering. Cross-kind
/// algorithms that don't need direction semantics use `source()` /
/// `target()` directly.
pub trait Edge {
    fn source(&self) -> Uuid;
    fn target(&self) -> Uuid;
    fn created_at(&self) -> DateTime<Utc>;
    fn archived_at(&self) -> Option<DateTime<Utc>>;

    fn is_active(&self) -> bool {
        self.archived_at().is_none()
    }
    fn is_archived(&self) -> bool {
        self.archived_at().is_some()
    }
    fn involves(&self, node: Uuid) -> bool {
        self.source() == node || self.target() == node
    }

    fn archive(&mut self);
    fn unarchive(&mut self);

    /// Construct an edge from just its endpoints, filling per-kind
    /// metadata with defaults. Lets the generic `Graph::add_edge` /
    /// `UndirectedGraph::add_edge` trait methods work uniformly
    /// across edge kinds without taking kind-specific parameters.
    /// Kind-specific code that wants to set metadata explicitly
    /// constructs the concrete struct directly (e.g.
    /// `BlocksEdge::new(blocker, blocked, Severity::High)`) and
    /// pushes it via `add_edge_with_metadata`.
    ///
    /// `where Self: Sized` keeps the trait object-safe (methods
    /// taking ownership / returning `Self` aren't callable on
    /// `&dyn Edge` anyway, but the trait itself stays usable as a
    /// trait object for cross-kind read code).
    fn from_endpoints(source: Uuid, target: Uuid) -> Self
    where
        Self: Sized;
}

impl Edge for EdgeBase {
    fn source(&self) -> Uuid {
        self.source
    }
    fn target(&self) -> Uuid {
        self.target
    }
    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    fn archived_at(&self) -> Option<DateTime<Utc>> {
        self.archived_at
    }
    fn archive(&mut self) {
        if self.archived_at.is_none() {
            self.archived_at = Some(Utc::now());
        }
    }
    fn unarchive(&mut self) {
        self.archived_at = None;
    }
    fn from_endpoints(source: Uuid, target: Uuid) -> Self {
        EdgeBase::new(source, target)
    }
}

/// A weighted edge between two nodes.
///
/// The relationship kind (parent-of, blocks, relates-to, ...) is
/// encoded by which sub-graph the edge lives in — see
/// [`crate::graph::DagGraph`] and [`crate::graph::UndirectedGraph`] —
/// not by a per-edge field. Dropping the previous `E` type parameter
/// also drops the serde-required `E: Default` constraint and the
/// `edge_type: null` placeholder that used to ride along on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyEdge {
    /// Source node identifier
    pub source: Uuid,
    /// Target node identifier
    pub target: Uuid,
    /// Direction of the edge
    pub direction: EdgeDirection,
    /// Optional weight for weighted graph algorithms
    pub weight: Option<f32>,
    /// When this edge was created
    pub created_at: DateTime<Utc>,
    /// When this edge was archived (None = active)
    pub archived_at: Option<DateTime<Utc>>,
}

impl LegacyEdge {
    /// Create a new edge
    pub fn new(source: Uuid, target: Uuid, direction: EdgeDirection) -> Self {
        Self {
            source,
            target,
            direction,
            weight: None,
            created_at: Utc::now(),
            archived_at: None,
        }
    }

    /// Check if this edge is archived
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    /// Check if this edge is active (not archived)
    pub fn is_active(&self) -> bool {
        self.archived_at.is_none()
    }

    /// Archive this edge
    pub fn archive(&mut self) {
        if self.archived_at.is_none() {
            self.archived_at = Some(Utc::now());
        }
    }

    /// Unarchive this edge
    pub fn unarchive(&mut self) {
        self.archived_at = None;
    }

    /// Check if this edge involves a given node (source or target)
    pub fn involves(&self, node_id: Uuid) -> bool {
        self.source == node_id || self.target == node_id
    }

    /// Check if this edge connects two specific nodes (in either direction for bidirectional)
    pub fn connects(&self, node_a: Uuid, node_b: Uuid) -> bool {
        match self.direction {
            EdgeDirection::Directed => self.source == node_a && self.target == node_b,
            EdgeDirection::Bidirectional => {
                (self.source == node_a && self.target == node_b)
                    || (self.source == node_b && self.target == node_a)
            }
        }
    }
}

impl Edge for LegacyEdge {
    fn source(&self) -> Uuid {
        self.source
    }
    fn target(&self) -> Uuid {
        self.target
    }
    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    fn archived_at(&self) -> Option<DateTime<Utc>> {
        self.archived_at
    }
    fn archive(&mut self) {
        LegacyEdge::archive(self);
    }
    fn unarchive(&mut self) {
        LegacyEdge::unarchive(self);
    }
    fn from_endpoints(source: Uuid, target: Uuid) -> Self {
        // Transitional impl: LegacyEdge default direction is
        // Directed. UndirectedGraph<LegacyEdge> callers that need
        // Bidirectional construct the edge explicitly. Once step 8
        // drops LegacyEdge this impl goes away.
        LegacyEdge::new(source, target, EdgeDirection::Directed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_returns_active_edge_with_endpoints_set() {
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        let edge = LegacyEdge::new(source, target, EdgeDirection::Directed);

        assert_eq!(edge.source, source);
        assert_eq!(edge.target, target);
        assert!(edge.is_active());
        assert!(!edge.is_archived());
    }

    #[test]
    fn test_archive_then_unarchive_round_trips_active_state() {
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        let mut edge = LegacyEdge::new(source, target, EdgeDirection::Directed);

        edge.archive();
        assert!(edge.is_archived());
        assert!(!edge.is_active());

        edge.unarchive();
        assert!(edge.is_active());
        assert!(!edge.is_archived());
    }

    #[test]
    fn test_involves_returns_true_for_both_endpoints_only() {
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        let other = Uuid::new_v4();
        let edge = LegacyEdge::new(source, target, EdgeDirection::Directed);

        assert!(edge.involves(source));
        assert!(edge.involves(target));
        assert!(!edge.involves(other));
    }

    #[test]
    fn test_connects_directed_only_matches_source_to_target_order() {
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        let edge = LegacyEdge::new(source, target, EdgeDirection::Directed);

        assert!(edge.connects(source, target));
        assert!(!edge.connects(target, source));
    }

    #[test]
    fn test_connects_bidirectional_matches_either_endpoint_order() {
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let edge = LegacyEdge::new(node_a, node_b, EdgeDirection::Bidirectional);

        assert!(edge.connects(node_a, node_b));
        assert!(edge.connects(node_b, node_a));
    }

    // --- EdgeBase + Edge trait ---

    #[test]
    fn test_edge_base_new_sets_endpoints_and_active_state() {
        let s = Uuid::new_v4();
        let t = Uuid::new_v4();
        let base = EdgeBase::new(s, t);
        assert_eq!(<EdgeBase as Edge>::source(&base), s);
        assert_eq!(<EdgeBase as Edge>::target(&base), t);
        assert!(base.is_active());
        assert!(!base.is_archived());
    }

    #[test]
    fn test_edge_trait_archive_then_unarchive_round_trips_state() {
        let mut base = EdgeBase::new(Uuid::new_v4(), Uuid::new_v4());
        base.archive();
        assert!(base.is_archived());
        base.unarchive();
        assert!(base.is_active());
    }

    #[test]
    fn test_edge_trait_involves_returns_true_for_both_endpoints_only() {
        let s = Uuid::new_v4();
        let t = Uuid::new_v4();
        let other = Uuid::new_v4();
        let base = EdgeBase::new(s, t);
        assert!(base.involves(s));
        assert!(base.involves(t));
        assert!(!base.involves(other));
    }

    /// The Edge trait must be usable as a `&dyn Edge` so cross-kind
    /// algorithms can take a uniform view without knowing the concrete
    /// edge type. Object-safety is the load-bearing property here;
    /// if a trait method ever takes `Self`, this test fails to compile.
    #[test]
    fn test_edge_trait_is_object_safe() {
        fn _accepts_dyn(_e: &dyn Edge) {}
        fn _accepts_dyn_mut(_e: &mut dyn Edge) {}
    }

    #[test]
    fn test_legacy_edge_implements_edge_trait() {
        // Transition smoke test: the renamed legacy struct continues
        // to satisfy the new trait so old callers can be migrated
        // incrementally without breaking the world.
        let s = Uuid::new_v4();
        let t = Uuid::new_v4();
        let mut e = LegacyEdge::new(s, t, EdgeDirection::Directed);
        assert_eq!(<LegacyEdge as Edge>::source(&e), s);
        assert_eq!(<LegacyEdge as Edge>::target(&e), t);
        <LegacyEdge as Edge>::archive(&mut e);
        assert!(<LegacyEdge as Edge>::is_archived(&e));
    }
}
