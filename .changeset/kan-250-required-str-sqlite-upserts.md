---
bump: patch
---

SQLite upserts for boards, columns, cards, sprint logs, and sprint name
lists now reject empty strings for `TEXT NOT NULL` fields instead of
silently writing them. A new `required_str` helper returns an error if
a required field is blank, preventing corrupted rows that would fail to
load on next open.
