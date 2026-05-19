use crate::{CardEdgeType, KanbanOperations, KanbanResult};
use uuid::Uuid;

/// Service-layer interface to the card-relation graph.
///
/// The trait returns raw `Vec<Uuid>` rather than resolved
/// `Vec<CardSummary>`. Surfaces that need display data resolve ids
/// themselves (e.g. via `KanbanOperations::get_card`). This keeps the
/// trait focused on graph topology and avoids forcing every consumer
/// (notably the TUI, which already has cards in memory) to pay for
/// resolution it doesn't need.
///
/// Decoupled from the 51-method `KanbanOperations` god-trait (see
/// KAN-483); graph operations cluster as their own concern.
///
/// # Currently exposed via surfaces (CLI / MCP / TUI)
/// - `CardEdgeType::ParentOf` (parent/child)
///
/// # Future edge kinds
/// `Blocks` and `RelatesTo` are recognized by the underlying
/// `CardEdgeType` enum but are not yet wired through user surfaces.
/// Each addition is a separate card that extends the impl on
/// `KanbanContext` and adds the matching CLI / MCP / TUI surfaces.
///
/// # Note
/// Cross-board parent/child is permitted at the domain layer today and
/// this trait preserves that behavior. Board-scoping is a separate
/// decision.
pub trait GraphOperations: KanbanOperations {
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
}
