use crate::card_list::{CardList, CardListId};
use crossterm::event::KeyCode;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum CardListAction {
    Select(Uuid),
    Edit(Uuid),
    Complete(Uuid),
    TogglePriority(Uuid),
    AssignSprint(Uuid),
    ReassignSprint(Uuid),
    Sort,
    OrderCards,
    MoveColumn(Uuid, bool),
    Create,
    ToggleMultiSelect(Uuid),
    ClearMultiSelect,
    SelectAll,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CardListActionType {
    Navigation,
    Selection,
    Editing,
    Completion,
    Priority,
    Sprint,
    Sorting,
    Movement,
    Creation,
    MultiSelect,
}

pub struct CardListComponentConfig {
    pub enabled_actions: Vec<CardListActionType>,
    pub allow_multi_select: bool,
    pub allow_reordering: bool,
    pub allow_movement: bool,
    pub show_sprint_names: bool,
}

impl Default for CardListComponentConfig {
    fn default() -> Self {
        Self {
            enabled_actions: vec![
                CardListActionType::Navigation,
                CardListActionType::Selection,
                CardListActionType::Editing,
                CardListActionType::Completion,
                CardListActionType::Priority,
                CardListActionType::Sprint,
                CardListActionType::Sorting,
                CardListActionType::Movement,
                CardListActionType::Creation,
                CardListActionType::MultiSelect,
            ],
            allow_multi_select: true,
            allow_reordering: true,
            allow_movement: true,
            show_sprint_names: true,
        }
    }
}

impl CardListComponentConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_actions(mut self, actions: Vec<CardListActionType>) -> Self {
        self.enabled_actions = actions;
        self
    }

    pub fn with_multi_select(mut self, allow: bool) -> Self {
        self.allow_multi_select = allow;
        self
    }

    pub fn with_reordering(mut self, allow: bool) -> Self {
        self.allow_reordering = allow;
        self
    }

    pub fn with_movement(mut self, allow: bool) -> Self {
        self.allow_movement = allow;
        self
    }

    pub fn with_sprint_names(mut self, show: bool) -> Self {
        self.show_sprint_names = show;
        self
    }

    pub fn is_action_enabled(&self, action_type: &CardListActionType) -> bool {
        self.enabled_actions.contains(action_type)
    }

    pub fn help_text(&self) -> String {
        let mut parts = vec!["ESC: cancel"];

        if self.is_action_enabled(&CardListActionType::Navigation) {
            parts.push("j/k: navigate");
        }

        if self.is_action_enabled(&CardListActionType::Selection) {
            parts.push("Enter/Space: select");
        }

        if self.is_action_enabled(&CardListActionType::Editing) {
            parts.push("e: edit");
        }

        if self.is_action_enabled(&CardListActionType::Completion) {
            parts.push("c: complete");
        }

        if self.is_action_enabled(&CardListActionType::Priority) {
            parts.push("p: priority");
        }

        if self.is_action_enabled(&CardListActionType::Sprint) {
            parts.push("s: assign sprint");
        }

        if self.is_action_enabled(&CardListActionType::Sorting) {
            parts.push("o: sort");
        }

        if self.is_action_enabled(&CardListActionType::Movement) {
            parts.push("H/L: move");
        }

        if self.is_action_enabled(&CardListActionType::Creation) {
            parts.push("n: new");
        }

        if self.allow_multi_select {
            parts.push("v: select card | V: multi-select");
        }

        parts.join(" | ")
    }
}

pub struct CardListComponent {
    pub card_list: CardList,
    pub config: CardListComponentConfig,
    pub multi_selected: std::collections::HashSet<Uuid>,
}

impl CardListComponent {
    pub fn new(list_id: CardListId, config: CardListComponentConfig) -> Self {
        Self {
            card_list: CardList::new(list_id),
            config,
            multi_selected: std::collections::HashSet::new(),
        }
    }

    pub fn with_config(list_id: CardListId, config: CardListComponentConfig) -> Self {
        Self::new(list_id, config)
    }

    pub fn update_cards(&mut self, cards: Vec<Uuid>) {
        self.card_list.update_cards(cards);
    }

    pub fn get_selected_card_id(&self) -> Option<Uuid> {
        self.card_list.get_selected_card_id()
    }

    pub fn get_multi_selected(&self) -> Vec<Uuid> {
        self.multi_selected.iter().copied().collect()
    }

    pub fn toggle_multi_select(&mut self, card_id: Uuid) {
        if self.config.allow_multi_select {
            if self.multi_selected.contains(&card_id) {
                self.multi_selected.remove(&card_id);
            } else {
                self.multi_selected.insert(card_id);
            }
        }
    }

    pub fn clear_multi_select(&mut self) {
        self.multi_selected.clear();
    }

    pub fn select_all(&mut self) {
        if self.config.allow_multi_select {
            for card_id in &self.card_list.cards {
                self.multi_selected.insert(*card_id);
            }
        }
    }

    pub fn navigate_up(&mut self) -> bool {
        self.card_list.navigate_up()
    }

    pub fn navigate_down(&mut self) -> bool {
        self.card_list.navigate_down()
    }

    pub fn is_empty(&self) -> bool {
        self.card_list.is_empty()
    }

    pub fn len(&self) -> usize {
        self.card_list.len()
    }

    pub fn get_selected_index(&self) -> Option<usize> {
        self.card_list.get_selected_index()
    }

    pub fn set_selected_index(&mut self, index: Option<usize>) {
        self.card_list.set_selected_index(index);
    }

    pub fn help_text(&self) -> String {
        self.config.help_text()
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<CardListAction> {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.config.is_action_enabled(&CardListActionType::Navigation) {
                    self.navigate_down();
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.config.is_action_enabled(&CardListActionType::Navigation) {
                    self.navigate_up();
                }
                None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if self.config.is_action_enabled(&CardListActionType::Selection) {
                    self.get_selected_card_id().map(CardListAction::Select)
                } else {
                    None
                }
            }
            KeyCode::Char('e') => {
                if self.config.is_action_enabled(&CardListActionType::Editing) {
                    self.get_selected_card_id().map(CardListAction::Edit)
                } else {
                    None
                }
            }
            KeyCode::Char('c') => {
                if self.config.is_action_enabled(&CardListActionType::Completion) {
                    self.get_selected_card_id().map(CardListAction::Complete)
                } else {
                    None
                }
            }
            KeyCode::Char('p') => {
                if self.config.is_action_enabled(&CardListActionType::Priority) {
                    self.get_selected_card_id().map(CardListAction::TogglePriority)
                } else {
                    None
                }
            }
            KeyCode::Char('s') => {
                if self.config.is_action_enabled(&CardListActionType::Sprint) {
                    self.get_selected_card_id().map(CardListAction::AssignSprint)
                } else {
                    None
                }
            }
            KeyCode::Char('S') => {
                if self.config.is_action_enabled(&CardListActionType::Sprint) {
                    self.get_selected_card_id().map(CardListAction::ReassignSprint)
                } else {
                    None
                }
            }
            KeyCode::Char('o') => {
                if self.config.is_action_enabled(&CardListActionType::Sorting) {
                    Some(CardListAction::Sort)
                } else {
                    None
                }
            }
            KeyCode::Char('O') => {
                if self.config.is_action_enabled(&CardListActionType::Sorting) {
                    Some(CardListAction::OrderCards)
                } else {
                    None
                }
            }
            KeyCode::Char('H') => {
                if self.config.is_action_enabled(&CardListActionType::Movement) && self.config.allow_movement {
                    self.get_selected_card_id().map(|id| CardListAction::MoveColumn(id, false))
                } else {
                    None
                }
            }
            KeyCode::Char('L') => {
                if self.config.is_action_enabled(&CardListActionType::Movement) && self.config.allow_movement {
                    self.get_selected_card_id().map(|id| CardListAction::MoveColumn(id, true))
                } else {
                    None
                }
            }
            KeyCode::Char('n') => {
                if self.config.is_action_enabled(&CardListActionType::Creation) {
                    Some(CardListAction::Create)
                } else {
                    None
                }
            }
            KeyCode::Char('v') => {
                if self.config.is_action_enabled(&CardListActionType::MultiSelect) {
                    self.get_selected_card_id().map(CardListAction::ToggleMultiSelect)
                } else {
                    None
                }
            }
            KeyCode::Char('V') => {
                if self.config.is_action_enabled(&CardListActionType::MultiSelect) && self.config.allow_multi_select {
                    Some(CardListAction::SelectAll)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_component() -> CardListComponent {
        let mut component = CardListComponent::new(CardListId::All, CardListComponentConfig::new());
        // Manually select first card position for testing
        component.card_list.selection.set(Some(0));
        component
    }

    fn create_test_component_with_config(config: CardListComponentConfig) -> CardListComponent {
        let mut component = CardListComponent::new(CardListId::All, config);
        // Manually select first card position for testing
        component.card_list.selection.set(Some(0));
        component
    }

    // Configuration Tests
    #[test]
    fn test_default_config_has_all_actions_enabled() {
        let config = CardListComponentConfig::default();
        assert_eq!(config.enabled_actions.len(), 10);
        assert!(config.is_action_enabled(&CardListActionType::Navigation));
        assert!(config.is_action_enabled(&CardListActionType::Selection));
        assert!(config.is_action_enabled(&CardListActionType::Editing));
        assert!(config.is_action_enabled(&CardListActionType::Completion));
        assert!(config.is_action_enabled(&CardListActionType::Priority));
        assert!(config.is_action_enabled(&CardListActionType::Sprint));
        assert!(config.is_action_enabled(&CardListActionType::Sorting));
        assert!(config.is_action_enabled(&CardListActionType::Movement));
        assert!(config.is_action_enabled(&CardListActionType::Creation));
        assert!(config.is_action_enabled(&CardListActionType::MultiSelect));
    }

    #[test]
    fn test_config_builder_with_actions() {
        let config = CardListComponentConfig::new()
            .with_actions(vec![CardListActionType::Navigation, CardListActionType::Selection]);
        assert_eq!(config.enabled_actions.len(), 2);
        assert!(config.is_action_enabled(&CardListActionType::Navigation));
        assert!(config.is_action_enabled(&CardListActionType::Selection));
        assert!(!config.is_action_enabled(&CardListActionType::Editing));
    }

    #[test]
    fn test_config_builder_with_multi_select() {
        let config = CardListComponentConfig::new().with_multi_select(false);
        assert!(!config.allow_multi_select);
    }

    #[test]
    fn test_config_builder_with_movement() {
        let config = CardListComponentConfig::new().with_movement(false);
        assert!(!config.allow_movement);
    }

    #[test]
    fn test_config_help_text_all_actions() {
        let config = CardListComponentConfig::default();
        let help = config.help_text();
        assert!(help.contains("j/k: navigate"));
        assert!(help.contains("Enter/Space: select"));
        assert!(help.contains("e: edit"));
        assert!(help.contains("c: complete"));
        assert!(help.contains("p: priority"));
        assert!(help.contains("s: assign sprint"));
        assert!(help.contains("o: sort"));
        assert!(help.contains("H/L: move"));
        assert!(help.contains("n: new"));
        assert!(help.contains("v: select card"));
    }

    #[test]
    fn test_config_help_text_limited_actions() {
        let config = CardListComponentConfig::new()
            .with_actions(vec![CardListActionType::Navigation, CardListActionType::Selection]);
        let help = config.help_text();
        assert!(help.contains("j/k: navigate"));
        assert!(help.contains("Enter/Space: select"));
        assert!(!help.contains("e: edit"));
        assert!(!help.contains("c: complete"));
    }

    // Navigation Tests
    #[test]
    fn test_navigation_with_cards() {
        let mut component = create_test_component();
        let card1 = Uuid::new_v4();
        let card2 = Uuid::new_v4();
        let card3 = Uuid::new_v4();

        component.update_cards(vec![card1, card2, card3]);

        assert_eq!(component.get_selected_index(), Some(0));

        component.handle_key(KeyCode::Char('j'));
        assert_eq!(component.get_selected_index(), Some(1));

        component.handle_key(KeyCode::Char('j'));
        assert_eq!(component.get_selected_index(), Some(2));

        component.handle_key(KeyCode::Char('k'));
        assert_eq!(component.get_selected_index(), Some(1));
    }

    #[test]
    fn test_navigation_down_key() {
        let mut component = create_test_component();
        let card1 = Uuid::new_v4();
        let card2 = Uuid::new_v4();

        component.update_cards(vec![card1, card2]);
        assert_eq!(component.get_selected_index(), Some(0));

        component.handle_key(KeyCode::Down);
        assert_eq!(component.get_selected_index(), Some(1));
    }

    #[test]
    fn test_navigation_up_key() {
        let mut component = create_test_component();
        let card1 = Uuid::new_v4();
        let card2 = Uuid::new_v4();

        component.update_cards(vec![card1, card2]);
        component.handle_key(KeyCode::Down);
        assert_eq!(component.get_selected_index(), Some(1));

        component.handle_key(KeyCode::Up);
        assert_eq!(component.get_selected_index(), Some(0));
    }

    #[test]
    fn test_navigation_disabled() {
        let config = CardListComponentConfig::new()
            .with_actions(vec![CardListActionType::Selection]);
        let mut component = create_test_component_with_config(config);
        let card1 = Uuid::new_v4();
        let card2 = Uuid::new_v4();

        component.update_cards(vec![card1, card2]);
        assert_eq!(component.get_selected_index(), Some(0));

        component.handle_key(KeyCode::Char('j'));
        assert_eq!(component.get_selected_index(), Some(0)); // Should not move
    }

    // Selection Action Tests
    #[test]
    fn test_select_action_with_enter() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Enter);
        assert_eq!(action, Some(CardListAction::Select(card_id)));
    }

    #[test]
    fn test_select_action_with_space() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char(' '));
        assert_eq!(action, Some(CardListAction::Select(card_id)));
    }

    #[test]
    fn test_select_action_disabled() {
        let config = CardListComponentConfig::new()
            .with_actions(vec![CardListActionType::Navigation]);
        let mut component = create_test_component_with_config(config);
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Enter);
        assert_eq!(action, None);
    }

    // Action Tests
    #[test]
    fn test_edit_action() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char('e'));
        assert_eq!(action, Some(CardListAction::Edit(card_id)));
    }

    #[test]
    fn test_edit_action_disabled() {
        let config = CardListComponentConfig::new()
            .with_actions(vec![CardListActionType::Navigation]);
        let mut component = create_test_component_with_config(config);
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char('e'));
        assert_eq!(action, None);
    }

    #[test]
    fn test_complete_action() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char('c'));
        assert_eq!(action, Some(CardListAction::Complete(card_id)));
    }

    #[test]
    fn test_priority_action() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char('p'));
        assert_eq!(action, Some(CardListAction::TogglePriority(card_id)));
    }

    #[test]
    fn test_assign_sprint_action() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char('s'));
        assert_eq!(action, Some(CardListAction::AssignSprint(card_id)));
    }

    #[test]
    fn test_reassign_sprint_action() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char('S'));
        assert_eq!(action, Some(CardListAction::ReassignSprint(card_id)));
    }

    #[test]
    fn test_sort_action() {
        let mut component = create_test_component();

        let action = component.handle_key(KeyCode::Char('o'));
        assert_eq!(action, Some(CardListAction::Sort));
    }

    #[test]
    fn test_order_cards_action() {
        let mut component = create_test_component();

        let action = component.handle_key(KeyCode::Char('O'));
        assert_eq!(action, Some(CardListAction::OrderCards));
    }

    #[test]
    fn test_sort_action_disabled() {
        let config = CardListComponentConfig::new()
            .with_actions(vec![CardListActionType::Navigation]);
        let mut component = create_test_component_with_config(config);

        let action = component.handle_key(KeyCode::Char('o'));
        assert_eq!(action, None);
    }

    #[test]
    fn test_move_column_left() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char('H'));
        assert_eq!(action, Some(CardListAction::MoveColumn(card_id, false)));
    }

    #[test]
    fn test_move_column_right() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char('L'));
        assert_eq!(action, Some(CardListAction::MoveColumn(card_id, true)));
    }

    #[test]
    fn test_move_column_disabled() {
        let config = CardListComponentConfig::new()
            .with_movement(false);
        let mut component = create_test_component_with_config(config);
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char('H'));
        assert_eq!(action, None);
    }

    #[test]
    fn test_create_action() {
        let mut component = create_test_component();

        let action = component.handle_key(KeyCode::Char('n'));
        assert_eq!(action, Some(CardListAction::Create));
    }

    #[test]
    fn test_create_action_disabled() {
        let config = CardListComponentConfig::new()
            .with_actions(vec![CardListActionType::Navigation]);
        let mut component = create_test_component_with_config(config);

        let action = component.handle_key(KeyCode::Char('n'));
        assert_eq!(action, None);
    }

    // Multi-Select Tests
    #[test]
    fn test_toggle_multi_select() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        component.toggle_multi_select(card_id);
        assert!(component.multi_selected.contains(&card_id));

        component.toggle_multi_select(card_id);
        assert!(!component.multi_selected.contains(&card_id));
    }

    #[test]
    fn test_toggle_multi_select_disabled() {
        let config = CardListComponentConfig::new().with_multi_select(false);
        let mut component = create_test_component_with_config(config);
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        component.toggle_multi_select(card_id);
        assert!(!component.multi_selected.contains(&card_id));
    }

    #[test]
    fn test_toggle_multi_select_action() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char('v'));
        assert_eq!(action, Some(CardListAction::ToggleMultiSelect(card_id)));
    }

    #[test]
    fn test_select_all_action() {
        let mut component = create_test_component();
        let card_id = Uuid::new_v4();
        component.update_cards(vec![card_id]);

        let action = component.handle_key(KeyCode::Char('V'));
        assert_eq!(action, Some(CardListAction::SelectAll));
    }

    #[test]
    fn test_select_all() {
        let mut component = create_test_component();
        let card1 = Uuid::new_v4();
        let card2 = Uuid::new_v4();
        let card3 = Uuid::new_v4();

        component.update_cards(vec![card1, card2, card3]);
        component.select_all();

        assert_eq!(component.multi_selected.len(), 3);
        assert!(component.multi_selected.contains(&card1));
        assert!(component.multi_selected.contains(&card2));
        assert!(component.multi_selected.contains(&card3));
    }

    #[test]
    fn test_select_all_disabled() {
        let config = CardListComponentConfig::new().with_multi_select(false);
        let mut component = create_test_component_with_config(config);
        let card1 = Uuid::new_v4();
        let card2 = Uuid::new_v4();

        component.update_cards(vec![card1, card2]);
        component.select_all();

        assert_eq!(component.multi_selected.len(), 0);
    }

    #[test]
    fn test_clear_multi_select() {
        let mut component = create_test_component();
        let card1 = Uuid::new_v4();
        let card2 = Uuid::new_v4();

        component.update_cards(vec![card1, card2]);
        component.select_all();
        assert_eq!(component.multi_selected.len(), 2);

        component.clear_multi_select();
        assert_eq!(component.multi_selected.len(), 0);
    }

    #[test]
    fn test_get_multi_selected() {
        let mut component = create_test_component();
        let card1 = Uuid::new_v4();
        let card2 = Uuid::new_v4();
        let card3 = Uuid::new_v4();

        component.update_cards(vec![card1, card2, card3]);
        component.toggle_multi_select(card1);
        component.toggle_multi_select(card3);

        let selected = component.get_multi_selected();
        assert_eq!(selected.len(), 2);
        assert!(selected.contains(&card1));
        assert!(selected.contains(&card3));
        assert!(!selected.contains(&card2));
    }

    // Misc Tests
    #[test]
    fn test_unknown_key() {
        let mut component = create_test_component();

        let action = component.handle_key(KeyCode::F(1));
        assert_eq!(action, None);
    }

    #[test]
    fn test_component_is_empty() {
        let component = create_test_component();
        assert!(component.is_empty());
    }

    #[test]
    fn test_component_length() {
        let mut component = create_test_component();
        let cards = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        component.update_cards(cards.clone());

        assert_eq!(component.len(), 3);
        assert!(!component.is_empty());
    }

    #[test]
    fn test_set_selected_index() {
        let mut component = create_test_component();
        let cards = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        component.update_cards(cards);

        component.set_selected_index(Some(2));
        assert_eq!(component.get_selected_index(), Some(2));

        component.set_selected_index(None);
        assert_eq!(component.get_selected_index(), None);
    }

    #[test]
    fn test_get_selected_card_id() {
        let mut component = create_test_component();
        let card1 = Uuid::new_v4();
        let card2 = Uuid::new_v4();

        component.update_cards(vec![card1, card2]);
        assert_eq!(component.get_selected_card_id(), Some(card1));

        component.handle_key(KeyCode::Char('j'));
        assert_eq!(component.get_selected_card_id(), Some(card2));
    }

    #[test]
    fn test_help_text() {
        let component = create_test_component();
        let help = component.help_text();
        assert!(help.contains("ESC: cancel"));
        assert!(help.contains("j/k: navigate"));
    }
}
