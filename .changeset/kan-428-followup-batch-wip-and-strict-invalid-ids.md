---
bump: patch
---

Follow-up to KAN-428: batch-level WIP pre-check, trusted-column hint, strict invalid-ID handling, and second-review tidy-ups

- `KanbanContext::move_cards` and `KanbanContext::move_cards_detailed` now perform a single batch-level WIP check inside `build_move_cards_batch` (reusing the column listing it already fetches for position computation) before emitting per-card `MoveCard` commands. A `WipLimitExceeded` error from a batch move is now reported once at the batch level instead of per-card.
- `CommandContext` gains `wip_trusted_columns: HashSet<Uuid>` populated via `CommandContext::new(store).with_trusted_columns(...)`. When the target column of a `MoveCard::execute` is in the set, the per-card `check_wip_limit` is skipped. The service layer passes the moved-into column as trusted *unconditionally* after its pre-check — reducing the per-batch WIP-check cost from O(N) full column-count scans to O(1), and avoiding the wasted per-card `get_column` lookup on the no-WIP-limit path too. The trust-set field is `pub(crate)` so only same-crate command code can read it; external callers continue to set it through the `with_trusted_columns` builder.
- `KanbanContext::move_cards` now validates that every input id corresponds to a known card before invoking `build_move_cards_batch`. Without this, a stray unknown id could inflate `ids.len()` in the batch WIP pre-check and surface a misleading `WipLimitExceeded` when the real cause is `not_found`.
- `kanban_domain::card_lifecycle::compute_move_positions` deduplicates repeated input ids (first occurrence wins) and now returns `Option<Vec<(Uuid, i32)>>`, returning `None` if the computed target position would overflow `i32`. The service caller maps `None` to a loud `KanbanError::Internal` instead of writing wrapped positions.
- **Behaviour change**: `KanbanContext::move_cards` (and the MCP `move_cards` tool) now error and roll back the entire batch when any input card ID is unknown, instead of silently dropping invalid IDs. Callers that need partial-success semantics should use `move_cards_detailed`, which continues to report per-ID failures without rolling back the rest of the batch.
