# kanban-tui

Terminal user interface for the kanban project management tool.

## Features

- 🖥️ **Beautiful TUI**: Powered by ratatui for rich terminal rendering
- ⌨️ **Keyboard-Driven**: Vim-like navigation inspired by lazygit
- 🎨 **Panel-Based Layout**: Multiple views (boards, tasks, details)
- 🔄 **Real-Time Updates**: Interactive state management with crossterm
- 📝 **External Editor**: Seamless integration with vim, nvim, nano, etc.
- 🎯 **Multi-Select**: Bulk operations on cards
- 📊 **Sprint Filtering**: Toggle views for active sprint focus

## Purpose

This crate provides the terminal user interface layer:

- `app` - Application state and main event loop
- `ui` - Rendering components and ratatui widgets
- `events` - Keyboard and terminal event handling
- `editor` - External editor integration for text editing

## Architecture

The TUI layer sits between the CLI and domain, consuming domain models and presenting them in a terminal interface:

```
kanban-core
    ↑
    ├── kanban-domain
    │       ↑
    │       └── kanban-tui (TUI layer)
    │               ↑
    │               └── kanban-cli
```

## Key Components

### Application State
- Board and card management
- Current selection and navigation state
- Active panel tracking
- Multi-select mode

### UI Panels
- **Projects Panel**: List of boards with metadata
- **Tasks Panel**: Cards grouped by column
- **Detail Views**: Card/board/sprint details with tabs
- **Dialogs**: Input forms for creation and editing

### Event Handling
- Keyboard shortcuts (vim-style navigation)
- Context-aware command execution
- Modal dialog management
- External editor spawning

## Keyboard Shortcuts

**Main View:**
- `q` - Quit
- `1` / `2` - Switch panels
- `j` / `k` - Navigate
- `n` - Create new
- `r` - Rename
- `e` - Edit details
- `v` - Multi-select toggle
- `a` - Assign to sprint
- `c` - Toggle completion
- `t` - Toggle sprint filter
- `x` / `X` - Export (current/all)
- `i` - Import
- `Enter` / `Space` - Activate/view

**Detail Views:**
- `ESC` - Return to previous view
- `1` / `2` / `3` - Switch detail tabs
- `e` - Edit current panel

## External Editor Integration

Supports opening external editors for long-form text with automatic fallback:

1. `$EDITOR` environment variable
2. Search for: nvim → vim → nano → vi (notepad on Windows)
3. Fallback: vi (notepad on Windows)

## Design Pattern

- Event-driven architecture with crossterm
- Component-based rendering with ratatui
- State machine for view navigation
- Async runtime with tokio

## Usage

```rust
use kanban_tui::App;

let mut app = App::new(file_path)?;
app.run().await?;
```

## License

Apache 2.0 - See [LICENSE.md](../../LICENSE.md) for details
