---
bump: minor
---

Every CLI command and MCP tool now accepts a human-readable name (or sprint
number) anywhere it previously required a raw UUID. Plain UUIDs continue to
work, so existing scripts are unaffected.

You can now write things like:

```
kanban board get Kanban
kanban column list --board-id Kanban
kanban sprint activate yarara-release
kanban sprint get 15
kanban card create --board-id Kanban --column-id TODO --title "Hi"
kanban card move KAN-12 --column-id Doing
kanban card move-cards --ids KAN-1,KAN-2 --column-id Doing
kanban card assign-cards-to-sprint --ids KAN-1,KAN-2 --sprint-id yarara-release
```

The same input flexibility applies to every MCP tool. Board, column, and
sprint fields in tool schemas now read "UUID or name" (or "UUID, name, or
number" for sprints) instead of demanding a UUID. The batch tools
`archive_cards`, `move_cards`, and `assign_cards_to_sprint` take a JSON array
of card UUIDs or identifiers (for example `["KAN-1", "KAN-2"]`) in place of
the old comma-separated string.

When a name does not match, the error tells you exactly what is available,
for example: `Column 'done' not found. Available: 'TODO', 'Doing', 'Complete'`.
When a name is ambiguous across boards, the error names the boards in
conflict and asks you to disambiguate by UUID or a unique name.

For card move-cards and assign-cards-to-sprint, the selection must now share
a single board so the target column or sprint can be resolved unambiguously
within it; mixing cards from different boards produces a clear "Batch
operation requires all cards on the same board" error.

Names are case-insensitive. Sprints accept either a name (matched against
the board's stored sprint names) or a sprint number. Cards accept their
prefix-number identifier (such as `KAN-5`) or a bare card number, in
addition to the full UUID.

The TUI is unchanged in this release, but the same resolver functions are
now available to it for future text-input features (command palette,
jump-to-board, and so on).
