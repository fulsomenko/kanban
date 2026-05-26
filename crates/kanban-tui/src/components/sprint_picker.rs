use crate::components::radio_list::{ListItem, RadioList};
use crate::components::sprint_assign_list::SprintAssignEntry;
use chrono::{DateTime, Utc};
use kanban_domain::{Board, Sprint};
use ratatui::{layout::Rect, Frame};
use uuid::Uuid;

#[allow(dead_code)]
pub struct SprintPicker<'a> {
    entries: Vec<SprintAssignEntry<'a>>,
    items: Vec<ListItem<Option<Uuid>>>,
    board: &'a Board,
    current_sprint_id: Option<Uuid>,
    initial: Option<usize>,
}

impl<'a> SprintPicker<'a> {
    pub fn for_card_assignment(
        _sprints: &'a [Sprint],
        _board: &'a Board,
        _current_sprint_id: Option<Uuid>,
        _now: DateTime<Utc>,
    ) -> Self {
        todo!()
    }

    pub fn for_board(_sprints: &'a [Sprint], _board: &'a Board, _now: DateTime<Utc>) -> Self {
        todo!()
    }

    pub fn initial_selection(&self) -> Option<usize> {
        todo!()
    }

    pub fn value_at(&self, _idx: usize) -> Option<Option<Uuid>> {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn render(&self, _frame: &mut Frame, _area: Rect, _selected: Option<usize>) {
        todo!()
    }
}

// Suppress unused-import warnings for the stub phase; the impl uses these.
#[allow(dead_code)]
fn _silence_unused(_: RadioList<'static, Option<Uuid>>) {}
