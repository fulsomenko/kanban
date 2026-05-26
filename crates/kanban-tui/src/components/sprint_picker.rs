use crate::components::sprint_assign_list::{
    build_entries, next_selectable, prev_selectable, sprint_id_of, SprintAssignEntry,
};
use crate::components::sprint_picker_view::SprintPickerView;
use chrono::{DateTime, Utc};
use crossterm::event::KeyCode;
use kanban_core::SelectionState;
use kanban_domain::{Board, Sprint};
use ratatui::{layout::Rect, Frame};
use uuid::Uuid;

/// Stateful sprint picker for dialogs that embed a sprint selector
/// alongside other inputs (e.g. the Create Card dialog). Owns the
/// selection, applies the "pre-select sole active sprint" rule on open,
/// translates navigation keys into selection moves, and exposes the
/// resolved sprint id for the host to apply.
///
/// Composition: `SprintPicker` -> `SprintPickerView` (stateless adapter
/// that turns sprints into ListItem rows) -> `RadioList<Option<Uuid>>`
/// (generic radio-list primitive). The stateful layer adds selection
/// state and key handling on top of the existing stateless presenter.
#[derive(Default)]
pub struct SprintPicker {
    selection: SelectionState,
}

impl SprintPicker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset selection for a fresh dialog opening. If the board has exactly
    /// one Active (non-ended) sprint at `now`, that sprint becomes the
    /// initial selection; otherwise the "None" entry at index 0.
    pub fn reset_for_board(&mut self, sprints: &[Sprint], board: &Board, now: DateTime<Utc>) {
        let picker = SprintPickerView::for_board(sprints, board, now);
        self.selection.set(picker.initial_selection());
    }

    pub fn clear(&mut self) {
        self.selection.clear();
    }

    /// Try to consume a navigation key. Returns true when the key moved the
    /// selection (Up/Down/j/k); false when the key should fall through to
    /// other handlers (typing into a text input, etc.).
    pub fn handle_key(
        &mut self,
        key_code: KeyCode,
        sprints: &[Sprint],
        board: &Board,
        now: DateTime<Utc>,
    ) -> bool {
        let entries = build_entries(sprints, board.id, now);
        let cur = self.selection.get();
        match key_code {
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(next) = next_selectable(&entries, cur) {
                    self.selection.set(Some(next));
                }
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(prev) = prev_selectable(&entries, cur) {
                    self.selection.set(Some(prev));
                }
                true
            }
            _ => false,
        }
    }

    /// Resolve the currently-selected sprint id, or None if the "None"
    /// entry is selected or the picker is empty.
    pub fn selected_sprint_id(
        &self,
        sprints: &[Sprint],
        board: &Board,
        now: DateTime<Utc>,
    ) -> Option<Uuid> {
        let entries = build_entries(sprints, board.id, now);
        let idx = self.selection.get()?;
        let entry = entries.get(idx)?;
        match entry {
            SprintAssignEntry::None | SprintAssignEntry::Header(_) => None,
            _ => sprint_id_of(entry),
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        sprints: &[Sprint],
        board: &Board,
        now: DateTime<Utc>,
    ) {
        let picker = SprintPickerView::for_card_assignment(sprints, board, None, now);
        picker.render(frame, area, self.selection.get());
    }

    #[cfg(test)]
    pub(crate) fn selection_index(&self) -> Option<usize> {
        self.selection.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use kanban_domain::SprintStatus;

    fn make_board() -> Board {
        Board::new("B".to_string(), Some("TST".to_string()))
    }

    fn active_sprint(board_id: Uuid, n: u32, now: DateTime<Utc>) -> Sprint {
        Sprint {
            id: Uuid::new_v4(),
            board_id,
            sprint_number: n,
            name_index: None,
            prefix: None,
            card_prefix: None,
            status: SprintStatus::Active,
            start_date: Some(now - Duration::days(1)),
            end_date: Some(now + Duration::days(7)),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_reset_with_sole_active_sprint_pre_selects_that_sprint() {
        let now = Utc::now();
        let board = make_board();
        let sprint = active_sprint(board.id, 1, now);
        let sprints = vec![sprint.clone()];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);

        assert_eq!(picker.selected_sprint_id(&sprints, &board, now), Some(sprint.id));
    }

    #[test]
    fn test_reset_with_no_active_sprint_selects_none_entry() {
        let now = Utc::now();
        let board = make_board();
        let sprints: Vec<Sprint> = vec![];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);

        assert_eq!(picker.selected_sprint_id(&sprints, &board, now), None);
    }

    #[test]
    fn test_reset_with_multiple_active_sprints_falls_back_to_none() {
        let now = Utc::now();
        let board = make_board();
        let s1 = active_sprint(board.id, 1, now);
        let s2 = active_sprint(board.id, 2, now);
        let sprints = vec![s1, s2];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);

        assert_eq!(
            picker.selected_sprint_id(&sprints, &board, now),
            None,
            "ambiguous active sprints should fall back to the None entry"
        );
    }

    #[test]
    fn test_handle_key_down_advances_selection_and_returns_true() {
        let now = Utc::now();
        let board = make_board();
        // No active sprint, just two planning sprints, so the picker opens on
        // the "None" entry at index 0 and Down can advance to a sprint row.
        let planning_a = Sprint::new(board.id, 1, None, None);
        let planning_b = Sprint::new(board.id, 2, None, None);
        let sprints = vec![planning_a, planning_b];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        let before = picker.selection_index();

        let consumed = picker.handle_key(KeyCode::Down, &sprints, &board, now);
        let after = picker.selection_index();

        assert!(consumed, "Down should be consumed by the picker");
        assert_ne!(before, after, "Down should move selection when more entries exist");
    }

    #[test]
    fn test_handle_key_other_returns_false_so_host_can_handle_it() {
        let now = Utc::now();
        let board = make_board();
        let sprints: Vec<Sprint> = vec![];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);

        assert!(!picker.handle_key(KeyCode::Char('a'), &sprints, &board, now));
        assert!(!picker.handle_key(KeyCode::Enter, &sprints, &board, now));
        assert!(!picker.handle_key(KeyCode::Backspace, &sprints, &board, now));
    }

    #[test]
    fn test_clear_resets_selection_to_none() {
        let now = Utc::now();
        let board = make_board();
        let sprint = active_sprint(board.id, 1, now);
        let sprints = vec![sprint];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        assert!(picker.selection_index().is_some());

        picker.clear();
        assert_eq!(picker.selection_index(), None);
    }
}
