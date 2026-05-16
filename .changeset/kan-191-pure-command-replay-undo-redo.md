---
bump: minor
---

Undo and redo now work the same way for every storage backend, and SQLite
files remember their undo history across sessions.

**Unlimited undo within a session.** The previous 200-step cap is gone.
Undo can rewind through every change you've made since opening the file,
whether that's 5 edits or 5,000.

**SQLite: undo survives closing and reopening the file.** Close the app,
open the same `.sqlite3` file again, and your undo stack is still
intact — keep pressing `u` to rewind exactly as you did before. JSON
files continue to scope undo to the current session, by design.

**Lower memory use during long sessions.** Each undo step now costs the
size of the commands that produced it (typically a few hundred bytes)
instead of a full clone of the entire board state, so heavy editing
sessions no longer pile up snapshot copies in RAM.

**Compatibility note.** SQLite databases created before this release will
have their internal command-log layout transparently upgraded the first
time they are opened. No user action is required and your data is not
touched. JSON files are unaffected.

This release also lays the foundation for an upcoming audit-log feature
that will expose the persisted command history through the UI and MCP.
