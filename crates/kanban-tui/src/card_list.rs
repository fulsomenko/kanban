use crate::selection::SelectionState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardListId {
    All,
    Column(Uuid),
}

pub struct CardList {
    pub id: CardListId,
    pub cards: Vec<Uuid>,
    pub selection: SelectionState,
}

impl CardList {
    pub fn new(id: CardListId) -> Self {
        Self {
            id,
            cards: Vec::new(),
            selection: SelectionState::new(),
        }
    }

    pub fn with_cards(id: CardListId, cards: Vec<Uuid>) -> Self {
        let mut list = Self::new(id);
        list.update_cards(cards);
        list
    }

    pub fn update_cards(&mut self, cards: Vec<Uuid>) {
        let current_card = self.get_selected_card_id();
        self.cards = cards;

        if let Some(card_id) = current_card {
            if !self.select_card(card_id) {
                if !self.cards.is_empty() {
                    self.selection.set(Some(0));
                } else {
                    self.selection.clear();
                }
            }
        } else if !self.cards.is_empty() && self.selection.get().is_some() {
            let clamped_idx = self.selection.get().unwrap().min(self.cards.len() - 1);
            self.selection.set(Some(clamped_idx));
        }
    }

    pub fn get_selected_card_id(&self) -> Option<Uuid> {
        self.selection
            .get()
            .and_then(|idx| self.cards.get(idx).copied())
    }

    pub fn select_card(&mut self, card_id: Uuid) -> bool {
        if let Some(idx) = self.cards.iter().position(|&id| id == card_id) {
            self.selection.set(Some(idx));
            true
        } else {
            false
        }
    }

    pub fn navigate_up(&mut self) -> bool {
        if self.cards.is_empty() {
            return false;
        }
        let was_at_top = self.selection.get() == Some(0) || self.selection.get().is_none();
        self.selection.prev();
        was_at_top
    }

    pub fn navigate_down(&mut self) -> bool {
        if self.cards.is_empty() {
            return false;
        }
        let was_at_bottom = self.selection.get() == Some(self.cards.len() - 1);
        self.selection.next(self.cards.len());
        was_at_bottom
    }

    pub fn clear(&mut self) {
        self.selection.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn get_selected_index(&self) -> Option<usize> {
        self.selection.get()
    }

    pub fn set_selected_index(&mut self, index: Option<usize>) {
        if let Some(idx) = index {
            if idx < self.cards.len() {
                self.selection.set(Some(idx));
            } else {
                self.selection.clear();
            }
        } else {
            self.selection.clear();
        }
    }
}
