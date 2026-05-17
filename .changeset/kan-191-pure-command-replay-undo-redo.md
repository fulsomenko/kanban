---
bump: minor
---

Undo and redo are now implemented as **inverse-command CRUD operations**
against current state — no more whole-board snapshot clones held in RAM,
no more wipe-and-replay when you press `u`.

**Unlimited undo within a session.** The previous 200-step cap is gone.
Undo can rewind through every change you've made since opening the file,
whether that's 5 edits or 5,000.

**Lower memory and CPU during heavy editing.** Each undo step now costs
the size of the commands that produced it (a few hundred bytes) instead
of a full clone of the entire board state. Sessions no longer accumulate
snapshot copies in RAM. The pre-execute snapshot that used to run before
every command — even for safe operations — is gone too; rollback on a
failed batch now uses the backend's native transaction (SQLite) or a
cheap in-memory state copy (JSON in-memory; in-process JSON files).

**Two stories, properly separated.** The previous design conflated two
concerns under a single "command store":

- **UndoStack** lives in memory for the duration of a session and drives
  `u` / `Ctrl+R`. Closing the app discards it.
- **CommandLog** is the persisted audit history of every action — what
  happened, when, by which forward command. This is now distinct
  infrastructure, kept on disk for SQLite files, and lays the
  foundation for the upcoming audit-log UI (KAN-36).

**Compatibility note.** SQLite databases created before this release
will have their internal command-log layout transparently re-keyed on
open. No user action is required; your entity data is not modified.
JSON files are unaffected.

**Cross-session undo is deferred.** The previous attempt at making undo
survive `close → reopen` shipped a `apply_snapshot(empty) + replay`
flow that conflicted with treating SQLite as a CRUD store. Cross-
session undo needs a separate design with conflict invalidation — it
will return as its own feature once that design lands.
