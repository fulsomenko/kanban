---
bump: patch
---

Pressing `q` while a storage migration is in progress no longer silently
abandons the migration. The app now shows a warning banner and requires a
second `q` to confirm the abort. If the migration completes before the
second `q` is pressed, the confirmation clears automatically and the next
`q` exits cleanly with no data loss.

This fixes a data loss scenario where triggering a JSON→SQLite migration
via the config editor and immediately pressing `q` would leave the
destination file unwritten.

Also fixes a startup regression where supplying an explicit file argument
(e.g. `kanban myboard.json`) was incorrectly treated as a SQLite file when
the config had `storage_backend = "sqlite"` set, causing a load error.
