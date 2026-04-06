# kanban-persistence

Persistence trait layer for the kanban workspace. **Contains no I/O code.** Defines the interfaces implemented by `kanban-persistence-json` and `kanban-persistence-sqlite`, plus shared serialization types used across all persistence crates.

## Traits

### `PersistenceStore`

```rust
#[async_trait]
pub trait PersistenceStore: Send + Sync {
    async fn load(&self) -> Result<(StoreSnapshot, PersistenceMetadata), PersistenceError>;
    async fn save(&self, snapshot: StoreSnapshot, metadata: PersistenceMetadata) -> Result<(), PersistenceError>;
    async fn exists(&self) -> bool;
    fn locator(&self) -> &str;
    fn instance_id(&self) -> &str;
}
```

- `load` returns a `StoreSnapshot` (raw bytes) and `PersistenceMetadata` (timestamps, version, instance ID).
- `save` is expected to be atomic: implementations must guarantee that a crash during save cannot leave the file in a partially-written state.
- `exists` returns `false` if the backing store has never been written.
- `locator` returns the file path or connection string.

### `StoreFactory`

```rust
pub trait StoreFactory: Send + Sync {
    fn name(&self) -> &str;
    fn supported_patterns(&self) -> &[&str];
    fn matches(&self, locator: &str) -> bool;
    fn create(&self, locator: &str) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError>;
}
```

Backend plugins implement `StoreFactory` and register themselves with `StoreRegistry`. The JSON backend is the catch-all fallback; the SQLite backend matches on file extension.

### `StoreRegistry`

The registry holds a prioritized list of `StoreFactory` implementations.

```rust
impl StoreRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, factory: Box<dyn StoreFactory>);
    pub fn detect_backend(&self, locator: &str) -> Option<&str>;
    pub fn create_store(
        &self,
        backend: &str,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError>;
}
```

- `detect_backend` iterates factories in registration order and returns the name of the first factory whose `matches(locator)` returns true.
- `create_store` finds the factory by name and calls `factory.create(locator)`.

### `ChangeDetector` trait

Provides file-watching capability for detecting external changes.

```rust
pub trait ChangeDetector: Send + Sync {
    fn has_changed(&self) -> bool;
    fn reset(&mut self);
}
```

### `Serializer<T>` trait

```rust
pub trait Serializer<T>: Send + Sync {
    fn serialize(&self, value: &T) -> Result<Vec<u8>, PersistenceError>;
    fn deserialize(&self, data: &[u8]) -> Result<T, PersistenceError>;
}
```

### `MigrationStrategy` trait

```rust
pub trait MigrationStrategy: Send + Sync {
    fn can_migrate(&self, from_version: FormatVersion) -> bool;
    fn migrate(&self, data: &[u8]) -> Result<Vec<u8>, PersistenceError>;
}
```

---

## Shared Types

### `StoreSnapshot`

```rust
pub struct StoreSnapshot {
    pub data: Vec<u8>,    // Serialized snapshot (JSON bytes of kanban_domain::Snapshot)
    pub version: FormatVersion,
}
```

### `PersistenceMetadata`

```rust
pub struct PersistenceMetadata {
    pub version: FormatVersion,
    pub saved_at: DateTime<Utc>,
    pub instance_id: String,
    pub save_count: u64,
}
```

### `FormatVersion`

```rust
pub enum FormatVersion {
    V1,
    V2,
}
```

### `ConflictResolver`

Conflict resolution semantics: when `save` is called with a stale `instance_id`, the store may return `PersistenceError::Conflict`. The caller (typically `KanbanContext`) surfaces this to the user.

### `PersistenceError`

```rust
pub enum PersistenceError {
    Io(std::io::Error),
    Serialization(String),
    Conflict { path: String },
    Database(String),
    NotFound(String),
    Migration(String),
}
```

---

## Helper Functions

```rust
pub fn snapshot_to_json_bytes(snapshot: &kanban_domain::Snapshot) -> Result<Vec<u8>, KanbanError>;
pub fn snapshot_from_json_bytes(data: &[u8]) -> Result<kanban_domain::Snapshot, KanbanError>;
```

Used by both backends and the service layer to serialize/deserialize the domain `Snapshot`.

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `kanban-core` | `KanbanError`, `KanbanResult` |
| `kanban-domain` | `Snapshot` type |
| `serde` + `serde_json` | Serialization |
| `async-trait` | Async trait methods |
| `chrono` | Timestamps in metadata |
| `thiserror` | Error derivation |
