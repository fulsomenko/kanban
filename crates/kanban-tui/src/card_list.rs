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

    pub fn navigate_up(&mut self, viewport_height: usize) -> bool {
        if self.cards.is_empty() {
            return true;
        }
        let was_at_top = self.selection.get() == Some(0) || self.selection.get().is_none();

        if !was_at_top {
            let current_idx = self.selection.get().unwrap_or(0);

            // Check if there's a "cards above" indicator
            let has_cards_above = self.scroll_offset > 0;

            // First visible card index (accounting for "cards above" indicator)
            let first_visible_idx = self.scroll_offset + (if has_cards_above { 1 } else { 0 });

            // If at first visible card and there are cards above, scroll up and move selection
            if current_idx == first_visible_idx && self.scroll_offset > 0 {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                self.selection.prev();
            } else if current_idx > first_visible_idx {
                // Can move selection without scrolling (we're past the first visible position)
                self.selection.prev();
            }
            // If at first visible and can't scroll more, stay in place (don't call prev)
        }

        self.clamp_selection_to_visible(viewport_height);
        was_at_top
    }

    pub fn navigate_down(&mut self, viewport_height: usize) -> bool {
        if self.cards.is_empty() {
            return true;
        }
        let total_cards = self.cards.len();
        let was_at_bottom = self.selection.get() == Some(total_cards - 1);

        if !was_at_bottom {
            let current_idx = self.selection.get().unwrap_or(0);

            // Calculate indicator space to determine actual visible cards
            let has_cards_above = self.scroll_offset > 0;
            let has_cards_below = self.scroll_offset + viewport_height < total_cards;
            let num_indicators = (has_cards_above as usize) + (has_cards_below as usize);
            let visible_cards = viewport_height.saturating_sub(num_indicators);

            // Last visible card index in absolute terms
            let last_visible_idx = self.scroll_offset + visible_cards.saturating_sub(1);

            // If at last visible card and there are more cards to scroll to, scroll and move selection
            if current_idx == last_visible_idx && self.scroll_offset + visible_cards < total_cards {
                self.scroll_offset = self.scroll_offset.saturating_add(1);

                // After scrolling, recalculate the new last visible position based on updated state
                let new_has_cards_above = self.scroll_offset > 0;
                let new_has_cards_below = self.scroll_offset + viewport_height < total_cards;
                let new_num_indicators = (new_has_cards_above as usize) + (new_has_cards_below as usize);
                let new_visible_cards = viewport_height.saturating_sub(new_num_indicators);
                let new_last_visible_idx = self.scroll_offset + new_visible_cards.saturating_sub(1);

                // Move selection to the new last visible position
                self.selection.set(Some(new_last_visible_idx.min(total_cards - 1)));
            } else if current_idx < last_visible_idx {
                // Can move selection without scrolling
                self.selection.next(total_cards);
            }
            // If at last visible and can't scroll more, stay in place (don't call next)
        }

        self.clamp_selection_to_visible(viewport_height);
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

    pub fn clamp_selection_to_visible(&mut self, viewport_height: usize) {
        if self.cards.is_empty() {
            return;
        }

        if let Some(selected_idx) = self.selection.get() {
            // Calculate indicators for current scroll state
            let has_cards_above = self.scroll_offset > 0;
            let has_cards_below = self.scroll_offset + viewport_height < self.cards.len();
            let num_indicators = (has_cards_above as usize) + (has_cards_below as usize);
            let visible_cards = viewport_height.saturating_sub(num_indicators);

            // First and last visible card positions
            let first_visible = self.scroll_offset + (if has_cards_above { 1 } else { 0 });
            let last_visible = self.scroll_offset + visible_cards.saturating_sub(1);

            // Clamp selection to visible range
            let clamped = selected_idx.max(first_visible).min(last_visible.min(self.cards.len() - 1));
            self.selection.set(Some(clamped));
        }
    }
}
