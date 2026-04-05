---
bump: patch
---

History-aware execute and StateManager slimming

- Make `execute()` capture undo history by default — all `KanbanOperations` consumers get undo/redo for free
- Add native `BulkArchiveCards` and `BulkMoveCards` batch commands (single undo entry per bulk op)
- Extract `clear_history()` from `reload()` — callers decide whether to clear
- Move conflict detection (`has_conflict`/`set_conflict`/`clear_conflict`) from StateManager to KanbanContext
- Slim StateManager to purely a save coordinator (channels + file watcher)
- Add MCP `undo` and `redo` tools
