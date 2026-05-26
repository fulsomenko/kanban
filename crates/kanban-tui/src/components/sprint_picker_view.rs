use crate::components::radio_list::{ListItem, RadioList};
use crate::components::sprint_assign_list::{
    build_entries, render_entry_line, sprint_id_of, SprintAssignEntry,
};
use chrono::{DateTime, Utc};
use kanban_domain::{Board, Sprint, SprintStatus};
use ratatui::{layout::Rect, Frame};
use uuid::Uuid;

pub struct SprintPickerView<'a> {
    entries: Vec<SprintAssignEntry<'a>>,
    board: &'a Board,
    current_sprint_id: Option<Uuid>,
    initial: Option<usize>,
}

impl<'a> SprintPickerView<'a> {
    pub fn for_card_assignment(
        sprints: &'a [Sprint],
        board: &'a Board,
        current_sprint_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Self {
        let entries = build_entries(sprints, board.id, now);
        let initial = match current_sprint_id {
            Some(id) => entries
                .iter()
                .position(|e| sprint_id_of(e) == Some(id))
                .or(Some(0)),
            None => Some(0),
        };
        Self {
            entries,
            board,
            current_sprint_id,
            initial,
        }
    }

    pub fn for_board(sprints: &'a [Sprint], board: &'a Board, now: DateTime<Utc>) -> Self {
        let entries = build_entries(sprints, board.id, now);
        let active_non_ended: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| match entry {
                SprintAssignEntry::ActiveOrPlanned(s)
                    if s.status == SprintStatus::Active && !s.is_ended(now) =>
                {
                    Some(idx)
                }
                _ => None,
            })
            .collect();
        let initial = if active_non_ended.len() == 1 {
            Some(active_non_ended[0])
        } else {
            Some(0)
        };
        Self {
            entries,
            board,
            current_sprint_id: None,
            initial,
        }
    }

    pub fn initial_selection(&self) -> Option<usize> {
        self.initial
    }

    pub fn value_at(&self, idx: usize) -> Option<Option<Uuid>> {
        match self.entries.get(idx)? {
            SprintAssignEntry::Header(_) => None,
            SprintAssignEntry::None => Some(None),
            SprintAssignEntry::ActiveOrPlanned(s)
            | SprintAssignEntry::Completed(s)
            | SprintAssignEntry::Ended(s) => Some(Some(s.id)),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, selected: Option<usize>) {
        let items: Vec<ListItem<Option<Uuid>>> = self
            .entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let is_selected = selected == Some(idx);
                let label =
                    render_entry_line(entry, is_selected, self.current_sprint_id, self.board);
                ListItem {
                    value: sprint_id_of(entry),
                    label,
                    selectable: entry.is_selectable(),
                }
            })
            .collect();

        let list = RadioList::new(&items).with_sticky_header(|items, sel_idx| {
            let upper = sel_idx.min(items.len());
            for i in (0..upper).rev() {
                if !items[i].selectable {
                    return Some((i, items[i].label.clone()));
                }
            }
            None
        });
        list.render(frame, area, selected);
    }
}
