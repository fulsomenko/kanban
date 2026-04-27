---
bump: patch
---

## Fixes

- Card descriptions now display correctly when opening card details — previously the description field appeared empty even when content existed
- Editing a card or board field in the detail view now immediately reflects changes without requiring a manual refresh
- Empty card descriptions now show a placeholder prompt instead of a blank field
- Snapshot load errors during rendering are now logged as warnings instead of being silently swallowed
- Stale model reads after `execute_command` eliminated by capturing card/column UUIDs upfront before state mutation
- Archived cards are now indexed in `Model` for O(1) lookup and cached as a flat list to avoid per-frame clones
- Scroll offset is now preserved in `ColumnListsLayout.refresh_lists` after mutations
- Archived cards panel title is now dynamic (shows live card count) instead of hardcoded
- `ArchivedCardsView` is excluded from the global `q` quit intercept — `q` now closes the view instead of quitting the app

## Refactors

- Replaced the manual `refresh_view()` call pattern with an automatic per-frame render loop (`prepare_frame`), eliminating a class of stale-data bugs where UI state could fall out of sync after mutations
- Introduced a `Model` struct as the single source of truth for all board, column, card, sprint, and dependency graph data rendered each frame
- Removed the intermediate `RenderData`/`ViewState` layer in favour of direct `Model` reads
- Removed granular cache-invalidation methods (`invalidate_boards`, `invalidate_cards`, etc.) — the per-frame full reload makes them unnecessary
- Removed cloning accessors (`boards()`, `sprints()`) from `TuiContext`; callers now read from `Model` or the domain snapshot directly

## Features

- `SqliteStore` now implements `PersistenceStore` — `path` and `instance_id` fields added; `instance_id` is persisted in the `metadata` table and survives reopens
- `SqliteStoreFactory` added to `kanban-persistence-sqlite`, implementing `StoreFactory` with magic-byte content sniffing (`SQLite format 3\0`)
- `SqliteStoreFactory` registered first in `default_registry()` so SQLite files are detected by content before JSON extension matching
- `is_sqlite` / `open_sqlite` bypass removed from `McpContext` and `CliContext` — all storage backends now routed uniformly through the registry
- `VERSION` constant extracted to a shared module; MCP and CLI now share a single version string source
- MCP server handles `-V` / `--version` flag cleanly — responds with version string and exits without error output
