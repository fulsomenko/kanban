---
bump: patch
---

Fix undo crash and strip command log from persistence (KAN-404, KAN-405)

- Undo is now in-session only for all backends — the command log is never written to `kanban.json` or SQLite
- Opening a file with a stale `commands` section no longer causes a crash or corrupts board state when pressing undo
- Existing data files with embedded command logs are silently cleaned up on next save
