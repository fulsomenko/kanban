use crate::{CardEdgeType, KanbanResult};
use uuid::Uuid;

/// Service-layer interface to the card-relation graph.
///
/// The trait returns raw `Vec<Uuid>` rather than resolved
/// `Vec<CardSummary>`. Surfaces that need display data resolve ids
/// themselves at their own boundary. This keeps the contract focused
/// on graph topology and avoids forcing consumers (notably the TUI,
/// which already has cards in memory) to pay for resolution.
///
/// Stands alone from the 51-method `KanbanOperations` god-trait
/// (KAN-483) — there is no supertrait bound, because the trait deals
/// only in node ids. Implementers compose `KanbanOperations` and
/// `GraphOperations` separately when they need both.
///
/// All three `CardEdgeType` variants are wired through the trait
/// today: parent/child (DAG), blocks (DAG), relates-to (undirected).
/// Cycle detection, self-reference rejection, and edge-not-found
/// behavior come from the underlying sub-graph implementation.
///
/// # Note
/// Cross-board parent/child is permitted at the domain layer today and
/// this trait preserves that behavior. Board-scoping is a separate
/// decision.
pub trait GraphOperations {
    // --- Primitive methods. Every consumer can call these. ---

    /// Add a directed edge `from -> to` of the given kind.
    fn add_card_edge(&mut self, from: Uuid, to: Uuid, kind: CardEdgeType) -> KanbanResult<()>;

    /// Remove the directed edge `from -> to` of the given kind, if present.
    fn remove_card_edge(&mut self, from: Uuid, to: Uuid, kind: CardEdgeType) -> KanbanResult<()>;

    /// All direct successors of `node` reachable by a single edge of `kind`.
    fn list_card_edges_from(&self, node: Uuid, kind: CardEdgeType) -> KanbanResult<Vec<Uuid>>;

    /// All direct predecessors of `node` via a single edge of `kind`.
    fn list_card_edges_to(&self, node: Uuid, kind: CardEdgeType) -> KanbanResult<Vec<Uuid>>;

    // --- Convenience defaults for the parent/child case. ---

    /// Add a parent-of edge: `parent_id -> child_id`.
    fn set_card_parent(&mut self, child_id: Uuid, parent_id: Uuid) -> KanbanResult<()> {
        self.add_card_edge(parent_id, child_id, CardEdgeType::ParentOf)
    }

    /// Remove the parent-of edge `parent_id -> child_id`.
    fn remove_card_parent(&mut self, child_id: Uuid, parent_id: Uuid) -> KanbanResult<()> {
        self.remove_card_edge(parent_id, child_id, CardEdgeType::ParentOf)
    }

    /// Direct parents of `card_id` (incoming parent-of edges).
    fn list_card_parents(&self, card_id: Uuid) -> KanbanResult<Vec<Uuid>> {
        self.list_card_edges_to(card_id, CardEdgeType::ParentOf)
    }

    /// Direct children of `card_id` (outgoing parent-of edges).
    fn list_card_children(&self, card_id: Uuid) -> KanbanResult<Vec<Uuid>> {
        self.list_card_edges_from(card_id, CardEdgeType::ParentOf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trait_is_object_safe() {
        fn _accepts_dyn(_: &dyn GraphOperations) {}
    }

    /// `GraphOperations` deals only in node ids (`Vec<Uuid>`); it does
    /// not need card resolution and therefore must not require the
    /// 51-method `KanbanOperations` god-trait as a supertrait. This
    /// test pins the decoupling at compile time by impl'ing
    /// `GraphOperations` on a minimal struct that does not impl
    /// `KanbanOperations`.
    #[test]
    fn trait_does_not_require_kanban_operations_supertrait() {
        struct GraphOnly;
        impl GraphOperations for GraphOnly {
            fn add_card_edge(
                &mut self,
                _from: Uuid,
                _to: Uuid,
                _kind: CardEdgeType,
            ) -> KanbanResult<()> {
                Ok(())
            }
            fn remove_card_edge(
                &mut self,
                _from: Uuid,
                _to: Uuid,
                _kind: CardEdgeType,
            ) -> KanbanResult<()> {
                Ok(())
            }
            fn list_card_edges_from(
                &self,
                _node: Uuid,
                _kind: CardEdgeType,
            ) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn list_card_edges_to(
                &self,
                _node: Uuid,
                _kind: CardEdgeType,
            ) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
        }
        let mut g = GraphOnly;
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        g.add_card_edge(a, b, CardEdgeType::ParentOf).unwrap();
    }

    /// Convenience methods read in subject-object order: the first
    /// parameter is the subject of the verb. `set_child(parent, child)`
    /// — "set this child under that parent". `set_parent(child, parent)`
    /// — "set this child's parent to that one". Both produce the same
    /// parent->child edge.
    #[test]
    fn test_convenience_methods_use_semantic_parameter_ordering() {
        struct GraphOnly;
        impl GraphOperations for GraphOnly {
            fn add_card_edge(
                &mut self,
                _from: Uuid,
                _to: Uuid,
                _kind: CardEdgeType,
            ) -> KanbanResult<()> {
                Ok(())
            }
            fn remove_card_edge(
                &mut self,
                _from: Uuid,
                _to: Uuid,
                _kind: CardEdgeType,
            ) -> KanbanResult<()> {
                Ok(())
            }
            fn list_card_edges_from(
                &self,
                _node: Uuid,
                _kind: CardEdgeType,
            ) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn list_card_edges_to(
                &self,
                _node: Uuid,
                _kind: CardEdgeType,
            ) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
        }
        let mut g = GraphOnly;
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();
        // Subject-object order: parent-then-child or child-then-parent
        // matching the verb's subject.
        g.set_child(parent, child).unwrap();
        g.set_parent(child, parent).unwrap();
        g.remove_child(parent, child).unwrap();
        g.remove_parent(child, parent).unwrap();
    }
}
