---
bump: minor
---

### Added
- **SQLite storage backend** — use `.sqlite`, `.sqlite3`, or `.db` file extensions to store kanban data in a relational database instead of JSON
- **Command-replay undo/redo** — all mutations are recorded as replayable commands with full history persistence across sessions
- **Indexed snapshots** — undo/redo on SQLite is O(1) via compressed snapshots stored alongside each command, eliminating full replay from baseline
- **Board ordering** — boards now have an explicit `position` field for deterministic sort order
- **Magic bytes detection** — CLI and MCP automatically detect whether a file is SQLite or JSON by reading file headers, with extension-based fallback for new files

### Changed
- `undo()` and `redo()` now return `KanbanResult<bool>` instead of `bool`, propagating storage errors to callers
- Board import clears command history after completion — imported data is baked into the baseline snapshot and cannot be individually undone
- `MigrateSprintLogs` selectively persists only cards whose sprint logs actually changed, reducing unnecessary writes

### Fixed
- SQLite databases created before the `card_counter` feature now auto-migrate on open instead of crashing with "no such column: card_counter"
- Input lag when holding navigation keys — buffered key events are now drained before each redraw
- TUI no longer renders at 60fps when idle — redraws are event-driven, reducing CPU usage to near zero when not interacting
- Eliminated O(n²) card cloning in the render loop (was cloning all cards per visible card per frame)
- Eliminated N+1 SQL query pattern when loading sprint logs and board auxiliary data on the SQLite backend

### Removed
- `SqliteBlobStore` and `SqliteStoreFactory` — replaced by `SqliteStore` (formerly `SqliteDataStore`), wired directly through `StoreManager`
- `InMemoryDataStore` type alias — use `InMemoryStore` directly
- `UndoPointId` and snapshot-based undo-point methods from `DataStore` trait — superseded by command-replay undo
- Command log methods from `PersistenceStore` trait — moved to the dedicated `CommandStore` trait

### Internal
- `DataStore` trait provides on-demand entity queries (get/list/upsert/delete) replacing full in-memory snapshot
- `CommandStore` trait handles command persistence and indexed snapshot storage
- `KanbanBackend` supertrait combines `DataStore + CommandStore` with manual impls per backend
- Create commands embed deterministic UUIDs for reproducible replay
- TUI render path reads from `ViewState` cache populated by `refresh_view()` — no storage queries during frame rendering
