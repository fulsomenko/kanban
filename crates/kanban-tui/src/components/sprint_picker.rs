use crate::components::sprint_assign_list::{
    build_entries, next_selectable, prev_selectable, sprint_id_of, SprintAssignEntry,
};
use crate::components::sprint_picker_view::SprintPickerView;
use chrono::{DateTime, Utc};
use crossterm::event::KeyCode;
use kanban_domain::{Board, Sprint};
use ratatui::{layout::Rect, Frame};
use uuid::Uuid;

/// What the picker currently has selected. Stored as an identity, not as
/// an index into the rendered list, so that the selection survives
/// changes to the underlying sprint list between frames (clock crossing
/// a sprint's `end_date`, a background reload, an undo elsewhere, ...).
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// Picker has not been initialised yet (default before `reset_for_board`).
    #[default]
    Unset,
    /// The "(None)" entry is selected — user explicitly picked no sprint.
    NoSprint,
    /// A specific sprint is selected.
    Sprint(Uuid),
}

/// Stateful sprint picker for dialogs that embed a sprint selector
/// alongside other inputs (e.g. the Create Card dialog). Owns the
/// selection, applies the "pre-select sole active sprint" rule on open,
/// translates navigation keys into selection moves, and exposes the
/// resolved sprint id for the host to apply.
///
/// Composition: `SprintPicker` -> `SprintPickerView` (stateless adapter
/// that turns sprints into ListItem rows) -> `RadioList<Option<Uuid>>`
/// (generic radio-list primitive). The stateful layer adds identity-
/// based selection state and key handling on top of the existing
/// stateless presenter.
#[derive(Default)]
pub struct SprintPicker {
    selection: Selection,
}

impl SprintPicker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset selection for a fresh dialog opening. If the board has exactly
    /// one Active (non-ended) sprint at `now`, that sprint becomes the
    /// initial selection; otherwise the "(None)" entry.
    pub fn reset_for_board(&mut self, sprints: &[Sprint], board: &Board, now: DateTime<Utc>) {
        let view = SprintPickerView::for_board(sprints, board, now);
        let entries = build_entries(sprints, board.id, now);
        self.selection = view
            .initial_selection()
            .and_then(|idx| entries.get(idx).map(Self::entry_to_selection))
            .unwrap_or(Selection::NoSprint);
    }

    pub fn clear(&mut self) {
        self.selection = Selection::Unset;
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
        let cur = self.current_index(&entries);
        let next_idx = match key_code {
            KeyCode::Down | KeyCode::Char('j') => next_selectable(&entries, cur),
            KeyCode::Up | KeyCode::Char('k') => prev_selectable(&entries, cur),
            _ => return false,
        };
        if let Some(idx) = next_idx {
            if let Some(entry) = entries.get(idx) {
                self.selection = Self::entry_to_selection(entry);
            }
        }
        true
    }

    /// Resolve the currently-selected sprint id. Returns None when the
    /// "(None)" entry is selected or the picker is uninitialised.
    pub fn selected_sprint_id(&self) -> Option<Uuid> {
        match self.selection {
            Selection::Sprint(id) => Some(id),
            Selection::NoSprint | Selection::Unset => None,
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
        let view = SprintPickerView::for_card_assignment(sprints, board, None, now);
        let entries = build_entries(sprints, board.id, now);
        view.render(frame, area, self.current_index(&entries));
    }

    /// Map the stored identity back to the row index in the current entry
    /// list. Returns `None` when the selection is `Unset` or when the
    /// selected sprint no longer appears in the list (e.g. it was deleted
    /// between frames — the caller should treat that as a lost selection).
    fn current_index(&self, entries: &[SprintAssignEntry]) -> Option<usize> {
        match self.selection {
            Selection::Unset => None,
            Selection::NoSprint => entries
                .iter()
                .position(|e| matches!(e, SprintAssignEntry::None)),
            Selection::Sprint(id) => entries.iter().position(|e| sprint_id_of(e) == Some(id)),
        }
    }

    fn entry_to_selection(entry: &SprintAssignEntry) -> Selection {
        match entry {
            SprintAssignEntry::None => Selection::NoSprint,
            SprintAssignEntry::Header(_) => Selection::Unset,
            SprintAssignEntry::ActiveOrPlanned(s)
            | SprintAssignEntry::Completed(s)
            | SprintAssignEntry::Ended(s) => Selection::Sprint(s.id),
        }
    }

    #[cfg(test)]
    pub(crate) fn raw_selection(&self) -> Selection {
        self.selection
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

        assert_eq!(picker.selected_sprint_id(), Some(sprint.id));
    }

    #[test]
    fn test_reset_with_no_active_sprint_selects_none_entry() {
        let now = Utc::now();
        let board = make_board();
        let sprints: Vec<Sprint> = vec![];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);

        assert_eq!(picker.selected_sprint_id(), None);
        assert_eq!(picker.raw_selection(), Selection::NoSprint);
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
            picker.selected_sprint_id(),
            None,
            "ambiguous active sprints should fall back to the None entry"
        );
        assert_eq!(picker.raw_selection(), Selection::NoSprint);
    }

    #[test]
    fn test_handle_key_down_advances_selection_and_returns_true() {
        let now = Utc::now();
        let board = make_board();
        // No active sprint, just two planning sprints, so the picker opens on
        // the "(None)" entry and Down can advance to a sprint row.
        let planning_a = Sprint::new(board.id, 1, None, None);
        let planning_b = Sprint::new(board.id, 2, None, None);
        let sprints = vec![planning_a, planning_b];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        let before = picker.raw_selection();

        let consumed = picker.handle_key(KeyCode::Down, &sprints, &board, now);
        let after = picker.raw_selection();

        assert!(consumed, "Down should be consumed by the picker");
        assert_ne!(
            before, after,
            "Down should move selection when more entries exist"
        );
        assert!(matches!(after, Selection::Sprint(_)));
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
    fn test_selection_survives_clock_advance_that_ends_the_selected_sprint() {
        // Open time: sprint A is Active non-ended (end_date in the future).
        // The "sole active" rule pre-selects it. Then time advances past
        // A's end_date so A moves from the active-or-planned section into
        // the completed-or-ended section in build_entries. An index-based
        // store would now point at whatever entry occupies A's old slot
        // (a header, or sprint B); the identity-based store still
        // resolves to A.
        let board = make_board();
        let now_open = Utc::now();
        let now_confirm = now_open + Duration::days(30);
        let sprint_a = Sprint {
            id: Uuid::new_v4(),
            board_id: board.id,
            sprint_number: 1,
            name_index: None,
            prefix: None,
            card_prefix: None,
            status: SprintStatus::Active,
            start_date: Some(now_open - Duration::days(1)),
            end_date: Some(now_open + Duration::days(1)),
            created_at: now_open,
            updated_at: now_open,
        };
        let sprint_b = Sprint::new(board.id, 2, None, None);
        let sprints = vec![sprint_a.clone(), sprint_b];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now_open);
        assert_eq!(picker.selected_sprint_id(), Some(sprint_a.id));

        // At now_confirm A has crossed its end_date — entries layout
        // shifts, but the picker still reports A.
        assert!(
            sprint_a.is_ended(now_confirm),
            "test precondition: sprint A is ended at now_confirm"
        );
        assert_eq!(picker.selected_sprint_id(), Some(sprint_a.id));
    }

    #[test]
    fn test_clear_resets_selection_to_unset() {
        let now = Utc::now();
        let board = make_board();
        let sprint = active_sprint(board.id, 1, now);
        let sprints = vec![sprint];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        assert_ne!(picker.raw_selection(), Selection::Unset);

        picker.clear();
        assert_eq!(picker.raw_selection(), Selection::Unset);
    }
}
