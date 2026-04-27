---
bump: patch
---

SQLite storage now flushes pending writes to the main database file after
every save. Previously, SQLite's WAL mode accumulated changes in a
`.wal` sidecar file that could grow to several MB between checkpoints,
meaning a backup of just the `.db` file could be missing recent data.

Every write — whether from the TUI, CLI, or MCP server — now triggers a
`PRAGMA wal_checkpoint(TRUNCATE)`, keeping the WAL file at near-zero size
after each operation. Backups of the `.sqlite` file are now always
complete and self-contained.
