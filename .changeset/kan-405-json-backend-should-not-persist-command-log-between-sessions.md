---
bump: minor
---

Fix undo crash and strip command log from persistence (KAN-404, KAN-405)

- Undo is now in-session only for all backends — the command log is never written to `kanban.json` or SQLite
- Opening a file with a stale `commands` section no longer causes a crash or corrupts board state when pressing undo
- Existing data files with embedded command logs are cleaned up on the next open — JSON files are rewritten in place and SQLite files have the legacy `command_log`/`undo_state` tables dropped. Both backends announce the cleanup via the application log
- After upgrading, downgrading to a pre-405 build is not supported on SQLite databases — the legacy `command_log` and `undo_state` tables are dropped on first open
- Card sort is now deterministic when multiple cards tie on the primary sort key — tied cards order by ascending `card_number` regardless of how the backend yielded them, so cards no longer visibly jump on every render
- Archiving a card and triggering the resulting column compaction now form a single undo step instead of two, so one undo restores the previous state cleanly
- Archive selection now stays anchored to the focused card's column when archiving across multiple columns, instead of jumping to an unrelated card
