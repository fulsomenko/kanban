use crate::dependencies::{RelatesKind, Severity};
use crate::KanbanResult;
use uuid::Uuid;

/// Service-layer interface to the card-relation graph.
///
/// Per-kind methods carry per-kind metadata directly in their
/// signatures: severity for blocks, kind for relates, nothing extra
/// for spawns. No runtime kind discriminator — the type system
/// expresses what kind is being mutated.
///
/// The trait returns raw `Vec<Uuid>` for list queries rather than
/// resolved `Vec<CardSummary>`. Surfaces that need display data
/// resolve ids themselves at their own boundary.
///
/// Stands alone from the `KanbanOperations` god-trait — there is no
/// supertrait bound, because the trait deals only in node ids.
/// Implementers compose `KanbanOperations` and `GraphOperations`
/// separately when they need both.
///
/// # Note
/// Cross-board parent/child is permitted at the domain layer today
/// and this trait preserves that behavior. Board-scoping is a
/// separate decision.
pub trait GraphOperations {
    // --- Spawns (parent / child) ---

    /// Add a `parent -> child` Spawns edge. No metadata.
    fn add_spawns_edge(&mut self, parent_id: Uuid, child_id: Uuid) -> KanbanResult<()>;
    fn remove_spawns_edge(&mut self, parent_id: Uuid, child_id: Uuid) -> KanbanResult<()>;
    fn list_spawns_children(&self, parent_id: Uuid) -> KanbanResult<Vec<Uuid>>;
    fn list_spawns_parents(&self, child_id: Uuid) -> KanbanResult<Vec<Uuid>>;

    // --- Blocks ---

    /// Add a `blocker -> blocked` Blocks edge with a severity.
    fn add_blocks_edge(
        &mut self,
        blocker: Uuid,
        blocked: Uuid,
        severity: Severity,
    ) -> KanbanResult<()>;
    fn remove_blocks_edge(&mut self, blocker: Uuid, blocked: Uuid) -> KanbanResult<()>;
    /// Cards `blocker` blocks (outgoing).
    fn list_blocked(&self, blocker: Uuid) -> KanbanResult<Vec<Uuid>>;
    /// Cards that block `blocked` (incoming).
    fn list_blockers(&self, blocked: Uuid) -> KanbanResult<Vec<Uuid>>;

    // --- Relates ---

    /// Add an undirected `a <-> b` RelatesTo edge with a sub-kind.
    fn add_relates_edge(&mut self, a: Uuid, b: Uuid, kind: RelatesKind) -> KanbanResult<()>;
    fn remove_relates_edge(&mut self, a: Uuid, b: Uuid) -> KanbanResult<()>;
    /// Cards related to `card` via any active relates edge.
    fn list_related(&self, card: Uuid) -> KanbanResult<Vec<Uuid>>;

    // --- Convenience defaults (parent / child case) ---
    //
    // The CLI / MCP / TUI surfaces all talk about parent/child by
    // name. These aliases mirror that vocabulary so call sites read
    // naturally; they forward to add_spawns_edge / remove_spawns_edge
    // and the list_spawns_* methods.
    //
    // Two perspectives on the same edge: child-first (`set_parent`,
    // `remove_parent`) for callers thinking "give this child a
    // parent"; parent-first (`add_child`, `remove_child`) for callers
    // thinking "give this parent a child". Both ultimately mutate the
    // same Spawns edge; pick whichever reads better at the call site.

    fn set_parent(&mut self, child_id: Uuid, parent_id: Uuid) -> KanbanResult<()> {
        self.add_spawns_edge(parent_id, child_id)
    }
    fn remove_parent(&mut self, child_id: Uuid, parent_id: Uuid) -> KanbanResult<()> {
        self.remove_spawns_edge(parent_id, child_id)
    }
    fn add_child(&mut self, parent_id: Uuid, child_id: Uuid) -> KanbanResult<()> {
        self.add_spawns_edge(parent_id, child_id)
    }
    fn remove_child(&mut self, parent_id: Uuid, child_id: Uuid) -> KanbanResult<()> {
        self.remove_spawns_edge(parent_id, child_id)
    }
    fn list_card_parents(&self, card_id: Uuid) -> KanbanResult<Vec<Uuid>> {
        self.list_spawns_parents(card_id)
    }
    fn list_card_children(&self, card_id: Uuid) -> KanbanResult<Vec<Uuid>> {
        self.list_spawns_children(card_id)
    }

    // --- Atomic multi-child batch (Spawns) ---
    //
    // CLI/MCP invocations that attach or detach several children in a
    // single call need an all-or-nothing contract: a mid-list failure
    // (cycle, self-ref, unknown card, missing edge) must leave both
    // in-memory and on-disk state untouched. Implementations must
    // run the batch inside a single transaction; the loop-of-singles
    // shape is forbidden because it can persist a partial state.
    //
    // Arg order is parent-first so call sites read "attach these
    // children to this parent" — the natural reading order for the
    // CLI invocation `kanban relation add PARENT C1 C2 ...`.

    /// Attach every `child` in `children` to `parent_id` atomically.
    /// Rolls back the full batch on any failure.
    fn add_children(&mut self, parent_id: Uuid, children: Vec<Uuid>) -> KanbanResult<()>;

    /// Detach every `child` in `children` from `parent_id` atomically.
    /// Rolls back the full batch on any failure.
    fn remove_children(&mut self, parent_id: Uuid, children: Vec<Uuid>) -> KanbanResult<()>;
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
    /// `KanbanOperations` god-trait as a supertrait. This test pins
    /// the decoupling at compile time by impl'ing `GraphOperations`
    /// on a minimal struct that does not impl `KanbanOperations`.
    #[test]
    fn trait_does_not_require_kanban_operations_supertrait() {
        struct GraphOnly;
        impl GraphOperations for GraphOnly {
            fn add_spawns_edge(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn remove_spawns_edge(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn list_spawns_children(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn list_spawns_parents(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn add_blocks_edge(&mut self, _: Uuid, _: Uuid, _: Severity) -> KanbanResult<()> {
                Ok(())
            }
            fn remove_blocks_edge(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn list_blocked(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn list_blockers(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn add_relates_edge(&mut self, _: Uuid, _: Uuid, _: RelatesKind) -> KanbanResult<()> {
                Ok(())
            }
            fn remove_relates_edge(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn list_related(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn add_children(&mut self, _: Uuid, _: Vec<Uuid>) -> KanbanResult<()> {
                Ok(())
            }
            fn remove_children(&mut self, _: Uuid, _: Vec<Uuid>) -> KanbanResult<()> {
                Ok(())
            }
        }
        let mut g = GraphOnly;
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        g.add_spawns_edge(a, b).unwrap();
        g.add_blocks_edge(a, b, Severity::High).unwrap();
        g.add_relates_edge(a, b, RelatesKind::Duplicates).unwrap();
    }

    /// Convenience methods (`set_parent` / `remove_parent` /
    /// `list_card_parents` / `list_card_children`) compile through
    /// the trait without an explicit per-kind constant.
    #[test]
    fn test_convenience_parent_methods_compile_through_trait() {
        struct GraphOnly;
        impl GraphOperations for GraphOnly {
            fn add_spawns_edge(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn remove_spawns_edge(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn list_spawns_children(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn list_spawns_parents(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn add_blocks_edge(&mut self, _: Uuid, _: Uuid, _: Severity) -> KanbanResult<()> {
                Ok(())
            }
            fn remove_blocks_edge(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn list_blocked(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn list_blockers(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn add_relates_edge(&mut self, _: Uuid, _: Uuid, _: RelatesKind) -> KanbanResult<()> {
                Ok(())
            }
            fn remove_relates_edge(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn list_related(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn add_children(&mut self, _: Uuid, _: Vec<Uuid>) -> KanbanResult<()> {
                Ok(())
            }
            fn remove_children(&mut self, _: Uuid, _: Vec<Uuid>) -> KanbanResult<()> {
                Ok(())
            }
        }
        let mut g = GraphOnly;
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();
        g.set_parent(child, parent).unwrap();
        g.remove_parent(child, parent).unwrap();
        g.list_card_parents(child).unwrap();
        g.list_card_children(parent).unwrap();
    }
}
