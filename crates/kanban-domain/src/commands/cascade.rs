//! Canonical builders for cascade-delete command batches.
//!
//! Each builder reads the live state to compute the cascade and returns a
//! `Vec<Command>` that the service layer passes to `KanbanContext::execute(...)`
//! so the whole cascade runs as one undo unit with snapshot/rollback.

use super::cascade_commands::{
    CascadeCommand, DeleteArchivedCardsByColumns, DeleteCardEdges, DeleteCardsByColumns,
    DeleteColumnsByBoard, DeleteSprintsByBoard,
};
use super::{BoardCommand, Command, DeleteBoard};
use crate::data_store::DataStore;
use crate::KanbanResult;
use uuid::Uuid;

/// Build the ordered cascade for deleting `board_id` and everything that hangs
/// off it (graph edges → cards → archived cards → columns → sprints → board).
///
/// Returns the empty vector wrapped in `Ok` if the board has no columns; the
/// final `DeleteBoard` is still appended so a no-op delete is still recorded
/// as a single command (and is consistent with the SQLite atomic delete).
pub fn delete_board(store: &dyn DataStore, board_id: Uuid) -> KanbanResult<Vec<Command>> {
    let column_ids: Vec<Uuid> = store
        .list_columns_by_board(board_id)?
        .iter()
        .map(|c| c.id)
        .collect();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_helpers::TestContext;
    use crate::{ArchivedCard, Board, Card, Column, Sprint};

    #[test]
    fn test_delete_board_builds_six_command_batch() {
        let tc = TestContext::new();
        let mut board = Board::new("B".into(), None);
        let board_id = board.id;
        let col = Column::new(board_id, "Col".into(), 0);
        let card = Card::new(&mut board, col.id, "C".into(), 0);
        let sprint = Sprint::new(board_id, 1, None, None);
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();
        tc.store.upsert_card(card).unwrap();
        tc.store.upsert_sprint(sprint).unwrap();

        let cmds = delete_board(&tc.store, board_id).unwrap();
        assert_eq!(cmds.len(), 6);
        assert!(matches!(
            cmds[0],
            Command::Cascade(CascadeCommand::DeleteCardEdges(_))
        ));
        assert!(matches!(
            cmds[1],
            Command::Cascade(CascadeCommand::DeleteCardsByColumns(_))
        ));
        assert!(matches!(
            cmds[2],
            Command::Cascade(CascadeCommand::DeleteArchivedCardsByColumns(_))
        ));
        assert!(matches!(
            cmds[3],
            Command::Cascade(CascadeCommand::DeleteColumnsByBoard(_))
        ));
        assert!(matches!(
            cmds[4],
            Command::Cascade(CascadeCommand::DeleteSprintsByBoard(_))
        ));
        assert!(matches!(cmds[5], Command::Board(BoardCommand::Delete(_))));
    }

    #[test]
    fn test_delete_board_collects_card_ids_from_columns_and_archive() {
        let tc = TestContext::new();
        let mut board = Board::new("B".into(), None);
        let board_id = board.id;
        let col = Column::new(board_id, "Col".into(), 0);
        let col_id = col.id;
        let active_card = Card::new(&mut board, col_id, "A".into(), 0);
        let active_id = active_card.id;
        let arch_card = Card::new(&mut board, col_id, "X".into(), 1);
        let arch_id = arch_card.id;
        let archived = ArchivedCard::new(arch_card, col_id, 0);
        tc.store.upsert_board(board).unwrap();
        tc.store.upsert_column(col).unwrap();
        tc.store.upsert_card(active_card).unwrap();
        tc.store.insert_archived_card(archived).unwrap();

        let cmds = delete_board(&tc.store, board_id).unwrap();
        match &cmds[0] {
            Command::Cascade(CascadeCommand::DeleteCardEdges(c)) => {
                assert!(c.ids.contains(&active_id));
                assert!(c.ids.contains(&arch_id));
            }
            _ => panic!("expected DeleteCardEdges as first command"),
        }
    }
}
