---
bump: minor
---

Undo and redo are now implemented as **inverse-command CRUD operations**
against current state — no more whole-board snapshot clones held in
RAM, no more wipe-and-replay when you press `u`.

**Unlimited undo within a session.** The previous 200-step cap is gone.
Undo can rewind through every change you've made since opening the
file, whether that's 5 edits or 5,000.

**Lower memory and CPU during heavy editing.** Each undo step now costs
the size of the commands that produced it (a few hundred bytes) instead
of a full clone of the entire board state. Sessions no longer
accumulate snapshot copies in RAM. The pre-execute snapshot that used
to run before every command — even for safe operations — is gone too.
Snapshots are only used as a rollback fallback if a batch fails
partway through.

**Failed undos and redos can be retried.** If an undo or redo encounters
an error (e.g. a validation rule fires on an inverse), the operation
rolls back cleanly and the undo stack stays where it was — the next
attempt sees the same entry, instead of skipping over it.

**Sprint history is no longer bloated by undo cycles.** Previously,
undoing a "card assigned to sprint" would push a *new* sprint-log
entry instead of removing the one the forward action added. A user
repeatedly toggling a sprint assignment could grow a card's
`sprint_logs` vec indefinitely. Inverses now restore the card's full
prior state, so sprint history round-trips cleanly.

**Two stories, properly separated.** The previous design conflated two
concerns under a single "command store":

- **UndoStack** lives in memory for the duration of a session and
  drives `u` / `Ctrl+R`. Closing the app discards it.
- **CommandLog** is the audit history of every action — what happened,
  when, by which forward command. Foundation for the upcoming
  audit-log UI (KAN-36). Currently session-scoped on both backends;
  the on-disk SQLite table is in the schema but is wired up in a
  separate follow-up.

**Cross-session undo is deferred.** The previous attempt at making undo
survive `close → reopen` shipped an `apply_snapshot(empty) + replay`
flow that conflicted with treating SQLite as a CRUD store. Cross-
session undo needs a separate design with conflict invalidation — it
will return as its own feature once that design lands.
