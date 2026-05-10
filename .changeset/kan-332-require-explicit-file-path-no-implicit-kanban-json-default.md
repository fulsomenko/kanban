---
bump: patch
---

Require explicit file path — no implicit kanban.json default (KAN-332)

- Running `kanban <subcommand>` without a file argument, `KANBAN_FILE` env var, or `storage_location` config setting now fails with a clear error that lists all three ways to provide a file, instead of silently falling back to `kanban.json` in the current directory
- `kanban` with no args and no configured file now opens the TUI backed by an in-memory store instead of silently creating `kanban.json` — the TUI is fully usable without a file; data is not persisted until a storage location is configured from within the settings
- `kanban` with `KANBAN_FILE` or a `storage_location` config setting continues to open the TUI with that file as before
- `kanban completions` and `kanban migrate` are not affected — they do not operate on a data file
- README Quick Start updated to remove the implication that `kanban.json` is created automatically on first run
