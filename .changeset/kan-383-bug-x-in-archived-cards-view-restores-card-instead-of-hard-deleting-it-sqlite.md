---
bump: patch
---

### Bug fix: permanently deleting an archived card no longer restores it as active (SQLite)

When using a SQLite-backed board, pressing `x` on a card in the Archived Cards view is supposed to
permanently remove it. Instead, the card reappeared in the normal kanban view as if it had been
restored — as though the action had triggered a restore rather than a deletion.

**The card is now fully removed in both tables** when hard-deleted. It will no longer ghost back
into the active board after pressing `x`.

This fix also closes a broader durability gap: every mutation on the SQLite backend (create, update,
move, archive, undo, redo) now immediately checkpoints the write-ahead log, so the database file on
disk always reflects the latest state. Previously the WAL was only flushed when the app exited
cleanly — meaning a crash or force-quit could silently discard recent changes. That risk is now
eliminated regardless of which interface (TUI, CLI, or MCP) is used.
