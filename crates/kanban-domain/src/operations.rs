use crate::KanbanResult;
use crate::{
    ArchivedCard, Board, BoardUpdate, Card, CardStatus, CardSummary, CardUpdate, Column,
    ColumnUpdate, CreateCardOptions, KanbanError, Sprint, SprintUpdate,
};
use uuid::Uuid;

/// Filter options for listing cards
#[derive(Default, Clone)]
pub struct CardListFilter {
    pub board_id: Option<Uuid>,
    pub column_id: Option<Uuid>,
    pub sprint_id: Option<Uuid>,
    pub status: Option<CardStatus>,
}

/// Trait ensuring TUI and CLI implement the same operations.
/// Adding a method here forces both implementations to add it.
pub trait KanbanOperations {
    // Board operations
    fn create_board(&mut self, name: String, card_prefix: Option<String>) -> KanbanResult<Board>;
    fn list_boards(&self) -> KanbanResult<Vec<Board>>;
    fn get_board(&self, id: Uuid) -> KanbanResult<Option<Board>>;
    fn update_board(&mut self, id: Uuid, updates: BoardUpdate) -> KanbanResult<Board>;
    fn delete_board(&mut self, id: Uuid) -> KanbanResult<()>;

    // Column operations
    fn create_column(
        &mut self,
        board_id: Uuid,
        name: String,
        position: Option<i32>,
    ) -> KanbanResult<Column>;
    fn list_columns(&self, board_id: Uuid) -> KanbanResult<Vec<Column>>;
    fn get_column(&self, id: Uuid) -> KanbanResult<Option<Column>>;
    fn update_column(&mut self, id: Uuid, updates: ColumnUpdate) -> KanbanResult<Column>;
    fn delete_column(&mut self, id: Uuid) -> KanbanResult<()>;
    fn reorder_column(&mut self, id: Uuid, new_position: i32) -> KanbanResult<Column>;

    // Card operations
    fn create_card(
        &mut self,
        board_id: Uuid,
        column_id: Uuid,
        title: String,
        options: CreateCardOptions,
    ) -> KanbanResult<Card>;
    fn list_cards(&self, filter: CardListFilter) -> KanbanResult<Vec<CardSummary>>;
    fn get_card(&self, id: Uuid) -> KanbanResult<Option<Card>>;
    fn find_cards_by_identifier(&self, identifier: &str) -> KanbanResult<Vec<Card>>;
    fn update_card(&mut self, id: Uuid, updates: CardUpdate) -> KanbanResult<Card>;
    fn move_card(&mut self, id: Uuid, column_id: Uuid, position: Option<i32>)
        -> KanbanResult<Card>;
    fn archive_card(&mut self, id: Uuid) -> KanbanResult<()>;
    fn restore_card(&mut self, id: Uuid, column_id: Option<Uuid>) -> KanbanResult<Card>;
    fn delete_card(&mut self, id: Uuid) -> KanbanResult<()>;
    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>>;

    // Card sprint operations
    fn assign_card_to_sprint(&mut self, card_id: Uuid, sprint_id: Uuid) -> KanbanResult<Card>;
    fn unassign_card_from_sprint(&mut self, card_id: Uuid) -> KanbanResult<Card>;

    // Card utilities
    fn get_card_branch_name(&self, id: Uuid) -> KanbanResult<String>;
    fn get_card_git_checkout(&self, id: Uuid) -> KanbanResult<String>;

    // Multi-card operations
    fn archive_cards(&mut self, ids: Vec<Uuid>) -> KanbanResult<usize>;
    fn move_cards(&mut self, ids: Vec<Uuid>, column_id: Uuid) -> KanbanResult<usize>;
    /// Apply per-card updates as a single undo unit. For each entry, the service
    /// layer auto-syncs the status ↔ completion-column invariant:
    /// - `status` set + `column_id` unset → chain a move to the completion column
    /// - `column_id` set + `status` unset → chain a status flip when the move crosses
    ///   the completion-column boundary
    /// - both set → caller intent wins, no chaining
    fn update_cards(&mut self, updates: Vec<(Uuid, CardUpdate)>) -> KanbanResult<usize>;
    fn assign_cards_to_sprint(&mut self, ids: Vec<Uuid>, sprint_id: Uuid) -> KanbanResult<usize>;
    fn carry_over_sprint_cards(
        &mut self,
        from_sprint_id: Uuid,
        to_sprint_id: Uuid,
    ) -> KanbanResult<usize>;

    // Sprint operations
    fn create_sprint(
        &mut self,
        board_id: Uuid,
        prefix: Option<String>,
        name: Option<String>,
    ) -> KanbanResult<Sprint>;
    fn list_sprints(&self, board_id: Uuid) -> KanbanResult<Vec<Sprint>>;
    fn get_sprint(&self, id: Uuid) -> KanbanResult<Option<Sprint>>;
    fn update_sprint(&mut self, id: Uuid, updates: SprintUpdate) -> KanbanResult<Sprint>;
    fn activate_sprint(&mut self, id: Uuid, duration_days: Option<i32>) -> KanbanResult<Sprint>;
    fn complete_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint>;
    fn cancel_sprint(&mut self, id: Uuid) -> KanbanResult<Sprint>;
    fn delete_sprint(&mut self, id: Uuid) -> KanbanResult<()>;

    // Import/Export
    fn export_board(&self, board_id: Option<Uuid>) -> KanbanResult<String>;
    fn import_board(&mut self, data: &str) -> KanbanResult<Board>;

    // ---------- Name/UUID resolvers (shared by CLI, MCP, anything else) ----------
    //
    // Each resolver accepts a raw string that may be a UUID, an entity name, or
    // (for sprints) a sprint number. The UUID fast path returns immediately;
    // otherwise the resolver lists matches and errors with a human-friendly
    // "not found, here are the alternatives" or "ambiguous, here are the
    // matches" message.

    fn resolve_board_id(&self, raw: &str) -> KanbanResult<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(raw) {
            return Ok(uuid);
        }
        let boards = self.list_boards()?;
        let matches = crate::search::find_boards_by_name(raw, &boards);
        match matches.as_slice() {
            [] => Err(KanbanError::validation(format!(
                "Board '{}' not found. Available: {}",
                raw,
                join_quoted(boards.iter().map(|b| b.name.as_str()))
            ))),
            [b] => Ok(b.id),
            many => Err(KanbanError::validation(format!(
                "Board '{}' is ambiguous: {} matches. Specify by UUID. Matching IDs: {}",
                raw,
                many.len(),
                many.iter()
                    .map(|b| b.id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }

    fn resolve_column_id(&self, raw: &str, board_id: Uuid) -> KanbanResult<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(raw) {
            return Ok(uuid);
        }
        let columns = self.list_columns(board_id)?;
        let matches = crate::search::find_columns_by_name(raw, &columns);
        match matches.as_slice() {
            [] => Err(KanbanError::validation(format!(
                "Column '{}' not found on this board. Available: {}",
                raw,
                join_quoted(columns.iter().map(|c| c.name.as_str()))
            ))),
            [c] => Ok(c.id),
            many => Err(KanbanError::validation(format!(
                "Column '{}' is ambiguous on this board: {} matches. Specify by UUID.",
                raw,
                many.len()
            ))),
        }
    }

    fn resolve_column_id_global(&self, raw: &str) -> KanbanResult<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(raw) {
            return Ok(uuid);
        }
        let boards = self.list_boards()?;
        let mut all_columns = Vec::new();
        for board in &boards {
            all_columns.extend(self.list_columns(board.id)?);
        }
        let matches = crate::search::find_columns_by_name(raw, &all_columns);
        match matches.as_slice() {
            [] => Err(KanbanError::validation(format!(
                "Column '{}' not found. Available: {}",
                raw,
                join_quoted(all_columns.iter().map(|c| c.name.as_str()))
            ))),
            [c] => Ok(c.id),
            many => {
                let board_names: Vec<String> = many
                    .iter()
                    .map(|c| {
                        boards
                            .iter()
                            .find(|b| b.id == c.board_id)
                            .map(|b| b.name.clone())
                            .unwrap_or_else(|| "(unknown)".to_string())
                    })
                    .collect();
                Err(KanbanError::validation(format!(
                    "Column '{}' is ambiguous: found on boards {}. Use the UUID or a unique name.",
                    raw,
                    join_quoted(board_names.iter().map(|s| s.as_str()))
                )))
            }
        }
    }

    fn resolve_sprint_id(&self, raw: &str, board_id: Uuid) -> KanbanResult<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(raw) {
            return Ok(uuid);
        }
        let sprints = self.list_sprints(board_id)?;
        let board = self
            .get_board(board_id)?
            .ok_or_else(|| KanbanError::not_found("board", board_id))?;
        let boards_vec = vec![board];
        let matches = crate::search::find_sprints_by_query(raw, &sprints, &boards_vec);
        match matches.as_slice() {
            [] => {
                let available = sprints
                    .iter()
                    .map(|s| {
                        let label = s.get_name(&boards_vec[0]).unwrap_or("(unnamed)");
                        format!("#{} {}", s.sprint_number, label)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(KanbanError::validation(format!(
                    "Sprint '{}' not found on this board. Available: {}",
                    raw, available
                )))
            }
            [s] => Ok(s.id),
            many => Err(KanbanError::validation(format!(
                "Sprint '{}' is ambiguous on this board: {} matches. Specify by UUID.",
                raw,
                many.len()
            ))),
        }
    }

    fn resolve_sprint_id_global(&self, raw: &str) -> KanbanResult<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(raw) {
            return Ok(uuid);
        }
        let boards = self.list_boards()?;
        let mut all_sprints = Vec::new();
        for board in &boards {
            all_sprints.extend(self.list_sprints(board.id)?);
        }
        let matches = crate::search::find_sprints_by_query(raw, &all_sprints, &boards);
        match matches.as_slice() {
            [] => Err(KanbanError::validation(format!(
                "Sprint '{}' not found across any board.",
                raw
            ))),
            [s] => Ok(s.id),
            many => {
                let board_names: Vec<String> = many
                    .iter()
                    .map(|s| {
                        boards
                            .iter()
                            .find(|b| b.id == s.board_id)
                            .map(|b| b.name.clone())
                            .unwrap_or_else(|| "(unknown)".to_string())
                    })
                    .collect();
                Err(KanbanError::validation(format!(
                    "Sprint '{}' is ambiguous: found on boards {}. Use the UUID or a unique name.",
                    raw,
                    join_quoted(board_names.iter().map(|s| s.as_str()))
                )))
            }
        }
    }

    fn resolve_card_id(&self, raw: &str) -> KanbanResult<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(raw) {
            return Ok(uuid);
        }
        let matches = self.find_cards_by_identifier(raw)?;
        match matches.as_slice() {
            [] => Err(KanbanError::validation(format!(
                "Card not found: '{}'",
                raw
            ))),
            [c] => Ok(c.id),
            many => Err(KanbanError::validation(
                crate::search::format_ambiguous_matches(raw, many),
            )),
        }
    }

    fn resolve_card_ids(&self, raws: &[String]) -> KanbanResult<Vec<Uuid>> {
        let mut resolved = Vec::with_capacity(raws.len());
        let mut errors = Vec::new();
        for raw in raws {
            match self.resolve_card_id(raw) {
                Ok(id) => resolved.push(id),
                Err(e) => errors.push(format!("'{}': {}", raw, e)),
            }
        }
        if !errors.is_empty() {
            return Err(KanbanError::validation(format!(
                "Could not resolve {} card(s): {}",
                errors.len(),
                errors.join("; ")
            )));
        }
        Ok(resolved)
    }

    /// For batch operations, verify all the given cards belong to the same board.
    /// Returns the shared `board_id` on success; otherwise a validation error
    /// listing the boards involved.
    fn require_same_board(&self, card_ids: &[Uuid]) -> KanbanResult<Uuid> {
        let mut board_ids = std::collections::BTreeSet::new();
        let mut found_boards = Vec::new();
        for cid in card_ids {
            let card = self
                .get_card(*cid)?
                .ok_or_else(|| KanbanError::not_found("card", *cid))?;
            let column = self
                .get_column(card.column_id)?
                .ok_or_else(|| KanbanError::not_found("column", card.column_id))?;
            if board_ids.insert(column.board_id) {
                found_boards.push(column.board_id);
            }
        }
        match found_boards.as_slice() {
            [] => Err(KanbanError::validation(
                "No cards provided for batch operation.".to_string(),
            )),
            [b] => Ok(*b),
            many => {
                let names: Vec<String> = many
                    .iter()
                    .map(|bid| {
                        self.get_board(*bid)
                            .ok()
                            .flatten()
                            .map(|b| b.name)
                            .unwrap_or_else(|| bid.to_string())
                    })
                    .collect();
                Err(KanbanError::validation(format!(
                    "Batch operation requires all cards on the same board, but the selection spans boards: {}",
                    join_quoted(names.iter().map(|s| s.as_str()))
                )))
            }
        }
    }
}

fn join_quoted<'a, I: IntoIterator<Item = &'a str>>(iter: I) -> String {
    iter.into_iter()
        .map(|s| format!("'{}'", s))
        .collect::<Vec<_>>()
        .join(", ")
}
