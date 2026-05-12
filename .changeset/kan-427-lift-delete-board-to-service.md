---
bump: patch
---

Lift DeleteBoard cascade orchestration from domain to service (KAN-427)

- `BoardCommand::Delete(DeleteBoard)` is now atomic — it only deletes the board record
- Cascade orchestration (graph edges, cards, archived cards, columns, sprints) is performed by `KanbanContext::delete_board` in the service layer as a single `execute(commands)` batch
- The batch is one undo unit and is fully rollback-safe — partial failure restores the pre-delete state
- New `Command::Cascade(CascadeCommand)` variant groups the bypass-validation cascade primitives (`DeleteCardEdges`, `DeleteCardsByColumns`, `DeleteArchivedCardsByColumns`, `DeleteColumnsByBoard`, `DeleteSprintsByBoard`)
- New `commands::cascade::delete_board(store, id)` is the canonical builder for the cascade batch
- New `DataStore::list_cards_by_columns` batch method (SQLite override) — eliminates the per-column N+1 read in the cascade
- User-visible behaviour is unchanged
