# kanban-persistence-json

JSON file storage backend for the kanban workspace. Implements `StoreFactory` and `PersistenceStore` from `kanban-persistence`.

## `JsonFileStore`

```rust
pub struct JsonFileStore {
    path: String,
    instance_id: String,
}

impl JsonFileStore {
    pub fn new(path: &str) -> Self;
    pub fn with_instance_id(path: &str, instance_id: &str) -> Self;
}
```

`instance_id` is a random UUID generated per process; used for conflict detection.

---

## V2 Envelope Format

```json
{
  "version": 2,
  "metadata": {
    "saved_at": "2024-06-15T10:30:00Z",
    "instance_id": "550e8400-e29b-41d4-a716-446655440000",
    "save_count": 42
  },
  "data": { /* kanban_domain::Snapshot */ }
}
```

V1 format was a bare `Snapshot` JSON object with no envelope.

---

## Save Flow

1. Check for conflict: compare file metadata (size + mtime) against the last-seen value
2. Update `PersistenceMetadata` (increment `save_count`, set `saved_at`)
3. Wrap snapshot in the V2 envelope
4. Write to a temporary file (`.tmp` suffix) in the same directory
5. Atomic rename: `temp → final path`

The atomic rename means a crash at any point leaves either the old file or the new file intact — never a partial write.

**Debounced saving**: a 500ms minimum interval is enforced between saves to avoid thrashing on rapid successive mutations.

---

## Load Flow

1. Read the file
2. Detect format version: V2 envelopes begin with `{"version":2`; anything else is treated as V1
3. **V1 migration**: rename existing file to `<path>.v1.backup`, parse as bare `Snapshot`, re-save as V2
4. Parse the V2 envelope, extract and return `data` as `StoreSnapshot`

---

## Conflict Detection

`FileMetadata` captures the file's size and modification time at last load/save. On the next save, the current file metadata is compared:

- If they match → no external write occurred; proceed
- If they differ → another instance wrote the file; return `PersistenceError::Conflict`

---

## `JsonStoreFactory`

```rust
pub struct JsonStoreFactory;

impl StoreFactory for JsonStoreFactory {
    fn name(&self) -> &str { "json" }
    fn supported_patterns(&self) -> &[&str] { &["*.json"] }
    fn matches(&self, locator: &str) -> bool { /* see below */ }
    fn create(&self, locator: &str) -> Result<...>;
}
```

`matches` logic:
1. If the locator ends with `.json` → true
2. If the locator starts with `sqlite://` or ends with `.sqlite`/`.sqlite3`/`.db` → false
3. Otherwise → **true** (catch-all fallback for plain file paths)

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `kanban-persistence` | `PersistenceStore`, `StoreFactory` traits |
| `kanban-domain` | `Snapshot` type |
| `serde_json` | JSON parsing |
| `tokio` | Async I/O |
| `tempfile` | Temp file for atomic writes |
