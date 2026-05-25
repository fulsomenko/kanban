---
bump: patch
---

Actions you take on the currently-open card right after an external write (a CLI update, an MCP tool call, another TUI saving) now operate on the card you are actually viewing, instead of silently operating on whichever card happens to occupy that slot in the freshly reloaded list.

Affected interactions include pressing `e` on the card detail Metadata section, opening the Manage Parents and Manage Children dialogs, the parent and child counts shown in the detail sidebar, the priority popup, the sprint-assign popup, the points dialog, editing the card title or description, copying the branch name or git checkout command, and the current-priority and current-sprint indicators in their respective dialogs. Pressing Backspace to return through the detail-view navigation history also now resolves the previous card by identity rather than by position, so back-navigation lands on the originally visited card even after the cards list has been re-sorted underneath it.

The underlying bug was that the TUI tracked the active card by both its stable UUID and its position in the cards list. After an external write the file watcher reloaded the list sorted by most recently updated, but the stored position pointed at a now-different card. Every action that resolved the active card by position silently operated on the wrong target. All such call sites now resolve the active card through the model's UUID-keyed lookup, the navigation history stores UUIDs instead of positions, and the active-card type no longer carries a position at all, so this class of bug can no longer be reintroduced by future handlers.
