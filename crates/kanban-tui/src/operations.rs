use crate::app::App;
use crate::state::commands::{
    ActivateSprint, ArchiveCard, AssignCardToSprint, CancelSprint, CompleteSprint, CreateBoard,
    CreateCard, CreateColumn, CreateSprint, DeleteBoard, DeleteCard, DeleteColumn, DeleteSprint,
    MoveCard, RestoreCard, UnassignCardFromSprint, UpdateBoard, UpdateCard, UpdateColumn,
    UpdateSprint,
};
use kanban_core::KanbanResult;
use kanban_domain::{
    ArchivedCard, Board, BoardUpdate, Card, CardFilter, CardUpdate, Column, ColumnUpdate,
    FieldUpdate, KanbanOperations, Sprint, SprintUpdate,
};
use uuid::Uuid;

impl KanbanOperations for App {
    fn create_board(&mut self, name: String, card_prefix: Option<String>) -> KanbanResult<Board> {
        let cmd = Box::new(CreateBoard { name, card_prefix });
        self.execute_command(cmd)?;
        Ok(self.boards.last().unwrap().clone())
    }

    fn list_boards(&self) -> KanbanResult<Vec<Board>> {
        Ok(self.boards.clone())
    }

    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>> {
        Ok(self.boards.iter().find(|b| b.id == id).cloned())
    }

    fn update_board(&mut self, id: Uuid, updates: BoardUpdate) -> KanbanResult<Board> {
        let cmd = Box::new(UpdateBoard {
            board_id: id,
            updates,
        });
        self.execute_command(cmd)?;
        self.get_board(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Board {}", id)))
    }

    fn delete_board(&mut self, id: Uuid) -> KanbanResult<()> {
        let cmd = Box::new(DeleteBoard { board_id: id });
        self.execute_command(cmd)
    }

    fn create_column(
        &mut self,
        board_id: Uuid,
        name: String,
        position: Option<i32>,
    ) -> KanbanResult<Column> {
        let position = position.unwrap_or_else(|| {
            self.columns
                .iter()
                .filter(|c| c.board_id == board_id)
                .count() as i32
        });
        let cmd = Box::new(CreateColumn {
            board_id,
            name,
            position,
        });
        self.execute_command(cmd)?;
        Ok(self.columns.last().unwrap().clone())
    }

    fn list_columns(&self, board_id: Uuid) -> KanbanResult<Vec<Column>> {
        Ok(self
            .columns
            .iter()
            .filter(|c| c.board_id == board_id)
            .cloned()
            .collect())
    }

    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>> {
        Ok(self.columns.iter().find(|c| c.id == id).cloned())
    }

    fn update_column(&mut self, id: Uuid, updates: ColumnUpdate) -> KanbanResult<Column> {
        let cmd = Box::new(UpdateColumn {
            column_id: id,
            updates,
        });
        self.execute_command(cmd)?;
        self.get_column(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Column {}", id)))
    }

    fn delete_column(&mut self, id: Uuid) -> KanbanResult<()> {
        let cmd = Box::new(DeleteColumn { column_id: id });
        self.execute_command(cmd)
    }

    fn reorder_column(&mut self, id: Uuid, new_position: i32) -> KanbanResult<Column> {
        let updates = ColumnUpdate {
            name: None,
            position: Some(new_position),
            wip_limit: FieldUpdate::NoChange,
        };
        self.update_column(id, updates)
    }

    fn create_card(
        &mut self,
        board_id: Uuid,
        column_id: Uuid,
        title: String,
    ) -> KanbanResult<Card> {
        let position = self
            .cards
            .iter()
            .filter(|c| c.column_id == column_id)
            .count() as i32;
        let cmd = Box::new(CreateCard {
            board_id,
            column_id,
            title,
            position,
        });
        self.execute_command(cmd)?;
        Ok(self.cards.last().unwrap().clone())
    }

    fn list_cards(&self, filter: CardFilter) -> KanbanResult<Vec<Card>> {
        let mut cards: Vec<_> = self.cards.clone();

        if let Some(board_id) = filter.board_id {
            let board_columns: Vec<_> = self
                .columns
                .iter()
                .filter(|c| c.board_id == board_id)
                .map(|c| c.id)
                .collect();
            cards.retain(|c| board_columns.contains(&c.column_id));
        }

        if let Some(column_id) = filter.column_id {
            cards.retain(|c| c.column_id == column_id);
        }

        if let Some(sprint_id) = filter.sprint_id {
            cards.retain(|c| c.sprint_id == Some(sprint_id));
        }

        if let Some(status) = filter.status {
            cards.retain(|c| c.status == status);
        }

        Ok(cards)
    }

    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>> {
        Ok(self.cards.iter().find(|c| c.id == id).cloned())
    }

    fn update_card(&mut self, id: Uuid, updates: CardUpdate) -> KanbanResult<Card> {
        let cmd = Box::new(UpdateCard {
            card_id: id,
            updates,
        });
        self.execute_command(cmd)?;
        self.get_card(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", id)))
    }

    fn move_card(
        &mut self,
        id: Uuid,
        column_id: Uuid,
        position: Option<i32>,
    ) -> KanbanResult<Card> {
        let position = position.unwrap_or_else(|| {
            self.cards
                .iter()
                .filter(|c| c.column_id == column_id)
                .count() as i32
        });
        let cmd = Box::new(MoveCard {
            card_id: id,
            new_column_id: column_id,
            new_position: position,
        });
        self.execute_command(cmd)?;
        self.get_card(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", id)))
    }

    fn archive_card(&mut self, id: Uuid) -> KanbanResult<()> {
        let cmd = Box::new(ArchiveCard { card_id: id });
        self.execute_command(cmd)
    }

    fn restore_card(&mut self, id: Uuid, column_id: Option<Uuid>) -> KanbanResult<Card> {
        let archived = self
            .archived_cards
            .iter()
            .find(|ac| ac.card.id == id)
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Archived card {}", id)))?;
        let target_column = column_id.unwrap_or(archived.original_column_id);
        let position = archived.original_position;
        let cmd = Box::new(RestoreCard {
            card_id: id,
            column_id: target_column,
            position,
        });
        self.execute_command(cmd)?;
        self.get_card(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", id)))
    }

    fn delete_card(&mut self, id: Uuid) -> KanbanResult<()> {
        let cmd = Box::new(DeleteCard { card_id: id });
        self.execute_command(cmd)
    }

    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>> {
        Ok(self.archived_cards.clone())
    }

    fn assign_card_to_sprint(&mut self, card_id: Uuid, sprint_id: Uuid) -> KanbanResult<Card> {
        let sprint = self
            .get_sprint(sprint_id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Sprint {}", sprint_id)))?;
        let board = self
            .boards
            .iter()
            .find(|b| b.id == sprint.board_id)
            .ok_or_else(|| {
                kanban_core::KanbanError::NotFound(format!("Board {}", sprint.board_id))
            })?;
        let sprint_name = sprint.get_name(board).map(|s| s.to_string());
        let cmd = Box::new(AssignCardToSprint {
            card_id,
            sprint_id,
            sprint_number: sprint.sprint_number,
            sprint_name,
            sprint_status: format!("{:?}", sprint.status),
        });
        self.execute_command(cmd)?;
        self.get_card(card_id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", card_id)))
    }

    fn unassign_card_from_sprint(&mut self, card_id: Uuid) -> KanbanResult<Card> {
        let cmd = Box::new(UnassignCardFromSprint { card_id });
        self.execute_command(cmd)?;
        self.get_card(card_id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", card_id)))
    }

    fn get_card_branch_name(&self, id: Uuid) -> KanbanResult<String> {
        let card = self
            .get_card(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", id)))?;
        let column = self
            .columns
            .iter()
            .find(|c| c.id == card.column_id)
            .ok_or_else(|| {
                kanban_core::KanbanError::NotFound(format!("Column {}", card.column_id))
            })?;
        let board = self
            .boards
            .iter()
            .find(|b| b.id == column.board_id)
            .ok_or_else(|| {
                kanban_core::KanbanError::NotFound(format!("Board {}", column.board_id))
            })?;
        Ok(card.branch_name(board, &self.sprints, "task"))
    }

    fn get_card_git_checkout(&self, id: Uuid) -> KanbanResult<String> {
        let card = self
            .get_card(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Card {}", id)))?;
        let column = self
            .columns
            .iter()
            .find(|c| c.id == card.column_id)
            .ok_or_else(|| {
                kanban_core::KanbanError::NotFound(format!("Column {}", card.column_id))
            })?;
        let board = self
            .boards
            .iter()
            .find(|b| b.id == column.board_id)
            .ok_or_else(|| {
                kanban_core::KanbanError::NotFound(format!("Board {}", column.board_id))
            })?;
        Ok(card.git_checkout_command(board, &self.sprints, "task"))
    }

    fn bulk_archive_cards(&mut self, ids: Vec<Uuid>) -> KanbanResult<usize> {
        let mut count = 0;
        for id in ids {
            if self.archive_card(id).is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    fn bulk_move_cards(&mut self, ids: Vec<Uuid>, column_id: Uuid) -> KanbanResult<usize> {
        let mut count = 0;
        for id in ids {
            if self.move_card(id, column_id, None).is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    fn bulk_assign_sprint(&mut self, ids: Vec<Uuid>, sprint_id: Uuid) -> KanbanResult<usize> {
        let mut count = 0;
        for id in ids {
            if self.assign_card_to_sprint(id, sprint_id).is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    fn create_sprint(
        &mut self,
        board_id: Uuid,
        prefix: Option<String>,
        name: Option<String>,
    ) -> KanbanResult<Sprint> {
        let (sprint_number, name_index, effective_prefix) = {
            let board = self
                .boards
                .iter_mut()
                .find(|b| b.id == board_id)
                .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Board {}", board_id)))?;

            let effective_prefix = prefix
                .or_else(|| board.sprint_prefix.clone())
                .unwrap_or_else(|| "sprint".to_string());

            board.ensure_sprint_counter_initialized(&effective_prefix, &self.sprints);
            let sprint_number = board.get_next_sprint_number(&effective_prefix);
            let name_index = name.map(|n| board.add_sprint_name_at_used_index(n));

            (sprint_number, name_index, effective_prefix)
        };

        let cmd = Box::new(CreateSprint {
            board_id,
            sprint_number,
            name_index,
            prefix: Some(effective_prefix),
        });
        self.execute_command(cmd)?;
        Ok(self.sprints.last().unwrap().clone())
    }

    fn list_sprints(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>> {
        Ok(self
            .sprints
            .iter()
            .filter(|s| s.board_id == board_id)
            .cloned()
            .collect())
    }

    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>> {
        Ok(self.sprints.iter().find(|s| s.id == id).cloned())
    }

    fn update_sprint(&mut self, id: Uuid, updates: SprintUpdate) -> KanbanResult<Sprint> {
        let cmd = Box::new(UpdateSprint {
            sprint_id: id,
            updates,
        });
        self.execute_command(cmd)?;
        self.get_sprint(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Sprint {}", id)))
    }

    fn activate_sprint(&mut self, id: Uuid, duration_days: Option<i32>) -> KanbanResult<Sprint> {
        let duration = duration_days.unwrap_or(14) as u32;
        let cmd = Box::new(ActivateSprint {
            sprint_id: id,
            duration_days: duration,
        });
        self.execute_command(cmd)?;
        self.get_sprint(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Sprint {}", id)))
    }

    fn complete_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        let cmd = Box::new(CompleteSprint { sprint_id: id });
        self.execute_command(cmd)?;
        self.get_sprint(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Sprint {}", id)))
    }

    fn cancel_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint> {
        let cmd = Box::new(CancelSprint { sprint_id: id });
        self.execute_command(cmd)?;
        self.get_sprint(id)?
            .ok_or_else(|| kanban_core::KanbanError::NotFound(format!("Sprint {}", id)))
    }

    fn delete_sprint(&mut self, id: Uuid) -> KanbanResult<()> {
        let cmd = Box::new(DeleteSprint { sprint_id: id });
        self.execute_command(cmd)
    }

    fn export_board(&self, board_id: Option<Uuid>) -> KanbanResult<String> {
        use crate::state::DataSnapshot;

        let snapshot = if let Some(id) = board_id {
            let boards: Vec<_> = self.boards.iter().filter(|b| b.id == id).cloned().collect();
            let columns: Vec<_> = self
                .columns
                .iter()
                .filter(|c| c.board_id == id)
                .cloned()
                .collect();
            let column_ids: Vec<_> = columns.iter().map(|c| c.id).collect();
            let cards: Vec<_> = self
                .cards
                .iter()
                .filter(|c| column_ids.contains(&c.column_id))
                .cloned()
                .collect();
            let sprints: Vec<_> = self
                .sprints
                .iter()
                .filter(|s| s.board_id == id)
                .cloned()
                .collect();
            DataSnapshot {
                boards,
                columns,
                cards,
                archived_cards: vec![],
                sprints,
            }
        } else {
            DataSnapshot {
                boards: self.boards.clone(),
                columns: self.columns.clone(),
                cards: self.cards.clone(),
                archived_cards: self.archived_cards.clone(),
                sprints: self.sprints.clone(),
            }
        };

        serde_json::to_string_pretty(&snapshot)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))
    }

    fn import_board(&mut self, data: &str) -> KanbanResult<Board> {
        use crate::state::DataSnapshot;

        let imported: DataSnapshot = serde_json::from_str(data)
            .map_err(|e| kanban_core::KanbanError::Serialization(e.to_string()))?;

        let board = imported
            .boards
            .first()
            .cloned()
            .ok_or_else(|| kanban_core::KanbanError::NotFound("No board in import".to_string()))?;

        self.boards.extend(imported.boards);
        self.columns.extend(imported.columns);
        self.cards.extend(imported.cards);
        self.sprints.extend(imported.sprints);

        Ok(board)
    }
}
