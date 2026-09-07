use super::Controller;
use kanban_domain::{filter_and_sort_boards, Board, BoardListFilter, Card, LoadState, Model};

/// The join of a flat collection's state and its archival-marker tier's
/// state, payload blanked: `Failed` wins, then `Missing`, then `NotLoaded`.
/// `Loaded` only when both sides are.
fn joined<T, A, B>(flat: LoadState<A>, markers: LoadState<B>) -> LoadState<Vec<T>> {
    match (flat, markers) {
        (LoadState::Failed(e), _) | (_, LoadState::Failed(e)) => LoadState::Failed(e),
        (LoadState::Missing, _) | (_, LoadState::Missing) => LoadState::Missing,
        (LoadState::NotLoaded, _) | (_, LoadState::NotLoaded) => LoadState::NotLoaded,
        (LoadState::Loaded(_), LoadState::Loaded(_)) => LoadState::Loaded(Vec::new()),
    }
}

impl Controller {
    pub(super) fn rebuild_card_partitions(&mut self, model: &Model) {
        match (model.cards_state().as_ref(), model.archived_cards_state()) {
            (LoadState::Loaded(cards), LoadState::Loaded(_)) => {
                let (archived_cards, live_cards): (Vec<Card>, Vec<Card>) = cards
                    .iter()
                    .cloned()
                    .partition(|c| model.archived_card_ids().contains(&c.id));
                self.displayed_cards_live = LoadState::Loaded(live_cards);
                self.displayed_cards_archived = LoadState::Loaded(archived_cards);
            }
            (LoadState::Loaded(cards), markers) => {
                self.displayed_cards_live = LoadState::Loaded(cards.clone());
                self.displayed_cards_archived = markers.map(|_| Vec::new());
            }
            (flat, markers) => {
                self.displayed_cards_live = flat.clone().map(|_| Vec::new());
                self.displayed_cards_archived = joined(flat, markers);
            }
        }
    }

    pub(super) fn rebuild_board_partitions(&mut self, model: &Model) {
        match (model.boards_state().as_ref(), model.archived_boards_state()) {
            (LoadState::Loaded(boards), LoadState::Loaded(_)) => {
                let (archived_boards, live_boards): (Vec<Board>, Vec<Board>) = boards
                    .iter()
                    .cloned()
                    .partition(|b| model.archived_board_ids().contains(&b.id));
                self.displayed_boards_live = LoadState::Loaded(live_boards);
                self.displayed_boards_archived = LoadState::Loaded(archived_boards);
            }
            (LoadState::Loaded(boards), markers) => {
                self.displayed_boards_live = LoadState::Loaded(boards.clone());
                self.displayed_boards_archived = markers.map(|_| Vec::new());
            }
            (flat, markers) => {
                self.displayed_boards_live = flat.clone().map(|_| Vec::new());
                self.displayed_boards_archived = joined(flat, markers);
            }
        }
        self.sort_partitions();
    }

    /// Sort BOTH cached board partitions, each against its own independent
    /// field/order pair. Called on sync and whenever either sort dimension
    /// changes, so the rendered lists and the selection resolvers (which read
    /// these partitions) stay consistent. Only a `Loaded` partition is sorted;
    /// any other state is left as-is.
    pub(super) fn sort_partitions(&mut self) {
        if let LoadState::Loaded(boards) = &self.displayed_boards_live {
            let live_filter = BoardListFilter {
                sort: Some(self.live_board_sort_field),
                sort_order: Some(self.live_board_sort_order),
                ..Default::default()
            };
            let sorted =
                filter_and_sort_boards(boards, &live_filter, &self.archived_board_at, None);
            self.displayed_boards_live = LoadState::Loaded(sorted);
        }
        if let LoadState::Loaded(boards) = &self.displayed_boards_archived {
            let archived_filter = BoardListFilter {
                sort: Some(self.archived_board_sort_field),
                sort_order: Some(self.archived_board_sort_order),
                ..Default::default()
            };
            let sorted =
                filter_and_sort_boards(boards, &archived_filter, &self.archived_board_at, None);
            self.displayed_boards_archived = LoadState::Loaded(sorted);
        }
    }
}
