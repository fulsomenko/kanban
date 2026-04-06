# kanban-persistence-sqlite

SQLite storage backend for the kanban workspace. Implements `StoreFactory` and `PersistenceStore` from `kanban-persistence`.

## `SqliteStore`

```rust
pub struct SqliteStore {
    path: String,
    instance_id: String,
    // connection pool (lazily initialized)
}

impl SqliteStore {
    pub fn new(path: &str) -> Self;
    pub fn with_instance_id(path: &str, instance_id: &str) -> Self;
}
```

The connection pool is initialized lazily on the first operation. The database file is created automatically on first use.

---

## Connection Pool Configuration

| Setting | Value |
|---------|-------|
| Max connections | 2 |
| Journal mode | WAL (Write-Ahead Logging) |
| Foreign keys | Enforced (`PRAGMA foreign_keys = ON`) |

---

## Schema

The schema consists of 14 tables:

| Table | Description |
|-------|-------------|
| `metadata` | Store metadata (version, instance_id, save_count) |
| `boards` | Board records |
| `board_card_counters` | Per-prefix card number counters |
| `board_sprint_names` | Sprint name pool per board |
| `columns` | Column records |
| `cards` | Active card records |
| `card_sprint_logs` | Sprint assignment history per card |
| `archived_cards` | Archived card records |
| `sprints` | Sprint records |
| `dependency_edges` | Card dependency graph edges |
| `tags` | Tag definitions (reserved for future use) |
| `card_tags` | Card–tag associations (reserved for future use) |
| `schema_version` | Schema migration tracking |

All foreign key relationships are enforced. `boards` → `columns` → `cards` cascade on delete.

---

## Save Flow

1. Begin a write transaction
2. Upsert all tables (boards, columns, cards, archived_cards, sprints, sprint_logs, dependency_edges, metadata)
3. Delete rows no longer present in the snapshot
4. Commit

All operations occur within a single transaction; a failure at any point rolls back completely.

---

## Load Flow

1. Begin a read transaction
2. Read all tables in join order
3. Reconstruct `kanban_domain::Snapshot` from relational rows
4. Return snapshot + metadata

---

## Schema Versioning

The `schema_version` table records the current schema version (`v1`). A migration skeleton is in place for future schema upgrades.

---

## `SqliteStoreFactory`

```rust
pub struct SqliteStoreFactory;

impl StoreFactory for SqliteStoreFactory {
    fn name(&self) -> &str { "sqlite" }
    fn supported_patterns(&self) -> &[&str] { &["*.sqlite", "*.sqlite3", "*.db"] }
    fn matches(&self, locator: &str) -> bool { /* extension check + magic bytes */ }
    fn create(&self, locator: &str) -> Result<...>;
}
```

`matches` checks:
1. File extension is `.sqlite`, `.sqlite3`, or `.db` → true
2. First 16 bytes of an existing file equal the SQLite magic (`SQLite format 3\0`) → true
3. Otherwise → false

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `kanban-persistence` | `PersistenceStore`, `StoreFactory` traits |
| `kanban-domain` | `Snapshot` type |
| `sqlx` | Async SQLite with connection pooling |
| `tokio` | Async runtime |
| `serde_json` | JSON serialization for complex column types |
