# kanban-core

Foundation crate providing core abstractions, error handling, and result types for the kanban workspace.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kanban-core = { path = "../kanban-core" }
```

## API Reference

### Result Type

```rust
pub type KanbanResult<T> = Result<T, KanbanError>;
```

Standard result type used throughout the workspace for consistent error handling.

### Errors

`KanbanError` enum with variants:

- `Connection(String)` - Connection/network errors
- `NotFound(String)` - Resource not found
- `Validation(String)` - Input validation failures
- `Io(std::io::Error)` - File system and I/O errors
- `Serialization(String)` - JSON/serde errors
- `Internal(String)` - Unexpected internal errors
- `ConflictDetected { path, source }` - File modified by another instance
- `CycleDetected` - Adding an edge would create a circular dependency
- `SelfReference` - Self-referencing edge not allowed
- `EdgeNotFound` - Graph edge not found

### Configuration

`AppConfig` - Cross-platform application configuration:

```rust
pub struct AppConfig {
    pub default_sprint_prefix: Option<String>,
    pub default_card_prefix: Option<String>,
}

impl AppConfig {
    pub async fn load() -> KanbanResult<Self>
    pub fn config_path() -> PathBuf
}
```

Loads from platform-specific paths:
- macOS/Linux: `~/.config/kanban/config.toml`
- Windows: `%APPDATA%\kanban\config.toml`

### Logging

`Loggable` trait for entities to maintain audit logs:

```rust
pub trait Loggable {
    fn add_log(&mut self, entry: LogEntry);
    fn get_logs(&self) -> &[LogEntry];
}

pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub message: String,
}
```

### Traits

**Editable Pattern** - Safe entity modification via DTOs:

```rust
pub trait Editable<T>: Sized {
    fn from_entity(entity: &T) -> Self;
    fn apply_to(self, entity: &mut T) -> KanbanResult<()>;
}
```

Consuming crates implement `Editable<Card>`, `Editable<Board>`, etc. to provide type-safe updates with validation.

**Repository Pattern** - Generic async data access:

```rust
pub trait Repository<T, Id>: Send + Sync {
    async fn find_by_id(&self, id: Id) -> KanbanResult<T>;
    async fn find_all(&self) -> KanbanResult<Vec<T>>;
    async fn save(&mut self, entity: T) -> KanbanResult<()>;
    async fn delete(&mut self, id: Id) -> KanbanResult<()>;
}
```

**Service Pattern** - Business logic abstraction:

```rust
pub trait Service<T, Id>: Send + Sync {
    async fn get(&self, id: Id) -> KanbanResult<T>;
    async fn list(&self) -> KanbanResult<Vec<T>>;
    async fn create(&mut self, entity: T) -> KanbanResult<Id>;
    async fn update(&mut self, entity: T) -> KanbanResult<()>;
    async fn delete(&mut self, id: Id) -> KanbanResult<()>;
}
```

### Graph

`Graph<E>` — Generic directed graph data structure used for card dependencies:

```rust
pub struct Graph<E: Edge> {
    // ...
}

pub trait Edge: Clone + PartialEq {
    fn is_acyclic(&self) -> bool;
}

pub trait GraphNode {
    fn node_id(&self) -> Uuid;
}
```

- Add/remove edges between nodes
- Automatic cycle detection for acyclic edge types
- Query neighbors, parents, and children by edge type
- Edge direction filtering (`Outgoing`, `Incoming`, `Both`)

### InputState

Cursor-aware text input buffer with correct multi-byte UTF-8 handling:

```rust
pub struct InputState { /* private fields */ }

impl InputState {
    pub fn new() -> Self
    pub fn insert_char(&mut self, c: char)
    pub fn backspace(&mut self)
    pub fn move_left(&mut self)
    pub fn move_right(&mut self)
    pub fn value(&self) -> &str
    pub fn cursor_byte_offset(&self) -> usize
}
```

### SelectionState

Generic single-item selection for list navigation:

```rust
pub struct SelectionState { /* private fields */ }

impl SelectionState {
    pub fn new() -> Self
    pub fn select(&mut self, index: usize)
    pub fn selected(&self) -> Option<usize>
    pub fn next(&mut self, total: usize)
    pub fn prev(&mut self)
    pub fn jump_to_first(&mut self)
    pub fn jump_to_last(&mut self, total: usize)
    pub fn clamp(&mut self, total: usize)
}
```

### Pagination

Viewport pagination with scroll position management:

```rust
pub struct PageInfo {
    pub visible_indices: Vec<usize>,
    pub items_above: usize,
    pub items_below: usize,
}

impl Page {
    pub fn new(viewport_height: usize) -> Self
    pub fn calculate(&self, total_items: usize, selected: usize) -> PageInfo
    pub fn scroll_to_visible(&mut self, index: usize)
}
```

## Architecture

Foundation layer with no workspace dependencies. All other crates depend on `kanban-core` for shared abstractions and error types.

```
kanban-core (foundation)
    ↑
    └── kanban-domain, kanban-tui, kanban-cli
```

## Examples

### Error Handling

```rust
use kanban_core::{KanbanError, KanbanResult};

fn validate_name(name: &str) -> KanbanResult<()> {
    if name.is_empty() {
        return Err(KanbanError::Validation("Name cannot be empty".into()));
    }
    Ok(())
}

async fn fetch_data() -> KanbanResult<Vec<u8>> {
    std::fs::read("data.json")
        .map_err(|e| KanbanError::Io(e))
}
```

### Configuration

```rust
use kanban_core::AppConfig;

let config = AppConfig::load().await?;
let prefix = config.default_card_prefix.unwrap_or("task".into());
```

### Implementing Editable

```rust
use kanban_core::Editable;

struct CardUpdate {
    title: String,
    priority: CardPriority,
}

impl Editable<Card> for CardUpdate {
    fn from_entity(card: &Card) -> Self {
        Self {
            title: card.title.clone(),
            priority: card.priority,
        }
    }

    fn apply_to(self, card: &mut Card) -> KanbanResult<()> {
        card.title = self.title;
        card.priority = self.priority;
        Ok(())
    }
}
```

## Dependencies

- `thiserror` - Ergonomic error handling macros
- `anyhow` - Context-aware error handling
- `serde`, `serde_json` - Serialization framework
- `uuid` - UUID generation for IDs
- `chrono` - Date and time types
- `async-trait` - Async trait method support
- `toml` - Configuration file parsing
- `dirs` - Cross-platform directory paths

## License

Apache 2.0 - See [LICENSE.md](../../LICENSE.md) for details
