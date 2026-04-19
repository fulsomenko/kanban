---
bump: patch
---

Adds an in-app error log panel that captures WARN and ERROR tracing events
without corrupting the TUI display.

Previously, `tracing::warn!` and `tracing::error!` calls would write directly
to stderr during raw mode, bleeding into the terminal buffer and garbling the
UI. Log output was also lost once the session ended with no way to review it.

The fix is two-pronged. A custom `InMemoryLogLayer` replaces the stderr
subscriber in TUI mode, intercepting all WARN/ERROR events into a shared
in-memory buffer. The buffer is then surfaced through a dedicated `ErrorLog`
panel that auto-opens whenever a new ERROR is captured, and can be toggled
on demand with F12 and dismissed with Escape. The footer shows a `[!] N errors`
badge while there are unread errors.
