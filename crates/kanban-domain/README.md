# kanban-domain

Pure domain logic — zero I/O, zero async, zero infrastructure dependencies. Depends only on `kanban-core`.

## Models

### `Board`

Top-level container for columns, cards, and sprints.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique identifier |
| `name` | `String` | Display name |
| `description` | `Option<String>` | Optional description |
| `card_prefix` | `Option<String>` | Default prefix for card identifiers (e.g. `"KAN"` → `KAN-1`) |
| `sprint_prefix` | `Option<String>` | Default prefix for sprint names |
| `sprint_names` | `Vec<String>` | Pool of sprint name tokens (consumed in order) |
| `prefix_counters` | `HashMap<String, u32>` | Per-prefix card number counter |
| `created_at` | `DateTime<Utc>` | Creation timestamp |
| `updated_at` | `DateTime<Utc>` | Last modification timestamp |

**Key methods**:

```rust
board.get_next_card_number(prefix: &str) -> u32
// Atomically increments and returns the next card number for the given prefix.

board.resolve_completion_column(columns: &[Column]) -> Option<&Column>
// Returns the rightmost column, used as the "done" column when toggling completion.

board.consume_sprint_name() -> Option<String>
// Pops and returns the next sprint name from sprint_names, if any.
```

**Partial update**:

```rust
pub struct BoardUpdate {
    pub name: Option<String>,
    pub description: FieldUpdate<String>,
    pub sprint_prefix: FieldUpdate<String>,
    pub card_prefix: FieldUpdate<String>,
    // ... additional fields
}
```

---

### `Column`

A swim lane within a board.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique identifier |
| `board_id` | `Uuid` | Parent board |
| `name` | `String` | Display name |
| `position` | `i32` | Display order (lower = left) |
| `wip_limit` | `Option<i32>` | Advisory WIP limit — shown in the UI as a guide |
| `created_at` | `DateTime<Utc>` | Creation timestamp |
| `updated_at` | `DateTime<Utc>` | Last modification timestamp |

WIP limits are **advisory**: the UI surfaces it as a visual cue; card creation always succeeds.

**Partial update**: `ColumnUpdate { name, position, wip_limit: FieldUpdate<i32> }`

---

### `Card`

The primary work item.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique identifier |
| `column_id` | `Uuid` | Parent column |
| `title` | `String` | Card title |
| `description` | `Option<String>` | Long-form description (markdown supported) |
| `priority` | `CardPriority` | `Low` / `Medium` / `High` / `Critical` |
| `status` | `CardStatus` | `Todo` / `InProgress` / `Blocked` / `Done` |
| `position` | `i32` | Display order within the column |
| `due_date` | `Option<DateTime<Utc>>` | Optional due date |
| `points` | `Option<u8>` | Story points (1–5) |
| `card_number` | `u32` | Sequential number within the prefix namespace |
| `sprint_id` | `Option<Uuid>` | Assigned sprint, if any |
| `assigned_prefix` | `Option<String>` | Prefix used when the card was created (e.g. `"KAN"`) |
| `card_prefix` | `Option<String>` | Optional per-card prefix override |
| `completed_at` | `Option<DateTime<Utc>>` | Set when status transitions to `Done`, cleared otherwise |
| `sprint_logs` | `Vec<SprintLog>` | History of sprint assignments |
| `created_at` | `DateTime<Utc>` | Creation timestamp |
| `updated_at` | `DateTime<Utc>` | Last modification timestamp |

**Status transitions**: `card.update_status(status)` — automatically sets `completed_at = Some(now)` when transitioning to `Done`, and clears it when transitioning away from `Done`.

**Branch name generation**: derived from `assigned_prefix` + `card_number` + slugified title (lowercase, hyphens replace whitespace and punctuation, truncated for length).

**Partial update**: `CardUpdate { title, description, priority, status, position, column_id, points, due_date, sprint_id, assigned_prefix, card_prefix }` — all fields are `Option<T>` or `FieldUpdate<T>`.

---

### `Sprint`

A time-boxed work period.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique identifier |
| `board_id` | `Uuid` | Parent board |
| `sprint_number` | `u32` | Sequential sprint number within the board |
| `name_index` | `Option<usize>` | Index into `board.sprint_names` for the human-readable name |
| `prefix` | `Option<String>` | Sprint prefix override (falls back to board prefix) |
| `card_prefix` | `Option<String>` | Card prefix override for this sprint |
| `status` | `SprintStatus` | `Planning` / `Active` / `Completed` / `Cancelled` |
| `start_date` | `Option<DateTime<Utc>>` | Set when activated |
| `end_date` | `Option<DateTime<Utc>>` | Set when activated (= start + duration) |
| `created_at` | `DateTime<Utc>` | Creation timestamp |
| `updated_at` | `DateTime<Utc>` | Last modification timestamp |

**Status lifecycle**:

```
Planning ──activate()──► Active ──complete()──► Completed
                               └──cancel()───► Cancelled
```

**Key methods**:

```rust
sprint.activate(duration_days: u32)
// Sets status to Active, records start_date = now, end_date = now + duration.

sprint.formatted_name(board: &Board, default_prefix: &str) -> String
// e.g. "KAN-3/bugfix-week" or "KAN-3"

sprint.is_ended(now: DateTime<Utc>) -> bool
// True if Active and end_date is before `now`.

Sprint::for_assignment_dialog(
    sprints: &[Sprint],
    board_id: Uuid,
    now: DateTime<Utc>,
) -> (Vec<&Sprint>, Vec<&Sprint>)
// Splits a board's sprints into (active_or_planned, completed_or_ended).
// Cancelled sprints are excluded; each section is sorted by sprint_number desc.
```

---

### `SprintLog`

Records a single sprint assignment event for a card.

| Field | Type | Description |
|-------|------|-------------|
| `sprint_id` | `Uuid` | Sprint that was assigned |
| `assigned_at` | `DateTime<Utc>` | When the assignment occurred |
| `unassigned_at` | `Option<DateTime<Utc>>` | When the assignment ended, if applicable |

A card accumulates one `SprintLog` entry per unique sprint assignment. Re-assigning to the same sprint is a no-op (deduplication by `sprint_id`).

---

### `ArchivedCard`

A card that has been moved out of active columns.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique identifier |
| `original_column_id` | `Uuid` | Column the card was in before archiving |
| `original_position` | `i32` | Position before archiving |
| `card` | `Card` | Full card snapshot |
| `archived_at` | `DateTime<Utc>` | When the card was archived |

Restoring an archived card places it back in `original_column_id` at `original_position` (or a specified column if provided).

---

### `FieldUpdate<T>`

Three-state enum for partial updates to optional fields.

```rust
pub enum FieldUpdate<T> {
    NoChange,   // Leave the field as-is
    Set(T),     // Set the field to this value
    Clear,      // Set the field to None
}

// Apply to a mutable target:
update.apply_to(&mut self.due_date);
```

Used throughout all `*Update` structs to distinguish "not provided" from "explicitly set to None".

---

### `DependencyGraph`

DAG of card relationships stored alongside the board snapshot.

```rust
pub struct DependencyGraph {
    pub edges: Vec<CardEdge>,
}

pub struct CardEdge {
    pub from: Uuid,
    pub to: Uuid,
    pub edge_type: EdgeType,
}

pub enum EdgeType {
    Parent,   // `from` is a parent of `to`
    Child,    // `from` is a child of `to`
    Blocks,   // `from` blocks `to`
}
```

Cycle detection is enforced on every `add_edge` call. Self-references are rejected.

---

### `HistoryManager`

Undo/redo stack for `KanbanContext`.

```rust
pub struct HistoryManager { /* private */ }
```

| Method | Description |
|--------|-------------|
| `capture_before_command(snapshot)` | Push snapshot onto undo stack |
| `pop_undo()` | Pop and return the most recent undo snapshot |
| `push_redo(snapshot)` | Push snapshot onto redo stack |
| `suppress()` | Temporarily disable capture (used during undo/redo) |
| `clear()` | Clear both stacks (called on external reload) |

Both stacks are capped at **100 entries**. The oldest entries are dropped when the cap is exceeded.

---

## Error Types

### `KanbanError`

```rust
pub enum KanbanError {
    Domain(DomainError),
    Io(std::io::Error),
    Serialization(String),
    ConflictDetected { path: String, source: Option<Box<dyn Error + Send + Sync>> },
    Database(String),
    Internal(String),
}
```

### `DomainError`

```rust
pub enum DomainError {
    NotFound { entity: &'static str, id: Uuid },
    Validation(String),
    Dependency(DependencyError),
}
```

### `DependencyError`

```rust
pub enum DependencyError {
    CycleDetected,
    SelfReference,
    EdgeNotFound,
}
```

**Helper constructors** on `KanbanError`:

```rust
KanbanError::not_found(entity, id)
KanbanError::validation(msg)
KanbanError::serialization(msg)
KanbanError::is_not_found(&self) -> bool
KanbanError::is_validation(&self) -> bool
KanbanError::is_cycle_detected(&self) -> bool
KanbanError::is_conflict_detected(&self) -> bool
```

---

## Business Rules

- **Card numbering**: per-prefix counter stored in `Board::prefix_counters`; monotonically increasing, permanently unique per prefix
- **Sprint assignment deduplication**: assigning a card to a sprint it already belongs to is a no-op
- **Completion column**: the rightmost column is used when toggling a card to Done (falls back to the rightmost column)
- **WIP limits**: advisory only — enforcement is the UI's responsibility
- **Sprint assignability**: only `Planning` and `Active` sprints can be assigned to cards

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `kanban-core` | Error types, config, graph |
| `serde` + `serde_json` | Serialization |
| `uuid` | `Uuid` type |
| `chrono` | Timestamps |
| `thiserror` | Error derivation |
