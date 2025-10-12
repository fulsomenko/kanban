# kanban-cli

Command-line interface for the kanban project management tool.

## Features

- 🚀 **CLI Entry Point**: Main binary for launching the application
- 📝 **File Management**: JSON import/export with auto-save
- 🔧 **Argument Parsing**: Clean command-line interface with clap
- 📊 **Logging**: Structured logging with tracing
- ⚡ **Async Runtime**: Tokio-powered async execution

## Purpose

This crate serves as the application entry point:

- Command-line argument parsing
- Logging and tracing initialization
- TUI application launch
- File path handling and validation

## Architecture

The CLI is the top-level crate that coordinates all other layers:

```
kanban-core
    ↑
    ├── kanban-domain
    │       ↑
    │       └── kanban-tui
    │               ↑
    │               └── kanban-cli (entry point)
```

## Usage

```bash
# Launch interactive TUI
kanban

# Launch with specific file
kanban boards.json

# Launch with file import
kanban myproject.json
```

## Command-Line Arguments

- `[FILE]` - Optional path to JSON file for import/export
  - If file exists, boards are loaded on startup
  - If file doesn't exist, it's created with empty structure
  - Changes are auto-saved on exit (`q`)

## File Format

The CLI works with JSON files in this format:

```json
{
  "boards": [
    {
      "board": { "id": "...", "name": "My Board", ... },
      "columns": [ { "id": "...", "name": "Todo", ... } ],
      "cards": [ { "id": "...", "title": "Task", ... } ],
      "sprints": [ { "id": "...", "sprint_number": 1, ... } ]
    }
  ]
}
```

## Logging

Structured logging with tracing:
- Info level for normal operation
- Debug level for detailed diagnostics
- Error level for failures

Set log level with `RUST_LOG` environment variable:
```bash
RUST_LOG=debug kanban
```

## Design Pattern

- Entry point coordination
- Dependency injection setup
- Error handling with anyhow
- Async/await with tokio runtime

## Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Install locally
cargo install --path .
```

## Binary Name

The CLI produces a binary named `kanban` (configured in `Cargo.toml`).

## License

Apache 2.0 - See [LICENSE.md](../../LICENSE.md) for details
