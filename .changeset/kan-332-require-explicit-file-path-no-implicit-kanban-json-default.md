---
bump: patch
---

Require explicit file path — no implicit kanban.json default (KAN-332)

- Running `kanban <subcommand>` without a file argument, `KANBAN_FILE` env var, or `storage_location` config setting now fails with a clear error that lists all three ways to provide a file, instead of silently falling back to `kanban.json` in the current directory
- The TUI launched without a file (`kanban` with no args and no config) likewise errors cleanly rather than creating a stray `kanban.json` on startup
- `kanban completions` and `kanban migrate` are not affected — they do not operate on a data file
- README Quick Start updated to reflect that a file path is always required
