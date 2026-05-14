---
bump: patch
---

Fix Windows failure when launching `kanban` with an existing data file (KAN-445)

- On Windows, launching `kanban kanban.json` with the file already on disk
  no longer fails with the misleading error `Path traversal not allowed:
  'kanban.json' resolves outside current directory`
- The path validator now uses `dunce::canonicalize`, which returns the
  ordinary `C:\…` form on Windows instead of the verbatim `\\?\C:\…` UNC
  form that `std::fs::canonicalize` emits. The traversal guard's prefix
  comparison against the current working directory now succeeds for paths
  inside the cwd, as intended
- Absolute paths that point at existing files are likewise returned in
  their plain form, so downstream consumers no longer see surprise UNC
  prefixes leaking out of the service layer
- No behaviour change on Linux or macOS, and no change to the path
  traversal protection — escapes via `..` are still rejected
