---
bump: patch
---

Kanban now refuses to silently mishandle data files written by a newer
version of itself, and surfaces enough information for you to diagnose
version mismatches at a glance.

**Refuse-on-future-version.** Opening a JSON file whose `version` is higher
than the binary supports, or a SQLite database whose `schema_version`
exceeds the binary's, now returns a typed error rather than silently
coercing the file to a lower format (which previously dropped fields the
old reader did not understand). The error message tells you the file's
version, the binary's maximum, and asks you to upgrade.

**Writer stamp on save.** Every save now records which kanban produced the
file: a semver version string and the build's git commit. Old files that
lack the stamp continue to load cleanly; the new fields show up the first
time the file is rewritten.

**F12 diagnostics popup.** The "Error Log" popup that already lived behind
F12 has been renamed to **Diagnostics** and now shows:

- File path
- Format version
- Writer (the kanban that last wrote this file)
- Binary (the kanban you're running right now)
- Last saved timestamp
- Log entries in a separate, titled section below

When the file's writer is a newer semver than the running binary, the
Writer line is highlighted in yellow with a `(newer than this binary)`
suffix, so a mismatch is one keystroke away from being diagnosed instead
of buried in tracing output.

**SQLite schema bump.** The SQLite metadata table gains `writer_version`
and `writer_commit` columns and the on-disk `schema_version` is bumped to
2. Existing databases are upgraded transparently on open via the same
idempotent `ALTER TABLE ADD COLUMN` mechanism used for previous SQLite
schema additions — no manual migration required.

**Renamed const.** Internal API: `kanban_core::VERSION` is renamed to
`kanban_core::CLI_VERSION_DISPLAY` to distinguish the multi-line clap
display string from the new raw `KANBAN_VERSION` / `KANBAN_COMMIT`
components used by the writer stamp. This is only relevant to anyone
embedding kanban-core as a library.
