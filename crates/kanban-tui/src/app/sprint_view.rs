use crate::card_list::{CardList, CardListId};
use crate::card_list_component::{CardListActionType, CardListComponent, CardListComponentConfig};

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
        Self {
            panel: SprintTaskPanel::Uncompleted,
            uncompleted_cards: CardList::new(CardListId::All),
            completed_cards: CardList::new(CardListId::All),
            uncompleted_component: CardListComponent::new(
                CardListId::All,
                CardListComponentConfig::new()
                    .with_actions(vec![
                        CardListActionType::Navigation,
                        CardListActionType::Selection,
                        CardListActionType::Editing,
                        CardListActionType::Completion,
                        CardListActionType::Priority,
                        CardListActionType::Sorting,
                    ])
                    .with_movement(false),
            ),
            completed_component: CardListComponent::new(
                CardListId::All,
                CardListComponentConfig::new()
                    .with_actions(vec![
                        CardListActionType::Navigation,
                        CardListActionType::Selection,
                        CardListActionType::Sorting,
                    ])
                    .with_multi_select(false),
            ),
        }
    }
}
