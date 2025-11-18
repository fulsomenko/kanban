# kanban-tui

Terminal user interface for the kanban project management tool. Event-driven, component-based TUI powered by ratatui and crossterm.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kanban-tui = { path = "../kanban-tui" }
```

## API Reference

### App - Main Entry Point

```rust
pub struct App {
    pub mode: AppMode,
    pub focus: Focus,
    pub boards: Vec<Board>,
    pub columns: Vec<Column>,
    pub cards: Vec<Card>,
    pub archived_cards: Vec<ArchivedCard>,
    pub sprints: Vec<Sprint>,
    pub selected_cards: HashSet<Uuid>,
    pub animating_cards: HashMap<Uuid, CardAnimation>,
    pub search: SearchState,
    // ...more fields
}

pub enum AppMode {
    Normal, CardDetail, BoardDetail, SprintDetail, ArchivedCardsView, // ...more
}

pub enum Focus { Boards, Cards }

impl App {
    pub fn new(file_path: Option<String>) -> Result<Self>
    pub async fn run(&mut self) -> Result<()>
}
```

### Key Types

```rust
pub struct SearchState {
    pub query: String,
    pub results: Vec<CardId>,
    pub active: bool,
}

pub struct CardAnimation {
    pub card_id: Uuid,
    pub animation_type: AnimationType,
    pub start_time: Instant,
    pub duration: Duration,  // 150ms
}

pub enum AnimationType { Archiving, Restoring, Deleting }
```

## Architecture

Event-driven, component-based TUI layer consuming domain models:

```
kanban-core
    ↑
    └── kanban-domain
            ↑
            └── kanban-tui (event-driven, component-based)
                    ↑
                    └── kanban-cli
```

### Design Patterns

**Event-Driven Architecture** - Crossterm event loop with async tokio runtime:
- Terminal events (keyboard, resize) processed in main loop
- Non-blocking event handling
- Graceful shutdown on quit

**Component-Based Rendering** - Reusable ratatui widgets:
- `CardListComponent` - Reusable card list rendering
- `SelectionList` - Multi-select list component
- `DetailView` - Tabbed detail view component
- `PopupDialog` - Modal input dialogs

**Strategy Pattern** - Pluggable view and rendering logic:
- `ViewStrategy` trait for view rendering
- `RenderStrategy` for rendering pipelines
- `LayoutStrategy` for layout algorithms
- `KeybindingProvider` for context-aware shortcuts

**State Machine** - AppMode enum for view transitions:
- Normal mode → Detail view → Back to Normal
- Modal dialogs overlay Normal mode
- Mutually exclusive view states

**Multi-Select Mode** - HashSet-based selection with bulk operations:
- `v` toggles selection on current card
- Bulk archive/restore/delete
- Visual indicator for selected cards
- Independent from focused card

**Animation System** - Time-based visual feedback:
- 150ms animations for card operations
- Smooth archive/restore/delete transitions
- Non-blocking animation updates
- Concurrent animations supported

## Keyboard Shortcuts Reference

### Help
- `?` - Show help dialog

### Navigation & View Switching
- `q` - Quit application
- `1` - Switch to Boards panel
- `2` - Switch to Tasks/Cards panel
- `j` / `↓` - Navigate down
- `k` / `↑` - Navigate up
- `h` / `←` - Previous column
- `l` / `→` - Next column
- `H` / `L` - Move card left/right between columns

### Board Management
- `n` - Create new board
- `r` - Rename current board
- `e` - Edit board settings
- `Enter` / `Space` - View board detail

### Card Management
- `n` - Create new card
- `e` - Edit card details
- `d` - Archive card
- `D` - View archived cards
- `c` - Toggle completion (Todo ↔ Done)
- `Enter` / `Space` - View card detail

### Priority & Metadata
- `p` - Change card priority (Low → Medium → High → Critical)

### Sprint Management
- `a` - Assign card to sprint
- `s` - Open sprint management
- `t` - Toggle sprint filter (show all cards vs. active sprint only)

### Multi-Select Operations
- `v` - Toggle selection on current card
- `V` - Toggle task list view mode (Flat/GroupedByColumn/ColumnView)

### Sorting & Ordering
- `o` - Sort by field (Points, Priority, CreatedAt, UpdatedAt, Status, Default)
- `O` - Toggle sort order (Ascending ↔ Descending)

### Search & Filter
- `/` - Search cards (substring match in title/description)
- `f` - Open filter dialog (filter by Status, Priority, Sprint)

### Card Movement
- `m` - Move card to specific column

### Clipboard & Git Integration
- `y` - Copy card branch name to clipboard
- `Y` - Copy full git checkout command to clipboard
- Example: `feature-1/implement-login`

### View Modes
- Card Detail: `1` (Title) / `2` (Metadata) / `3` (Description)
- Card Detail: `e` - Edit current tab in external editor

### Import/Export
- `x` - Export current board to JSON file
- `X` - Export all boards to JSON file
- `i` - Import board from JSON file

### Detail Views
- `ESC` - Return to previous view
- `e` - Edit in external editor (description, board settings, etc.)

## Features in Depth

### External Editor Integration

Seamless integration for editing long-form text (card descriptions, board details):

1. Respects `$EDITOR` environment variable
2. Fallback search: `nvim` → `vim` → `nano` → `vi` (→ `notepad` on Windows)
3. Creates temporary file, spawns editor, reads result on exit
4. Full markdown support in descriptions

### Search

- `/ query` - Substring search across card titles and descriptions
- Results shown in green highlight
- Clear search with `ESC`
- Non-blocking, real-time results

### Filtering

- Status filter: Todo, InProgress, Blocked, Done
- Priority filter: Low, Medium, High, Critical
- Sprint assignment filter
- Composable filters (all conditions combined)
- Visual indicator when filters active

### Clipboard Support

- Copy branch names automatically formatted for git: `feature-1/card-title`
- Copy full checkout commands: `git checkout feature-1/card-title`
- Cross-platform support via arboard crate
- Useful for quick git integration

### Markdown Rendering

- Descriptions rendered as markdown
- Support for: headings, lists, code blocks, emphasis, links
- Terminal-aware formatting with ratatui
- Syntax highlighting in code blocks

### Card Animations

- 150ms smooth animations on:
  - Card archival
  - Card restoration (from archived)
  - Card deletion (permanent)
- Non-blocking, concurrent animation support
- Visual feedback without UI freeze

## Examples

### Creating and Running the App

```rust
use kanban_tui::App;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new(Some("boards.json".to_string()))?;
    app.run().await?;
    Ok(())
}
```

### Accessing Application State

```rust
// Current focus panel
if app.focus == Focus::Cards {
    // Working with cards/tasks
}

// Iterate over cards in active column
for card in app.cards.iter() {
    if Some(card.column_id) == current_column {
        // Process card
    }
}

// Check multi-select mode
let selected_count = app.selected_cards.len();
```

## Dependencies

- `ratatui` - Terminal UI framework
- `crossterm` - Terminal manipulation (events, styling)
- `tokio` - Async runtime for event handling
- `arboard` - Cross-platform clipboard access
- `pulldown-cmark` - Markdown parsing and rendering
- `serde`, `serde_json` - Serialization
- `uuid`, `chrono` - ID and date/time handling
- `tracing` - Structured logging

## License

Apache 2.0 - See [LICENSE.md](../../LICENSE.md) for details
