use crate::selection::SelectionState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardListId {
    All,
    Column(Uuid),
}

#[derive(Debug, Clone)]
struct ViewportInfo {
    has_cards_above: bool,
    has_cards_below: bool,
    cards_to_show: usize,
    total_cards: usize,
}

#[derive(Debug, Clone)]
pub struct CardListRenderInfo {
    pub visible_card_indices: Vec<usize>,
    pub show_above_indicator: bool,
    pub cards_above_count: usize,
    pub show_below_indicator: bool,
    pub cards_below_count: usize,
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
            let first_visible_idx = self.scroll_offset;

            if current_idx == first_visible_idx && self.scroll_offset > 0 {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                self.selection.prev();
            } else if current_idx > first_visible_idx {
                self.selection.prev();
            }
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

            let info = self.calculate_viewport_info(viewport_height);
            let actual_cards_to_show = (self.scroll_offset..total_cards)
                .take(info.cards_to_show)
                .count();
            let last_visible_idx = self.scroll_offset + actual_cards_to_show.saturating_sub(1);

            if current_idx == last_visible_idx && current_idx < total_cards - 1 {
                let target_selection = (current_idx + 1).min(total_cards - 1);

                self.scroll_offset = self.scroll_offset.saturating_add(1);

                let mut new_info = self.calculate_viewport_info(viewport_height);
                let mut new_actual_cards = (self.scroll_offset..total_cards)
                    .take(new_info.cards_to_show)
                    .count();
                let mut new_last_visible_idx =
                    self.scroll_offset + new_actual_cards.saturating_sub(1);

                while target_selection > new_last_visible_idx && target_selection < total_cards {
                    self.scroll_offset = self.scroll_offset.saturating_add(1);
                    new_info = self.calculate_viewport_info(viewport_height);
                    new_actual_cards = (self.scroll_offset..total_cards)
                        .take(new_info.cards_to_show)
                        .count();
                    new_last_visible_idx = self.scroll_offset + new_actual_cards.saturating_sub(1);
                }

                self.selection.set(Some(target_selection));
            } else if current_idx < last_visible_idx {
                self.selection.next(total_cards);
            }
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

    fn calculate_viewport_info(&self, viewport_height: usize) -> ViewportInfo {
        let total_cards = self.cards.len();
        let has_cards_above = self.scroll_offset > 0;

        let available_space = viewport_height.saturating_sub(has_cards_above as usize);
        let has_cards_below = self.scroll_offset + available_space < total_cards;

        let num_indicator_lines = (has_cards_above as usize) + (has_cards_below as usize);
        let cards_to_show = viewport_height.saturating_sub(num_indicator_lines);

        ViewportInfo {
            has_cards_above,
            has_cards_below,
            cards_to_show,
            total_cards,
        }
    }

    pub fn get_render_info(&self, viewport_height: usize) -> CardListRenderInfo {
        if self.cards.is_empty() {
            return CardListRenderInfo {
                visible_card_indices: Vec::new(),
                show_above_indicator: false,
                cards_above_count: 0,
                show_below_indicator: false,
                cards_below_count: 0,
            };
        }

        let info = self.calculate_viewport_info(viewport_height);

        let visible_indices: Vec<usize> = (0..info.cards_to_show)
            .map(|i| self.scroll_offset + i)
            .filter(|&idx| idx < self.cards.len())
            .collect();

        let cards_below_count = if visible_indices.is_empty() {
            info.total_cards.saturating_sub(self.scroll_offset)
        } else {
            let last_visible_idx = *visible_indices.last().unwrap();
            info.total_cards.saturating_sub(last_visible_idx + 1)
        };

        CardListRenderInfo {
            visible_card_indices: visible_indices,
            show_above_indicator: info.has_cards_above,
            cards_above_count: self.scroll_offset,
            show_below_indicator: info.has_cards_below,
            cards_below_count,
        }
    }

    pub fn clamp_selection_to_visible(&mut self, viewport_height: usize) {
        if self.cards.is_empty() {
            return;
        }

        if let Some(selected_idx) = self.selection.get() {
            let info = self.calculate_viewport_info(viewport_height);
            let first_visible = self.scroll_offset;
            let last_visible = self.scroll_offset + info.cards_to_show.saturating_sub(1);

            let clamped = selected_idx
                .max(first_visible)
                .min(last_visible.min(self.cards.len() - 1));
            self.selection.set(Some(clamped));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_list_with_cards(count: usize) -> CardList {
        let cards = (0..count).map(|_| Uuid::new_v4()).collect();
        CardList::with_cards(CardListId::All, cards)
    }

    #[test]
    fn test_empty_list_render_info() {
        let list = CardList::new(CardListId::All);
        let info = list.get_render_info(10);

        assert!(info.visible_card_indices.is_empty());
        assert!(!info.show_above_indicator);
        assert!(!info.show_below_indicator);
        assert_eq!(info.cards_above_count, 0);
        assert_eq!(info.cards_below_count, 0);
    }

    #[test]
    fn test_single_card_no_scrolling() {
        let list = create_list_with_cards(1);
        let info = list.get_render_info(10);

        assert_eq!(info.visible_card_indices, vec![0]);
        assert!(!info.show_above_indicator);
        assert!(!info.show_below_indicator);
        assert_eq!(info.cards_below_count, 0);
    }

    #[test]
    fn test_viewport_exactly_fits_cards() {
        let list = create_list_with_cards(5);
        let info = list.get_render_info(5);

        assert_eq!(info.visible_card_indices, vec![0, 1, 2, 3, 4]);
        assert!(!info.show_above_indicator);
        assert!(!info.show_below_indicator);
    }

    #[test]
    fn test_cards_below_indicator() {
        let list = create_list_with_cards(10);
        let info = list.get_render_info(5);

        assert_eq!(info.visible_card_indices, vec![0, 1, 2, 3]);
        assert!(!info.show_above_indicator);
        assert!(info.show_below_indicator);
        assert_eq!(info.cards_below_count, 6);
    }

    #[test]
    fn test_cards_above_and_below_indicators() {
        let mut list = create_list_with_cards(16);
        list.scroll_offset = 6;

        let info = list.get_render_info(10);

        assert!(info.show_above_indicator);
        assert_eq!(info.cards_above_count, 6);
        assert!(info.show_below_indicator);
        assert_eq!(info.cards_below_count, 2);
    }

    #[test]
    fn test_bug_scenario_card_15_visibility() {
        let mut list = create_list_with_cards(16);
        list.scroll_offset = 6;

        let info = list.get_render_info(10);

        let last_visible = *info.visible_card_indices.last().unwrap();
        assert_eq!(last_visible, 13);
        assert_eq!(info.cards_below_count, 2);

        assert!(
            info.show_below_indicator,
            "Should show indicator for cards 14 and 15"
        );
    }

    #[test]
    fn test_scroll_to_last_card_visible() {
        let mut list = create_list_with_cards(16);
        list.scroll_offset = 15;

        let info = list.get_render_info(10);

        assert_eq!(info.visible_card_indices, vec![15]);
        assert!(info.show_above_indicator);
        assert!(!info.show_below_indicator);
        assert_eq!(info.cards_below_count, 0);
    }

    #[test]
    fn test_viewport_height_accounting_for_indicators() {
        let mut list = create_list_with_cards(20);
        list.scroll_offset = 5;

        let info = list.get_render_info(10);

        assert!(info.show_above_indicator);
        assert!(info.show_below_indicator);

        let cards_shown = info.visible_card_indices.len();
        assert_eq!(cards_shown, 8);
    }

    #[test]
    fn test_navigate_down_from_middle() {
        let mut list = create_list_with_cards(20);
        list.selection.set(Some(5));

        list.navigate_down(10);

        assert_eq!(list.get_selected_index(), Some(6));
        assert_eq!(list.scroll_offset, 0);
    }

    #[test]
    fn test_navigate_down_triggers_scroll() {
        let mut list = create_list_with_cards(20);
        list.selection.set(Some(8));
        list.scroll_offset = 0;

        list.navigate_down(10);

        assert_eq!(list.scroll_offset, 2);
        assert_eq!(list.get_selected_index(), Some(9));
    }

    #[test]
    fn test_navigate_up_from_middle() {
        let mut list = create_list_with_cards(20);
        list.selection.set(Some(5));
        list.scroll_offset = 0;

        list.navigate_up(10);

        assert_eq!(list.get_selected_index(), Some(4));
    }

    #[test]
    fn test_navigate_up_triggers_scroll() {
        let mut list = create_list_with_cards(20);
        list.selection.set(Some(5));
        list.scroll_offset = 5;

        list.navigate_up(10);

        assert_eq!(list.scroll_offset, 4);
    }

    #[test]
    fn test_clamp_selection_with_above_indicator() {
        let mut list = create_list_with_cards(20);
        list.selection.set(Some(0));
        list.scroll_offset = 5;

        list.clamp_selection_to_visible(10);

        assert!(list.get_selected_index().unwrap() >= 5);
    }

    #[test]
    fn test_render_info_indices_are_valid() {
        let mut list = create_list_with_cards(20);
        list.scroll_offset = 8;

        let info = list.get_render_info(7);

        for idx in &info.visible_card_indices {
            assert!(*idx < list.cards.len(), "Index {} out of bounds", idx);
        }
    }

    #[test]
    fn test_navigate_to_last_card_with_many_cards_above() {
        let mut list = create_list_with_cards(93);
        list.scroll_offset = 46;
        list.selection.set(Some(92));

        let info = list.get_render_info(48);
        assert_eq!(info.cards_above_count, 46);
        assert!(
            !info.show_below_indicator,
            "Last card should have no 'below' indicator"
        );

        list.navigate_down(48);

        assert_eq!(
            list.get_selected_index(),
            Some(92),
            "Should stay at last card (92)"
        );
    }

    #[test]
    fn test_select_all_cards_from_start_to_end() {
        let mut list = create_list_with_cards(93);
        list.selection.set(Some(0));

        for _ in 0..92 {
            let before_idx = list.get_selected_index();
            list.navigate_down(48);
            let after_idx = list.get_selected_index();

            if before_idx.is_some() && after_idx.is_some() {
                assert!(
                    after_idx.unwrap() >= before_idx.unwrap(),
                    "Selection should move down or stay"
                );
            }
        }

        assert_eq!(
            list.get_selected_index(),
            Some(92),
            "Should reach the last card"
        );
    }

    #[test]
    fn test_indicator_space_changes_on_scroll_boundary() {
        let mut list = create_list_with_cards(93);
        list.scroll_offset = 45;
        list.selection.set(Some(92));

        let info_before = list.get_render_info(50);
        assert!(
            info_before.show_above_indicator,
            "Should have 'above' indicator"
        );

        let has_below_before = info_before.show_below_indicator;

        list.navigate_down(50);

        let info_after = list.get_render_info(50);
        assert!(
            info_after.show_above_indicator,
            "Should still have 'above' indicator"
        );

        assert_eq!(
            list.get_selected_index(),
            Some(92),
            "Should stay at last card"
        );

        if has_below_before {
            assert!(
                !info_after.show_below_indicator,
                "Below indicator should disappear at the end"
            );
        }
    }

    #[test]
    fn test_scroll_through_all_117_cards() {
        let mut list = create_list_with_cards(117);
        list.selection.set(Some(0));

        let mut iterations = 0;
        while list.get_selected_index() != Some(116) && iterations < 200 {
            list.navigate_down(48);
            iterations += 1;
        }

        assert_eq!(
            list.get_selected_index(),
            Some(116),
            "Should reach card 116 after {} iterations",
            iterations
        );
        assert!(
            iterations < 120,
            "Should reach last card in reasonable iterations"
        );
    }
}
