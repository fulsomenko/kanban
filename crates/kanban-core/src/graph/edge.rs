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

/// A weighted edge between two nodes.
///
/// The relationship kind (parent-of, blocks, relates-to, ...) is
/// encoded by which sub-graph the edge lives in — see
/// [`crate::graph::DagGraph`] and [`crate::graph::UndirectedGraph`] —
/// not by a per-edge field. Dropping the previous `E` type parameter
/// also drops the serde-required `E: Default` constraint and the
/// `edge_type: null` placeholder that used to ride along on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
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

impl Edge {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_returns_active_edge_with_endpoints_set() {
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        let edge = Edge::new(source, target, EdgeDirection::Directed);

        assert_eq!(edge.source, source);
        assert_eq!(edge.target, target);
        assert!(edge.is_active());
        assert!(!edge.is_archived());
    }

    #[test]
    fn test_archive_then_unarchive_round_trips_active_state() {
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        let mut edge = Edge::new(source, target, EdgeDirection::Directed);

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
        let edge = Edge::new(source, target, EdgeDirection::Directed);

        assert!(edge.involves(source));
        assert!(edge.involves(target));
        assert!(!edge.involves(other));
    }

    #[test]
    fn test_connects_directed_only_matches_source_to_target_order() {
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        let edge = Edge::new(source, target, EdgeDirection::Directed);

        assert!(edge.connects(source, target));
        assert!(!edge.connects(target, source));
    }

    #[test]
    fn test_connects_bidirectional_matches_either_endpoint_order() {
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let edge = Edge::new(node_a, node_b, EdgeDirection::Bidirectional);

        assert!(edge.connects(node_a, node_b));
        assert!(edge.connects(node_b, node_a));
    }
}
