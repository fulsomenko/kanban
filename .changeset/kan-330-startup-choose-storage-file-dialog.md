---
bump: patch
---

Show a "choose storage file" dialog on TUI startup when no file is configured (KAN-330)

- Opening `kanban` with no file argument, no `KANBAN_FILE` env var, and no `storage_location` config now shows a startup dialog explaining both modes instead of silently opening an ephemeral in-memory board
- The dialog has a JSON/SQLite radio (default JSON); pressing `Tab` toggles the selection and swaps the filename's extension to match (`.json` ↔ `.sqlite`)
- The filename input is pre-filled with `boards.json` and shows a "Will be saved at: <abs path>" preview that updates as you type — pressing Enter creates the file at that path
- Pressing Escape dismisses the dialog and continues in memory, with the existing `x` export available to save work to a file at any time
- Choosing a file fully adopts that backend: the in-memory state is transferred to the new on-disk backend, the file is created, undo state is reinitialised, and subsequent changes (creating boards, etc.) are persisted normally
- If the chosen path cannot be opened (e.g. parent directory missing) the dialog stays open with the input preserved and an error banner explains what went wrong, so the user can correct the path and retry
- Layout reads top-to-bottom as description → filename input + path preview → format selector → action keys, with `x`, `Tab`, `Enter`, and `Esc` rendered in bold so the keyboard hints stand out from the surrounding prose
