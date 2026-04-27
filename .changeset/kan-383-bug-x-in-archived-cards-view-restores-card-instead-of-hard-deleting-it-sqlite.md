---
bump: patch
---

### Bug fix: permanently deleting an archived card no longer restores it as active (SQLite)

When using a SQLite-backed board, pressing `x` on a card in the Archived Cards view is supposed to
permanently remove it. Instead, the card reappeared in the normal kanban view as if it had been
restored.

The card is now fully removed in both cases — it will no longer ghost back into the active board
after a hard delete.
