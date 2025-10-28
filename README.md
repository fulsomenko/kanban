# Kanban

A terminal-based kanban/project management tool inspired by [lazygit](https://github.com/jesseduffield/lazygit), built with Rust.

![Kanban Demo](demo.gif)

## Features

- 🎯 **SOLID Architecture**: Clean separation of concerns with Cargo workspaces
- ⚡ **Fast & Responsive**: Written in Rust with async/await
- 🖥️ **Terminal UI**: Beautiful TUI powered by ratatui
- 💾 **File Persistence**: JSON import/export with auto-save support
- ⌨️ **Keyboard-Driven**: Vim-like navigation and shortcuts
- ✅ **Task Management**: Task completion tracking, priority levels, and story points
- 🏃 **Sprint Management**: Plan, activate, and filter tasks by sprint
- 🎯 **Multi-select**: Bulk assign cards to sprints or toggle completion
- 📊 **Metadata**: Assign story points (1-5), sprint tracking
- 🔄 **Reproducible Builds**: Nix flakes for development environment

## Quick Start

### Using Nix (Recommended)

```bash
# Enter development environment
nix develop

# Run the application
cargo run

# Run the app and import boards
cargo run -- myboard.json
```

### Manual Setup

Requirements:
- Rust 1.70+

```bash
# Build and run
cargo build --release
cargo run --release

# Run with file import
cargo run --release -- myboard.json
```

## Architecture

The project follows SOLID principles with a clean layered architecture:

```
crates/
├── kanban-core     → Core traits and error handling
├── kanban-domain   → Domain models (Board, Card, Sprint, Column, Tag)
├── kanban-tui      → Terminal user interface
└── kanban-cli      → CLI entry point
```

## Development

```bash
# Run with auto-reload
cargo watch -x run

# Run tests
cargo test

# Code coverage
cargo tarpaulin

# Linting
cargo clippy

# Format code
cargo fmt
```

## Usage

```bash
kanban                # Launch interactive TUI
kanban boards.json    # Launch with import
```

### Keyboard Shortcuts

**Main View:**
- `q` - Quit application
- `1` / `2` - Switch between Projects and Tasks panels
- `j` / `k` - Navigate up/down
- `n` - Create new board/card (context-aware)
- `r` - Rename selected board
- `e` - Edit selected board details
- `v` - Select/deselect card (multi-select)
- `a` - Assign selected cards to sprint
- `c` - Toggle card completion (works with multi-select)
- `t` - Toggle sprint filter (show active sprint only)
- `x` - Export current board to JSON file
- `X` - Export all boards to JSON file
- `i` - Import board(s) from JSON file
- `Enter` / `Space` - Activate board or view card details

**Card Detail View:**
- `ESC` - Return to main view
- `q` - Quit application
- `1` / `2` / `3` - Switch between Title, Metadata, and Description panels
- `e` - Edit current panel (title, points, or description)
- `a` - Assign card to sprint

**Board Detail View:**
- `ESC` - Return to main view
- `q` - Quit application
- `1` / `2` / `3` / `4` - Switch between Name, Description, Settings, and Sprints panels
- `e` - Edit current panel
- `s` - Create new sprint (in Sprints panel)

**Sprint Detail View:**
- `ESC` - Return to board detail
- `a` - Activate sprint (Planning → Active)
- `c` - Complete sprint
- `x` - Cancel sprint

**Dialogs:**
- `ESC` - Cancel
- `Enter` - Confirm
- Standard text editing for input fields

## Data Persistence

Kanban uses JSON for all imports and exports:

```json
{
  "boards": [
    {
      "board": { "id": "...", "name": "My Board", ... },
      "columns": [ { "id": "...", "name": "Todo", ... } ],
      "cards": [ { "id": "...", "title": "My Task", ... } ],
      "sprints": [ { "id": "...", "sprint_number": 1, "status": "Active", ... } ]
    }
  ]
}
```

When providing a file path:
- If file exists, boards are loaded on startup
- If file doesn't exist, it's created with empty boards array: `{"boards":[]}`
- Changes are automatically saved on exit (`q`)

## Card Metadata

Cards support rich metadata:
- **Title**: Card title
- **Description**: Long-form details (supports external editor)
- **Status**: Todo, InProgress, Blocked, Done
- **Priority**: Low, Medium, High, Critical
- **Points**: Story points 1-5 (color-coded: 1=Cyan, 2=Green, 3=Yellow, 4=Magenta, 5=Red)
- **Sprint**: Optional sprint assignment
- **Due Date**: Timestamp for deadlines
- **Created/Updated**: Automatic timestamp tracking

## Sprint Management

Configure sprints at the board level:
- **Duration**: Sprint length in days (configurable per board)
- **Prefix**: Custom sprint naming prefix (e.g., "MVP", "Sprint")
- **Name List**: Pre-defined sprint names automatically consumed on creation
- **Lifecycle**: Planning → Active → Completed/Cancelled
- **Filtering**: Toggle task list to show only active sprint cards

### External Editor Support

When editing titles or descriptions, Kanban opens an external editor. The editor is selected with the following fallback:

1. `$EDITOR` environment variable (if set)
2. Search for installed editors:
   - **Unix/Linux/macOS**: nvim → vim → nano → vi
   - **Windows**: nvim → vim → nano → notepad
3. Default fallback:
   - **Unix/Linux/macOS**: vi
   - **Windows**: notepad

**Tip**: Set your preferred editor with `export EDITOR=vim` (or add to your shell profile)

## License

Apache 2.0 - See [LICENSE.md](LICENSE.md) for details
