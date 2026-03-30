# kanban-persistence-json

JSON file storage backend for the kanban project management tool. Implements `StoreFactory` from `kanban-persistence`.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kanban-persistence-json = { path = "../kanban-persistence-json" }
```

## Overview

Provides `JsonFileStore` (implements `PersistenceStore`) and `JsonStoreFactory` (implements `StoreFactory`) for persisting kanban data as JSON files. This is the default backend and acts as a catch-all for any file path that doesn't match a more specific backend.

## Matching Patterns

- `*.json` — explicit JSON extension
- Any file path without `://` — catch-all fallback for unknown extensions

## API Reference

### `JsonStoreFactory`

```rust
use kanban_persistence_json::JsonStoreFactory;

let factory = JsonStoreFactory;
assert!(factory.matches_locator("board.json"));
assert!(factory.matches_locator("myboard"));        // catch-all
assert!(!factory.matches_locator("http://example")); // URI excluded
```

### `JsonFileStore`

```rust
use kanban_persistence_json::JsonFileStore;
use kanban_persistence::PersistenceStore;

let store = JsonFileStore::new("board.json");
let (snapshot, metadata) = store.load().await?;
store.save(snapshot).await?;
```

## Format Specification

### V2 Format (Current)

```json
{
  "version": 2,
  "metadata": {
    "instance_id": "uuid-here",
    "saved_at": "2024-01-15T10:30:00Z"
  },
  "data": {
    "boards": [],
    "columns": [],
    "cards": [],
    "sprints": [],
    "archived_cards": []
  }
}
```

### V1 Format (Deprecated)

Legacy format without version field or metadata:

```json
{
  "boards": [],
  "columns": [],
  "cards": [],
  "sprints": []
}
```

## Migration

### Automatic V1→V2 Migration

On first load of a V1 file:

1. **Detection**: Checks for `version` field presence
2. **Backup**: Original V1 file copied to `.v1.backup`
3. **Transform**: Data wrapped with V2 metadata envelope
4. **Write**: Migrated file written atomically
5. **Logging**: Migration progress logged for visibility

### Manual Migration

```rust
use kanban_persistence_json::migration::{Migrator, FormatVersion};

let version = Migrator::detect_version("board.json").await?;
if version == FormatVersion::V1 {
    Migrator::migrate(FormatVersion::V1, FormatVersion::V2, "board.json").await?;
}
```

## Performance Characteristics

- **Debounced Saving**: 500ms minimum interval between disk writes
- **Atomic Writes**: Crash-safe pattern (write to temp file, then atomic rename)
- **Steady-state**: ~2 saves/second maximum under rapid edits

## Dependencies

- `kanban-persistence` — `PersistenceStore` and `StoreFactory` traits
- `kanban-domain` — Domain models
- `serde`, `serde_json` — JSON serialization
- `tokio` — Async runtime
- `uuid` — ID generation

## License

Apache 2.0 — See [LICENSE.md](../../LICENSE.md) for details
