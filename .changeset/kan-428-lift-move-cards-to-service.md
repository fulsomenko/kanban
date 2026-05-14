---
bump: patch
---

Lift MoveCards batch position calculation from domain to service (KAN-428)

- Add pure `kanban_domain::card_lifecycle::compute_move_positions` that returns target positions for a batch move given the current column contents and the moving card IDs.
- Add pure helper `kanban_domain::card_lifecycle::dedup_preserving_order<T: Hash + Eq + Copy>(items: &[T]) -> Vec<T>`, used internally by `compute_move_positions` and by the service-layer move orchestration.
- Remove the `MoveCards` (`CardCommand::MoveMultiple`) domain command — its position-computation orchestration is now performed in the service layer.
- `KanbanContext::move_cards_detailed` and `KanbanContext::move_cards` build a batch of atomic `MoveCard` commands plus the existing chained status updates, executed in a single `execute` call (one undo unit, snapshot rollback on failure).
- `build_move_cards_batch` performs a single batch-level WIP pre-check using the column listing it already fetches for position computation, so a `WipLimitExceeded` error from a batch move is reported once at the batch level instead of per-card. The pre-check compares against the deduplicated mover count, so callers passing duplicates that would fit under the limit are not falsely rejected.
- `InMemoryStore` is now indexed by column: `count_cards_in_column`, `count_cards_in_column_excluding`, and `list_cards_by_column` run in O(column_size) instead of O(total_cards). The index is maintained transactionally across `upsert_card`, `delete_card`, `delete_cards_by_columns`, and `apply_snapshot`. SQLite already does the equivalent indexed lookup via `WHERE column_id = ?`, so behaviour is consistent across backends.
- `KanbanContext::move_cards` and `move_cards_detailed` validate input ids via per-id `backend.get_card(id)` instead of an upfront `list_all_cards()` HashSet — strictly cheaper for typical small batches. Validation is consolidated inside `build_move_cards_batch` so an unknown id surfaces as `not_found` before the WIP pre-check can miscount it. `move_cards_detailed` also dedupes its input upfront so both `succeeded` and `failed` report each id at most once.
- **Behaviour change**: `KanbanContext::move_cards` (and the MCP `move_cards` tool) now error and roll back the entire batch when any input card ID is unknown, instead of silently dropping invalid IDs. Callers that need partial-success semantics should use `move_cards_detailed`, which continues to report per-ID failures without rolling back the rest of the batch.
