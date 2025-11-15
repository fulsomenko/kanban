use crate::card_list::{CardList, CardListId};
use crate::view_strategy::ViewRefreshContext;
use crate::services::filter_and_sort_cards_by_column;

pub trait LayoutStrategy {
    fn get_active_task_list(&self) -> Option<&CardList>;
    fn get_active_task_list_mut(&mut self) -> Option<&mut CardList>;
    fn get_all_task_lists(&self) -> Vec<&CardList>;
    fn navigate_left(&mut self, select_last: bool) -> bool;
    fn navigate_right(&mut self, select_last: bool) -> bool;
    fn refresh_lists(&mut self, ctx: &ViewRefreshContext);
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn as_any(&self) -> &dyn std::any::Any;
}

pub struct SingleListLayout {
    task_list: CardList,
}

impl SingleListLayout {
    pub fn new() -> Self {
        Self {
            task_list: CardList::new(CardListId::All),
        }
    }
}

impl Default for SingleListLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutStrategy for SingleListLayout {
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

    fn refresh_lists(&mut self, ctx: &ViewRefreshContext) {
        use crate::services::filter_and_sort_cards;

        let card_ids = filter_and_sort_cards(ctx);
        self.task_list.update_cards(card_ids);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ColumnListsLayout {
    column_lists: Vec<CardList>,
    active_column_index: usize,
}

impl ColumnListsLayout {
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

impl Default for ColumnListsLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutStrategy for ColumnListsLayout {
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

    fn refresh_lists(&mut self, ctx: &ViewRefreshContext) {
        let mut board_columns: Vec<_> = ctx
            .all_columns
            .iter()
            .filter(|col| col.board_id == ctx.board.id)
            .collect();
        board_columns.sort_by_key(|col| col.position);

        let mut new_column_lists = Vec::new();

        for column in board_columns.iter() {
            let card_ids = filter_and_sort_cards_by_column(ctx, column.id);

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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
