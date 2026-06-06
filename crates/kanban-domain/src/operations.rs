use crate::search::{CardSearcher, CompositeSearcher};
use crate::sort::{resolve_sort, sort_cards_in_place};
use crate::KanbanResult;
use crate::{
    AmbiguousMatch, ArchivedCard, BatchResolutionCause, BatchResolutionFailure, Board, BoardUpdate,
    Card, CardStatus, CardSummary, CardUpdate, Column, ColumnUpdate, CreateCardOptions,
    KanbanError, SortField, SortOrder, Sprint, SprintUpdate,
};
use std::borrow::Borrow;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Default, Clone)]
pub struct CardListFilter {
    pub board_id: Option<Uuid>,
    pub column_id: Option<Uuid>,
    pub sprint_id: Option<Uuid>,
    pub sprint_ids: Option<HashSet<Uuid>>,
    pub hide_assigned: bool,
    pub status: Option<CardStatus>,
    /// `CompositeSearcher::all` semantics; empty string is a no-op.
    pub search: Option<String>,
    pub sort: Option<SortField>,
    pub sort_order: Option<SortOrder>,
}

#[derive(Default, Clone)]
pub struct ArchivedCardListFilter {
    pub board_id: Option<Uuid>,
    pub sort: Option<SortField>,
    pub sort_order: Option<SortOrder>,
}

fn allowed_column_ids(columns: &[Column], board_id: Option<Uuid>) -> Option<HashSet<Uuid>> {
    board_id.map(|bid| {
        columns
            .iter()
            .filter(|c| c.board_id == bid)
            .map(|c| c.id)
            .collect()
    })
}

fn build_searcher(filter: &CardListFilter) -> Option<CompositeSearcher> {
    filter
        .search
        .as_deref()
        .filter(|q| !q.is_empty())
        .map(|q| CompositeSearcher::all(q.to_string()))
}

fn passes_filter(
    card: &Card,
    allowed_columns: Option<&HashSet<Uuid>>,
    searcher: Option<&CompositeSearcher>,
    board: Option<&Board>,
    sprints: &[Sprint],
    filter: &CardListFilter,
) -> bool {
    if let Some(allowed) = allowed_columns {
        if !allowed.contains(&card.column_id) {
            return false;
        }
    }
    if let Some(column_id) = filter.column_id {
        if card.column_id != column_id {
            return false;
        }
    }
    if let Some(sprint_id) = filter.sprint_id {
        if card.sprint_id != Some(sprint_id) {
            return false;
        }
    }
    if let Some(ref ids) = filter.sprint_ids {
        if !ids.is_empty() {
            match card.sprint_id {
                Some(sid) if ids.contains(&sid) => {}
                _ => return false,
            }
        }
    }
    if filter.hide_assigned && card.sprint_id.is_some() {
        return false;
    }
    if let Some(status) = filter.status {
        if card.status != status {
            return false;
        }
    }
    if let Some(searcher) = searcher {
        let Some(board) = board else { return true };
        if !searcher.matches(card, board, sprints) {
            return false;
        }
    }
    true
}

/// Single filter + sort entry point for in-memory card slices, generic
/// over anything that borrows a `Card` (so archived cards flow through
/// the same predicate via their `Borrow<Card>` impl).
pub fn filter_and_sort_cards<T: Borrow<Card> + Clone>(
    cards: &[T],
    columns: &[Column],
    sprints: &[Sprint],
    board: Option<&Board>,
    filter: &CardListFilter,
) -> Vec<T> {
    let allowed = allowed_column_ids(columns, filter.board_id);
    let searcher = build_searcher(filter);
    let mut result: Vec<T> = cards
        .iter()
        .filter(|c| {
            passes_filter(
                (*c).borrow(),
                allowed.as_ref(),
                searcher.as_ref(),
                board,
                sprints,
                filter,
            )
        })
        .cloned()
        .collect();
    if let Some((field, order)) = resolve_sort(filter.sort, filter.sort_order, board) {
        sort_cards_in_place(&mut result, field, order);
    }
    result
}

/// Count-only variant that shares the predicate without allocating a
/// result vector or sorting. Used by the TUI badge/count render path.
pub fn count_filtered_cards<T: Borrow<Card>>(
    cards: &[T],
    columns: &[Column],
    sprints: &[Sprint],
    board: Option<&Board>,
    filter: &CardListFilter,
) -> usize {
    let allowed = allowed_column_ids(columns, filter.board_id);
    let searcher = build_searcher(filter);
    cards
        .iter()
        .filter(|c| {
            passes_filter(
                (*c).borrow(),
                allowed.as_ref(),
                searcher.as_ref(),
                board,
                sprints,
                filter,
            )
        })
        .count()
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
    /// Single-query snapshot of every card across all boards. Used by
    /// resolvers and batch operations that need to scan once and reason
    /// in memory. Implementors must back this with a single backend
    /// query — do not compose from `list_cards_by_column`.
    fn list_all_cards(&self) -> KanbanResult<Vec<Card>>;
    /// Single-query snapshot of every column across all boards.
    fn list_all_columns(&self) -> KanbanResult<Vec<Column>>;
    /// Single-query snapshot of every sprint across all boards.
    fn list_all_sprints(&self) -> KanbanResult<Vec<Sprint>>;
    fn update_card(&mut self, id: Uuid, updates: CardUpdate) -> KanbanResult<Card>;
    fn move_card(&mut self, id: Uuid, column_id: Uuid, position: Option<i32>)
        -> KanbanResult<Card>;
    fn archive_card(&mut self, id: Uuid) -> KanbanResult<()>;
    fn restore_card(&mut self, id: Uuid, column_id: Option<Uuid>) -> KanbanResult<Card>;
    fn delete_card(&mut self, id: Uuid) -> KanbanResult<()>;
    fn list_archived_cards(&self) -> KanbanResult<Vec<ArchivedCard>>;

    fn list_archived_cards_sorted(
        &self,
        filter: ArchivedCardListFilter,
    ) -> KanbanResult<Vec<ArchivedCard>> {
        let cards = self.list_archived_cards()?;
        let board = match filter.board_id {
            Some(bid) => self.get_board(bid)?,
            None => None,
        };
        let columns = match filter.board_id {
            Some(bid) => self.list_columns(bid)?,
            None => Vec::new(),
        };
        let card_filter = CardListFilter {
            board_id: filter.board_id,
            sort: filter.sort,
            sort_order: filter.sort_order,
            ..Default::default()
        };
        Ok(filter_and_sort_cards(
            &cards,
            &columns,
            &[],
            board.as_ref(),
            &card_filter,
        ))
    }

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
    // otherwise the resolver returns `DomainError::NotFoundByName` (carrying the
    // available alternatives) or `DomainError::Ambiguous` (carrying the matches).
    // CLI/MCP layers just stringify the error; the human-friendly message lives
    // in `DomainError`'s `Display` impl.
    //
    // Each resolver call takes one fresh snapshot of the relevant slice. No
    // caching across calls; the snapshot lives only for the duration of the
    // call. Batch resolvers (`resolve_card_ids`, `require_same_board`) take a
    // single snapshot for the whole batch — internally consistent by definition.

    fn resolve_board_id(&self, raw: &str) -> KanbanResult<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(raw) {
            return Ok(uuid);
        }
        let boards = self.list_boards()?;
        let matches = crate::search::find_boards_by_name(raw, &boards);
        match matches.as_slice() {
            [] => Err(KanbanError::not_found_by_name(
                "Board",
                raw,
                boards.iter().map(|b| b.name.clone()).collect(),
            )),
            [b] => Ok(b.id),
            many => Err(KanbanError::ambiguous(
                "Board",
                raw,
                many.iter()
                    .map(|b| AmbiguousMatch {
                        label: format!("'{}'", b.name),
                        id: b.id,
                    })
                    .collect(),
            )),
        }
    }

    fn resolve_column_id(&self, raw: &str, board_id: Uuid) -> KanbanResult<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(raw) {
            return Ok(uuid);
        }
        let columns = self.list_columns(board_id)?;
        let matches = crate::search::find_columns_by_name(raw, &columns);
        match matches.as_slice() {
            [] => Err(KanbanError::not_found_by_name(
                "Column",
                raw,
                columns.iter().map(|c| c.name.clone()).collect(),
            )),
            [c] => Ok(c.id),
            many => Err(KanbanError::ambiguous(
                "Column",
                raw,
                many.iter()
                    .map(|c| AmbiguousMatch {
                        label: format!("'{}'", c.name),
                        id: c.id,
                    })
                    .collect(),
            )),
        }
    }

    fn resolve_column_id_global(&self, raw: &str) -> KanbanResult<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(raw) {
            return Ok(uuid);
        }
        // Single snapshot — no N+1.
        let all_columns = self.list_all_columns()?;
        let matches = crate::search::find_columns_by_name(raw, &all_columns);
        match matches.as_slice() {
            [] => Err(KanbanError::not_found_by_name(
                "Column",
                raw,
                all_columns.iter().map(|c| c.name.clone()).collect(),
            )),
            [c] => Ok(c.id),
            many => {
                // Only need board names for the ambiguity message — one extra query.
                let boards = self.list_boards()?;
                let matches: Vec<AmbiguousMatch> = many
                    .iter()
                    .map(|c| {
                        let board_name = boards
                            .iter()
                            .find(|b| b.id == c.board_id)
                            .map(|b| b.name.as_str())
                            .unwrap_or("(unknown)");
                        AmbiguousMatch {
                            label: format!("on board '{}'", board_name),
                            id: c.id,
                        }
                    })
                    .collect();
                Err(KanbanError::ambiguous("Column", raw, matches))
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
        let matches = crate::search::find_sprints_by_query_on_board(raw, &sprints, &board);
        match matches.as_slice() {
            [] => {
                let available = sprints
                    .iter()
                    .map(|s| {
                        let label = s.get_name(&board).unwrap_or("(unnamed)");
                        format!("#{} {}", s.sprint_number, label)
                    })
                    .collect();
                Err(KanbanError::not_found_by_name("Sprint", raw, available))
            }
            [s] => Ok(s.id),
            many => Err(KanbanError::ambiguous(
                "Sprint",
                raw,
                many.iter()
                    .map(|s| {
                        let name = s.get_name(&board).unwrap_or("(unnamed)");
                        AmbiguousMatch {
                            label: format!("#{} '{}'", s.sprint_number, name),
                            id: s.id,
                        }
                    })
                    .collect(),
            )),
        }
    }

    fn resolve_sprint_id_global(&self, raw: &str) -> KanbanResult<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(raw) {
            return Ok(uuid);
        }
        // Single snapshot — no N+1.
        let all_sprints = self.list_all_sprints()?;
        let boards = self.list_boards()?;
        let matches = crate::search::find_sprints_by_query_global(raw, &all_sprints, &boards);
        match matches.as_slice() {
            [] => {
                let available = all_sprints
                    .iter()
                    .map(|s| {
                        let label = boards
                            .iter()
                            .find(|b| b.id == s.board_id)
                            .and_then(|b| s.get_name(b))
                            .unwrap_or("(unnamed)");
                        format!("#{} {}", s.sprint_number, label)
                    })
                    .collect();
                Err(KanbanError::not_found_by_name("Sprint", raw, available))
            }
            [s] => Ok(s.id),
            many => {
                let matches: Vec<AmbiguousMatch> = many
                    .iter()
                    .map(|s| {
                        let board = boards.iter().find(|b| b.id == s.board_id);
                        let board_name = board.map(|b| b.name.as_str()).unwrap_or("(unknown)");
                        let sprint_name = board.and_then(|b| s.get_name(b)).unwrap_or("(unnamed)");
                        AmbiguousMatch {
                            label: format!(
                                "#{} '{}' on board '{}'",
                                s.sprint_number, sprint_name, board_name
                            ),
                            id: s.id,
                        }
                    })
                    .collect();
                Err(KanbanError::ambiguous("Sprint", raw, matches))
            }
        }
    }

    fn resolve_card_id(&self, raw: &str) -> KanbanResult<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(raw) {
            return Ok(uuid);
        }
        let matches = self.find_cards_by_identifier(raw)?;
        match matches.as_slice() {
            [] => Err(KanbanError::not_found_by_name(
                "Card",
                raw,
                Vec::new(), // cards aren't enumerated in the error — too many.
            )),
            [c] => Ok(c.id),
            many => Err(KanbanError::ambiguous(
                "Card",
                raw,
                many.iter()
                    .map(|c| AmbiguousMatch {
                        label: format!("'{}'", c.title),
                        id: c.id,
                    })
                    .collect(),
            )),
        }
    }

    /// Resolve a batch of card identifiers against a single snapshot. Pure
    /// in-memory matching against `find_cards_by_identifier`; one set of
    /// `list_all_*` calls regardless of batch size.
    ///
    /// On failure, returns `KanbanError::BatchResolutionFailed` with per-input
    /// typed causes so callers can introspect (which raw inputs failed, and
    /// for what reason).
    fn resolve_card_ids(&self, raws: &[String]) -> KanbanResult<Vec<Uuid>> {
        // Single snapshot for the whole batch — no per-element backend round-trips.
        let cards = self.list_all_cards()?;
        let columns = self.list_all_columns()?;
        let boards = self.list_boards()?;
        let sprints = self.list_all_sprints()?;

        let mut resolved = Vec::with_capacity(raws.len());
        let mut failures = Vec::new();
        for raw in raws {
            if let Ok(uuid) = Uuid::parse_str(raw) {
                resolved.push(uuid);
                continue;
            }
            let matches =
                crate::search::find_cards_by_identifier(raw, &cards, &columns, &boards, &sprints);
            match matches.as_slice() {
                [] => failures.push(BatchResolutionFailure {
                    raw_input: raw.clone(),
                    cause: BatchResolutionCause::NotFound,
                }),
                [c] => resolved.push(c.id),
                many => failures.push(BatchResolutionFailure {
                    raw_input: raw.clone(),
                    cause: BatchResolutionCause::Ambiguous(
                        many.iter()
                            .map(|c| AmbiguousMatch {
                                label: format!("'{}'", c.title),
                                id: c.id,
                            })
                            .collect(),
                    ),
                }),
            }
        }
        if !failures.is_empty() {
            return Err(KanbanError::batch_resolution_failed("Card", failures));
        }
        Ok(resolved)
    }

    /// For batch operations, verify all the given cards belong to the same board.
    /// Returns the shared `board_id` on success; otherwise a validation error
    /// listing the boards involved. Single snapshot — O(1) backend queries.
    fn require_same_board(&self, card_ids: &[Uuid]) -> KanbanResult<Uuid> {
        use std::collections::HashMap;
        if card_ids.is_empty() {
            return Err(KanbanError::validation(
                "No cards provided for batch operation.".to_string(),
            ));
        }
        let cards = self.list_all_cards()?;
        let columns = self.list_all_columns()?;
        // column_id -> board_id, one lookup per card afterward.
        let col_to_board: HashMap<Uuid, Uuid> =
            columns.iter().map(|c| (c.id, c.board_id)).collect();
        let card_index: HashMap<Uuid, &Card> = cards.iter().map(|c| (c.id, c)).collect();

        let mut seen = std::collections::BTreeSet::new();
        let mut found_boards = Vec::new();
        for cid in card_ids {
            let card = card_index
                .get(cid)
                .ok_or_else(|| KanbanError::not_found("card", *cid))?;
            let board_id = col_to_board
                .get(&card.column_id)
                .copied()
                .ok_or_else(|| KanbanError::not_found("column", card.column_id))?;
            if seen.insert(board_id) {
                found_boards.push(board_id);
            }
        }
        match found_boards.as_slice() {
            [b] => Ok(*b),
            many => {
                let boards = self.list_boards()?;
                let names: Vec<String> = many
                    .iter()
                    .map(|bid| {
                        boards
                            .iter()
                            .find(|b| b.id == *bid)
                            .map(|b| format!("'{}'", b.name))
                            .unwrap_or_else(|| bid.to_string())
                    })
                    .collect();
                Err(KanbanError::validation(format!(
                    "Batch operation requires all cards on the same board, but the selection spans boards: {}",
                    names.join(", ")
                )))
            }
        }
    }
}
