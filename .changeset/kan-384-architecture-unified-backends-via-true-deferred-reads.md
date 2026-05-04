---
bump: minor
---

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
