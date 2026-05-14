# kanban-core

Foundation crate for the kanban workspace. Provides shared types, error handling, configuration, state primitives, and the dependency graph used by all other crates.

## Modules

### Error Types

#### `CoreError`

```rust
pub enum CoreError {
    Validation(String),  // Input validation failure
    Config(String),      // Configuration error
}

pub type CoreResult<T> = Result<T, CoreError>;
```

### `AppConfig`

Application configuration loaded from a TOML or JSON config file.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `configuration_format` | `Option<String>` | `"toml"` | Config file format (`"json"` or `"toml"`) |
| `configuration_location` | `Option<String>` | system default | Path to the config file |
| `default_card_prefix` | `Option<String>` | `"task"` | Default card identifier prefix (e.g. `"KAN"` → `KAN-1`) |
| `default_sprint_prefix` | `Option<String>` | `"sprint"` | Default sprint identifier prefix |
| `editing_format` | `Option<String>` | `"json"` | Format used for external editor (`"json"` or `"toml"`) |
| `storage_backend` | `Option<String>` | `"json"` | Storage backend (`"json"` or `"sqlite"`) |
| `storage_location` | `Option<String>` | `"boards.json"` / `"boards.sqlite"` | Path to the data file |

**Effective-value getters** (return the value or its default):

```rust
config.effective_default_card_prefix()   // → "task"
config.effective_default_sprint_prefix() // → "sprint"
config.effective_storage_backend()       // → "json"
config.effective_editing_format()        // → "json"
config.effective_configuration_format()  // → "toml"
config.effective_storage_location()      // → "boards.json"
```

**Validation**: `config.validate_values()` returns `CoreError::Validation` if any field is out of range.

**Branch prefix validation**: `validate_branch_prefix(prefix: &str) -> bool` — non-empty, alphanumeric + hyphens/underscores, must start and end with an alphanumeric character.

### `PaginatedList<T>`

Serialized pagination envelope used by CLI and MCP list responses.

```rust
pub struct PaginatedList<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}
```

- `PaginatedList::paginate(items, page, page_size)` — slices `items` and returns the envelope. Returns `CoreError::Validation` if `page_size > MAX_PAGE_SIZE` (500) or `page_size == 0`.
- `resolve_page_params(page: Option<u32>, page_size: Option<u32>) -> CoreResult<(usize, usize)>` — applies defaults (`page=1`, `page_size=50`) and validates.
- Constants: `DEFAULT_PAGE = 1`, `DEFAULT_PAGE_SIZE = 50`, `MAX_PAGE_SIZE = 500`.

### `Page` / `PageInfo`

TUI viewport pagination — manages which items are visible in a terminal viewport given a scroll offset. **Pure in-memory state — lives only in the TUI process.**

```rust
pub struct PageInfo {
    pub visible_indices: Vec<usize>,
    pub first_visible: usize,
    pub last_visible: usize,
    pub items_per_page: usize,
    pub show_above_indicator: bool,
    pub items_above: usize,
    pub show_below_indicator: bool,
    pub items_below: usize,
    pub current_page: usize,
    pub total_pages: usize,
}
```

`Page` computes a `PageInfo` for a given item count, viewport height, and scroll offset.

### `InputState`

UTF-8–aware text input with cursor management. Used for all text entry dialogs in the TUI.

```rust
pub struct InputState {
    pub value: String,       // Current text
    pub cursor_pos: usize,   // Byte offset of cursor
}
```

Methods: `insert_char`, `delete_char_before`, `delete_char_after`, `move_left`, `move_right`, `jump_to_start`, `jump_to_end`, `clear`.

The cursor tracks byte offsets that always fall on UTF-8 character boundaries.

### `SelectionState`

Cursor navigation for a fixed-size list.

```rust
pub struct SelectionState {
    pub selected: usize,
    pub total: usize,
}
```

Methods: `next()`, `prev()`, `clamp()` — wraps around at boundaries.

### `Editable<T>` trait

```rust
pub trait Editable<T> {
    fn to_editable(&self) -> T;
    fn from_editable(value: T) -> Self;
}
```

Implemented by domain types to support round-trip serialization through an external editor.

### `Graph<E>` / `GraphNode` / `Edge<E>` / `EdgeDirection`

Generic directed acyclic graph with cycle detection. Used by `kanban-domain` for card dependency tracking.

```rust
pub struct Graph<E> {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<Edge<E>>,
}

pub struct GraphNode {
    pub id: Uuid,
}

pub struct Edge<E> {
    pub from: Uuid,
    pub to: Uuid,
    pub edge_type: E,
}

pub enum EdgeDirection {
    Outgoing,   // Edges starting from a node
    Incoming,   // Edges pointing to a node
}
```

`Graph::add_edge` returns `CoreError` if adding the edge would create a cycle or a self-reference.

### `LogEntry` / `Loggable` trait

Structured logging support for domain events.

```rust
pub trait Loggable {
    fn log_entries(&self) -> Vec<LogEntry>;
}
```

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `serde` + `serde_json` | Serialization |
| `uuid` | `Uuid` type |
| `thiserror` | Error derivation |
| `chrono` | Timestamps |
