---
bump: patch
---

Editing a card's metadata through the external editor (`e` on the
Metadata section of Card Detail) now accepts plain `YYYY-MM-DD` dates
in addition to full RFC 3339 timestamps. Previously the editor
silently dropped any value that wasn't full RFC 3339, leaving the
field unchanged with no feedback. This matches what the CLI and MCP
already accepted and what the TUI already displays.

A date like `2024-01-15` is stored as midnight UTC on that day. A
full timestamp like `2024-01-15T14:30:00Z` is stored at the exact
instant the user supplied. When you re-open the editor, midnight-UTC
values are written back as `2024-01-15` (so the format you typed is
the format you see), and any non-midnight value is written back as
RFC 3339.

Malformed dates such as `"yesterday"` no longer disappear silently:
the editor now surfaces a clear error banner explaining the supported
formats. ISO 8601 zero-padding is required (`2024-1-5` is rejected
with the same banner), keeping behaviour predictable.

No file-format changes. Existing kanban files load unchanged and
existing RFC 3339 values continue to round-trip exactly as before.
