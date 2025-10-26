use crate::card_list::{CardList, CardListId};
use crate::search::{CardSearcher, CompositeCardSearcher};
use crate::services::filter::CardFilter;
use crate::services::{get_sorter_for_field, BoardFilter, OrderedSorter};
use kanban_domain::{Board, Card, Column, Sprint, SprintStatus};
use uuid::Uuid;

pub trait ViewStrategy {
    fn get_active_task_list(&self) -> Option<&CardList>;
    fn get_active_task_list_mut(&mut self) -> Option<&mut CardList>;
    fn get_all_task_lists(&self) -> Vec<&CardList>;
    fn navigate_left(&mut self, select_last: bool) -> bool;
    fn navigate_right(&mut self, select_last: bool) -> bool;
    fn refresh_task_lists(
        &mut self,
        board: &Board,
        all_cards: &[Card],
        all_columns: &[Column],
        all_sprints: &[Sprint],
        active_sprint_filter: Option<Uuid>,
        hide_assigned_cards: bool,
        search_query: Option<&str>,
    );
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub struct FlatViewStrategy {
    task_list: CardList,
}

impl Default for FlatViewStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl FlatViewStrategy {
    pub fn new() -> Self {
        Self {
            task_list: CardList::new(CardListId::All),
        }
    }
}

impl ViewStrategy for FlatViewStrategy {
    fn get_active_task_list(&self) -> Option<&CardList> {
        Some(&self.task_list)
    }

    fn get_active_task_list_mut(&mut self) -> Option<&mut CardList> {
        Some(&mut self.task_list)
    }

    fn get_all_task_lists(&self) -> Vec<&CardList> {
        vec![&self.task_list]
    }

    fn navigate_left(&mut self, _select_last: bool) -> bool {
        false
    }

    fn navigate_right(&mut self, _select_last: bool) -> bool {
        false
    }

    fn refresh_task_lists(
        &mut self,
        board: &Board,
        all_cards: &[Card],
        all_columns: &[Column],
        all_sprints: &[Sprint],
        active_sprint_filter: Option<Uuid>,
        hide_assigned_cards: bool,
        search_query: Option<&str>,
    ) {
        let board_filter = BoardFilter::new(board.id, all_columns);

        let completed_sprint_ids: std::collections::HashSet<Uuid> = all_sprints
            .iter()
            .filter(|s| s.status == SprintStatus::Completed)
            .map(|s| s.id)
            .collect();

        let search_filter = search_query.map(|q| CompositeCardSearcher::new(q.to_string()));

        let mut filtered_cards: Vec<&Card> = all_cards
            .iter()
            .filter(|c| {
                if !board_filter.matches(c) {
                    return false;
                }
                if let Some(sprint_id) = c.sprint_id {
                    if completed_sprint_ids.contains(&sprint_id) {
                        return false;
                    }
                }
                if let Some(sprint_id) = active_sprint_filter {
                    if c.sprint_id != Some(sprint_id) {
                        return false;
                    }
                }
                if hide_assigned_cards && c.sprint_id.is_some() {
                    return false;
                }
                if let Some(ref searcher) = search_filter {
                    if !searcher.matches(c, board, all_sprints) {
                        return false;
                    }
                }
                true
            })
            .collect();

        let sorter = get_sorter_for_field(board.task_sort_field);
        let ordered_sorter = OrderedSorter::new(sorter, board.task_sort_order);
        ordered_sorter.sort(&mut filtered_cards);

        let card_ids: Vec<Uuid> = filtered_cards.iter().map(|c| c.id).collect();
        self.task_list.update_cards(card_ids);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct GroupedViewStrategy {
    column_lists: Vec<CardList>,
    active_column_index: usize,
}

impl Default for GroupedViewStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl GroupedViewStrategy {
    pub fn new() -> Self {
        Self {
            column_lists: Vec::new(),
            active_column_index: 0,
        }
    }

    pub fn get_active_column_index(&self) -> usize {
        self.active_column_index
    }

    pub fn set_active_column_index(&mut self, index: usize) {
        if index < self.column_lists.len() {
            self.active_column_index = index;
        }
    }
}

impl ViewStrategy for GroupedViewStrategy {
    fn get_active_task_list(&self) -> Option<&CardList> {
        self.column_lists.get(self.active_column_index)
    }

    fn get_active_task_list_mut(&mut self) -> Option<&mut CardList> {
        self.column_lists.get_mut(self.active_column_index)
    }

    fn get_all_task_lists(&self) -> Vec<&CardList> {
        self.column_lists.iter().collect()
    }

    fn navigate_left(&mut self, select_last: bool) -> bool {
        if self.active_column_index > 0 {
            self.active_column_index -= 1;
            if let Some(list) = self.get_active_task_list_mut() {
                if list.is_empty() {
                    list.clear();
                } else if select_last {
                    list.set_selected_index(Some(list.len() - 1));
                } else if list.get_selected_index().is_none() {
                    list.set_selected_index(Some(0));
                }
            }
            true
        } else {
            false
        }
    }

    fn navigate_right(&mut self, select_last: bool) -> bool {
        if self.active_column_index < self.column_lists.len().saturating_sub(1) {
            self.active_column_index += 1;
            if let Some(list) = self.get_active_task_list_mut() {
                if list.is_empty() {
                    list.clear();
                } else if select_last {
                    list.set_selected_index(Some(list.len() - 1));
                } else if list.get_selected_index().is_none() {
                    list.set_selected_index(Some(0));
                }
            }
            true
        } else {
            false
        }
    }

    fn refresh_task_lists(
        &mut self,
        board: &Board,
        all_cards: &[Card],
        all_columns: &[Column],
        all_sprints: &[Sprint],
        active_sprint_filter: Option<Uuid>,
        hide_assigned_cards: bool,
        search_query: Option<&str>,
    ) {
        let mut board_columns: Vec<_> = all_columns
            .iter()
            .filter(|col| col.board_id == board.id)
            .collect();
        board_columns.sort_by_key(|col| col.position);

        let board_filter = BoardFilter::new(board.id, all_columns);

        let completed_sprint_ids: std::collections::HashSet<Uuid> = all_sprints
            .iter()
            .filter(|s| s.status == SprintStatus::Completed)
            .map(|s| s.id)
            .collect();

        let search_filter = search_query.map(|q| CompositeCardSearcher::new(q.to_string()));

        let filtered_cards: Vec<&Card> = all_cards
            .iter()
            .filter(|c| {
                if !board_filter.matches(c) {
                    return false;
                }
                if let Some(sprint_id) = c.sprint_id {
                    if completed_sprint_ids.contains(&sprint_id) {
                        return false;
                    }
                }
                if let Some(sprint_id) = active_sprint_filter {
                    if c.sprint_id != Some(sprint_id) {
                        return false;
                    }
                }
                if hide_assigned_cards && c.sprint_id.is_some() {
                    return false;
                }
                if let Some(ref searcher) = search_filter {
                    if !searcher.matches(c, board, all_sprints) {
                        return false;
                    }
                }
                true
            })
            .collect();

        let sorter = get_sorter_for_field(board.task_sort_field);
        let ordered_sorter = OrderedSorter::new(sorter, board.task_sort_order);

        let mut new_column_lists = Vec::new();

        for column in board_columns.iter() {
            let mut column_cards: Vec<&Card> = filtered_cards
                .iter()
                .copied()
                .filter(|c| c.column_id == column.id)
                .collect();

            ordered_sorter.sort(&mut column_cards);

            let card_ids: Vec<Uuid> = column_cards.iter().map(|c| c.id).collect();

            let existing_list = self
                .column_lists
                .iter()
                .find(|list| list.id == CardListId::Column(column.id));

            let mut task_list = if let Some(existing) = existing_list {
                let mut list = CardList::new(CardListId::Column(column.id));
                list.selection = existing.selection.clone();
                list
            } else {
                CardList::new(CardListId::Column(column.id))
            };

            task_list.update_cards(card_ids);
            new_column_lists.push(task_list);
        }

        self.column_lists = new_column_lists;

        if self.active_column_index >= self.column_lists.len() {
            self.active_column_index = self.column_lists.len().saturating_sub(1);
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct KanbanViewStrategy {
    column_lists: Vec<CardList>,
    active_column_index: usize,
}

impl Default for KanbanViewStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl KanbanViewStrategy {
    pub fn new() -> Self {
        Self {
            column_lists: Vec::new(),
            active_column_index: 0,
        }
    }

    pub fn get_active_column_index(&self) -> usize {
        self.active_column_index
    }

    pub fn set_active_column_index(&mut self, index: usize) {
        if index < self.column_lists.len() {
            self.active_column_index = index;
        }
    }
}

impl ViewStrategy for KanbanViewStrategy {
    fn get_active_task_list(&self) -> Option<&CardList> {
        self.column_lists.get(self.active_column_index)
    }

    fn get_active_task_list_mut(&mut self) -> Option<&mut CardList> {
        self.column_lists.get_mut(self.active_column_index)
    }

    fn get_all_task_lists(&self) -> Vec<&CardList> {
        self.column_lists.iter().collect()
    }

    fn navigate_left(&mut self, select_last: bool) -> bool {
        if self.active_column_index > 0 {
            self.active_column_index -= 1;
            if let Some(list) = self.get_active_task_list_mut() {
                if list.is_empty() {
                    list.clear();
                } else if select_last {
                    list.set_selected_index(Some(list.len() - 1));
                } else if list.get_selected_index().is_none() {
                    list.set_selected_index(Some(0));
                }
            }
            true
        } else {
            false
        }
    }

    fn navigate_right(&mut self, select_last: bool) -> bool {
        if self.active_column_index < self.column_lists.len().saturating_sub(1) {
            self.active_column_index += 1;
            if let Some(list) = self.get_active_task_list_mut() {
                if list.is_empty() {
                    list.clear();
                } else if select_last {
                    list.set_selected_index(Some(list.len() - 1));
                } else if list.get_selected_index().is_none() {
                    list.set_selected_index(Some(0));
                }
            }
            true
        } else {
            false
        }
    }

    fn refresh_task_lists(
        &mut self,
        board: &Board,
        all_cards: &[Card],
        all_columns: &[Column],
        all_sprints: &[Sprint],
        active_sprint_filter: Option<Uuid>,
        hide_assigned_cards: bool,
        search_query: Option<&str>,
    ) {
        let mut board_columns: Vec<_> = all_columns
            .iter()
            .filter(|col| col.board_id == board.id)
            .collect();
        board_columns.sort_by_key(|col| col.position);

        let board_filter = BoardFilter::new(board.id, all_columns);

        let completed_sprint_ids: std::collections::HashSet<Uuid> = all_sprints
            .iter()
            .filter(|s| s.status == SprintStatus::Completed)
            .map(|s| s.id)
            .collect();

        let search_filter = search_query.map(|q| CompositeCardSearcher::new(q.to_string()));

        let filtered_cards: Vec<&Card> = all_cards
            .iter()
            .filter(|c| {
                if !board_filter.matches(c) {
                    return false;
                }
                if let Some(sprint_id) = c.sprint_id {
                    if completed_sprint_ids.contains(&sprint_id) {
                        return false;
                    }
                }
                if let Some(sprint_id) = active_sprint_filter {
                    if c.sprint_id != Some(sprint_id) {
                        return false;
                    }
                }
                if hide_assigned_cards && c.sprint_id.is_some() {
                    return false;
                }
                if let Some(ref searcher) = search_filter {
                    if !searcher.matches(c, board, all_sprints) {
                        return false;
                    }
                }
                true
            })
            .collect();

        let sorter = get_sorter_for_field(board.task_sort_field);
        let ordered_sorter = OrderedSorter::new(sorter, board.task_sort_order);

        let mut new_column_lists = Vec::new();

        for column in board_columns.iter() {
            let mut column_cards: Vec<&Card> = filtered_cards
                .iter()
                .copied()
                .filter(|c| c.column_id == column.id)
                .collect();

            ordered_sorter.sort(&mut column_cards);

            let card_ids: Vec<Uuid> = column_cards.iter().map(|c| c.id).collect();

            let existing_list = self
                .column_lists
                .iter()
                .find(|list| list.id == CardListId::Column(column.id));

            let mut task_list = if let Some(existing) = existing_list {
                let mut list = CardList::new(CardListId::Column(column.id));
                list.selection = existing.selection.clone();
                list
            } else {
                CardList::new(CardListId::Column(column.id))
            };

            task_list.update_cards(card_ids);
            new_column_lists.push(task_list);
        }

        self.column_lists = new_column_lists;

        if self.active_column_index >= self.column_lists.len() {
            self.active_column_index = self.column_lists.len().saturating_sub(1);
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
