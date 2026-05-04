# kanban-service

Service layer for the kanban workspace. Provides `KanbanContext` — the single in-memory state machine for all board data — along with persistence orchestration, undo/redo, and utility functions.

## `KanbanContext`

The central type. Holds all domain data in memory and delegates to a `PersistenceStore` for load/save operations.

```rust
pub struct KanbanContext {
    // private fields:
    boards: Vec<Board>,
    columns: Vec<Column>,
    cards: Vec<Card>,
    sprints: Vec<Sprint>,
    archived_cards: Vec<ArchivedCard>,
    graph: DependencyGraph,
    app_config: AppConfig,
    store: Arc<dyn PersistenceStore + Send + Sync>,
    history: HistoryManager,
    dirty: bool,
    conflict_pending: bool,
}
```

### Construction

```rust
KanbanContext::load(store, config) -> KanbanResult<Self>
KanbanContext::load_with_defaults(store) -> KanbanResult<Self>
KanbanContext::empty(store, config) -> Self
```

### State Accessors

```rust
ctx.boards() -> &[Board]
ctx.columns() -> &[Column]
ctx.cards() -> &[Card]
ctx.sprints() -> &[Sprint]
ctx.archived_cards() -> &[ArchivedCard]
ctx.graph() -> &DependencyGraph
ctx.app_config() -> &AppConfig
ctx.is_dirty() -> bool
ctx.has_conflict() -> bool
ctx.set_conflict(bool)
ctx.clear_conflict()
```

### Persistence

```rust
ctx.save() -> KanbanResult<()>
ctx.reload() -> KanbanResult<()>
ctx.replace_store(store)
ctx.snapshot() -> Snapshot
ctx.apply_snapshot(snapshot)
```

### Undo / Redo

```rust
ctx.undo() -> bool        // Returns true if there was something to undo
ctx.redo() -> bool        // Returns true if there was something to redo
ctx.can_undo() -> bool
ctx.can_redo() -> bool
ctx.undo_depth() -> usize
ctx.redo_depth() -> usize
ctx.clear_history()
```

History is captured before every mutating operation. Stacks are capped at 100 entries.

### Board Operations

| Method | Description |
|--------|-------------|
| `create_board(name, card_prefix)` | Create a new board |
| `list_boards()` | List all boards |
| `get_board(id)` | Get a board by ID |
| `update_board(id, updates)` | Partially update a board |
| `delete_board(id)` | Delete board and all its data |

### Column Operations

| Method | Description |
|--------|-------------|
| `create_column(board_id, name, position)` | Create a column |
| `list_columns(board_id)` | List columns for a board |
| `get_column(id)` | Get a column by ID |
| `update_column(id, updates)` | Partially update a column |
| `delete_column(id)` | Delete column and its cards |
| `reorder_column(id, position)` | Move column to new position |

### Card Operations

| Method | Description |
|--------|-------------|
| `create_card(board_id, column_id, title, options)` | Create a card |
| `list_cards(filter)` | List cards with `CardListFilter` |
| `list_cards_paged(filter, page, page_size)` | Paginated card list |
| `get_card(id)` | Get full card by ID |
| `find_cards_by_identifier(s)` | Find card(s) by UUID or `KAN-5` format |
| `update_card(id, updates)` | Partially update a card |
| `move_card(id, column_id, position)` | Move card to a column |
| `archive_card(id)` | Archive a card |
| `restore_card(id, column_id)` | Restore an archived card |
| `delete_card(id)` | Permanently delete a card |
| `list_archived_cards()` | List all archived cards |

### Card–Sprint Operations

| Method | Description |
|--------|-------------|
| `assign_card_to_sprint(card_id, sprint_id)` | Assign a card to a sprint |
| `unassign_card_from_sprint(card_id)` | Remove card from its sprint |
| `get_card_branch_name(id)` | Generate git branch name for a card |
| `get_card_git_checkout(id)` | Generate `git checkout -b <branch>` command |

### Bulk Operations

| Method | Description |
|--------|-------------|
| `archive_cards(ids)` | Archive multiple cards; returns count |
| `move_cards(ids, column_id)` | Move multiple cards; returns count |
| `assign_cards_to_sprint(ids, sprint_id)` | Bulk sprint assignment; returns count |
| `archive_cards_detailed(ids)` | Archive with per-card success/failure report |
| `move_cards_detailed(ids, column_id)` | Move with per-card success/failure report |
| `assign_cards_to_sprint_detailed(ids, sprint_id)` | Bulk assign with report |

### Sprint Operations

| Method | Description |
|--------|-------------|
| `create_sprint(board_id, prefix, name)` | Create a sprint |
| `list_sprints(board_id)` | List sprints for a board |
| `get_sprint(id)` | Get a sprint by ID |
| `update_sprint(id, updates)` | Partially update a sprint |
| `activate_sprint(id, duration_days)` | Activate a sprint |
| `complete_sprint(id)` | Complete a sprint |
| `cancel_sprint(id)` | Cancel a sprint |
| `delete_sprint(id)` | Delete a sprint |
| `carry_over_sprint_cards(from, to)` | Move uncompleted cards to a new sprint |

---

## `BatchOperationResult`

```rust
pub struct BatchOperationResult {
    pub succeeded: Vec<Uuid>,
    pub failed: Vec<BatchOperationFailure>,
}

pub struct BatchOperationFailure {
    pub id: Uuid,
    pub error: String,
}
```

Returned by the `*_detailed` bulk operation methods.

---

## `DataSnapshot`

```rust
pub struct DataSnapshot {
    // mirrors kanban_domain::Snapshot; used for serialization
}
```

---

## Utility Functions

```rust
// Build the default store registry (SQLite first, JSON as catch-all)
pub fn default_registry() -> StoreRegistry;

// Detect which backend matches a locator string
pub fn detect_backend(locator: &str) -> Option<String>;

// Create a store from backend name + locator
pub fn make_store(backend: &str, locator: &str) -> KanbanResult<Arc<dyn PersistenceStore + Send + Sync>>;

// Create a store from an optional file path + AppConfig
pub fn make_store_with_config(file: Option<&str>, config: &AppConfig) -> KanbanResult<Arc<dyn PersistenceStore + Send + Sync>>;

// Load and validate a store (returns error if file doesn't exist)
pub async fn validate_and_load_store(backend: &str, path: &str) -> KanbanResult<Snapshot>;

// Export board selection to a new SQLite file
pub async fn export_to_sqlite(export: AllBoardsExport, filename: &str) -> KanbanResult<()>;

// Migrate all data from one store to another
pub async fn migrate_store(source: &str, target: &str) -> KanbanResult<()>;

// Sync config storage_backend field to match the file's actual backend
pub fn sync_backend_with_file(locator: &str, config: &mut AppConfig) -> bool;
```

---

## Command Execution Flow

```
caller
  │
  ▼
KanbanContext::execute(commands: Vec<Box<dyn Command>>)
  │
  ├─ 1. history.capture_before_command(current_snapshot)
  │
  ├─ 2. for each command:
  │       command.execute(&mut CommandContext)   ← mutates boards/columns/cards/...
  │       on error: restore from undo snapshot → return Err
  │
  ├─ 3. dirty = true
  │
  └─ (caller calls ctx.save() → store.save(snapshot, metadata))
```

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `kanban-core` | `AppConfig`, `KanbanResult` |
| `kanban-domain` | All domain types |
| `kanban-persistence` | `PersistenceStore`, `StoreRegistry` |
| `kanban-persistence-json` | JSON backend (feature `json-storage`) |
| `kanban-persistence-sqlite` | SQLite backend (feature `sqlite-storage`) |
| `tokio` | Async runtime |
| `serde` | Serialization |
