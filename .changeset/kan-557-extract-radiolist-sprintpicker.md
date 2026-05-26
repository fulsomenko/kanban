---
bump: patch
---

Internal refactor with no user-visible behaviour change. The two sprint-assignment
dialogs (single-card and bulk) and the existing list-component navigation now
share a single set of reusable building blocks, making future selection dialogs
quicker and safer to add.

The sprint-assignment dialogs render the same Active / Planned and Completed /
Ended sections, the same green-bold "(current)" indicator, the same sticky
section header when scrolling past it, and the same colour coding for Completed
(green) versus Ended (red) sprints. Keyboard navigation, dialog framing, and
selection persistence are unchanged.

Under the hood the rendering and navigation pieces are now factored as:

- `RadioList<T>` — a domain-agnostic single-select list with optional sticky
  section-header overlay, used by both sprint-assignment dialogs.
- `SprintPicker` — a thin adapter on top of `RadioList<Option<Uuid>>` that
  knows about sprint sections, the "(current)" suffix, and the pre-selection
  rule for the create-card flow that's coming next.
- `list_nav` — pure selectable-skipping navigation helpers shared by
  `RadioList`, `sprint_assign_list`, and `ListComponent`. The duplicate
  index-step helpers on `Page` in `kanban-core` have been removed.

The refactor unlocks two upcoming changes: sprint selection at card creation
time (KAN-556) reuses `RadioList` + `SprintPicker` directly, and the planned
multi-select picker (KAN-558) will share the same `ListItem<T>` shape and
`list_nav` primitives rather than duplicating them.
