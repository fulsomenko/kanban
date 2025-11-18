# kanban-cli

Command-line entry point and file management for the kanban project management tool. Coordinates workspace layers and manages JSON persistence.

## Installation

### From Source

```bash
cargo install --path .
```

### After Building

```bash
cargo build --release
# Binary located at: target/release/kanban
```

## Usage

### Basic Usage

```bash
# Launch interactive TUI
kanban

# Launch with specific file
kanban boards.json

# Launch with absolute path
kanban /path/to/myproject.json
```

## Command-Line Interface

### Arguments

```
kanban [FILE]
```

- `[FILE]` - Optional path to JSON file for import/export (default: none)
  - If file exists: loads boards on startup
  - If file doesn't exist: creates with empty structure
  - On graceful shutdown: auto-saves all changes
  - Without file arg: loads/saves to temporary file (lost on exit)

### Environment Variables

**Logging Configuration:**

```bash
# Set log level (trace, debug, info, warn, error)
RUST_LOG=debug kanban boards.json
RUST_LOG=info kanban

# Multiple crates
RUST_LOG=kanban_tui=debug,kanban_domain=info kanban
```

**Custom Editor (optional, via kanban-tui):**

```bash
EDITOR=vim kanban boards.json
```

## File Format Specification

JSON structure for board files:

```json
{
  "boards": [
    {
      "board": {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "name": "My Project",
        "description": "Project description",
        "sprint_prefix": "sprint",
        "card_prefix": "task",
        "sprint_duration_days": 14,
        "task_sort_field": "Default",
        "task_sort_order": "Ascending"
      },
      "columns": [
        {
          "id": "550e8400-e29b-41d4-a716-446655440001",
          "board_id": "550e8400-e29b-41d4-a716-446655440000",
          "name": "Todo",
          "position": 0,
          "wip_limit": null
        }
      ],
      "cards": [
        {
          "id": "550e8400-e29b-41d4-a716-446655440002",
          "column_id": "550e8400-e29b-41d4-a716-446655440001",
          "title": "Implement feature",
          "description": null,
          "priority": "Medium",
          "status": "Todo",
          "points": 3,
          "position": 0,
          "card_number": 1,
          "assigned_prefix": null
        }
      ],
      "sprints": [
        {
          "id": "550e8400-e29b-41d4-a716-446655440003",
          "board_id": "550e8400-e29b-41d4-a716-446655440000",
          "sprint_number": 1,
          "name_index": 0,
          "prefix": null,
          "status": "Planning"
        }
      ]
    }
  ]
}
```

## File Management

### Initialization

When launching with a file path:

1. **File exists**: Load boards from JSON
2. **File doesn't exist**: Create empty file with `{"boards": []}`
3. **No file argument**: Use temporary storage (lost on exit)

### Auto-Save Behavior

- **On graceful exit** (press `q` in TUI): File automatically updated with latest state
- **On force quit** (Ctrl+C): Changes may be lost (graceful shutdown recommended)
- **In-memory only**: Without file argument, all data discarded on exit

## Logging and Diagnostics

### Structured Logging

The application uses `tracing` for structured logging:

```bash
# Debug level for development
RUST_LOG=debug kanban

# Info level for normal operation
RUST_LOG=info kanban

# Trace level for detailed diagnostics
RUST_LOG=trace kanban
```

### Log Levels

- `TRACE` - Extremely detailed internal state
- `DEBUG` - Debug information for troubleshooting
- `INFO` - Normal operational information
- `WARN` - Warning messages (unexpected but handled)
- `ERROR` - Error messages (application failures)

## Architecture

Entry point layer coordinating all workspace crates:

```
kanban-core (foundation)
    ↑
    └── kanban-domain (domain logic)
            ↑
            └── kanban-tui (TUI layer)
                    ↑
                    └── kanban-cli (entry point & file management)
```

### Responsibilities

- Argument parsing with clap
- File initialization and path handling
- Logging setup with tracing
- Tokio async runtime initialization
- TUI application launch and coordination
- Graceful shutdown handling

## Examples

### Launch with Logging

```bash
# Start with debug logging
RUST_LOG=debug kanban myproject.json

# Filter by crate
RUST_LOG=kanban_domain=info,kanban_tui=debug kanban
```

### File Operations

```bash
# Create new file if doesn't exist
kanban new_project.json
# → Creates new_project.json with empty boards array

# Use existing file
kanban existing.json
# → Loads boards from existing.json

# No file (temporary, loses data on exit)
kanban
# → Uses in-memory storage only
```

## Dependencies

- `kanban-core` - Foundation types and traits
- `kanban-domain` - Domain models
- `kanban-tui` - Terminal UI
- `clap` - CLI argument parsing
- `tokio` - Async runtime
- `anyhow` - Error handling
- `tracing`, `tracing-subscriber` - Structured logging

## Binary Configuration

**Cargo.toml:**
```toml
[[bin]]
name = "kanban"
path = "src/main.rs"
```

Produces executable: `kanban` (named `kanban.exe` on Windows)

## License

Apache 2.0 - See [LICENSE.md](../../LICENSE.md) for details
