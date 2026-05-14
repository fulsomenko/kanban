---
bump: patch
---

Fix: sync card status ↔ completion column across CLI, MCP, and TUI (KAN-394)

Card status and column position now stay in lockstep with the board's completion column, regardless of which surface initiates the change:

- Marking a card as done via `kanban card update --status done` (CLI), the MCP `update_card` tool, or the TUI's `c` key automatically moves it to the board's resolved completion column and stamps `completed_at`.
- Moving a card *into* the completion column via CLI `kanban card move`, MCP `move_card`, the TUI's `h`/`l` keys, or any of the multi-select batch equivalents now sets `status=done` and stamps `completed_at`. Moving back out clears both.
- Multi-select batch operations (`c`, `h`/`l` on multiple cards, sprint-detail batch toggle) execute as a **single undo unit** — one `undo()` reverses every card and every chained command together — and produce **distinct positions** in the destination column instead of all colliding on the same one.
- Atomic updates that already specify both `column_id` and `status` explicitly are respected as-is; auto-sync only fires when the caller leaves one side unspecified.

Internally, the sync is orchestrated at the service layer (`KanbanContext::update_card`, `move_card`, `update_cards`, `move_cards`) by composing chained commands on top of the existing `execute(Vec<Command>)` atomic-batch infrastructure. Domain commands (`UpdateCard`, `MoveCard`, `MoveCards`) remain pure single-responsibility primitives. A new trait method `KanbanOperations::update_cards(Vec<(Uuid, CardUpdate)>)` provides the batched entry point used by the TUI multi-select handlers.
