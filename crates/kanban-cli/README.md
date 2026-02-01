# kanban-cli

Command-line interface for the kanban project management tool. Supports both an interactive TUI mode and a scriptable CLI mode for automation and integration.

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

### Interactive TUI Mode

```bash
# Launch interactive TUI
kanban

# Launch with specific file
kanban boards.json

# Launch with absolute path
kanban /path/to/myproject.json
```

### CLI Mode

```bash
# All CLI commands require a data file
kanban myproject.json board list
kanban myproject.json card create --board-id <ID> --column-id <ID> --title "New task"

# Or use KANBAN_FILE environment variable
export KANBAN_FILE=myproject.json
kanban board list
kanban card list --board-id <ID>
```

## CLI Commands

### Board Operations

```bash
# List all boards
kanban board list

# Create a new board
kanban board create --name "My Project"
kanban board create --name "My Project" --sprint-prefix "SPRINT" --card-prefix "TASK"

# Get board details
kanban board get <BOARD_ID>

# Update a board
kanban board update <BOARD_ID> --name "New Name"
kanban board update <BOARD_ID> --sprint-prefix "SP" --card-prefix "TSK"

# Delete a board
kanban board delete <BOARD_ID>
```

### Column Operations

```bash
# List columns for a board
kanban column list --board-id <BOARD_ID>

# Create a column
kanban column create --board-id <BOARD_ID> --name "In Progress"
kanban column create --board-id <BOARD_ID> --name "Review" --position 2

# Reorder a column (change position)
kanban column reorder <COLUMN_ID> --position 2

# Delete a column
kanban column delete <COLUMN_ID>
```

### Card Operations

```bash
# List cards
kanban card list --board-id <BOARD_ID>
kanban card list --board-id <BOARD_ID> --column-id <COLUMN_ID>
kanban card list --board-id <BOARD_ID> --sprint-id <SPRINT_ID>

# Create a card
kanban card create --board-id <BOARD_ID> --column-id <COLUMN_ID> --title "Implement feature"
kanban card create --board-id <BOARD_ID> --column-id <COLUMN_ID> --title "Bug fix" \
  --priority high --points 3 --description "Fix the login bug"

# Get card details
kanban card get <CARD_ID>

# Update a card
kanban card update <CARD_ID> --title "Updated title"
kanban card update <CARD_ID> --priority high --status done --points 5

# Move a card to another column
kanban card move <CARD_ID> --column-id <NEW_COLUMN_ID>
kanban card move <CARD_ID> --column-id <NEW_COLUMN_ID> --position 0

# Archive/restore/delete cards
kanban card archive <CARD_ID>
kanban card restore <CARD_ID>
kanban card delete <CARD_ID>  # permanently delete archived card

# Sprint assignment
kanban card assign-sprint <CARD_ID> --sprint-id <SPRINT_ID>
kanban card unassign-sprint <CARD_ID>

# Git integration
kanban card branch-name <CARD_ID>
kanban card git-checkout <CARD_ID>

# Bulk operations (comma-separated IDs)
kanban card bulk-archive --ids <ID1>,<ID2>,<ID3>
kanban card bulk-move --ids <ID1>,<ID2>,<ID3> --column-id <COLUMN_ID>
kanban card bulk-assign-sprint --ids <ID1>,<ID2>,<ID3> --sprint-id <SPRINT_ID>
```

### Sprint Operations

```bash
# List sprints for a board
kanban sprint list --board-id <BOARD_ID>

# Create a sprint
kanban sprint create --board-id <BOARD_ID>
kanban sprint create --board-id <BOARD_ID> --card-prefix "HOTFIX"

# Sprint lifecycle
kanban sprint activate <SPRINT_ID>
kanban sprint complete <SPRINT_ID>
kanban sprint cancel <SPRINT_ID>
```

### Export/Import

```bash
# Export a single board (outputs JSON to stdout)
kanban export --board-id <BOARD_ID>
kanban export --board-id <BOARD_ID> > board.json

# Export all boards
kanban export > all-boards.json

# Import boards from file
kanban import --file boards.json
```

### Shell Completions

```bash
# Generate completions for your shell
kanban completions bash > /etc/bash_completion.d/kanban
kanban completions zsh > ~/.zsh/completions/_kanban
kanban completions fish > ~/.config/fish/completions/kanban.fish
```

## Output Format

All CLI commands output JSON for easy parsing and scripting:

```json
{
  "success": true,
  "api_version": "0.2.0",
  "data": { ... }
}
```

```bash
# Pipe to jq for processing
kanban card list --board-id <ID> | jq '.data.items[] | .title'

# Check if operation succeeded
kanban board create --name "Test" | jq '.success'
```

## Environment Variables

**Data File:**

```bash
# Set default data file
export KANBAN_FILE=~/projects/kanban.json
kanban board list  # uses KANBAN_FILE
```

**Logging Configuration:**

```bash
# Set log level (trace, debug, info, warn, error)
RUST_LOG=debug kanban boards.json
RUST_LOG=info kanban

# Multiple crates
RUST_LOG=kanban_tui=debug,kanban_domain=info kanban
```

**Custom Editor (TUI mode):**

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

- **Immediate save**: Changes are saved automatically after each action
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
