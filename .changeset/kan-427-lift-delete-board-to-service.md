---
bump: patch
---

Lift `DeleteBoard` cascade orchestration from the domain layer to the service layer (KAN-427)

- `BoardCommand::Delete(DeleteBoard)` is now atomic — it only deletes the board record.
- The cascade (dependency-graph edges → active cards → archived cards → columns → sprints → board) is composed in `KanbanContext::delete_board` and executed as a single `execute(...)` batch, which gives one undo unit and snapshot-based rollback on partial failure.
- New `Command::Cascade(CascadeCommand)` variant groups the validation-bypassing cascade primitives: `DeleteCardEdges`, `DeleteCardsByColumns`, `DeleteArchivedCardsByColumns`, `DeleteColumnsByBoard`, `DeleteSprintsByBoard`.
- New `commands::cascade::delete_board(store, id)` is the canonical batch builder.
- New `DataStore::list_cards_by_columns` (SQLite-optimised) eliminates a per-column N+1 read in the cascade.
- User-visible behaviour is unchanged.
