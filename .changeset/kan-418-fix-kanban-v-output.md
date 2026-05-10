---
bump: patch
---

Fix `kanban -V` / `--version` / `--help` output (KAN-418)

- `kanban -V` and `kanban --version` now write to **stdout** with exit code **0** instead of stderr with exit 1, and no longer carry the spurious `Error:` prefix
- The trailing blank line after the version output is gone — output ends in a single newline
- `kanban --help` is fixed by the same path: stdout, exit 0, no `Error:` prefix
- Real argument errors (e.g. unknown flags) are unaffected — they continue to surface on stderr with a non-zero exit code
- The `commit:` line in `-V` output is still omitted when no commit hash is available at build time (e.g. `cargo install kanban` from crates.io)
