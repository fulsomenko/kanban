# kanban-domain

Domain models and business logic for the kanban project management tool. Pure domain layer with no infrastructure dependencies.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kanban-domain = { path = "../kanban-domain" }
```

## API Reference

### Core Entities

**Board** - Top-level kanban board

```rust
pub struct Board {
    pub id: BoardId,
    pub name: String,
    pub description: Option<String>,
    pub sprint_prefix: Option<String>,
    pub card_prefix: Option<String>,
    pub sprint_duration_days: Option<u32>,
    pub task_sort_field: SortField,
    pub task_sort_order: SortOrder,
    // ...more fields
}

pub enum SortField { Points, Priority, CreatedAt, UpdatedAt, Status, Default }
pub enum SortOrder { Ascending, Descending }
```

**Card** - Task with lifecycle and metadata

```rust
pub struct Card {
    pub id: CardId,
    pub column_id: ColumnId,
    pub title: String,
    pub description: Option<String>,
    pub priority: CardPriority,
    pub status: CardStatus,
    pub points: Option<u8>,      // 1-5 scale
    pub card_number: u32,        // Auto-assigned from board prefix counter
    pub assigned_prefix: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub sprint_id: Option<SprintId>,
    // ...more fields
}

pub enum CardStatus { Todo, InProgress, Blocked, Done }
pub enum CardPriority { Low, Medium, High, Critical }
```

**Sprint** - Sprint lifecycle management

```rust
pub struct Sprint {
    pub id: SprintId,
    pub board_id: BoardId,
    pub sprint_number: u32,
    pub name_index: usize,
    pub prefix: Option<String>,
    pub card_prefix: Option<String>,
    pub status: SprintStatus,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    // ...timestamps
}

pub enum SprintStatus { Planning, Active, Completed, Cancelled }
```

**Column** - Board organization

```rust
pub struct Column {
    pub id: ColumnId,
    pub board_id: BoardId,
    pub name: String,
    pub position: u32,
    pub wip_limit: Option<u32>,
    // ...timestamps
}
```

**Supporting Types**

```rust
pub struct ArchivedCard {
    pub card: Card,
    pub archived_at: DateTime<Utc>,
    pub original_column_id: ColumnId,
    pub original_position: u32,
}

pub struct SprintLog {
    pub sprint_id: SprintId,
    pub sprint_number: u32,
    pub sprint_name: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub status: SprintStatus,
}

pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub color: String,
}

pub enum TaskListView { Flat, GroupedByColumn, ColumnView }

// DTOs for safe entity modification
pub struct BoardSettingsDto {
    pub sprint_prefix: Option<String>,
    pub card_prefix: Option<String>,
    pub sprint_duration_days: Option<u32>,
    pub sprint_names: Vec<String>,
}

pub struct CardMetadataDto {
    pub priority: CardPriority,
    pub status: CardStatus,
    pub points: Option<u8>,
    pub due_date: Option<DateTime<Utc>>,
}
```

## Architecture

Pure domain layer depending only on `kanban-core`. Implements rich domain models with encapsulated business logic:

```
kanban-core
    ↑
    └── kanban-domain (pure business logic)
            ↑
            └── kanban-tui, kanban-cli
```

### Key Design Patterns

**Prefix Hierarchy** - Git branch naming with fallback chain:
- Card's assigned prefix → Sprint's card prefix → Board's card prefix → Default ("task")
- Enables per-card, per-sprint, or board-wide branch naming strategies

**Counter System** - Per-prefix independent numbering:
- Board maintains separate counters for each card prefix
- Board maintains separate counters for each sprint prefix
- Enables naming schemes like: `feature-1`, `bugfix-1`, `hotfix-1` all starting at 1
- Auto-migration for legacy single-counter boards

**Sprint History** - Audit trail of card movements:
- Each card maintains `SprintLog` entries when assigned to sprints
- Tracks: when assigned, which sprint, when ended
- Enables sprint velocity and card journey analysis

**Entity Update Safety** - DTO pattern via `Editable` trait:
- DTOs provide type-safe, validated updates
- Case-insensitive enum parsing (Priority::High, priority::high both valid)
- Two-way conversion between domain entities and DTOs

## Examples

### Branch Name Generation

```rust
use kanban_domain::{Board, Card, Sprint};

let mut board = Board::new("My Project", None);
board.update_card_prefix(Some("task".into()));

let mut card = Card::new(&mut board, column_id, "Fix bug", 0, "task");
card.set_assigned_prefix(Some("urgent".into()));

// Card prefix wins in hierarchy
let branch = card.branch_name(&board, &sprints, "task");
// Result: "urgent-1/fix-bug"
```

### Per-Prefix Card Numbering

```rust
use kanban_domain::Board;

let mut board = Board::new("My Project", None);

// Feature cards start at 1
let feature1 = board.get_next_card_number("feature");  // 1
let feature2 = board.get_next_card_number("feature");  // 2

// Bugfix cards are independent sequence
let bugfix1 = board.get_next_card_number("bugfix");    // 1
let bugfix2 = board.get_next_card_number("bugfix");    // 2
```

### Sprint Lifecycle

```rust
use kanban_domain::{Sprint, SprintStatus};

let mut sprint = Sprint::new(board_id, 1, None, None);
assert_eq!(sprint.status, SprintStatus::Planning);

// Start sprint (sets start_date and end_date)
sprint.activate(14);  // 14-day sprint
assert_eq!(sprint.status, SprintStatus::Active);

// Complete sprint
sprint.complete();
assert_eq!(sprint.status, SprintStatus::Completed);
```

### Card Sprint Assignment

```rust
use kanban_domain::Card;

let mut card = Card::new(&mut board, column_id, "Implement UI", 0, "task");

// Assign to sprint (creates SprintLog entry)
card.assign_to_sprint(sprint_id, 1, None, "Active");

// Get sprint history
let logs = card.get_sprint_history();
assert_eq!(logs.len(), 1);
assert_eq!(logs[0].sprint_number, 1);
```

### Updating Card Metadata

```rust
use kanban_domain::{CardMetadataDto, CardPriority, CardStatus};
use kanban_core::Editable;

let update = CardMetadataDto {
    priority: CardPriority::High,
    status: CardStatus::InProgress,
    points: Some(5),
    due_date: None,
};

// Apply to card with validation
update.apply_to(&mut card)?;
assert_eq!(card.priority, CardPriority::High);
assert_eq!(card.points, Some(5));
```

## Dependencies

- `kanban-core` - Foundation types and traits
- `serde`, `serde_json` - Serialization for JSON persistence
- `uuid` - ID generation and UUID types
- `chrono` - Date/time handling
- `async-trait` - Async trait support for future extensions

## License

Apache 2.0 - See [LICENSE.md](../../LICENSE.md) for details
