---
bump: minor
---

Every CLI command and MCP tool now accepts a human-readable name (or sprint
number) anywhere it previously required a raw UUID. Plain UUIDs continue to
work, so existing scripts that already use UUIDs still resolve correctly.

You can now write things like:

```
kanban board get Kanban
kanban column list --board Kanban
kanban sprint activate yarara-release
kanban sprint get 15
kanban card create --board Kanban --column TODO --title "Hi"
kanban card move KAN-12 --column Doing
kanban card move-cards --cards KAN-1,KAN-2 --column Doing
kanban card assign-cards-to-sprint --cards KAN-1,KAN-2 --sprint yarara-release
```

The same input flexibility applies to every MCP tool. Board, column, and
sprint fields in tool schemas now read "UUID or name" (or "UUID, name, or
number" for sprints) instead of demanding a UUID. The batch tools
`archive_cards`, `move_cards`, and `assign_cards_to_sprint` take a JSON array
of card UUIDs or identifiers (for example `["KAN-1", "KAN-2"]`) in place of
the old comma-separated string.

**Breaking flag and field renames.** Now that these inputs accept names, the
old `--*-id` flag names and `*_id` schema fields were misleading and have
been replaced with the bare entity name:

CLI flags:

| Old                                  | New                              |
| ------------------------------------ | -------------------------------- |
| `--board-id`                         | `--board`                        |
| `--column-id`                        | `--column`                       |
| `--sprint-id`                        | `--sprint`                       |
| `--ids` (on `card archive-cards`, `card move-cards`, `card assign-cards-to-sprint`) | `--cards` |

Positional arguments now show `<BOARD>` / `<COLUMN>` / `<CARD>` / `<SPRINT>`
in `--help` output instead of the generic `<ID>`.

MCP tool input fields:

| Old                  | New              |
| -------------------- | ---------------- |
| `board_id`           | `board`          |
| `column_id`          | `column`         |
| `sprint_id`          | `sprint`         |
| `card_id`            | `card`           |
| `ids` (batch tools)  | `cards`          |
| `from_sprint_id`     | `from_sprint`    |
| `to_sprint_id`       | `to_sprint`      |

No aliases are kept. Update scripts and MCP clients to the new spellings.

When a name does not match, the error tells you exactly what is available,
for example: `Column 'done' not found. Available: 'TODO', 'Doing', 'Complete'`.
When a name is ambiguous across boards, the error names the boards in
conflict and asks you to disambiguate by UUID or a unique name.

For `card move-cards` and `card assign-cards-to-sprint`, the selection must
now share a single board so the target column or sprint can be resolved
unambiguously within it; mixing cards from different boards produces a clear
"Batch operation requires all cards on the same board" error.

Names are case-insensitive. Sprints accept either a name (matched against
the board's stored sprint names) or a sprint number. Cards accept their
prefix-number identifier (such as `KAN-5`) or a bare card number, in
addition to the full UUID.

The TUI is unchanged in this release, but the same resolver functions are
now available to it for future text-input features (command palette,
jump-to-board, and so on).
