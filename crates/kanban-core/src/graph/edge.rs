use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
/// edges, `UndirectedGraph` carries undirected edges.
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

#[cfg(test)]
mod tests {
    use super::*;

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
    /// algorithms can take a uniform view without knowing the
    /// concrete edge type. Object-safety is the load-bearing
    /// property here; if a trait method ever takes `Self` without a
    /// `where Self: Sized` bound, this test fails to compile.
    #[test]
    fn test_edge_trait_is_object_safe() {
        fn _accepts_dyn(_e: &dyn Edge) {}
        fn _accepts_dyn_mut(_e: &mut dyn Edge) {}
    }
}
