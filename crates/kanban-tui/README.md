# kanban-tui

Terminal user interface for the kanban project management tool. A keyboard-driven, vim-inspired interface for managing your projects.

## Installation

### From crates.io

```bash
cargo install kanban-cli
kanban
```

### From Source

```bash
git clone https://github.com/fulsomenko/kanban
cd kanban
cargo install --path crates/kanban-cli
```

## Quick Start

```bash
kanban                    # Launch the app
kanban myboard.json       # Load a board from file
```

**First time?**
1. Press `n` to create a new board
2. Press `Enter` to activate it
3. Add cards with `n` and organize them
4. Press `?` to see all available shortcuts

## Keyboard Shortcuts

> **Tip:** Press `?` at any time to view context-aware help.

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Navigate down |
| `k` / `↑` | Navigate up |
| `h` / `←` | Previous column |
| `l` / `→` | Next column |
| `1` | Switch to Boards panel |
| `2` | Switch to Tasks panel |
| `q` | Quit application |

### Board Management

| Key | Action |
|-----|--------|
| `n` | Create new board |
| `r` | Rename board |
| `e` | Edit board settings |
| `Enter` | View board detail |

### Card Management

| Key | Action |
|-----|--------|
| `n` | Create new card |
| `e` | Edit card details |
| `r` | Rename card |
| `d` | Archive card |
| `D` | View archived cards |
| `c` | Toggle completion (Todo ↔ Done) |
| `p` | Change priority |
| `H` / `L` | Move card left/right between columns |
| `m` | Move card to specific column |
| `Enter` | View card detail |

### Multi-Select & Bulk Operations

| Key | Action |
|-----|--------|
| `v` | Toggle selection on current card |
| `V` | Toggle view mode (Flat/Grouped/Kanban) |

### Search & Filter

| Key | Action |
|-----|--------|
| `/` | Search cards |
| `f` | Filter by status, priority, or sprint |
| `Esc` | Clear search/filter |

### Sprint Management

| Key | Action |
|-----|--------|
| `s` | Open sprint management |
| `a` | Assign card to sprint |
| `t` | Toggle sprint filter |

### Undo / Redo

| Key | Action |
|-----|--------|
| `u` | Undo last action |
| `U` | Redo last undone action |

### Sorting

| Key | Action |
|-----|--------|
| `o` | Sort by field (Points, Priority, Date, Status, Position) |
| `O` | Toggle sort order (Ascending/Descending) |

### Clipboard & Git Integration

| Key | Action |
|-----|--------|
| `y` | Copy branch name to clipboard |
| `Y` | Copy full git checkout command |

### Import/Export

| Key | Action |
|-----|--------|
| `x` | Export current board to JSON |
| `X` | Export all boards |
| `i` | Import board from JSON |

### Detail View

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate between sections (and within parent/child lists) |
| `1` / `2` / `3` | Switch tabs (Title/Metadata/Description) |
| `e` | Edit in external editor |
| `Esc` | Return to previous view |

## Features

### Multiple View Modes

Switch between views with `V`:
- **Flat List**: All cards in a simple list
- **Grouped by Column**: Cards organized under columns
- **Kanban Board**: Classic columnar layout

### Search

Press `/` to search:
- Searches card titles and descriptions
- Results highlighted in green
- Real-time, non-blocking search

### Filtering

Press `f` to filter by:
- Status: Todo, In Progress, Blocked, Done
- Priority: Low, Medium, High, Critical
- Sprint assignment

Filters are composable (all conditions combined).

### External Editor Integration

Edit long-form text (descriptions, notes) in your preferred editor:
1. Respects `$EDITOR` environment variable
2. Auto-detects: nvim → vim → nano → vi
3. Full markdown support

### Clipboard Support

Copy branch names formatted for git:
- `y` → `feature-1/card-title`
- `Y` → `git checkout -b feature-1/card-title`

### Card Animations

Smooth 150ms animations for:
- Card archival
- Card restoration
- Permanent deletion

### Story Points

Assign 1-5 point estimates with color-coded display:
- Visual indicators for sprint planning
- Point totals per column/sprint

## License

Apache 2.0 - See [LICENSE.md](../../LICENSE.md) for details
