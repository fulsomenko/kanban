---
bump: patch
---

Fix MCP: sync card status ↔ completion column (KAN-394)

- `update_card` with `status=done` now auto-moves the card to the board's resolved completion column (and clears `completed_at` when status leaves Done)
- `move_card` to the completion column auto-sets `status=done` and populates `completed_at`; moving away clears them
- Orchestrated at the service layer (`KanbanContext`) via chained commands so the sync is observable in the command log and a single undo reverses both the status change and the chained column move atomically. Domain commands remain pure single-responsibility primitives.
- CLI, MCP, and TUI all benefit since they route writes through the same service methods
- Atomic updates that explicitly pass both `column_id` and `status` are respected as-is — auto-sync only kicks in when one side is left unspecified
