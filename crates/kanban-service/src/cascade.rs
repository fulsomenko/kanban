use kanban_domain::commands::cascade_commands::{
    CascadeCommand, DeleteArchivedCardsByColumns, DeleteCardEdges, DeleteCardsByColumns,
    DeleteColumnsByBoard, DeleteSprintsByBoard,
};
use kanban_domain::commands::{BoardCommand, Command, DeleteBoard};
use kanban_domain::data_store::DataStore;
use kanban_domain::KanbanResult;
use uuid::Uuid;

pub(crate) fn delete_board(store: &dyn DataStore, board_id: Uuid) -> KanbanResult<Vec<Command>> {
    let column_ids: Vec<Uuid> = store
        .list_columns_by_board(board_id)?
        .iter()
        .map(|c| c.id)
        .collect();

    if column_ids.is_empty() {
        return Ok(vec![Command::Board(BoardCommand::Delete(DeleteBoard {
            board_id,
        }))]);
    }

    let mut card_ids: Vec<Uuid> = store
        .list_cards_by_columns(&column_ids)?
        .iter()
        .map(|c| c.id)
        .collect();
    for ac in store.list_archived_cards_by_columns(&column_ids)? {
        card_ids.push(ac.card.id);
    }

    Ok(vec![
        Command::Cascade(CascadeCommand::DeleteCardEdges(DeleteCardEdges {
            ids: card_ids,
        })),
        Command::Cascade(CascadeCommand::DeleteCardsByColumns(DeleteCardsByColumns {
            column_ids: column_ids.clone(),
        })),
        Command::Cascade(CascadeCommand::DeleteArchivedCardsByColumns(
            DeleteArchivedCardsByColumns {
                column_ids: column_ids.clone(),
            },
        )),
        Command::Cascade(CascadeCommand::DeleteColumnsByBoard(DeleteColumnsByBoard {
            board_id,
        })),
        Command::Cascade(CascadeCommand::DeleteSprintsByBoard(DeleteSprintsByBoard {
            board_id,
        })),
        Command::Board(BoardCommand::Delete(DeleteBoard { board_id })),
    ])
}
