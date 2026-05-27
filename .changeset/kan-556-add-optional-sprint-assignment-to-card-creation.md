---
bump: minor
---

Cards can now be assigned to a sprint at creation time, in a single
action, across all three surfaces.

In the TUI, the Create Task dialog gains a sprint picker below the
title input. If the board has exactly one active (non-ended) sprint,
that sprint is pre-selected, so pressing Enter creates the card already
attached to the active sprint. With no active sprint or with multiple,
the picker defaults to "None" and the user can pick deliberately. Tab
toggles focus between the title input and the sprint picker; Down or
Esc on the title focus drops focus into the picker (Esc on the picker
focus closes the dialog); Up/Down or j/k navigate the picker; Enter
confirms from either side. The focused side is signalled with a bright
border on the title input or the picker block.

From the CLI, `kanban card create` accepts a new `--assign` / `-a`
flag. Pass a sprint UUID, name, or number to assign the new card to a
specific sprint, or pass the flag with no value to use the board's
sole active sprint. The flag fails with a clear message when there
are zero or multiple active sprints on the board.

The MCP `create_card` tool gains an optional `sprint_id` parameter
that accepts a UUID, sprint name, or sprint number and resolves it via
the same in-board sprint resolution used by `assign_card_to_sprint`.
The schema description hints to LLM callers that when a board has a
single active sprint, passing its id at create time avoids the extra
assign round-trip.

The on-disk format is unchanged. Existing scripts and integrations
keep working with no migration required; omitting the new flag/field
preserves the previous "create then assign separately" workflow.
