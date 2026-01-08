use chrono::Utc;
use kanban_core::{Edge, EdgeDirection, Graph, KanbanError, KanbanResult};

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

    /// Set parent for a card (creates ParentOf edge from parent to child)
    /// Returns error if would create cycle
    fn set_parent(&mut self, child_id: CardId, parent_id: CardId) -> KanbanResult<()>;

    /// Remove parent relationship
    fn remove_parent(&mut self, child_id: CardId, parent_id: CardId) -> KanbanResult<()>;

    /// Get direct children of a card (outgoing ParentOf edges)
    fn children(&self, parent_id: CardId) -> Vec<CardId>;

    /// Get direct parents of a card (incoming ParentOf edges)
    fn parents(&self, child_id: CardId) -> Vec<CardId>;

    /// Get all ancestors (for cycle prevention filtering in UI)
    fn ancestors(&self, child_id: CardId) -> Vec<CardId>;

    /// Get all descendants (for cycle prevention filtering in UI)
    fn descendants(&self, parent_id: CardId) -> Vec<CardId>;

    /// Count of direct children (for [N] badge)
    fn child_count(&self, parent_id: CardId) -> usize;
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

    fn set_parent(&mut self, child_id: CardId, parent_id: CardId) -> KanbanResult<()> {
        if child_id == parent_id {
            return Err(KanbanError::SelfReference);
        }

        // Check if adding this edge would create a cycle in the ParentOf subgraph
        // Build adjacency list for ParentOf edges only
        let mut adj_list = std::collections::HashMap::new();
        for edge in self.active_edges() {
            if edge.edge_type == CardEdgeType::ParentOf {
                adj_list
                    .entry(edge.source)
                    .or_insert_with(Vec::new)
                    .push(edge.target);
            }
        }

        if kanban_core::graph::algorithms::would_create_cycle(&adj_list, parent_id, child_id) {
            return Err(KanbanError::CycleDetected);
        }

        let edge = Edge {
            source: parent_id,
            target: child_id,
            edge_type: CardEdgeType::ParentOf,
            direction: EdgeDirection::Directed,
            weight: None,
            created_at: Utc::now(),
            archived_at: None,
        };

        self.add_edge(edge)
    }

    fn remove_parent(&mut self, child_id: CardId, parent_id: CardId) -> KanbanResult<()> {
        if self.remove_edge(parent_id, child_id) {
            Ok(())
        } else {
            Err(KanbanError::EdgeNotFound)
        }
    }

    fn children(&self, parent_id: CardId) -> Vec<CardId> {
        self.outgoing_active(parent_id)
            .into_iter()
            .filter(|e| e.edge_type == CardEdgeType::ParentOf)
            .map(|e| e.target)
            .collect()
    }

    fn parents(&self, child_id: CardId) -> Vec<CardId> {
        self.incoming_active(child_id)
            .into_iter()
            .filter(|e| e.edge_type == CardEdgeType::ParentOf)
            .map(|e| e.source)
            .collect()
    }

    fn ancestors(&self, child_id: CardId) -> Vec<CardId> {
        let mut ancestors = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Add all direct parents to start
        for parent in self.parents(child_id) {
            if visited.insert(parent) {
                ancestors.push(parent);
                queue.push_back(parent);
            }
        }

        // BFS traversal to find all ancestors
        while let Some(node) = queue.pop_front() {
            // Get parents of this node
            for edge in self.incoming_active(node) {
                if edge.edge_type == CardEdgeType::ParentOf {
                    if visited.insert(edge.source) {
                        ancestors.push(edge.source);
                        queue.push_back(edge.source);
                    }
                }
            }
        }

        ancestors
    }

    fn descendants(&self, parent_id: CardId) -> Vec<CardId> {
        // Build adjacency list for ParentOf edges only
        let mut adj_list = std::collections::HashMap::new();
        for edge in self.active_edges() {
            if edge.edge_type == CardEdgeType::ParentOf {
                adj_list
                    .entry(edge.source)
                    .or_insert_with(Vec::new)
                    .push(edge.target);
            }
        }

        let reachable = kanban_core::graph::algorithms::reachable_from(&adj_list, parent_id);

        // Remove the parent itself from the result
        reachable
            .into_iter()
            .filter(|&id| id != parent_id)
            .collect()
    }

    fn child_count(&self, parent_id: CardId) -> usize {
        self.children(parent_id).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

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

    // Parent/Child relationship tests

    #[test]
    fn test_set_parent() {
        let mut graph = CardDependencyGraph::new();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();

        assert!(graph.set_parent(child, parent).is_ok());

        let children = graph.children(parent);
        assert_eq!(children.len(), 1);
        assert!(children.contains(&child));

        let parents = graph.parents(child);
        assert_eq!(parents.len(), 1);
        assert!(parents.contains(&parent));
    }

    #[test]
    fn test_set_parent_prevents_cycle() {
        let mut graph = CardDependencyGraph::new();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();
        let card_c = Uuid::new_v4();

        // Create chain: A -> B -> C
        graph.set_parent(card_b, card_a).unwrap();
        graph.set_parent(card_c, card_b).unwrap();

        // Try to make A a child of C (would create cycle)
        let result = graph.set_parent(card_a, card_c);
        assert!(result.is_err());
        assert!(matches!(result, Err(KanbanError::CycleDetected)));
    }

    #[test]
    fn test_set_parent_prevents_self_reference() {
        let mut graph = CardDependencyGraph::new();
        let card = Uuid::new_v4();

        let result = graph.set_parent(card, card);
        assert!(result.is_err());
        assert!(matches!(result, Err(KanbanError::SelfReference)));
    }

    #[test]
    fn test_multiple_parents() {
        let mut graph = CardDependencyGraph::new();
        let parent_a = Uuid::new_v4();
        let parent_b = Uuid::new_v4();
        let child = Uuid::new_v4();

        graph.set_parent(child, parent_a).unwrap();
        graph.set_parent(child, parent_b).unwrap();

        let parents = graph.parents(child);
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&parent_a));
        assert!(parents.contains(&parent_b));

        assert_eq!(graph.children(parent_a).len(), 1);
        assert_eq!(graph.children(parent_b).len(), 1);
    }

    #[test]
    fn test_multiple_children() {
        let mut graph = CardDependencyGraph::new();
        let parent = Uuid::new_v4();
        let child_a = Uuid::new_v4();
        let child_b = Uuid::new_v4();
        let child_c = Uuid::new_v4();

        graph.set_parent(child_a, parent).unwrap();
        graph.set_parent(child_b, parent).unwrap();
        graph.set_parent(child_c, parent).unwrap();

        let children = graph.children(parent);
        assert_eq!(children.len(), 3);
        assert!(children.contains(&child_a));
        assert!(children.contains(&child_b));
        assert!(children.contains(&child_c));
    }

    #[test]
    fn test_remove_parent() {
        let mut graph = CardDependencyGraph::new();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();

        graph.set_parent(child, parent).unwrap();
        assert_eq!(graph.children(parent).len(), 1);

        let result = graph.remove_parent(child, parent);
        assert!(result.is_ok());
        assert_eq!(graph.children(parent).len(), 0);
        assert_eq!(graph.parents(child).len(), 0);
    }

    #[test]
    fn test_remove_parent_nonexistent() {
        let mut graph = CardDependencyGraph::new();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();

        let result = graph.remove_parent(child, parent);
        assert!(result.is_err());
        assert!(matches!(result, Err(KanbanError::EdgeNotFound)));
    }

    #[test]
    fn test_ancestors() {
        let mut graph = CardDependencyGraph::new();
        let grandparent = Uuid::new_v4();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();

        // Create hierarchy: grandparent -> parent -> child
        graph.set_parent(parent, grandparent).unwrap();
        graph.set_parent(child, parent).unwrap();

        let ancestors = graph.ancestors(child);
        assert_eq!(ancestors.len(), 2);
        assert!(ancestors.contains(&parent));
        assert!(ancestors.contains(&grandparent));

        let parent_ancestors = graph.ancestors(parent);
        assert_eq!(parent_ancestors.len(), 1);
        assert!(parent_ancestors.contains(&grandparent));

        let grandparent_ancestors = graph.ancestors(grandparent);
        assert_eq!(grandparent_ancestors.len(), 0);
    }

    #[test]
    fn test_ancestors_multiple_paths() {
        let mut graph = CardDependencyGraph::new();
        let grandparent_a = Uuid::new_v4();
        let grandparent_b = Uuid::new_v4();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();

        // Create diamond: grandparent_a -> parent -> child
        //                 grandparent_b -> parent -> child
        graph.set_parent(parent, grandparent_a).unwrap();
        graph.set_parent(parent, grandparent_b).unwrap();
        graph.set_parent(child, parent).unwrap();

        let ancestors = graph.ancestors(child);
        assert_eq!(ancestors.len(), 3);
        assert!(ancestors.contains(&parent));
        assert!(ancestors.contains(&grandparent_a));
        assert!(ancestors.contains(&grandparent_b));
    }

    #[test]
    fn test_descendants() {
        let mut graph = CardDependencyGraph::new();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();
        let grandchild = Uuid::new_v4();

        // Create hierarchy: parent -> child -> grandchild
        graph.set_parent(child, parent).unwrap();
        graph.set_parent(grandchild, child).unwrap();

        let descendants = graph.descendants(parent);
        assert_eq!(descendants.len(), 2);
        assert!(descendants.contains(&child));
        assert!(descendants.contains(&grandchild));

        let child_descendants = graph.descendants(child);
        assert_eq!(child_descendants.len(), 1);
        assert!(child_descendants.contains(&grandchild));

        let grandchild_descendants = graph.descendants(grandchild);
        assert_eq!(grandchild_descendants.len(), 0);
    }

    #[test]
    fn test_descendants_multiple_branches() {
        let mut graph = CardDependencyGraph::new();
        let parent = Uuid::new_v4();
        let child_a = Uuid::new_v4();
        let child_b = Uuid::new_v4();
        let grandchild_a = Uuid::new_v4();
        let grandchild_b = Uuid::new_v4();

        // Create tree: parent -> child_a -> grandchild_a
        //              parent -> child_b -> grandchild_b
        graph.set_parent(child_a, parent).unwrap();
        graph.set_parent(child_b, parent).unwrap();
        graph.set_parent(grandchild_a, child_a).unwrap();
        graph.set_parent(grandchild_b, child_b).unwrap();

        let descendants = graph.descendants(parent);
        assert_eq!(descendants.len(), 4);
        assert!(descendants.contains(&child_a));
        assert!(descendants.contains(&child_b));
        assert!(descendants.contains(&grandchild_a));
        assert!(descendants.contains(&grandchild_b));
    }

    #[test]
    fn test_child_count() {
        let mut graph = CardDependencyGraph::new();
        let parent = Uuid::new_v4();
        let child_a = Uuid::new_v4();
        let child_b = Uuid::new_v4();
        let child_c = Uuid::new_v4();

        assert_eq!(graph.child_count(parent), 0);

        graph.set_parent(child_a, parent).unwrap();
        assert_eq!(graph.child_count(parent), 1);

        graph.set_parent(child_b, parent).unwrap();
        graph.set_parent(child_c, parent).unwrap();
        assert_eq!(graph.child_count(parent), 3);
    }

    #[test]
    fn test_parent_child_independent_of_blocks() {
        let mut graph = CardDependencyGraph::new();
        let card_a = Uuid::new_v4();
        let card_b = Uuid::new_v4();
        let card_c = Uuid::new_v4();

        // Set parent relationship: A -> B (A is parent of B)
        graph.set_parent(card_b, card_a).unwrap();

        // Add blocking relationship: B -> C (B blocks C)
        // This is independent and doesn't create a cycle
        graph.add_blocks(card_b, card_c).unwrap();

        // Both relationships should exist independently
        assert_eq!(graph.children(card_a).len(), 1);
        assert_eq!(graph.parents(card_b).len(), 1);
        assert_eq!(graph.blocked_by(card_b).len(), 1);
        assert_eq!(graph.blockers(card_c).len(), 1);
    }

    #[test]
    fn test_parent_child_archived_edges() {
        let mut graph = CardDependencyGraph::new();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();

        graph.set_parent(child, parent).unwrap();
        assert_eq!(graph.children(parent).len(), 1);

        graph.archive_node(parent);
        assert_eq!(graph.children(parent).len(), 0);
        assert_eq!(graph.parents(child).len(), 0);

        graph.unarchive_node(parent);
        assert_eq!(graph.children(parent).len(), 1);
        assert_eq!(graph.parents(child).len(), 1);
    }
}
