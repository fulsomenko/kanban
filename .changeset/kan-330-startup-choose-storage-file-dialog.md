---
bump: patch
---

Show a "choose storage file" dialog on TUI startup when no file is configured (KAN-330)

- Opening `kanban` with no file argument, no `KANBAN_FILE` env var, and no `storage_location` config now shows a startup dialog explaining both modes instead of silently opening an ephemeral in-memory board
- The dialog is pre-filled with `kanban.json` — pressing Enter creates and saves to that file; typing a different name uses that path instead
- Pressing Escape dismisses the dialog and continues in memory, with the existing `x` export available to save work to a file at any time
- Choosing a file mid-session fully adopts that backend: the in-memory state is preserved, the file is created on disk, and subsequent changes are persisted normally
