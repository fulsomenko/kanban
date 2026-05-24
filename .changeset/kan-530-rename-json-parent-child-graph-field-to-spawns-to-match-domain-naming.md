---
bump: patch
---

The JSON storage backend now uses `spawns` as the key for the parent/child
dependency-graph bucket, matching the name used everywhere else in the app
(the `SpawnsEdge` type, the `spawns_edges` SQLite table). Previously the
JSON file alone exposed this bucket as `parent_child`, a leftover from an
older field name.

Existing kanban files written in the older format are upgraded
automatically on the next load. A `.v6.backup` copy of the original file
is written before the upgrade and removed once it completes successfully,
so a failed upgrade leaves a recoverable file in place. No manual action
is needed.

The on-disk envelope version advances from 6 to 7. Older builds of the
app will refuse to open V7 files (the existing future-version guard) to
prevent silently dropping data they don't understand. SQLite storage is
unaffected, since it already used the `spawns_edges` name internally.
