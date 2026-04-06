# kanban-cli

CLI entry point for the kanban workspace. Parses commands with [clap](https://docs.rs/clap), loads a `CliContext` wrapping `KanbanContext`, and emits JSON to stdout.

## Usage

```bash
kanban [FILE] [COMMAND]
kanban                        # Launch TUI with default file
kanban myboard.json           # Launch TUI with specific file
kanban myboard.json board list  # Run a CLI command
```

**File selection priority**:
1. Positional `FILE` argument
2. `KANBAN_FILE` environment variable
3. Config file `storage_location`
4. Default: `kanban.json`

All commands output JSON to stdout. Errors are written to stderr.

---

## Command Reference

### `board`

```bash
kanban board create --name <NAME> [--card-prefix <PREFIX>]
kanban board list
kanban board get <ID>
kanban board update <ID> [--name <NAME>] [--description <DESC>]
                         [--sprint-prefix <PREFIX>] [--card-prefix <PREFIX>]
kanban board delete <ID>
```

### `column`

```bash
kanban column create --board-id <ID> --name <NAME> [--position <N>]
kanban column list --board-id <ID>
kanban column get <ID>
kanban column update <ID> [--name <NAME>] [--position <N>] [--wip-limit <N>]
kanban column delete <ID>
kanban column reorder <ID> --position <N>
```

### `card`

```bash
# CRUD
kanban card create --board-id <ID> --column-id <ID> --title <TITLE>
                   [--description <DESC>] [--priority low|medium|high|critical]
                   [--points <N>] [--due-date <YYYY-MM-DD>]
kanban card list [--board-id <ID>] [--column-id <ID>] [--sprint-id <ID>]
                 [--status todo|in_progress|blocked|done]
                 [--page <N>] [--page-size <N>]
kanban card get <ID_OR_IDENTIFIER>
kanban card update <ID_OR_IDENTIFIER> [--title <TITLE>] [--description <DESC>]
                   [--priority <P>] [--status <S>] [--points <N>]
                   [--due-date <DATE>] [--clear-due-date]
kanban card delete <ID_OR_IDENTIFIER>

# Movement & archiving
kanban card move <ID_OR_IDENTIFIER> --column-id <ID> [--position <N>]
kanban card archive <ID_OR_IDENTIFIER>
kanban card restore <ID_OR_IDENTIFIER> [--column-id <ID>]

# Sprint
kanban card assign-sprint <ID_OR_IDENTIFIER> --sprint-id <ID>
kanban card unassign-sprint <ID_OR_IDENTIFIER>

# Git
kanban card branch-name <ID_OR_IDENTIFIER>
kanban card git-checkout <ID_OR_IDENTIFIER>

# Bulk operations
kanban card archive-cards --ids <UUID,UUID,...>
kanban card move-cards --ids <UUID,UUID,...> --column-id <ID>
kanban card assign-cards-to-sprint --ids <UUID,UUID,...> --sprint-id <ID>
```

**Card identifier resolution**: `<ID_OR_IDENTIFIER>` accepts:
- A full UUID: `550e8400-e29b-41d4-a716-446655440000`
- A prefix+number identifier: `KAN-5`
- If ambiguous, the command prints all matching cards and exits with error

### `sprint`

```bash
kanban sprint create --board-id <ID> [--name <NAME>] [--prefix <PREFIX>]
kanban sprint list --board-id <ID>
kanban sprint get <ID>
kanban sprint update <ID> [--name <NAME>] [--prefix <PREFIX>]
                          [--card-prefix <PREFIX>]
                          [--start-date <DATE>] [--end-date <DATE>]
kanban sprint activate <ID> [--duration-days <N>]
kanban sprint complete <ID>
kanban sprint cancel <ID>
kanban sprint delete <ID>
kanban sprint carry-over --from <ID> --to <ID>
```

### Top-level commands

```bash
kanban export [--board-id <ID>] [--output <FILE>]
kanban import <FILE>
kanban migrate <SOURCE> <TARGET>   # e.g. kanban.json kanban.sqlite
kanban completions <bash|zsh|fish|powershell>
```

---

## Output Format

All commands emit JSON to stdout:

```bash
$ kanban board list
[
  {
    "id": "...",
    "name": "My Project",
    ...
  }
]

$ kanban card get KAN-5
{
  "id": "...",
  "title": "Fix login bug",
  "priority": "High",
  ...
}
```

Bulk operations emit a `BatchOperationResult`:

```json
{
  "succeeded": ["uuid1", "uuid2"],
  "failed": [
    { "id": "uuid3", "error": "card not found" }
  ]
}
```

---

## Shell Completions

```bash
kanban completions bash   >> ~/.bashrc
kanban completions zsh    >> ~/.zshrc
kanban completions fish   > ~/.config/fish/completions/kanban.fish
kanban completions powershell >> $PROFILE
```

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `KANBAN_FILE` | Default data file path |
| `EDITOR` | External editor for description editing (TUI) |

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `kanban-service` | `KanbanContext`, all domain operations |
| `kanban-tui` | TUI launch |
| `clap` | CLI argument parsing |
| `tokio` | Async runtime |
| `serde_json` | JSON output formatting |
| `tracing` | Structured logging |
