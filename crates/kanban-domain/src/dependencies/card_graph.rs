use chrono::Utc;
use kanban_core::{Edge, EdgeDirection, Graph, KanbanError, KanbanResult};
use uuid::Uuid;

use super::CardEdgeType;
use crate::CardId;

/// Type alias for card dependency graph
pub type CardDependencyGraph = Graph<CardEdgeType>;

/// Extension trait for card-specific graph operations
pub trait CardGraphExt {
    /// Add a blocking relationship (A blocks B)
    /// Returns error if would create cycle
    fn add_blocks(&mut self, blocker: CardId, blocked: CardId) -> KanbanResult<()>;

    /// Add a relates-to relationship (A relates to B, bidirectional)
    fn add_relates_to(&mut self, card_a: CardId, card_b: CardId) -> KanbanResult<()>;

    /// Get all cards that block a given card (incoming Blocks edges)
    fn blockers(&self, card_id: CardId) -> Vec<CardId>;

    /// Get all cards blocked by a given card (outgoing Blocks edges)
    fn blocked_by(&self, card_id: CardId) -> Vec<CardId>;

    /// Get all related cards (RelatesTo edges, either direction)
    fn related(&self, card_id: CardId) -> Vec<CardId>;

    /// Check if card can be started (no incomplete blockers)
    ///
    /// Takes a closure that checks if a card is complete.
    /// Returns true if all blocking cards are complete.
    fn can_start<F>(&self, card_id: CardId, is_complete: F) -> bool
    where
        F: Fn(CardId) -> bool;
}

impl CardGraphExt for CardDependencyGraph {
    fn add_blocks(&mut self, blocker: CardId, blocked: CardId) -> KanbanResult<()> {
        if blocker == blocked {
            return Err(KanbanError::SelfReference);
        }

        if self.would_create_cycle(blocker, blocked) {
            return Err(KanbanError::CycleDetected);
        }

        let edge = Edge {
            source: blocker,
            target: blocked,
            edge_type: CardEdgeType::Blocks,
            direction: EdgeDirection::Directed,
            weight: None,
            created_at: Utc::now(),
            archived_at: None,
        };

        self.add_edge(edge)
    }

    fn add_relates_to(&mut self, card_a: CardId, card_b: CardId) -> KanbanResult<()> {
        if card_a == card_b {
            return Err(KanbanError::SelfReference);
        }

        let edge = Edge {
            source: card_a,
            target: card_b,
            edge_type: CardEdgeType::RelatesTo,
            direction: EdgeDirection::Bidirectional,
            weight: None,
            created_at: Utc::now(),
            archived_at: None,
        };

        self.add_edge(edge)
    }

    fn blockers(&self, card_id: CardId) -> Vec<CardId> {
        self.incoming_active(card_id)
            .into_iter()
            .filter(|e| e.edge_type == CardEdgeType::Blocks)
            .map(|e| e.source)
            .collect()
    }

    fn blocked_by(&self, card_id: CardId) -> Vec<CardId> {
        self.outgoing_active(card_id)
            .into_iter()
            .filter(|e| e.edge_type == CardEdgeType::Blocks)
            .map(|e| e.target)
            .collect()
    }

    fn related(&self, card_id: CardId) -> Vec<CardId> {
        let mut related = Vec::new();

        for edge in self.active_edges() {
            if edge.edge_type == CardEdgeType::RelatesTo {
                if edge.source == card_id {
                    related.push(edge.target);
                } else if edge.target == card_id {
                    related.push(edge.source);
                }
            }
        }

        related
    }

    fn can_start<F>(&self, card_id: CardId, is_complete: F) -> bool
    where
        F: Fn(CardId) -> bool,
    {
        let blockers = self.blockers(card_id);
        blockers.into_iter().all(is_complete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_blocks() {
        let mut graph = CardDependencyGraph::new();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        assert!(graph.add_blocks(card_a, card_b).is_ok());
        assert_eq!(graph.blocked_by(card_a).len(), 1);
        assert_eq!(graph.blockers(card_b).len(), 1);
    }

    #[test]
    fn test_add_blocks_prevents_cycle() {
        let mut graph = CardDependencyGraph::new();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();
        let card_c = Uuid::new_v4();

        graph.add_blocks(card_a, card_b).unwrap();
        graph.add_blocks(card_b, card_c).unwrap();

        let result = graph.add_blocks(card_c, card_a);
        assert!(result.is_err());
        assert!(matches!(result, Err(KanbanError::CycleDetected)));
    }

    #[test]
    fn test_add_blocks_prevents_self_reference() {
        let mut graph = CardDependencyGraph::new();
        let card_a = Uuid::new_v4();

        let result = graph.add_blocks(card_a, card_a);
        assert!(result.is_err());
        assert!(matches!(result, Err(KanbanError::SelfReference)));
    }

    #[test]
    fn test_add_relates_to() {
        let mut graph = CardDependencyGraph::new();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        assert!(graph.add_relates_to(card_a, card_b).is_ok());

        let related_a = graph.related(card_a);
        let related_b = graph.related(card_b);

        assert_eq!(related_a.len(), 1);
        assert_eq!(related_b.len(), 1);
        assert!(related_a.contains(&card_b));
        assert!(related_b.contains(&card_a));
    }

    #[test]
    fn test_blockers_and_blocked_by() {
        let mut graph = CardDependencyGraph::new();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();
        let card_c = Uuid::new_v4();

        graph.add_blocks(card_a, card_b).unwrap();
        graph.add_blocks(card_c, card_b).unwrap();

        let blockers = graph.blockers(card_b);
        assert_eq!(blockers.len(), 2);
        assert!(blockers.contains(&card_a));
        assert!(blockers.contains(&card_c));

        let blocked_by_a = graph.blocked_by(card_a);
        assert_eq!(blocked_by_a.len(), 1);
        assert!(blocked_by_a.contains(&card_b));
    }

    #[test]
    fn test_can_start() {
        let mut graph = CardDependencyGraph::new();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();
        let card_c = Uuid::new_v4();

        graph.add_blocks(card_a, card_c).unwrap();
        graph.add_blocks(card_b, card_c).unwrap();

        let is_complete = |id: Uuid| id == card_a;

        assert!(!graph.can_start(card_c, is_complete));

        let is_complete_all = |_id: Uuid| true;
        assert!(graph.can_start(card_c, is_complete_all));
    }

    #[test]
    fn test_archived_edges_excluded() {
        let mut graph = CardDependencyGraph::new();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();

        graph.add_blocks(card_a, card_b).unwrap();
        assert_eq!(graph.blockers(card_b).len(), 1);

        graph.archive_node(card_a);
        assert_eq!(graph.blockers(card_b).len(), 0);

        graph.unarchive_node(card_a);
        assert_eq!(graph.blockers(card_b).len(), 1);
    }
}
