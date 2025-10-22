use crate::card_list::{CardList, CardListId};
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
}
