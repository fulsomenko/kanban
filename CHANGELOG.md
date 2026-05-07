## [0.4.1] - 2026-05-07 ([#242](https://github.com/fulsomenko/kanban/pull/242))

### KAN-396 Fix Tui Make Settings Config Edit Tests Sandbox Safe For Nixpkgs (2026-05-07)

Fix settings_config_edit_tests failing in Nix build sandbox
- 5 tests in kanban-tui called apply_config_edit without a configuration_location
  in the JSON, causing save() to fall back to $HOME/.config/kanban/config.toml
- The Nix sandbox sets $HOME to a non-writable stub, so create_dir_all failed
  with Permission denied
- Fix: each test now creates a TempDir and passes its path as configuration_location
  so save() writes to $TMPDIR (writable in sandbox) instead of $HOME/.config


## [0.4.0] - 2026-05-04 ([#208](https://github.com/fulsomenko/kanban/pull/208))

### CAT-245 Surface Command Errors To User Via Banner In Tui Handlers (2026-05-04)

Adds an in-app error log panel that captures WARN and ERROR tracing events
without corrupting the TUI display.
Previously, `tracing::warn!` and `tracing::error!` calls would write directly
to stderr during raw mode, bleeding into the terminal buffer and garbling the
UI. Log output was also lost once the session ended with no way to review it.
The fix is two-pronged. A custom `InMemoryLogLayer` replaces the stderr
subscriber in TUI mode, intercepting all WARN/ERROR events into a shared
in-memory buffer. The buffer is then surfaced through a dedicated `ErrorLog`
panel that auto-opens whenever a new ERROR is captured, and can be toggled
on demand with F12 and dismissed with Escape. The footer shows a `[!] N errors`
badge while there are unread errors.

### CAT-260 Invert Storage Backend Plugin Architecture (2026-05-04)

- refactor(cli): extract run_with_args from run to enable injection of CLI args in tests
- fix(service): include root cause in export_to_sqlite error when sqlite backend is absent
- feat(cli,mcp): add json/sqlite forwarding features; gate with_defaults on cfg
- fix(cli): early-return Completions before no-backends guard
- refactor(tui): drop direct backend deps, use kanban_service::default_registry()
- refactor(mcp): drop direct backend deps, use kanban_service::default_registry()
- refactor(cli): drop direct backend deps, use kanban_service::default_registry()
- feat(service): add json/sqlite optional features and default_registry()
- fix(mcp): add empty-registry guard, use shared validate_path, remove local fn
- fix(cli): add empty-registry guard, validate file path, align tracing with env-filter
- refactor(service): extract shared validate_path from kanban-mcp
- refactor(service): add StoreManager::has_backends
- refactor(persistence): add StoreRegistry::is_empty
- refactor(cli): restrict internal module visibility to pub(crate)
- fix(service): improve export_to_sqlite error for unregistered sqlite backend
- fix(cli,mcp): use try_init to prevent double-init panic
- docs(service): document export_to_sqlite registry requirement
- fix(mcp): warn on backend auto-correction in McpContext
- refactor(cli,mcp): invert storage backend ownership via builders
- feat(service): introduce StoreManager with injectable StoreRegistry

### CAT-264 Lift Undo Redo Historymanager Into Kanbancontext (2026-05-04)

History-aware execute, StateManager slimming, and TuiContext encapsulation
- Unify `execute()` and `execute_batch()` into a single `execute(Vec<Box<dyn Command>>)` — fixes spurious undo-on-failure bug and provides one uniform API with atomic rollback semantics
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

### CAT-302 Starting The Tui Doesnt Select A Board (2026-05-04)

- feat(tui): preselect first board and refresh card view on startup
- test(tui): preselect first board and refresh card view on startup

### CAT-312 Adjust Landing Roadmap (2026-05-04)

- fix(web): correct completed elements output

### CAT-341 Redesign Card Identifier Model Drop Stored Prefixes Lock Board Sprint Prefix Simplify Counters (2026-05-04)

- fix(persistence-json): renumber colliding cards instead of aborting V2→V3 migration
- refactor(tui): remove assigned_prefix management from sprint assignment handlers
- test(service,persistence): update contract tests for card_counter
- fix(mcp,cli): remove dead card_prefix/assigned_prefix fields from CardUpdate
- feat(persistence-sqlite): schema v1→v2 migration; card_counter; drop prefix columns
- feat(persistence-json): add V2→V3 migration; strip prefix fields, set card_counter
- refactor(domain): update all Card::new call sites to drop prefix argument
- feat(domain): lock sprint card_prefix after card assigned; enforce prefix uniqueness
- feat(domain): lock board card_prefix after first card is created
- feat(domain): two-level identifier resolution (sprint.card_prefix → board.card_prefix)
- feat(domain): drop assigned_prefix and card_prefix from Card; simplify Card::new
- feat(domain): replace prefix_counters with card_counter on Board
- feat(persistence): add FormatVersion::V3

### KAN-159 Implement Sqlite Storage Backend (2026-05-04)

- docs: add .db extension to SQLite backend documentation
- test(service): add make_store test for .db extension
- fix(service): use store instance_id instead of throwaway UUID in save
- docs(service): document registry registration order rationale
- fix(persistence-json): exclude SQLite extensions from catch-all match
- feat(persistence-sqlite): add .db extension to SqliteStoreFactory
- docs(web): update landing page for SQLite backend support
- docs(service,cli,mcp): update READMEs for pluggable storage
- docs(persistence): add READMEs for JSON and SQLite backends
- docs(persistence): rewrite README as trait abstraction layer
- docs: update workspace structure and diagrams for pluggable persistence
- fix(persistence-sqlite): replace len comparison with is_empty in concurrent test
- test(persistence-json): align roundtrip test with shared snapshot
- refactor(persistence): replace hardcoded make_store with StoreFactory registry
- fix(persistence-sqlite): wrap load in read transaction and batch sync deletes
- docs(persistence-sqlite): document delete-and-reinsert pattern in upserts
- fix(persistence-sqlite): add points range validation
- refactor(persistence-sqlite): replace fragile enum parsing with parse_enum helper
- test(persistence-sqlite): add concurrent access test
- fix(persistence-sqlite): narrow mutex scope in save and add schema migration skeleton
- fix(service): remove dead let _ = ext binding
- fix(persistence-sqlite): require NOT NULL fields in upserts instead of silent defaults
- fix(persistence-sqlite): propagate parse errors for optional UUID and DateTime fields
- fix(persistence-sqlite): narrow sqlx features with default-features = false
- test(service): strengthen make_store no-extension test with save/load roundtrip
- refactor(persistence-sqlite): deduplicate fully_populated_snapshot in roundtrip test
- fix(service): make_store returns Result instead of panicking
- fix(persistence-sqlite): deterministic card order, async mutex, builder API, and pool size
- fix(persistence-sqlite): validate NOT NULL fields and sync edges incrementally
- fix(persistence-sqlite): return errors for unknown enum variants instead of silent fallback
- docs(persistence-sqlite): document missing FK rationale on card_edges table
- test(cli): add sqlite-to-sqlite migration roundtrip test
- test(persistence): add conflict detection contract test for stale metadata
- fix(service): guard .db extension when sqlite-storage feature is disabled
- refactor(persistence-sqlite): move WAL pragma to connection options and document sprint_logs FK
- refactor(persistence-sqlite): split sqlite_store.rs into helpers, builders, and upserts modules
- style: apply cargo fmt across workspace
- refactor(persistence): replace contract test glue with macro
- test(cli): add bidirectional migration tests
- refactor(cli): remove direct persistence crate dependencies
- feat(cli): make migrate command backend-agnostic
- refactor(persistence): remove tests superseded by contract suite
- test(persistence): wire contract tests to JSON and SQLite backends
- feat(service): add test-helpers feature with contract test suite
- test(service): add make_store factory dispatch tests
- refactor(service): extract shared make_store factory from CLI, MCP, and TUI
- feat(persistence): add instance_id() to PersistenceStore trait
- fix(persistence-sqlite): prevent silent data degradation on load
- fix(persistence-sqlite): normalize schema types and add missing constraints
- refactor(persistence): delete dead store/ and migration/ modules
- test(persistence-sqlite): add KanbanContext integration tests for SQLite backend
- test(persistence-sqlite): add dependency graph edges to roundtrip test
- feat(persistence-sqlite): normalize schema — replace JSON columns with relational tables
- fix: update error types and imports after rebase onto develop
- docs(contributing): add 'adding a domain field' checklist for schema enforcement
- test(persistence): add fully-populated roundtrip tests for JSON and SQLite plugins
- feat(domain): add PartialEq to domain types and graph primitives for roundtrip test assertions
- feat(cli/mcp/tui): wire up persistence plugin architecture
- feat(service): decouple KanbanContext from concrete persistence implementations
- feat(persistence-sqlite): add SQLite storage backend with typed struct enforcement in row_to_* functions
- feat(persistence-json): add JSON file store plugin crate
- feat(persistence): refactor trait crate to remove embedded store implementations
- feat(ci): add persistance-* crates to publish script
- fix(tui): remove unused PersistenceStore import missed during rebase
- fix(tui): restore TuiSnapshot export and fix SaveChannel type after rebase conflict
- fix(sqlite): address code review feedback
- refactor(sqlite): use Table enum instead of string literals
- fix(sqlite): use transaction for all save operations
- fix(tui): resolve clippy warnings for type complexity and unused import
- feat(tui,cli): add pluggable storage backend support
- feat(persistence): add JSON to SQLite migration utilities
- feat(persistence): implement SqliteStore with PersistenceStore trait
- feat(persistence): add SQLite schema for kanban data

### KAN-171 Replace Silent Failures With Proper Errors (2026-05-04)

- fix(domain): replace silent failure with NotFound error in CreateSubcardCommand (ef6aa323bdeeca39a07eb6afa273901719158b5f)
- fix(domain): replace silent failures with NotFound errors in sprint commands (6828c75d20bff02169e4f04e41d0b5fdc54de0e4)
- fix(domain): replace silent failure with NotFound error in UpdateColumn (a70a5c449f8236d8cb19e2ee59bec6fd3d9779fd)
- fix(domain): replace silent failures with NotFound errors in card commands (00d17452e29ed77e4de1edcdeaa69b7aec5cd878)
- fix(domain): replace silent failures with NotFound errors in board commands (d3810135a1131be6b0e9ce5acc4643248443433b)
- feat(domain): add CommandContext lookup helpers that return NotFound errors (0e2817bb9573a2e60bb0734f5b5b691ac04b610b)

### KAN-210 Carry Over Cards From Ended Sprint (2026-05-04)

- feat: rebind carry-over from R to M, always moves all uncompleted cards
- test: verify carry_over_sprint_cards skips Done cards
- feat: add R carry-over keybinding to sprint detail help
- feat: add R carry-over and bulk c/d actions in sprint detail view
- feat: add carry-over sprint popup navigation and confirm handler
- feat: trigger carry-over dialog on sprint completion with uncompleted cards
- feat: add CarryOverSprintDialog render component
- feat: add CarryOverSprint dialog mode and state
- feat: add carry_over_sprint_cards MCP tool
- feat: add sprint carry-over CLI subcommand
- feat: implement carry_over_sprint_cards on KanbanContext
- feat: add carry_over_sprint_cards to KanbanOperations trait

### KAN-211 Disambiguate Card Identifier Lookup Across Boards (2026-05-04)

- test: add CLI integration tests for ambiguous identifier resolution
- feat: return all matches from card get for ambiguous identifier
- test: add find_cards_by_identifier integration tests for MCP context
- feat: return ambiguity error when multiple cards match identifier
- refactor: rename find_card_by_identifier to find_cards_by_identifier returning Vec

### KAN-230 Submit Kanban To Aur (2026-05-04)

- ci: add AUR auto-publish workflow on release
- docs: add AUR installation instructions

### KAN-232 Fix C Key Can No Longer Complete Sprint From Sprint Detail View (2026-05-04)

- fix: suppress no planning sprint toast on c key sprint completion
- fix: restore c key sprint completion from sprint detail view

### KAN-233 Sync Web Index Html Roadmap With Readme Md (2026-05-04)

- feat: add roadmap item

### KAN-235 Per Layer Error Types Domainerror Persistenceerror Kanbanerror (2026-05-04)

- refactor(mcp,cli): use kanban_domain error types
- refactor(tui): use kanban_domain error types
- refactor(service): use kanban_domain::KanbanError with typed constructors
- feat(persistence): add PersistenceError/PersistenceResult
- feat(domain): add DomainError, DependencyError, and KanbanError wrapper
- refactor(core): slim to CoreError/CoreResult, remove KanbanError

### KAN-256 Fix Sqlite Db File Loading Read To String On Binary File (2026-05-04)

- fix: load SQLite files via persistence store instead of read_to_string

### KAN-258 Unify Initial File Loading Path Follow Up To Kan 256 (2026-05-04)

- test(tui): update tests to use async load_initial_state()
- fix(tui): unify initial file loading into async load_initial_state()
- feat(persistence-json): implement JSON content detection with BOM support
- feat(persistence-sqlite): implement SQLite content detection via magic bytes
- feat(persistence): add content-based detection to StoreFactory trait

### KAN-259 Make Sqlite A Default Feature In Kanban Cli (2026-05-04)

- feat: make sqlite-storage a default feature and remove redundant sqlite feature flags

### KAN-263 Rework Migrate Cli Backend As Positional Arg Filename As Option (2026-05-04)

- feat(cli): rework migrate command to use positional backend arg
- feat(service): add make_store_for_backend for explicit backend selection
- feat(persistence): add create_by_name and available_backend_names to StoreRegistry

### KAN-274 Settings Page Ui (2026-05-04)

## Settings page UI (`S`)
Press `S` from the boards view to open a two-column settings screen:
- **Configuration** panel — editing format, card/sprint prefixes, storage backend and location, config format and path. Navigate with `j`/`k` across rows, `h`/`l` or `1`/`2`/`3` to jump between panels.
- **Config File** panel — shows the resolved config path, whether it is loaded, and the serialization format.
- **Storage** panel — shows backend and data-file path; bottom row triggers the export dialog.
Press `e` or `Enter` (on Configuration panel) to open the config in an external editor. The file format respects `editing_format` (json or toml). Changes are validated and applied live; invalid values are rejected with an error banner.
## Storage backend switching
Changing `storage_location` in the editor triggers an async migration: data is copied to the new file, the store swaps in-place, and the UI reloads. If the destination already exists, data is loaded from it instead of migrated. The source backend is auto-detected from the file extension; mismatches between the configured backend and the actual file are corrected automatically with a warning.
## Export boards dialog (`x` in Settings)
Opens a board-selection checklist, then an options step to choose JSON or SQLite output and set a filename. JSON export is synchronous; SQLite export is async and reports success or failure via a banner when complete.
## `kanban migrate` CLI
```
kanban migrate <source> <backend> [--output <path>] [--source-backend <override>]
```
Source backend is auto-detected from the file extension. The output path defaults to the source stem with the target backend's extension.
## Config persistence (`~/.config/kanban/config.toml`)
Config is written only when at least one value differs from the compiled-in defaults. Default values are stripped before saving so the file stays minimal. Both TOML and JSON serialization formats are supported (`configuration_format`). The `editing_format` field now accepts `"toml"` in addition to `"json"`.
## Service layer additions
- `kanban_service::config::resolve_storage_location` — resolves relative storage paths to absolute (cwd join extracted from `kanban-core`, which is now a pure data crate).
- `kanban_service::migrate_store` — copies a snapshot between any two stores.
- `kanban_service::validate_and_load_store` — opens an existing store and verifies it is readable.
- `kanban_service::detect_backend` — infers the backend from a locator string.
- `KanbanContext::load_with_defaults` — convenience constructor used throughout tests.

### KAN-278 Hero Demo (2026-05-04)

Create polished hero demo for kanban TUI application
- Add pre-crafted JSON fixture with realistic development board
- Implement single VHS recording script showcasing core workflow
- Add Nix shell environment with vhs and neovim integration
- Create reproducible nvim wrapper for demo editing
- Build record script with automatic fixture reset
- Add comprehensive README documentation
- Replace fragile multi-tape setup with self-contained demo

### KAN-299 Extract Ui Rs Into Reusable Components And View Submodules (2026-05-04)

Splits the monolithic `ui.rs` (2,100 lines) in `kanban-tui` into focused, testable modules.
**New reusable components** (each with integration tests):
- `components/footer.rs` — keybinding footer bar
- `components/help_popup.rs` — help overlay and viewport height calculator
- `components/conflict_popup.rs` — file conflict and external-change dialogs
- `components/relationship_popup.rs` — parent/child card relationship picker
- `components/filter_popup.rs` — sprint/date/tag filter dialog
**View submodules** under `ui/`:
- `ui/mod.rs` — render entry point and dispatcher (~130 lines)
- `ui/main_view.rs` — main kanban board view
- `ui/settings_view.rs` — settings view
- `ui/card_detail.rs` — card detail view
- `ui/board_detail.rs` — board detail view
- `ui/sprint_detail.rs` — sprint detail view
- `ui/dialogs.rs` — thin dialog wrapper functions
No behaviour changes. All existing tests pass.

### KAN-300 Make Version Readout Of Web Landing Dynamic (2026-05-04)

- feat(web): inject version from workspace Cargo.toml at build time
- feat(web): replace hardcoded version with @VERSION@ placeholder

### KAN-305 Fix Config File Corruption And Unnecessary Writes (2026-05-04)

- fix(tui): clear cli_file_provided after migration so storage shows under Storage fields
- fix(tui): use correct selection indices for Active Storage rows in cli-only mode
- feat(tui): use Active Storage labels when storage source is CLI arg, Storage labels when config
- test(tui): add red-green tests for absolute path in Storage Location settings UI
- fix(tui): show resolved absolute path for Storage Location in settings UI
- fix(tui): unload cli_file_override when user explicitly provides storage in editor
- test(tui): add test for cli override unload when storage fields uncommented
- refactor(tui): extract is_storage_line helper; revert annotate editor change
- fix(tui): use annotate_storage_fields in editor when CLI file override is active
- fix(tui): add annotate_storage_fields to show storage as active lines with comment
- fix(tui): don't inject absolute storage path when CLI arg matches config default
- fix(tui): reset config storage to original values when DTO storage is unchanged
- test(tui): add tests for startup-injected absolute storage path not written to config
- fix(tui): strip unchanged storage from DTO to prevent spurious config writes
- test(tui): add test for unchanged storage not written to config
- fix(tui): CLI-supplied storage path is always session-only
- test(service): fix vacuous temp-file leak assertion in config write test
- fix(tui): skip config save when editor exits without changes
- fix(service): atomic write for config file to prevent corruption
- fix(service): promote tempfile to regular dependency

### KAN-326 Hide Grayed Config Storage Rows When Storage Is Not Set In Config (2026-05-04)

Hide grayed config storage rows when storage not set in config
- Only show grayed 'Storage Backend' / 'Storage Location' rows in the Configuration panel when storage is explicitly configured (original_storage_backend or original_storage_location is Some)
- When config defines no storage and a CLI file overrides the default, only Active Storage rows are shown, avoiding the misleading implication that CWD-resolved defaults are configured values

### KAN-339 Address Pr 208 0 4 0 Code Review Feedback (2026-05-04)

- fix(ci): validate release tag and fix sed delimiter in aur-publish
- fix(domain): cap redo stack at MAX_HISTORY_DEPTH
- test(domain): add redo stack bounded test
- fix(domain): validate column exists before restoring card
- test(domain): add restore card column validation test
- feat(domain): enforce WIP limits in CreateCard, MoveCard, MoveCards
- fix(domain): enforce WIP limits in RestoreCard
- test(domain): add failing WIP limit enforcement tests
- test(domain): add WIP limit enforcement test for RestoreCard
- feat(domain): add WipLimitExceeded error variant and predicate
- test(domain): add error.rs predicate and From conversion tests

### KAN-348 Refactor Storage To On Demand Querying Instead Of Full Snapshot In Memory (2026-05-04)

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

### KAN-364 Fix Tui Card Selection Opens Wrong Details (2026-05-04)

Fixed a bug where opening the card detail view would display the wrong card. The
detail view was resolving the selected card by indexing into
`cards_by_id.values()`, but `HashMap` iteration order is non-deterministic and
does not match the ordered position stored in `active_card_index`. This caused
the wrong card to be shown whenever the HashMap's internal order diverged from
the selection order.
The fix stores the selected card's UUID in `SelectionHub.active_card_id` when
entering the detail view and looks the card up directly by ID via the new
`App::get_card_for_detail_view` method, eliminating the ordering dependency.

### KAN-365 Block Quit During Migration With Double Q Ui (2026-05-04)

Pressing `q` while a storage migration is in progress no longer silently
abandons the migration. The app now shows a warning banner and requires a
second `q` to confirm the abort. If the migration completes before the
second `q` is pressed, the confirmation clears automatically and the next
`q` exits cleanly with no data loss.
This fixes a data loss scenario where triggering a JSON→SQLite migration
via the config editor and immediately pressing `q` would leave the
destination file unwritten.
Also fixes a startup regression where supplying an explicit file argument
(e.g. `kanban myboard.json`) was incorrectly treated as a SQLite file when
the config had `storage_backend = "sqlite"` set, causing a load error.

### KAN-366 Description Doesnt Load In Card Details (2026-05-04)

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
- `SqliteStoreFactory` added to `kanban-persistence-sqlite`, implementing `StoreFactory` with magic-byte content sniffing (`SQLite format 3 `)
- `SqliteStoreFactory` registered first in `default_registry()` so SQLite files are detected by content before JSON extension matching
- `is_sqlite` / `open_sqlite` bypass removed from `McpContext` and `CliContext` — all storage backends now routed uniformly through the registry
- `VERSION` constant extracted to a shared module; MCP and CLI now share a single version string source
- MCP server handles `-V` / `--version` flag cleanly — responds with version string and exits without error output

### KAN-371 Kanban Sqlite Add Explicit Wal Checkpoint On App Exit (2026-05-04)

SQLite storage now flushes pending writes to the main database file after
every save. Previously, SQLite's WAL mode accumulated changes in a
`.wal` sidecar file that could grow to several MB between checkpoints,
meaning a backup of just the `.db` file could be missing recent data.
Every write — whether from the TUI, CLI, or MCP server — now triggers a
`PRAGMA wal_checkpoint(TRUNCATE)`, keeping the WAL file at near-zero size
after each operation. Backups of the `.sqlite` file are now always
complete and self-contained.

### KAN-383 Bug X In Archived Cards View Restores Card Instead Of Hard Deleting It Sqlite (2026-05-04)

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

### KAN-384 Architecture Unified Backends Via True Deferred Reads (2026-05-04)

## Description
Unified the storage backend architecture so that both JSON and SQLite
backends are opened with zero I/O at construction time. Data is loaded
lazily on the first read, keeping startup fast and making the two
backends interchangeable through a single `open_context()` entry point.
## New Features
- **`open_context(locator, config)`** — single async function that
  opens any supported backend (JSON or SQLite) by detecting the file
  type automatically from magic bytes or extension, then returns a
  ready-to-use `KanbanContext`. No per-backend wiring required in
  callers.
- **Lazy JSON backend (`JsonDataStore`)** — wraps a JSON persistence
  store with an in-memory cache that is populated only on the first
  read. Subsequent reads are served from the cache; writes set a dirty
  flag and are flushed to disk explicitly via `save()` or by the
  background save worker.
- **`KanbanBackend` lifecycle methods** — `flush()`, `reload()`,
  `needs_flush()`, `needs_save_worker()`, and `on_undo_state_changed()`
  give callers a uniform interface for durability and conflict detection
  across all backend types.
## Improvements
- `KanbanContext::open` is now the single zero-I/O constructor for all
  backends. The legacy `open_sqlite` / `open_json` constructors are
  retained for backward compatibility but delegate to the new path.
- The TUI flush signal replaces the old snapshot-save channel, removing
  a layer of indirection and aligning JSON saves with the SQLite
  checkpoint model.
- Backend type is auto-detected from file content (magic bytes for
  SQLite, leading `{` / `[` for JSON), so files without a recognised
  extension are handled correctly.
## Fixes
- `StoreManager::make_backend` now correctly detects SQLite databases
  that have no file extension by reading the SQLite magic-byte header,
  preventing them from being opened as (invalid) JSON stores.
## Deprecations
None.
## Testing
Full contract coverage added for the new architecture:
- `KanbanBackend` lifecycle tests for `SqliteStore` (needs_flush, WAL
  checkpoint, reload no-op).
- `JsonDataStore` command-log round-trip (flush → reopen → command
  count matches).
- `StoreManager::make_backend` — JSON path, SQLite path, magic-byte
  detection, and content-sniffing for extension-less files.
- `KanbanContext::open` integration suite — zero-I/O construction,
  lazy load on first read, undo/redo with lazy baseline, save/reload
  delegation, and external-change pickup after `reload()`.
- `open_context()` end-to-end suite — JSON round-trip, SQLite
  round-trip, magic-byte auto-detection, new-file-starts-empty.

### KAN-391 Fix Validate Release Staleness (2026-05-04)

- fix(ci): derive release-script crate list dynamically via cargo metadata
- fix(ci): propagate list-crates failures cleanly to release-script consumers
- fix(ci): broaden crate-list-sync drift regex to catch inline arrays
- test(ci): add crate list sync invariant guard


## [0.3.5] - 2026-03-22 ([#193](https://github.com/fulsomenko/kanban/pull/193))

### KAN-229 Fix Publish Crates Order Add Kanban Service Before Kanban Mcp (2026-03-22)

- fix(ci): add kanban-service to publish script and order mcp as last


## [0.3.4] - 2026-03-22 ([#191](https://github.com/fulsomenko/kanban/pull/191))

### KAN-123 Escape Bind To Clear Search Enter To Apply (2026-03-22)

- fix: remove trailing spaces from active search footer text
- KAN-123: update search mode keybinding descriptions
- KAN-123: show active search filter indicator in footer
- KAN-123: highlight search matches in card titles
- KAN-123: split Enter/Esc in search mode and add n/N navigation

### KAN-221 Help Menu List Doesnt Scroll (2026-03-22)

- fix(help): fixed header/footer layout with ListComponent scroll in render_help_popup
- refactor(help): replace help_selection+help_page with help_list ListComponent
- refactor(generic_list): delegate get_adjusted_viewport_height to Page
- refactor(pagination): add get_adjusted_viewport_height to Page
- refactor: use render_scroll_indicators helper at all scroll indicator sites
- feat: add scroll support to help menu popup (KAN-221)
- refactor: generalize render_scroll_indicators to accept plain args and label

### KAN-222 Fix Post Search Ux Issues Gg Scroll Unicode Panic Footer Hint N N Nav (2026-03-22)

- fix: remove n/N search-navigation shortcuts — n is for new card only
- fix: drop n/N from active-search footer hint — redundant with j/k when results are filtered
- fix: active-search footer shows navigation hint alongside ESC
- refactor: add SearchState::active_query() and collapse repeated search_query expressions
- fix: Unicode panic in build_title_spans — map lowercase byte offsets back to original
- fix: gg jumps to top but doesn't scroll view — call ensure_selected_visible

### KAN-224 Decompose App Rs Into Focused Modules (2026-03-22)

- refactor: decompose app.rs into focused sub-modules
- Split 2060-line `app.rs` into 12 focused sub-modules under `app/`
- Each concern now lives in its own file: `mode`, `focus`, `selection`, `filter`, `multi_select`, `dialog_input`, `sprint_view`, `relationship`, `view`, `animation`, `persistence`, `ui_state`
- Zero behavioral change — all types re-exported from `app/mod.rs`

### KAN-226 Extract Kanban Service Crate From Kanban Cli Handlers (2026-03-22)

- docs: use graph LR for dependency diagram in README to match other docs
- docs: replace repeated workspace graphs in crate READMEs with CONTRIBUTING.md links
- docs: add kanban-service to architecture section with Mermaid diagram
- docs: update CONTRIBUTING.md workspace structure to 7 crates with Mermaid diagram
- docs: update CLAUDE.md workspace structure to 7 crates with Mermaid diagram
- docs: replace StateManager references with KanbanContext in persistence README
- docs: rewrite kanban-mcp README for in-process KanbanContext architecture
- docs: add kanban-service README
- test: verify reload() picks up external changes to the kanban file
- feat: reload from disk before every mutating_op in kanban-mcp
- feat: add reload() to KanbanContext to re-read state from disk
- test: restore McpContext in kanban-mcp integration tests, add persistence coverage
- test: add KanbanContext persistence round-trip tests
- refactor: replace parking_lot::Mutex with tokio::sync::Mutex in kanban-mcp
- refactor: remove instance_id field and save_sync from KanbanContext
- chore: remove kanban binary dep from kanban-mcp Nix build
- test: rewrite kanban-mcp integration tests to use KanbanContext directly
- refactor: migrate McpContext to KanbanContext, delete subprocess executor
- refactor: delegate CliContext to KanbanContext from kanban-service
- feat: add kanban-service crate with KanbanContext over PersistenceStore


## [0.3.3] - 2026-03-18 ([#184](https://github.com/fulsomenko/kanban/pull/184))

### KAN-220 Fix Kanban Binary Discovery In Mcp Integration Tests For Nix Builds (2026-03-18)

- fix: check direct target profiles before triple subdirs in kanban_bin()
- fix: discover kanban binary across target triples and profiles in integration tests


## [0.3.2] - 2026-03-18 ([#182](https://github.com/fulsomenko/kanban/pull/182))

### KAN-217 Mcp List Cards Pagination Returns Max 50 Cards Instead Of All Cards (2026-03-18)

- fix: pass page/page_size through to CLI subprocess in MCP list_cards
- refactor: change list_cards to return Vec<CardSummary> instead of Vec<Card>

### KAN-218 Gate Kanban Tui Behind Default Feature Flag In Kanban Cli (2026-03-18)

- ci: add no-tui build check
- fix: improve no-tui error message to point to --help
- feat: build kanban-mcp with no-tui kanban binary to skip wayland/xcb
- feat: gate kanban-tui behind optional 'tui' default feature


## [0.3.1] - 2026-03-17 ([#179](https://github.com/fulsomenko/kanban/pull/179))

### KAN-216 Changelog Md Grouping By Card (2026-03-17)

- docs: retroactively group CHANGELOG entries by changeset for 0.1.11–0.3.0
- fix: group changelog entries by changeset in aggregate-changelog.sh


## [0.3.0] - 2026-03-17 ([#175](https://github.com/fulsomenko/kanban/pull/175))

### KAN-193 Bring Mcp To Full Feature Parity With Cli Tui Via Kanbanoperations Trait (2026-03-17)

- test: add integration tests for MCP round-trips
- test: add unit tests for MCP helpers and ArgsBuilder
- feat: update MCP server tools for full CLI parity
- feat: bring MCP context to full parity with CLI
- fix: error handling in MCP executor
- feat: add sprint update fields to CLI (name, dates, clear flags)
- feat: add --clear-wip-limit flag to CLI column update
- feat: rewrite MCP server with 37 tools via KanbanOperations trait
- feat: remove McpTools trait, replaced by KanbanOperations from kanban-domain
- feat: add McpContext implementing KanbanOperations trait
- feat: replace async CliExecutor with sync SyncExecutor
- feat: add kanban-domain, kanban-core, uuid, chrono, tempfile deps to kanban-mcp
- fix: remove create_card_full bypass, use trait two-step create+update pattern
- fix: remove update_sprint_full bypass, route through trait's update_sprint
- feat: add name field to SprintUpdate for MCP name passthrough
- fix: remove broken clear_description and clear_points MCP flags
- refactor: remove 4 dead pre-animation functions from TUI card_handlers

### KAN-196 Redesign Release Workflow Defer Version Bump To Master Merge (2026-03-17)

- fix: address PR review findings for release workflow
- fix: quote variable in parameter expansion to satisfy shellcheck SC2295
- chore: wire all scripts into nix dev shell
- fix: use robust frontmatter parsing in changeset-check
- fix: reorder release workflow to validate before push
- refactor: extract changelog aggregation into standalone script
- fix: exclude README.md from changeset detection in bump-version.sh
- fix: defer version bump to master merge

### KAN-197 Add Card Identifier Search Prefix Number (2026-03-17)

- feat: add card identifier search (KAN-197)

### KAN-208 Fix Shift Y Branch Copy Crash On Linux Nixos Wayland (2026-03-17)

- docs: document Linux clipboard manager requirement
- refactor(tui): replace last_error with unified Banner system
- feat(tui): add reusable Banner component
- feat(tui): enable Wayland support with clipboard manager handoff
- chore: add Wayland/X11 clipboard dependencies

### KAN-209 Multi Select Cards (2026-03-17)

- feat(tui): add bulk priority popup rendering
- feat(tui): add selection mode indicator to footer
- feat(tui): handle SetMultipleCardsPriority dialog in event loop
- feat(tui): add keyboard shortcuts for multi-select
- feat(tui): wire keybinding actions in execute_action
- feat(tui): add bulk priority popup handler
- feat(tui): update escape handler for selection mode
- feat(tui): add auto-select on navigation in selection mode
- feat(tui): implement vim-style selection mode toggle
- feat(tui): add bulk move for selected cards
- feat(tui): add card selection handler functions
- feat(tui): add card list keybindings for bulk operations
- feat(tui): register bulk priority dialog provider
- feat(tui): add BulkPriorityDialog component
- feat(tui): add keybinding actions for multi-select operations
- feat(tui): add SetMultipleCardsPriority dialog mode
- feat(tui): add selection_mode_active field to App

### KAN-210 Find Cards By Prefix Increment Identifier E G Kan 5 (2026-03-17)

- feat(mcp): resolve card identifier (e.g. KAN-5) in all card tools
- feat(cli): accept card identifier (e.g. KAN-5) in all card commands
- feat(cli,tui,mcp): implement find_card_by_identifier in all contexts
- feat(domain): add find_card_by_identifier to KanbanOperations trait
- fix(domain): use sprint card_prefix in identifier resolution
- fix(domain): PrefixAndNumber with no resolved prefix returns no match instead of falling back to "task"
- fix(cli): remove redundant find-by-identifier subcommand (card get KAN-5 already works)

### KAN-212 Add Compact Names Only Flag To Card Listing For Token Efficient Search (2026-03-17)

- feat(core): add PaginatedList<T> with paginate() helper and resolve_page_params() utility
- feat(domain): add ArchivedCardSummary with From<&ArchivedCard> impl
- feat(cli): card list defaults to CardSummary (no description); use card get for full details
- feat(cli): add --page, --page-size flags to card, board, column, sprint list
- feat(cli): archived card list returns PaginatedList<ArchivedCardSummary>
- feat(mcp): tool_list_cards and tool_list_archived_cards return PaginatedList<CardSummary>
- test(cli): card list pagination, summary shape, out-of-bounds page

### KAN-215 Version Flag (2026-03-17)

- nix: inject self.rev as GIT_COMMIT_HASH in Nix builds
- fix: suppress commit: line in -V when git hash is unknown
- fmt: wrap long lines

## [0.2.0] - 2026-02-01

### KAN-134 Undo Action (2026-02-01)

- feat(tui): register undo/redo keybindings in CardList provider
- feat(tui): register undo/redo keybindings in BoardDetail provider
- feat(tui): register undo/redo keybindings in CardDetail provider
- feat(tui): register undo/redo keybindings in NormalMode providers
- feat(tui): add Undo and Redo KeybindingAction variants
- feat(tui): add undo() and redo() methods to App
- feat(tui): capture snapshots before command execution for undo history
- feat(tui): integrate HistoryManager into StateManager
- feat(tui): create HistoryManager module for undo/redo support

### KAN-170 Cascade Cleanup Delete Operations (2026-02-01)

- test: add cycle detection tests for dependency graph
- test: add integration tests for cascade cleanup operations
- feat(domain): unassign cards when deleting sprints
- feat(domain): add validation to DeleteColumn command
- feat(domain): implement cascade cleanup in card deletion and archival
- feat(domain): add cascade cleanup methods to DependencyGraph trait

### KAN-177 Parent And Child Relationship Boxes Layout (2026-02-01)

- feat(tui): implement backward wrap-around navigation from title to children
- feat(tui): add scrolling support to parent/child relationship boxes
- feat(tui): Implement interactive navigation for parent/child relationship boxes
- feat(tui): add infrastructure for parent/child relationship navigation
- feat(tui): Display parent/child relationship boxes side-by-side with increased height

### KAN-178 Tui To Domain Refactoring Migration (2026-02-01)

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

### KAN-6 Card Dependencies (2026-02-01)

- feat(tui): Add TUI for managing parent-child card relationships
- feat(domain): Add commands for parent-child card relationships
- feat(domain): Add ParentOf edge type for hierarchical card grouping
- feat(tui,cli): integrate dependency graph into persistence
- feat(domain): add dependency management commands
- feat(domain): add card dependency graph types
- feat(core): add graph-related error variants
- feat(core): add graph cycle detection algorithms
- feat(core): add generic Graph<E> data structure
- feat(core): add graph module with edge types and GraphNode trait


## [0.1.16] - 2025-12-21

### Other Changes (2025-12-21)

- chore: bump version to 16

### KAN-154 P Dialog Does Not Correctly Set Points (2025-12-21)

- fix: points dialog now correctly updates card from detail view

## [0.1.15] - 2025-12-21

### KAN-129 Include Commit Hash In V (2025-12-21)

- feat(cli): include git commit hash in version output

### KAN-139 If No Sprints Cant Scroll To Column Settings (2025-12-21)

- fix(tui): skip empty sprints section when navigating board details

### KAN-140 Filter Out Completed Sprints From Assign List (2025-12-21)

- fix: filter out completed and cancelled sprints from assign list

### KAN-141 Scrolling Up From Column Options Lands The Cursor On The First Sprint In The List (2025-12-21)

- fix: navigate to last sprint when scrolling up from columns in board settings

### KAN-142 Updating Fields Jumps The User Back To Board 2 (2025-12-21)

- fix: preserve navigation mode during auto-reload from external changes

### KAN-143 Gg G Works Poorly (2025-12-21)

- chore: cargo fmt
- fix(tui): fix gg/G vim navigation in grouped-by-column view
- chore: remove wip file

### KAN-144 Kanban View Switches Column On The Second To Last Item (2025-12-21)

- fix: prevent premature column switching in handle_navigation_down

### KAN-145 We Broke The File Watcher Having A Conflict With One Instance (2025-12-21)

- fix: Centralize file watcher pause/resume in StateManager

### KAN-146 Kanban Mcp (2025-12-21)

- feat: add kanban-mcp server
- feat(mcp): add McpTools trait for compile-time parity with KanbanOperations
- docs(mcp): add subprocess architecture documentation and Nix wrapper
- feat(mcp): add CLI executor for subprocess-based operations
- feat(mcp): enhance card operations and add delete/archive functionality
- feat: add kanban-mcp: Model Context Protocol server implementation

### KAN-147 Multiselecting And Assigning Cards Causes Write Race Condition (2025-12-21)

- fix: batch card creation with optional status update
- fix: batch card movements with conditional status updates
- fix: batch sprint activation and completion with board updates
- fix: batch column position swaps
- fix: batch card unassignment from sprint
- fix: batch card completion toggles
- fix: batch card moves when deleting column
- fix: batch default column creation to prevent conflict dialog on new board
- refactor: use batch command execution in sprint assignment handlers
- feat: add execute_commands_batch for race-free command execution
- fix: enhance AssignCardToSprint to handle sprint log transitions

### KAN-148 Archiving Deleting Cards Is Broken (2025-12-21)

- fix: batch card archive and delete operations in animation completion

### KAN-15 Progressive Saving Detect Changes To Current Json (2025-12-21)

- feat(persistence): create kanban-persistence crate structure
- feat(state): create Command trait and StateManager
- feat(domain): add CreateBoard command
- feat(domain): add active_sprint_id field to BoardUpdate
- feat(state): add debouncing to StateManager::save_if_needed()
- feat(persistence): add automatic V1→V2 migration on load
- feat(core,persistence): add conflict detection for multi-instance saves
- feat(persistence): detect file conflicts before save
- feat(state): propagate conflict errors in StateManager
- feat(tui): Implement conflict resolution dialog and event loop integration
- feat(tui): Integrate FileWatcher with App event loop
- feat(state): Add view refresh tracking to StateManager
- feat(tui): Add ExternalChangeDetected dialog
- feat(tui): add user-visible error display banner
- feat(app): prevent quit with pending saves
- feat(app): add save completion receiver to App struct
- feat(state): add bidirectional save completion channel

### KAN-150 File Path To A Non Existant File Crashes The App (2025-12-21)

- feat: Add migration verification and automatic backup cleanup
- fix: Add instance ID check to file watcher to prevent false positives
- fix: Remove redundant version fields from PersistenceMetadata

### KAN-151 Kanban Cli (2025-12-21)

- fix(tui): restoring restoring cards
- fix(cli): restoring to a non existing column
- docs: add CLI quick start section to root README
- docs: update CLI README with command documentation
- fix: use get_selected_card_in_context for points dialog
- feat: add TuiContext struct with KanbanOperations implementation
- feat: implement KanbanOperations trait for TUI App
- test: update CLI tests for positional ID arguments
- feat: make ID positional argument for single-resource commands
- fix: return descriptive errors for invalid priority and status values
- feat: add API version to CLI output and document never type
- feat: simplify CLI file argument and add shell completions
- fix: CLI context bugs and improve error messages
- fix: Support positional file argument for TUI mode
- test: Add comprehensive integration tests for CLI
- feat: Implement full CLI with subcommand interface
- feat: Add KanbanOperations trait for TUI/CLI feature parity

### KAN-152 Dont Include Description Of Card For Get Cards (2025-12-21)

- feat(mcp): omit description and sprint_logs from card list responses
- feat(cli): include git commit hash in version output (#132)

### KAN-155 Publish New Version (2025-12-21)

- fix: stabilize release pipeline for v0.1.15

### KAN-30 Vim Motions (2025-12-21)

Jumping cards

- fix: jump by actual visible cards count from render_info, not cards_to_show
- feat: add vim jump motions to normal mode keybinding display
- feat: add vim jump motions to card list keybinding display
- feat: wire up vim jump motions to keybinding handlers
- feat: add jump motion handlers
- feat: add jump methods to CardList
- feat: add jump_to_first and jump_to_last methods to SelectionState
- feat: add jump action variants to KeybindingAction enum
- feat: add pending_key field to App struct for multi-key sequences

### KAN-93 Dialogs Always Return To Main When Opened (2025-12-21)

Refactored dialog mode handling to use nested AppMode::Dialog(DialogMode) enum
for type-safe dialog management. Dialogs now correctly display their parent
view in the background instead of hardcoded destinations.

- Added DialogMode enum with all 23 dialog variants
- Simplified is_dialog_mode() to matches!(self.mode, AppMode::Dialog(_))
- Added get_base_mode() to determine parent view from mode_stack
- Two-phase rendering: base view first, then dialog overlay
- Converted all push_mode(AppMode::X) calls to open_dialog(DialogMode::X)

## [0.1.14] - 2025-11-17 ([#patch](https://github.com/fulsomenko/kanban/pull/patch))

### KAN-111 Sprint Binding Help Is Wrong (2025-11-17)

- refactor: integrate card list into keybinding registry
- refactor: unify keybinding management for footer and help popup

### KAN-118 Unfilter Tasks List On Completed Sprint (2025-11-17)

Remove filtering of cards from completed sprints

- fix: remove auto-hiding of completed sprint cards from app methods
- fix: remove auto-hiding of completed sprint cards from view strategies

### KAN-130 Three Card List Components To Become One (2025-11-17)

A refactoring.

- refactor: simplify navigation handlers to work with unified view strategy
- refactor: simplify card handlers to work with unified view strategy
- refactor: update app initialization to use UnifiedViewStrategy
- refactor: simplify render_tasks to use unified view strategy
- refactor: introduce UnifiedViewStrategy to compose layout and render strategies
- refactor: create render strategy abstraction for card list rendering
- refactor: create layout strategy abstraction for card list management
- refactor: extract card filtering and sorting logic into card_filter_service
- KAN-118/unfilter-tasks-list-on-completed-sprint (#93)
- KAN-111/sprint-binding-help-is-wrong (#92)
- KAN-33: Add Help mode with context-aware keybindings (#91)
- ci: automatically sync develop with master after release (#90)

### KAN-132 Urgent Migrations (2025-11-17)

- migration: add reconciliation of branch_prefix and sprint_prefix to migrate old boards
- migration: add serde default to support migration to archived cards board

### KAN-133 Scrolling Doesnt Work In Grouped By Columns List (2025-11-17)

- feat: Synchronize navigation viewport with grouped view column headers
- feat: Implement unified scrolling rendering for grouped view
- feat: Wire up VirtualUnifiedLayout for grouped view mode
- feat: Add VirtualUnifiedLayout for unified card scrolling in grouped view

### KAN-196 Make Help Menu Items Selectable And Activateable (2025-11-17)

- fix: help menu keybinding matching for special keys and /
- fix: implement missing action handlers for help menu
- refactor: couple keybindings with actions
- feat: add visual selection to help popup
- feat: add generic list component

### KAN-20 Remove A Card (2025-11-17)

- chore: simplify archived cards view keybindings
- refactor: rename delete to archive, permanent delete to delete
- refactor: consolidate keybinding providers into CardListProvider
- feat: add animation state infrastructure and types
- feat: add yellow border for deleted cards view visual distinction
- feat: add card deletion from detail view
- fix: card lookup in DeletedCardsView mode
- feat: add deleted cards UI rendering
- feat: add keybindings for card deletion
- feat: implement card deletion with position compacting
- feat: add DeletedCardsView mode to App
- feat: add deleted_cards persistence
- feat: add DeletedCard domain model

### KAN-33 Add Binding (2025-11-17)

Add help dialogue for keybindings.

- feat: implement Help popup rendering with context-aware keybindings
- feat: add global ? key handler for help across all modes
- refactor: make CardFocus and BoardFocus Copy
- feat: add Help app mode with context preservation
- feat: create keybinding registry to route contexts
- feat: implement keybinding providers for all contexts
- feat: create keybindings module with traits and data structures
- refactor: add keybindings module to lib
- ci: automatically sync develop with master after release (#90)

### KAN-55 Scroll In Cards List (2025-11-17)

- fix: ensure forward progress when viewport shrinks during down navigation
- fix: correct viewport height calculation across all renderers
- feat: add viewport calculation infrastructure to CardList
- fix: allow scrolling down to show the final card
- feat: update navigation to account for scroll indicator space
- feat: add scroll indicators showing tasks above and below viewport
- feat: use actual viewport_height instead of hardcoded value
- feat: calculate and update viewport_height during rendering
- feat: add viewport_height tracking to App
- fix: eliminate selector jitter by moving selection with scroll
- refactor: remove preemptive ensure_selected_visible calls
- refactor: update CardListComponent navigate methods for viewport awareness
- refactor: implement scroll-on-boundary logic in navigate methods
- feat: wire up automatic scroll adjustment on navigation
- feat: implement scroll-aware rendering for sprint detail panels
- feat: implement scroll-aware rendering in all card list views
- feat: expose scroll management in CardListComponent
- feat: add scroll offset tracking to CardList


## [0.1.12] - 2025-11-02 ([#patch](https://github.com/fulsomenko/kanban/pull/patch))

### KAN-117 Workflows And Releases (2025-11-02)

Update release flow

- chore: remove unnecessary backup logic from update-changelog script
- chore: update bump-version script to output new version
- ci: enhance release workflow with version bump and changelog
- ci: simplify aggregate-changesets workflow
- fix: prevent stdout pollution of GITHUB_OUTPUT in release workflow


## [0.1.11] - 2025-11-02 ([#patch](https://github.com/fulsomenko/kanban/pull/patch))

### KAN-105 We Probably Should Move Sprint Prefix Into Sprint Level Settings (2025-11-02)

- refactor: fix clippy enum variant naming warnings
- chore: cargo fmt
- refactor: consolidate copy methods with generic implementation
- refactor: create generic prefix dialog handler abstraction
- refactor: remove dead code render_sprint_task_panel
- fix: remove used for filtering output
- fix: scope sprint counter initialization to board context
- feat: show active sprint card prefix override in board details
- feat: add board_context module for board-related queries
- feat: initialize sprint counter when prefix is assigned
- feat: initialize sprint counter when creating new sprints
- feat: add Board::ensure_sprint_counter_initialized method
- fix: separate default prefixes for sprints and cards
- test: fix import test to include new card_prefix field
- test: add integration tests for export/import with prefixes (Phase 4D)
- test: add backward compatibility tests (Phase 4C)
- test: add card prefix hierarchy tests (Phase 4B)
- feat: display separate sprint and card prefix fields in UI
- feat: add UI rendering for SetSprintCardPrefix dialog
- fix: rename branch_prefix to sprint_prefix throughout codebase
- feat: update sprint creation to use per-prefix sprint counters
- feat: add separate sprint_prefix and card_prefix to BoardSettingsDto
- feat: add card_prefix field to Card domain model
- feat: add card_prefix field to Sprint domain model
- feat: add card_prefix field to Board domain model
- chore: cargo fmt
- chore: add changeset
- feat: add help text for sprint prefix collision confirmation
- feat: set assigned_prefix when assigning cards to sprints
- feat: add sprint prefix collision confirmation mode
- test: update Card::new() call sites to use prefix parameter
- fix: resolve borrow checker constraint in create_card handler
- feat: update Card::new() signature to accept and use prefix parameter
- feat: add assigned_prefix field to Card domain model
- feat: add prefix registry system to Board domain model
- feat: Implement sprint prefix editing UI and handlers
- feat: Add sprint prefix settings support to domain and app modes
- refactor: simplify effective_prefix() using or() instead of or_else()
- refactor: remove board.sprint_prefix from TUI layer
- refactor: add Sprint.effective_prefix() and update branch name logic
- refactor: remove sprint_prefix from Board and BoardSettingsDto
- refactor: rename Sprint.prefix_override to Sprint.prefix

### KAN-109 Choose Which Sprint To Filter By (2025-11-02)

Adding a dialogue to chose card filters

- feat: support filtering by multiple sprints simultaneously
- feat: display all active filters in card list header
- chore: cargo fmt
- fix: simplify Space key handler to remove clippy single-match warning
- refactor: merge unassigned sprints into sprints section with graphical separation
- feat: apply filters immediately when toggled in dialog
- feat: implement filter dialog item selection and cursor feedback
- feat: add filter dialog UI rendering
- feat: implement filter dialog handlers
- feat: add filters module with FilterOptions AppMode

### KAN-113 Dont Add To Sprint Log For A Card If The Same Sprint Is Added (2025-11-02)

- fix: prevent duplicate sprint log entries when reassigning to same sprint

### KAN-95 Marketing (2025-11-02)

- add demo

### MVP-108 Keep A Log Of Sprints For A Card (2025-11-02)

Implement log for sprints that a card has seen

- chore: cargo fmt
- feat: integrate sprint logging into card-to-sprint assignment
- feat: add sprint logging to Card domain model
- feat: add SprintLog struct for tracking sprint history
- feat: add logging abstraction to kanban-core

### MVP-110 In Card Metadata Show The Sprint Log For A Card (2025-11-02)

Adding a sprint history view to Card Details

- feat: increase sprint history display to 4 elements
- feat: show sprint history tail with correct absolute indexing
- feat: migrate sprint logs for existing assigned cards
- feat: display sprint history in card detail view

### MVP-40 Make Card Meta Data Editing Like Board Settings Edit (2025-11-02)

Introduce JSON editing for card meta

- refactor: swap keybindings - 'p' for points, 'P' for priority in card detail
- chore: cargo fmt
- refactor: remove unused BoardSettingsDto import from app.rs
- chore: update Cargo.lock
- feat: use generic editor for card metadata and board settings
- feat: add generic edit_entity_json_impl method for JSON-based entity editing
- feat: add BoardSettingsDto and CardMetadataDto with Editable implementations
- feat: add Editable<T> trait for entity subset editing


## [0.1.10] - 2025-11-02

### KAN-105 We Probably Should Move Sprint Prefix Into Sprint Level Settings (2025-11-02 15:57)

- refactor: fix clippy enum variant naming warnings
- chore: cargo fmt
- refactor: consolidate copy methods with generic implementation
- refactor: create generic prefix dialog handler abstraction
- refactor: remove dead code render_sprint_task_panel
- fix: remove used for filtering output
- fix: scope sprint counter initialization to board context
- feat: show active sprint card prefix override in board details
- feat: add board_context module for board-related queries
- feat: initialize sprint counter when prefix is assigned
- feat: initialize sprint counter when creating new sprints
- feat: add Board::ensure_sprint_counter_initialized method
- fix: separate default prefixes for sprints and cards
- test: fix import test to include new card_prefix field
- test: add integration tests for export/import with prefixes (Phase 4D)
- test: add backward compatibility tests (Phase 4C)
- test: add card prefix hierarchy tests (Phase 4B)
- feat: display separate sprint and card prefix fields in UI
- feat: add UI rendering for SetSprintCardPrefix dialog
- fix: rename branch_prefix to sprint_prefix throughout codebase
- feat: update sprint creation to use per-prefix sprint counters
- feat: add separate sprint_prefix and card_prefix to BoardSettingsDto
- feat: add card_prefix field to Card domain model
- feat: add card_prefix field to Sprint domain model
- feat: add card_prefix field to Board domain model
- chore: cargo fmt
- chore: add changeset
- feat: add help text for sprint prefix collision confirmation
- feat: set assigned_prefix when assigning cards to sprints
- feat: add sprint prefix collision confirmation mode
- test: update Card::new() call sites to use prefix parameter
- fix: resolve borrow checker constraint in create_card handler
- feat: update Card::new() signature to accept and use prefix parameter
- feat: add assigned_prefix field to Card domain model
- feat: add prefix registry system to Board domain model
- feat: Implement sprint prefix editing UI and handlers
- feat: Add sprint prefix settings support to domain and app modes
- refactor: simplify effective_prefix() using or() instead of or_else()
- refactor: remove board.sprint_prefix from TUI layer
- refactor: add Sprint.effective_prefix() and update branch name logic
- refactor: remove sprint_prefix from Board and BoardSettingsDto
- refactor: rename Sprint.prefix_override to Sprint.prefix

### KAN-109 Choose Which Sprint To Filter By (2025-11-02 15:57)

Adding a dialogue to chose card filters
- feat: support filtering by multiple sprints simultaneously
- feat: display all active filters in card list header
- chore: cargo fmt
- fix: simplify Space key handler to remove clippy single-match warning
- refactor: merge unassigned sprints into sprints section with graphical separation
- feat: apply filters immediately when toggled in dialog
- feat: implement filter dialog item selection and cursor feedback
- feat: add filter dialog UI rendering
- feat: implement filter dialog handlers
- feat: add filters module with FilterOptions AppMode

### KAN-113 Dont Add To Sprint Log For A Card If The Same Sprint Is Added (2025-11-02 15:57)

- fix: prevent duplicate sprint log entries when reassigning to same sprint

### KAN-95 Marketing (2025-11-02 15:57)

- add demo

### MVP-108 Keep A Log Of Sprints For A Card (2025-11-02 15:57)

Implement log for sprints that a card has seen
- chore: cargo fmt
- feat: integrate sprint logging into card-to-sprint assignment
- feat: add sprint logging to Card domain model
- feat: add SprintLog struct for tracking sprint history
- feat: add logging abstraction to kanban-core

### MVP-110 In Card Metadata Show The Sprint Log For A Card (2025-11-02 15:57)

Adding a sprint history view to Card Details
- feat: increase sprint history display to 4 elements
- feat: show sprint history tail with correct absolute indexing
- feat: migrate sprint logs for existing assigned cards
- feat: display sprint history in card detail view

### MVP-40 Make Card Meta Data Editing Like Board Settings Edit (2025-11-02 15:57)

Introduce JSON editing for card meta
- refactor: swap keybindings - 'p' for points, 'P' for priority in card detail
- chore: cargo fmt
- refactor: remove unused BoardSettingsDto import from app.rs
- chore: update Cargo.lock
- feat: use generic editor for card metadata and board settings
- feat: add generic edit_entity_json_impl method for JSON-based entity editing
- feat: add BoardSettingsDto and CardMetadataDto with Editable implementations
- feat: add Editable<T> trait for entity subset editing


## [0.1.10] - 2025-10-26 ([#75](https://github.com/fulsomenko/kanban/pull/75))

### MVP-77 Changeset Script To Add Timestamp Of Changeset Creation And Card Name (2025-10-26)
- feat: group changelog entries by card with timestamps and branch names

### MVP-101 Add Column Header For Non-Assigned Filter (2025-10-26)
- refactor: extract tasks panel title builder
- refactor: extract filter title suffix helper
- feat: add unassigned cards header to filter view

### MVP-29 Search In Cards List (2025-10-26)
- style: make search query text white for better visibility
- feat: add vim-style search query display in footer
- refactor: consolidate refresh_view and refresh_preview functions
- Add Search mode help text to footer
- Integrate search functionality into App
- Add search query parameter to view strategies
- Add search module to crate exports
- Add search module with trait-based architecture

### MVP-49 Hitting 'Q' In Dialogue Quits The Program (2025-10-26)
- fix: exclude AppModes with text input form the `q` to quit binding

### MVP-86 Missing Sprint Header For Sprint Filter In Kanban View (2025-10-26)
- feat: add sprint filter indicator to kanban view

### MVP-90 Moving Cards From Last Column Doesn't Uncomplete (2025-10-26)
- Update CardListAction::MoveColumn handler to reflect card status changes
- Fix handle_move_card_right to complete cards moved to last column
- Fix handle_move_card_left to uncomplete cards moved from last column

### KAN-81 J K Doesn't Work On Empty Columns (2025-10-26)
- Fix j/k navigation on empty card lists. Pressing j/k on an empty column now correctly navigates to adjacent columns instead of doing nothing.

### MVP-60 Move Card Out Of Completed Column Doesn't Unmark As Complete (2025-10-26)
- fix: moving cards from the last column should uncomplete said card

### KAN-94 When Opening A Dialogue Put Selector On The Currently Selected Item (2025-10-26)
- refactor: delegate dialog rendering to SelectionDialog components
- refactor: use dialog selection state helpers in event handlers
- refactor: export SelectionDialog component from components module
- refactor: create SelectionDialog trait and implementations
- refactor: add dialog selection state helpers to app
- Implement CardListComponent for reusable card list interactions (#65)

### MVP-35 Make J K Work For Changing Panels (2025-10-26)
- Add vim-style j/k navigation for panel changes in detail views
- Enable j/k keys to navigate between panels in CardDetail view (Title, Metadata, Description)
- Enable j/k keys to navigate between panels in BoardDetail view (Name, Description, Settings, Sprints, Columns)
- Wrap navigation at list boundaries: reaching the end of Sprints/Columns lists transitions to next panel
- Both arrow keys and vim-style j/k keys work consistently across all views

### MVP-68 Treesitter For Syntax Highlighting (2025-10-26)
- Add markdown rendering support for task and board descriptions
- Integrate pulldown-cmark for markdown parsing
- Support bold, italic, inline code, and code blocks with proper spacing
- Code blocks render as plain text with top/bottom margins and left indent for readability
- Enhance card detail view with formatted markdown descriptions
- Enhance board detail view with formatted markdown descriptions
- Add comprehensive integration tests for markdown renderer (9 tests)
- Note: Chose markdown-only approach over syntax highlighting to maintain simplicity and performance

### MVP-64 Create Task In The Focused Column (2025-10-26)
- feat: auto-complete cards created in last column when >2 columns exist
- feat: create cards in focused column of grouped and kanban views
- feat: add helper method to get focused column ID from view strategy
- KAN-59/fix card movement and completion display (#61)

### KAN-59 Fix Card Movement And Completion Display (2025-10-26)
- Add view refresh for card movement (H/L keys) in all view modes
- Add view refresh for card completion (c key) in all view modes
- Add smart column navigation: cards move to last column when marked Done, and to second-to-last column when unmarked from Done

## [0.1.8] - 2025-10-21 && [0.1.9] - 2025-10-21 ([#patch](https://github.com/fulsomenko/kanban/pull/patch))

- Fix critical release workflow issues that prevented successful publishing to crates.io:
- Fix Nix script path resolution in publish-crates (validate-release now called directly)
- Use portable sed syntax compatible with both Linux and macOS
- Preserve .changeset/README.md when cleaning up changesets
- Correct changeset description parsing in update-changelog script
- Add runtime dependencies (cargo, git, grep, sed, find) to Nix shell applications
- Add concurrency control to aggregate workflow to prevent race conditions
- Remove error suppression that was hiding failures
- Extract repository URL from git remote instead of hardcoding

## [0.2.1] - 2025-10-20 ([#patch](https://github.com/fulsomenko/kanban/pull/patch))

- Testing the release flow
- Created aggregate-changesets.sh: collects all changesets and determines highest priority bump type
- Created update-changelog.sh: merges changesets into CHANGELOG.md with version header and date
- Modified aggregate-changesets.yml: aggregates all pending changesets into single version bump, updates changelog, cleans up changesets
- Modified release.yml: uses version comparison (Cargo.toml vs git tags) instead of changeset checking - idempotent and race-condition free
- Eliminates race conditions by not pushing back to trigger branch
- Single version bump per release cycle instead of per feature
- Full changelog history preserved in CHANGELOG.md
- Fix CI workflow and publish workflow issues
- Implement monorepo versioning and release validation to prevent cross-crate API mismatches during publishing. Adds validate-release.sh script that runs in CI to catch version skew and dependency resolution issues before they reach crates.io.
- Fix cross-crate dependency version specifications to enable crates.io publishing. All workspace dependencies now include required version specs.


## [0.2.0] - 2025-10-20

---
Testing the release flow
- Created aggregate-changesets.sh: collects all changesets and determines highest priority bump type
- Created update-changelog.sh: merges changesets into CHANGELOG.md with version header and date
- Modified aggregate-changesets.yml: aggregates all pending changesets into single version bump, updates changelog, cleans up changesets
- Modified release.yml: uses version comparison (Cargo.toml vs git tags) instead of changeset checking - idempotent and race-condition free
- Eliminates race conditions by not pushing back to trigger branch
- Single version bump per release cycle instead of per feature
- Full changelog history preserved in CHANGELOG.md
---
Fix CI workflow and publish workflow issues
---
Implement monorepo versioning and release validation to prevent cross-crate API mismatches during publishing. Adds validate-release.sh script that runs in CI to catch version skew and dependency resolution issues before they reach crates.io.
---
Fix cross-crate dependency version specifications to enable crates.io publishing. All workspace dependencies now include required version specs.


## [0.2.0] - 2025-10-19 ([#40](https://github.com/fulsomenko/kanban/pull/40))

- Fix CI workflow and publish workflow issues
- Implement monorepo versioning and release validation to prevent cross-crate API mismatches during publishing. Adds validate-release.sh script that runs in CI to catch version skew and dependency resolution issues before they reach crates.io.


## [0.1.7] - 2025-10-18 ([#32](https://github.com/fulsomenko/kanban/pull/32))

- - update CONTRIBUTING.md with branching and release workflow
- check for changesets onm develop branch
- add create-changeset.sh
- Fix card selection in kanban column view
- Fix card selection in kanban column view
- Fixed bug where card operations (edit, move, toggle completion) were using incorrect card indices
- Card selection index now correctly maps to cards within the focused column in kanban view
- Added get_selected_card_id() helper method to resolve selection properly
- CI/CD improvements and grouped view navigation fixes
- Add comprehensive CI workflow with format, clippy, test, and build checks
- Add sync-develop workflow to prevent branch divergence
- Refactor GroupedViewStrategy to use per-column TaskLists
- Fix navigation and sorting in grouped by column view
- Add seamless column wrapping for grouped and kanban views
- Document required GitHub secrets in CONTRIBUTING.md
- Set cursor to newly created task after creation
- - feat: add kanban column navigation
- feat: implement three task list view modes
- feat: add column and view selection UI state
- feat: add task list view support to Board domain
- feat: add column management handlers
- feat: add TaskListView domain enum


## [0.1.6] - 2025-10-16 ([#25](https://github.com/fulsomenko/kanban/pull/25))

- Enable direct card description editing from task list
- Add 'e' key binding to edit card description when focus is on Cards
- Previously required entering CardDetail mode first (Enter then 'e')


## [0.1.5] - 2025-10-14 ([#24](https://github.com/fulsomenko/kanban/pull/24))

- - only show prefix+number as task label on filtered by sprint task list


## [0.1.4] - 2025-10-14 ([#23](https://github.com/fulsomenko/kanban/pull/23))

- Show branch name in sprint-filtered task list and fix UI issues
- Show branch name instead of redundant sprint name when task list filtered by sprint
- Fix duplicate title rendering in tasks panel (removed redundant title call)
- Change LABEL_TEXT color from Gray to DarkGray for better visual separation


## [0.1.3] - 2025-10-14 ([#22](https://github.com/fulsomenko/kanban/pull/22))

- Extract theme system and reusable UI components
- Add theme module with semantic colors and style functions
- Create composable components (ListItem, Panel, Popup, DetailView, CardListItem, SelectionList)
- Refactor ui.rs using new components (1227→869 lines, 29% reduction)
- Improve code reusability and maintainability through composition
- CardListItem provides reusable task list rendering for board and sprint views


## [0.1.2] - 2025-10-13 ([#20](https://github.com/fulsomenko/kanban/pull/20))

- KAN-45: Automated release workflow with changeset-based versioning
- Add GitHub Actions workflow for automated crates.io publishing
- Implement changeset system for version management
- Add changeset validation check for PRs to master
- Create Nix-based bump-version and publish-crates scripts
- Configure deploy key authentication for protected branch bypass
- Update `CHANGELOG.md` generation with PR links
- Add unified workspace versioning across all crates
- Document changeset workflow in `README.md` and `CONTRIBUTING.md`
- Add semantic commit message guidelines
- Add PR title and description format guidelines
- Cross-reference `CLAUDE.md`, `CONTRIBUTING.md`, and `README.md`


## [0.1.1] - 2025-10-13 ([#19](https://github.com/fulsomenko/kanban/pull/19))

- # Changesets
When creating a PR, add a changeset file to describe your changes.
## Creating a Changeset
Create a file `.changeset/<descriptive-name>.md`:
```md
Brief description of changes for the changelog
```
## Bump Types
- `patch` - Bug fixes, small changes (0.1.0 → 0.1.1)
- `minor` - New features, backwards compatible (0.1.0 → 0.2.0)
- `major` - Breaking changes (0.1.0 → 1.0.0)
## Example
`.changeset/add-vim-keybindings.md`:
```md
Add vim-style keybindings for navigation
```
On merge to master, this will:
1. Update CHANGELOG.md with the description
2. Bump version according to the highest bump type
3. Tag and publish to crates.io
4. Delete processed changesets
- Add automated release workflow with changeset-based version management


# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-10-10

- Initial release
- Terminal-based kanban board interface
- Nix development environment
