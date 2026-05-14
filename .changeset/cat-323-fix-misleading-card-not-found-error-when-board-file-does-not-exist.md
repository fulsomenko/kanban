---
bump: patch
---

Running `kanban missing.json card get KAN-1` (or any subcommand) when the
file does not exist now reports `Board file not found: 'missing.json'`
instead of the misleading `Card not found: 'KAN-1'`. The check covers
every path that can supply the file — positional argument, `KANBAN_FILE`
environment variable, and `storage_location` in the config file.

`board create` previously had an implicit dual role: it created the domain
entity AND initialised the storage file when it did not exist yet. That
responsibility has moved to `kanban <file>` with no subcommand. Running
`kanban newboard.json` now creates an empty storage file if one does not
exist and exits cleanly, making it safe to use in scripts and CI without a
live terminal. In a TTY the TUI launches as before.
