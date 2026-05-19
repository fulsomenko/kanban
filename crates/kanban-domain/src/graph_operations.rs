use crate::{CardEdgeType, CardSummary, KanbanOperations, KanbanResult};
use uuid::Uuid;

/// Service-layer interface to the card-relation graph.
///
/// Generic over edge kind: the four primitive methods accept any
/// `CardEdgeType`. Convenience defaults wrap the common parent/child
/// case so call sites stay readable.
///
/// The current implementation behind this trait is the parent-child
/// tree-style graph backed by `CardDependencyGraph` (see
/// `crates/kanban-domain/src/dependencies/`). A different graph
/// implementation can satisfy this trait without changes to CLI, MCP,
/// or TUI code. The trait is the contract; the implementation is
/// concrete and replaceable.
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
    ///
    /// Semantics (cycle detection, self-reference rejection, idempotency)
    /// are decided by the implementation. The current implementation
    /// rejects cycles for DAG-typed edges and self-references for all.
    fn add_card_edge(&mut self, from: Uuid, to: Uuid, kind: CardEdgeType) -> KanbanResult<()>;

    /// Remove the directed edge `from -> to` of the given kind, if present.
    fn remove_card_edge(&mut self, from: Uuid, to: Uuid, kind: CardEdgeType) -> KanbanResult<()>;

    /// All direct successors of `node` reachable by a single edge of `kind`.
    fn list_card_edges_from(
        &self,
        node: Uuid,
        kind: CardEdgeType,
    ) -> KanbanResult<Vec<CardSummary>>;

    /// All direct predecessors of `node` via a single edge of `kind`.
    fn list_card_edges_to(&self, node: Uuid, kind: CardEdgeType) -> KanbanResult<Vec<CardSummary>>;

    // --- Convenience defaults for the parent/child case. ---
    // These let surface code read naturally. Any future refactor of the
    // edge model leaves the high-level API intact because consumers can
    // keep calling these.

    /// Add a parent-of edge: `parent_id -> child_id`.
    fn set_card_parent(&mut self, child_id: Uuid, parent_id: Uuid) -> KanbanResult<()> {
        self.add_card_edge(parent_id, child_id, CardEdgeType::ParentOf)
    }

    /// Remove the parent-of edge `parent_id -> child_id`.
    fn remove_card_parent(&mut self, child_id: Uuid, parent_id: Uuid) -> KanbanResult<()> {
        self.remove_card_edge(parent_id, child_id, CardEdgeType::ParentOf)
    }

    /// Direct parents of `card_id` (incoming parent-of edges).
    fn list_card_parents(&self, card_id: Uuid) -> KanbanResult<Vec<CardSummary>> {
        self.list_card_edges_to(card_id, CardEdgeType::ParentOf)
    }

    /// Direct children of `card_id` (outgoing parent-of edges).
    fn list_card_children(&self, card_id: Uuid) -> KanbanResult<Vec<CardSummary>> {
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
