use crate::card_list::{CardList, CardListId};
use crate::card_list_component::{CardListComponent, CardListComponentConfig};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SprintTaskPanel {
    Uncompleted,
    Completed,
}

pub struct SprintViewState {
    pub panel: SprintTaskPanel,
    pub uncompleted_cards: CardList,
    pub completed_cards: CardList,
    pub uncompleted_component: CardListComponent,
    pub completed_component: CardListComponent,
}

impl Default for SprintViewState {
    fn default() -> Self {
        // Both panels share the same full action set as the main-board card list
        // (KAN-435). Panel-switching keybindings (h/l) are intercepted by the
        // sprint-detail key handler before they reach the component.
        Self {
            panel: SprintTaskPanel::Uncompleted,
            uncompleted_cards: CardList::new(CardListId::All),
            completed_cards: CardList::new(CardListId::All),
            uncompleted_component: CardListComponent::new(
                CardListId::All,
                CardListComponentConfig::default(),
            ),
            completed_component: CardListComponent::new(
                CardListId::All,
                CardListComponentConfig::default(),
            ),
        }
    }
}
