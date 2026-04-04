---
bump: patch
---

## Settings page UI (`S`)

Press `S` from the boards view to open a two-column settings screen:

- **Configuration** panel — editing format, card/sprint prefixes, storage backend and location, config format and path. Navigate with `j`/`k` across rows, `h`/`l` or `1`/`2`/`3` to jump between panels.
- **Config File** panel — shows the resolved config path, whether it is loaded, and the serialization format.
- **Storage** panel — shows backend and data-file path; bottom row triggers the export dialog.

Press `e` or `Enter` (on Configuration panel) to open the config in an external editor. The file format respects `editing_format` (json or toml). Changes are validated and applied live; invalid values are rejected with an error banner.

## Storage backend switching

Changing `storage_location` in the editor triggers an async migration: data is copied to the new file, the store swaps in-place, and the UI reloads. If the destination already exists, data is loaded from it instead of migrated. The source backend is auto-detected from the file extension; mismatches between the configured backend and the actual file are corrected automatically with a warning.

## Export boards dialog (`x` in Settings)

Opens a board-selection checklist, then an options step to choose JSON or SQLite output and set a filename. JSON export is synchronous; SQLite export is async and reports success or failure via a banner when complete.

## `kanban migrate` CLI

```
kanban migrate <source> <backend> [--output <path>] [--source-backend <override>]
```

Source backend is auto-detected from the file extension. The output path defaults to the source stem with the target backend's extension.

## Config persistence (`~/.config/kanban/config.toml`)

Config is written only when at least one value differs from the compiled-in defaults. Default values are stripped before saving so the file stays minimal. Both TOML and JSON serialization formats are supported (`configuration_format`). The `editing_format` field now accepts `"toml"` in addition to `"json"`.

## Service layer additions

- `kanban_service::config::resolve_storage_location` — resolves relative storage paths to absolute (cwd join extracted from `kanban-core`, which is now a pure data crate).
- `kanban_service::migrate_store` — copies a snapshot between any two stores.
- `kanban_service::validate_and_load_store` — opens an existing store and verifies it is readable.
- `kanban_service::detect_backend` — infers the backend from a locator string.
- `KanbanContext::load_with_defaults` — convenience constructor used throughout tests.
