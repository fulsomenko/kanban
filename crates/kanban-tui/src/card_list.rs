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
    pub scroll_offset: usize,
}

impl CardList {
    pub fn new(id: CardListId) -> Self {
        Self {
            id,
            cards: Vec::new(),
            selection: SelectionState::new(),
            scroll_offset: 0,
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

    pub fn navigate_up(&mut self, _viewport_height: usize) -> bool {
        if self.cards.is_empty() {
            return true;
        }
        let was_at_top = self.selection.get() == Some(0) || self.selection.get().is_none();

        if !was_at_top {
            let current_idx = self.selection.get().unwrap_or(0);

            if current_idx == self.scroll_offset && self.scroll_offset > 0 {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            } else {
                self.selection.prev();
            }
        }

        was_at_top
    }

    pub fn navigate_down(&mut self, viewport_height: usize) -> bool {
        if self.cards.is_empty() {
            return true;
        }
        let was_at_bottom = self.selection.get() == Some(self.cards.len() - 1);

        if !was_at_bottom {
            let current_idx = self.selection.get().unwrap_or(0);
            let viewport_bottom = self.scroll_offset + viewport_height.saturating_sub(1);
            let max_scroll = self.cards.len().saturating_sub(viewport_height.max(1));

            if current_idx == viewport_bottom && self.scroll_offset < max_scroll {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            } else {
                self.selection.next(self.cards.len());
            }
        }

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

    pub fn get_scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset.min(self.cards.len().saturating_sub(1));
    }

    pub fn ensure_selected_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }

        if let Some(selected_idx) = self.selection.get() {
            let scroll_start = self.scroll_offset;
            let scroll_end = scroll_start + viewport_height;

            if selected_idx < scroll_start {
                self.scroll_offset = selected_idx;
            } else if selected_idx >= scroll_end {
                self.scroll_offset = selected_idx.saturating_sub(viewport_height - 1);
            }
        }
    }
}
