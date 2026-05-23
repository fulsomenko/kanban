use crate::dependencies::{RelatesKind, Severity};
use crate::KanbanResult;
use uuid::Uuid;

/// Service-layer interface to the card-relation graph.
///
/// One canonical method per per-kind operation, with plural batch
/// primitives as the unit of atomicity. The singular variants are
/// default methods that delegate to the plural by wrapping a single
/// id in a `Vec` — this is the project-wide pattern (cf. `archive_card`
/// calling `archive_cards(vec![id])` on `KanbanContext`) and ensures
/// that every mutation routes through the same `execute(Vec<Command>)`
/// transactional path, so atomicity is inherited rather than
/// re-engineered per arity.
///
/// Per-kind methods carry per-kind metadata directly in their
/// signatures: severity for blocks, kind for relates, nothing extra
/// for spawns. No runtime kind discriminator — the type system
/// expresses what kind is being mutated.
///
/// List queries return raw `Vec<Uuid>` rather than resolved
/// `Vec<CardSummary>`. Surfaces that need display data resolve ids
/// themselves at their own boundary.
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
    // ─── Spawns (parent / child) ──────────────────────────────────

    /// Attach every `child` in `children` to `parent` atomically.
    /// Rolls back the full batch on any failure (cycle, self-ref,
    /// unknown card).
    fn spawn_children(&mut self, parent: Uuid, children: Vec<Uuid>) -> KanbanResult<()>;

    /// Detach every `child` in `children` from `parent` atomically.
    /// Rolls back the full batch on any failure.
    fn unspawn_children(&mut self, parent: Uuid, children: Vec<Uuid>) -> KanbanResult<()>;

    /// Singular convenience: forwards to [`spawn_children`] with a
    /// one-element batch. The atomic primitive is the plural.
    fn spawn_child(&mut self, parent: Uuid, child: Uuid) -> KanbanResult<()> {
        self.spawn_children(parent, vec![child])
    }

    /// Singular convenience: forwards to [`unspawn_children`].
    fn unspawn_child(&mut self, parent: Uuid, child: Uuid) -> KanbanResult<()> {
        self.unspawn_children(parent, vec![child])
    }

    /// List direct children of `parent`.
    fn list_children_of(&self, parent: Uuid) -> KanbanResult<Vec<Uuid>>;

    /// List direct parents of `child`. Cross-board parents are
    /// returned in the same list — board-scoping is the caller's
    /// concern.
    fn list_parents_of(&self, child: Uuid) -> KanbanResult<Vec<Uuid>>;

    // ─── Blocks ───────────────────────────────────────────────────

    /// Add a `blocker -> blocked` Blocks edge with a severity.
    fn block(&mut self, blocker: Uuid, blocked: Uuid, severity: Severity) -> KanbanResult<()>;

    /// Remove the `blocker -> blocked` Blocks edge.
    fn unblock(&mut self, blocker: Uuid, blocked: Uuid) -> KanbanResult<()>;

    /// Cards `blocker` blocks (outgoing).
    fn list_blocked_by(&self, blocker: Uuid) -> KanbanResult<Vec<Uuid>>;

    /// Cards that block `blocked` (incoming).
    fn list_blockers_of(&self, blocked: Uuid) -> KanbanResult<Vec<Uuid>>;

    // ─── Relates ──────────────────────────────────────────────────

    /// Add an undirected `a <-> b` RelatesTo edge with a sub-kind.
    fn relate(&mut self, a: Uuid, b: Uuid, kind: RelatesKind) -> KanbanResult<()>;

    /// Remove the undirected `a <-> b` RelatesTo edge.
    fn unrelate(&mut self, a: Uuid, b: Uuid) -> KanbanResult<()>;

    /// Cards related to `card` via any active relates edge.
    fn list_related_to(&self, card: Uuid) -> KanbanResult<Vec<Uuid>>;
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
            fn spawn_children(&mut self, _: Uuid, _: Vec<Uuid>) -> KanbanResult<()> {
                Ok(())
            }
            fn unspawn_children(&mut self, _: Uuid, _: Vec<Uuid>) -> KanbanResult<()> {
                Ok(())
            }
            fn list_children_of(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn list_parents_of(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn block(&mut self, _: Uuid, _: Uuid, _: Severity) -> KanbanResult<()> {
                Ok(())
            }
            fn unblock(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn list_blocked_by(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn list_blockers_of(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn relate(&mut self, _: Uuid, _: Uuid, _: RelatesKind) -> KanbanResult<()> {
                Ok(())
            }
            fn unrelate(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn list_related_to(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
        }
        let mut g = GraphOnly;
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        g.spawn_child(a, b).unwrap();
        g.block(a, b, Severity::High).unwrap();
        g.relate(a, b, RelatesKind::Duplicates).unwrap();
    }

    /// The singular `spawn_child` is a default method on the trait
    /// that forwards to `spawn_children(parent, vec![child])`. This
    /// pins the composition direction (singular → plural) so any
    /// future implementor that overrides the singular cannot
    /// silently bypass the atomic batch path. Same for `unspawn_child`.
    #[test]
    fn test_spawn_child_default_routes_through_spawn_children() {
        use std::cell::RefCell;
        struct Recorder {
            spawn_calls: RefCell<Vec<(Uuid, Vec<Uuid>)>>,
            unspawn_calls: RefCell<Vec<(Uuid, Vec<Uuid>)>>,
        }
        impl GraphOperations for Recorder {
            fn spawn_children(&mut self, parent: Uuid, children: Vec<Uuid>) -> KanbanResult<()> {
                self.spawn_calls.borrow_mut().push((parent, children));
                Ok(())
            }
            fn unspawn_children(&mut self, parent: Uuid, children: Vec<Uuid>) -> KanbanResult<()> {
                self.unspawn_calls.borrow_mut().push((parent, children));
                Ok(())
            }
            fn list_children_of(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn list_parents_of(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn block(&mut self, _: Uuid, _: Uuid, _: Severity) -> KanbanResult<()> {
                Ok(())
            }
            fn unblock(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn list_blocked_by(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn list_blockers_of(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
            fn relate(&mut self, _: Uuid, _: Uuid, _: RelatesKind) -> KanbanResult<()> {
                Ok(())
            }
            fn unrelate(&mut self, _: Uuid, _: Uuid) -> KanbanResult<()> {
                Ok(())
            }
            fn list_related_to(&self, _: Uuid) -> KanbanResult<Vec<Uuid>> {
                Ok(Vec::new())
            }
        }
        let mut r = Recorder {
            spawn_calls: RefCell::new(Vec::new()),
            unspawn_calls: RefCell::new(Vec::new()),
        };
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();
        r.spawn_child(parent, child).unwrap();
        r.unspawn_child(parent, child).unwrap();
        assert_eq!(
            r.spawn_calls.borrow().as_slice(),
            &[(parent, vec![child])],
            "spawn_child must route through spawn_children with vec![child]"
        );
        assert_eq!(
            r.unspawn_calls.borrow().as_slice(),
            &[(parent, vec![child])],
            "unspawn_child must route through unspawn_children with vec![child]"
        );
    }
}
