---
bump: patch
---

The SQLite backend's audit log now lives on disk. Earlier releases
kept a per-session in-memory log on SQLite even though a
`command_log` table existed in the schema, so the audit history was
lost on every session close. From this release, every batch of
commands executed against a SQLite-backed kanban file is appended to
`command_log` immediately and survives reopen. The append uses an
auto-allocated `batch_index`, so concurrent writers stay consistent
without a pre-read.

JSON files and the no-persistence in-memory backend are unchanged —
their audit log was already per-session and remains so. This release
just makes the SQLite story honest.

There is no audit-log UI yet. This change is foundation work for the
upcoming audit-log view (KAN-36). Existing SQLite files are picked up
seamlessly; nothing in the schema or on-disk layout changes apart
from the fact that rows now actually get inserted into the table that
has existed since v0.6.

Library users: the `CommandStore` programmatic interface returns
`Vec<CommandBatch>` instead of `Vec<Vec<Command>>` — same data, named
type. The `load_all_commands` convenience was removed; use
`command_count` followed by `load_commands` instead.
