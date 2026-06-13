---
bump: patch
---

Added a `suppress_next_write` default no-op method to the `ChangeDetector` trait. This is an internal infrastructure change with no user-visible behaviour difference. It prepares the persistence layer for the upcoming HTTP collaborative backend, where a `WebSocketChangeDetector` will implement `ChangeDetector` without needing write suppression (the server tags events with a writer ID instead).
