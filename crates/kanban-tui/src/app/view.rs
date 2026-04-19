use crate::card_list::CardListId;
use crate::card_list_component::{CardListComponent, CardListComponentConfig};
use crate::view_strategy::{UnifiedViewStrategy, ViewStrategy};
use kanban_domain::{Board, Card, Column, DependencyGraph, Sprint};
use ratatui::layout::Rect;
use std::collections::HashMap;
use uuid::Uuid;

pub struct ViewState {
    pub strategy: Box<dyn ViewStrategy>,
    pub card_list_component: CardListComponent,
    pub viewport_height: usize,
    pub last_frame_area: Rect,
    pub boards: Vec<Board>,
    pub sprints: Vec<Sprint>,
    pub columns: Vec<Column>,
    pub cards_by_id: HashMap<Uuid, Card>,
    pub graph: DependencyGraph,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            strategy: Box::new(UnifiedViewStrategy::grouped()),
            card_list_component: CardListComponent::new(
                CardListId::All,
                CardListComponentConfig::new(),
            ),
            viewport_height: 20,
            last_frame_area: Rect::default(),
            boards: Vec::new(),
            sprints: Vec::new(),
            columns: Vec::new(),
            cards_by_id: HashMap::new(),
            graph: DependencyGraph::new(),
        }
    }
}
