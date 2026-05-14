---
bump: patch
---

Fix Windows path handling in `validate_path` and storage migrations (KAN-445)

- On Windows, launching `kanban` with an existing data file (e.g. `kanban
  kanban.json`) no longer fails with the misleading error `Path traversal
  not allowed: 'kanban.json' resolves outside current directory`
- The path validator now uses `dunce::canonicalize`, which returns the
  ordinary `C:\…` form on Windows instead of the verbatim `\\?\C:\…` UNC
  form that `std::fs::canonicalize` emits. The traversal guard's prefix
  comparison against the current working directory now succeeds for paths
  inside the cwd, as intended
- The current working directory is canonicalized through the same path,
  so the comparison is robust even when the cwd itself is in non-canonical
  form (e.g. a UNC-shaped Windows cwd, or a `/var` → `/private/var`
  symlink on macOS)
- Absolute paths that point at existing files are likewise returned in
  their plain form, so downstream consumers no longer see surprise UNC
  prefixes leaking out of the service layer
- On Windows, a failed storage migration (`kanban migrate`) now actually
  removes the partially-written destination file instead of leaving an
  orphan that blocks retries. Previously the SQLite/JSON store still
  held an open file handle when cleanup ran, and Windows silently
  refuses to delete files with live handles. POSIX behaviour is
  unchanged
- No change to the path traversal protection — escapes via `..` are
  still rejected
