---
bump: patch
---

Fix MCP: sync card status ↔ completion column (KAN-394)

- `update_card` with `status=done` now auto-moves the card to the board's resolved completion column (and clears completed_at when status leaves Done)
- `move_card` to the completion column auto-sets `status=done` and populates `completed_at`; moving away clears them
- Invariant enforced at the domain command layer, so CLI, MCP, and TUI all get consistent behaviour
- Atomic updates that explicitly pass both `column_id` and `status` are respected as-is — auto-sync only kicks in when one side is left unspecified
