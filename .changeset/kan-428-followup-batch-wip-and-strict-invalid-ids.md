---
bump: patch
---

Follow-up to KAN-428: batch-level WIP pre-check, trusted-column hint, and strict invalid-ID handling

- `KanbanContext::move_cards` and `KanbanContext::move_cards_detailed` now perform a single batch-level WIP check inside `build_move_cards_batch` (reusing the column listing it already fetches for position computation) before emitting per-card `MoveCard` commands. A `WipLimitExceeded` error from a batch move is now reported once at the batch level instead of per-card.
- `CommandContext` gains `wip_trusted_columns: HashSet<Uuid>` populated via `CommandContext::new(store).with_trusted_columns(...)`. When the target column of a `MoveCard::execute` is in the set, the per-card `check_wip_limit` is skipped. The service layer passes the moved-into column as trusted after its batch pre-check — reducing the per-batch WIP-check cost from O(N) full column-count scans to O(1).
- **Behaviour change**: `KanbanContext::move_cards` (and the MCP `move_cards` tool) now error and roll back the entire batch when any input card ID is unknown, instead of silently dropping invalid IDs. Callers that need partial-success semantics should use `move_cards_detailed`, which continues to report per-ID failures without rolling back the rest of the batch.
