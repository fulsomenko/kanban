---
bump: minor
---

Extract business logic from kanban-tui into kanban-domain and kanban-core, establishing a clean layered architecture.

### kanban-core
- Add `InputState`, `SelectionState`, and `PageInfo` modules for reusable UI-agnostic state primitives

### kanban-domain
- Add `sort`, `filter`, `search`, and `query` modules for card filtering/sorting pipeline
- Add `CardQueryBuilder` with fluent API for composing card queries
- Add `card_lifecycle` module for card movement, completion toggling, and archival logic
- Add `HistoryManager` for bounded undo/redo (capped at 100 entries)
- Add `export`/`import` modules with `BoardExporter` and `BoardImporter`
- Add `Snapshot` serialization (`to_json_bytes`/`from_json_bytes`) directly on the domain type
- Add sprint query functions and `CardFilters` struct
- Replace dyn dispatch with enum dispatch in search and sort

### kanban-tui
- Remove re-export wrappers and thin delegation layers that proxied domain logic
- Replace inline business logic in handlers with `card_lifecycle` calls
- Replace duplicated filter/sort service with `CardQueryBuilder`
- Fix multi-byte UTF-8 cursor handling via core `InputState`
