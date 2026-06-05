---
bump: patch
---

Internal: domain constructors and mutators that always store their string
input now accept `impl Into<String>` instead of `String`. This means
callers can pass `"foo"` or `String::from("foo")` interchangeably without
a trailing `.to_string()`, and ownership decisions stay at the call site
rather than being forced at the domain boundary.

There is no behaviour change for users. Saved files, the CLI surface,
the MCP tool schemas, and the TUI all work exactly as before. The
refactor is API-source-compatible for any external caller already
passing `String`, and only loosens what those APIs accept.

Call sites across the service, persistence-sqlite, and TUI test suites
were updated to drop the now-redundant `.to_string()` allocations, which
removes a small amount of test setup noise. The contributor guide gains
a short note describing the "unconditional store rule" so future domain
APIs follow the same convention.
