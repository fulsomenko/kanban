---
bump: patch
---

Follow-up to KAN-428: batch-level WIP pre-check, O(column_size) WIP counts, strict invalid-ID handling

- `KanbanContext::move_cards` and `KanbanContext::move_cards_detailed` now perform a single batch-level WIP check inside `build_move_cards_batch` (reusing the column listing it already fetches for position computation) before emitting per-card `MoveCard` commands. A `WipLimitExceeded` error from a batch move is now reported once at the batch level instead of per-card.
- `InMemoryStore` is now indexed by column: `count_cards_in_column_excluding` and `count_cards_in_column` run in O(column_size + exclude.len()) instead of O(total_cards). The index is maintained transactionally across `upsert_card`, `delete_card`, `delete_cards_by_columns`, and `apply_snapshot`. SQLite already does the equivalent indexed lookup via `WHERE column_id = ?`, so behaviour is consistent across backends.
- `KanbanContext::move_cards` and `move_cards_detailed` now validate input ids via per-id `backend.get_card(id)` instead of an upfront `list_all_cards()` HashSet — strictly cheaper for typical small batches. Validation is consolidated inside `build_move_cards_batch` so an unknown id surfaces as `not_found` before the WIP pre-check can miscount it.
- New pure helper `kanban_domain::card_lifecycle::dedup_preserving_order<T: Hash + Eq + Copy>(items: &[T]) -> Vec<T>` extracted from the inline dedup inside `compute_move_positions`. `compute_move_positions` itself returns `Vec<(Uuid, i32)>` directly (no `Option` overflow guard — practically unreachable since it would require ~2B cards in a single column).
- **Behaviour change**: `KanbanContext::move_cards` (and the MCP `move_cards` tool) now error and roll back the entire batch when any input card ID is unknown, instead of silently dropping invalid IDs. Callers that need partial-success semantics should use `move_cards_detailed`, which continues to report per-ID failures without rolling back the rest of the batch.
