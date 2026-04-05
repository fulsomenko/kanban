---
bump: patch
---

History-aware execute, StateManager slimming, and TuiContext encapsulation

- Make `execute()` capture undo history by default — all `KanbanOperations` consumers get undo/redo for free
- Add native batch commands (`ArchiveCards`, `MoveCards`, `AssignCardsToSprint`) with single undo entry
- Extract `clear_history()` from `reload()` — callers decide whether to clear
- Move conflict detection (`has_conflict`/`set_conflict`/`clear_conflict`) from StateManager to KanbanContext
- Slim StateManager to purely a save coordinator (channels + file watcher)
- Add MCP `undo` and `redo` tools
- Encapsulate `TuiContext` by removing `Deref` and making `inner` private
- Remove all `_mut()` accessors from `TuiContext`, routing every mutation through domain commands
- Add `ImportEntities`, `ApplyBoardSettings`, `ApplyCardMetadata`, `CompactColumnPositions`, `MigrateSprintLogs` commands
- Lift sprint counter/name logic into `CreateSprint` command, eliminating caller-side board mutations
