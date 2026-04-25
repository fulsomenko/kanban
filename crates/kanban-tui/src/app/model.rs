use kanban_domain::{ArchivedCard, Board, Card, Column, DependencyGraph, Snapshot, Sprint};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Default)]
pub struct Model {
    boards: Option<Vec<Board>>,
    columns: Option<Vec<Column>>,
    cards: Option<Vec<Card>>,
    card_index: HashMap<Uuid, usize>,
    sprints: Option<Vec<Sprint>>,
    archived_cards: Option<Vec<ArchivedCard>>,
    archived_cards_flat: Option<Vec<Card>>,
    archived_card_index: HashMap<Uuid, usize>,
    graph: Option<DependencyGraph>,
}

impl Model {
    pub fn boards(&self) -> &[Board] {
        self.boards.as_deref().unwrap_or(&[])
    }

    pub fn columns(&self) -> &[Column] {
        self.columns.as_deref().unwrap_or(&[])
    }

    pub fn cards(&self) -> &[Card] {
        self.cards.as_deref().unwrap_or(&[])
    }

    pub fn card(&self, id: Uuid) -> Option<&Card> {
        let &idx = self.card_index.get(&id)?;
        self.cards.as_ref()?.get(idx)
    }

    pub fn sprints(&self) -> &[Sprint] {
        self.sprints.as_deref().unwrap_or(&[])
    }

    pub fn archived_cards(&self) -> &[ArchivedCard] {
        self.archived_cards.as_deref().unwrap_or(&[])
    }

    pub fn archived_cards_flat(&self) -> &[Card] {
        self.archived_cards_flat.as_deref().unwrap_or(&[])
    }

    pub fn archived_card(&self, id: Uuid) -> Option<&Card> {
        let &idx = self.archived_card_index.get(&id)?;
        self.archived_cards_flat.as_ref()?.get(idx)
    }

    pub fn graph(&self) -> &DependencyGraph {
        static DEFAULT: std::sync::LazyLock<DependencyGraph> =
            std::sync::LazyLock::new(DependencyGraph::default);
        self.graph.as_ref().unwrap_or(&DEFAULT)
    }

    pub fn load_from_snapshot(&mut self, snapshot: Snapshot) {
        self.card_index.clear();
        for (i, card) in snapshot.cards.iter().enumerate() {
            self.card_index.insert(card.id, i);
        }
        self.boards = Some(snapshot.boards);
        self.columns = Some(snapshot.columns);
        self.cards = Some(snapshot.cards);
        self.sprints = Some(snapshot.sprints);
        let flat: Vec<Card> = snapshot
            .archived_cards
            .iter()
            .map(|ac| ac.card.clone())
            .collect();
        self.archived_card_index.clear();
        for (i, card) in flat.iter().enumerate() {
            self.archived_card_index.insert(card.id, i);
        }
        self.archived_cards = Some(snapshot.archived_cards);
        self.archived_cards_flat = Some(flat);
        self.graph = Some(snapshot.graph);
    }
}
