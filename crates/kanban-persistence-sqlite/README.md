# kanban-persistence-sqlite

SQLite storage backend for the kanban project management tool. Implements `StoreFactory` from `kanban-persistence`.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kanban-persistence-sqlite = { path = "../kanban-persistence-sqlite" }
```

## Overview

Provides `SqliteStore` (implements `PersistenceStore`) and `SqliteStoreFactory` (implements `StoreFactory`) for persisting kanban data in a SQLite database. The database file is auto-created on first use.

## Matching Patterns

- `*.sqlite`
- `*.sqlite3`
- `*.db`

## API Reference

### `SqliteStoreFactory`

```rust
use kanban_persistence_sqlite::SqliteStoreFactory;

let factory = SqliteStoreFactory;
assert!(factory.matches_locator("board.sqlite"));
assert!(factory.matches_locator("project.sqlite3"));
assert!(factory.matches_locator("board.db"));
assert!(!factory.matches_locator("board.json"));
```

### `SqliteStore`

```rust
use kanban_persistence_sqlite::SqliteStore;
use kanban_persistence::PersistenceStore;

let store = SqliteStore::new("board.sqlite");
let (snapshot, metadata) = store.load().await?;
store.save(snapshot).await?;
```

## Schema Overview

The database uses 14 tables (schema version 1):

| Table | Description |
|-------|-------------|
| `metadata` | Singleton row: instance_id, saved_at, schema_version |
| `boards` | Board definitions with sort and prefix settings |
| `board_sprint_names` | Position-indexed sprint names per board |
| `board_prefix_counters` | Card number counters per prefix |
| `board_sprint_counters` | Sprint counters per prefix |
| `columns` | Columns with board FK, position, WIP limit |
| `sprints` | Sprint lifecycle with status, dates, prefix |
| `sprint_logs` | Historical sprint records (no FK for persistence after deletion) |
| `cards` | Cards with column FK, sprint FK, full metadata |
| `archived_cards` | Archived card metadata with original position |
| `card_edges` | Dependency graph edges (bulk-replace strategy) |

## Configuration

- **WAL mode**: Enabled for concurrent read/write performance
- **Foreign keys**: Enforced (`PRAGMA foreign_keys = ON`)
- **Connection pool**: Max 2 connections
- **Auto-create**: Database file created on first access if it doesn't exist

## Schema Versioning

The `metadata` table tracks `schema_version` (currently `1`). Future schema changes will use numbered migrations applied on startup.

## Dependencies

- `kanban-persistence` — `PersistenceStore` and `StoreFactory` traits
- `kanban-domain` — Domain models
- `sqlx` — SQLite driver with async support
- `tokio` — Async runtime
- `uuid` — ID generation
- `serde`, `serde_json` — Serialization for snapshot conversion

## License

Apache 2.0 — See [LICENSE.md](../../LICENSE.md) for details
