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
pub(crate) enum Selection {
    /// Picker has not been initialised yet (default before `reset_for_board`).
    #[default]
    Unset,
    /// The "(None)" entry is selected — user explicitly picked no sprint.
    NoSprint,
    /// A specific sprint is selected.
    Sprint(Uuid),
}

/// What the picker's keyboard cursor is pointing at. Stored as an
/// identity so it survives reflows of the underlying entries list.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum Cursor {
    /// Cursor parked on the "(None)" row at index 0.
    #[default]
    NoSprint,
    /// Cursor on a specific sprint row.
    Sprint(Uuid),
}

/// Stateful sprint picker for dialogs that embed a sprint selector
/// alongside other inputs (e.g. the Create Card dialog). The picker
/// keeps two independent identities:
///
/// - `cursor` is where Up/Down/j/k arrow keys move; it is what the
///   user is currently *pointing at*.
/// - `selection` is what gets the `[x]` checkbox and is what the host
///   reads back via `selected_sprint_id`. Only Space writes to it.
///
/// On open, the cursor is positioned on the sole Active non-ended
/// sprint when there is exactly one (so Space is one keystroke away
/// from assigning it), but selection starts `Unset` — the user must
/// deliberately press Space to commit a choice.
///
/// Composition: `SprintPicker` -> `SprintPickerView` (stateless adapter
/// that turns sprints into ListItem rows) -> `RadioList<Option<Uuid>>`
/// (generic radio-list primitive).
#[derive(Default)]
pub struct SprintPicker {
    selection: Selection,
    cursor: Cursor,
}

impl SprintPicker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset for a fresh dialog opening. The selection is cleared to
    /// `Unset` (no `[x]` shown yet — the user has to press Space to
    /// commit). The cursor lands on the sole Active non-ended sprint
    /// when there is exactly one; otherwise on the "(None)" row.
    pub fn reset_for_board(&mut self, sprints: &[Sprint], board: &Board, now: DateTime<Utc>) {
        self.selection = Selection::Unset;
        let view = SprintPickerView::for_board(sprints, board, now);
        let entries = build_entries(sprints, board.id, now);
        self.cursor = view
            .initial_selection()
            .and_then(|idx| entries.get(idx).map(Self::entry_to_cursor))
            .unwrap_or(Cursor::NoSprint);
    }

    pub fn clear(&mut self) {
        self.selection = Selection::Unset;
        self.cursor = Cursor::NoSprint;
    }

    /// Try to consume a key. Returns true when the picker handled it:
    /// Up/Down/j/k move the cursor; Space commits the cursor's row as
    /// the new selection (placing or moving `[x]`). Other keys fall
    /// through to the host (text input, etc.).
    pub fn handle_key(
        &mut self,
        key_code: KeyCode,
        sprints: &[Sprint],
        board: &Board,
        now: DateTime<Utc>,
    ) -> bool {
        if matches!(key_code, KeyCode::Char(' ')) {
            self.selection = match self.cursor {
                Cursor::NoSprint => Selection::NoSprint,
                Cursor::Sprint(id) => Selection::Sprint(id),
            };
            return true;
        }
        let entries = build_entries(sprints, board.id, now);
        let cur = self.cursor_index(&entries);
        let next_idx = match key_code {
            KeyCode::Down | KeyCode::Char('j') => next_selectable(&entries, cur),
            KeyCode::Up | KeyCode::Char('k') => prev_selectable(&entries, cur),
            _ => return false,
        };
        if let Some(idx) = next_idx {
            if let Some(entry) = entries.get(idx) {
                self.cursor = Self::entry_to_cursor(entry);
            }
        }
        true
    }

    /// Resolve the currently-checked sprint id. Returns None when
    /// nothing is checked yet or the user explicitly checked "(None)".
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
        let checked = match self.selection {
            Selection::Unset => None,
            Selection::NoSprint => view.index_of_sprint(None),
            Selection::Sprint(id) => view.index_of_sprint(Some(id)),
        };
        let cursor = match self.cursor {
            Cursor::NoSprint => view.index_of_sprint(None),
            Cursor::Sprint(id) => view.index_of_sprint(Some(id)),
        };
        view.render_with_cursor(frame, area, checked, cursor);
    }

    fn cursor_index(&self, entries: &[SprintAssignEntry]) -> Option<usize> {
        match self.cursor {
            Cursor::NoSprint => entries
                .iter()
                .position(|e| matches!(e, SprintAssignEntry::None)),
            Cursor::Sprint(id) => entries.iter().position(|e| sprint_id_of(e) == Some(id)),
        }
    }

    fn entry_to_cursor(entry: &SprintAssignEntry) -> Cursor {
        match entry {
            SprintAssignEntry::None => Cursor::NoSprint,
            SprintAssignEntry::Header(_) => Cursor::NoSprint,
            SprintAssignEntry::ActiveOrPlanned(s)
            | SprintAssignEntry::Completed(s)
            | SprintAssignEntry::Ended(s) => Cursor::Sprint(s.id),
        }
    }

    #[cfg(test)]
    pub(crate) fn raw_selection(&self) -> Selection {
        self.selection
    }

    #[cfg(test)]
    pub(crate) fn cursor_sprint_id(&self) -> Option<Uuid> {
        match self.cursor {
            Cursor::NoSprint => None,
            Cursor::Sprint(id) => Some(id),
        }
    }

    /// Used by the same-module race-resistance test that asserts the
    /// internal lookup still finds the checked sprint after reflow.
    #[cfg(test)]
    pub(crate) fn current_index(&self, entries: &[SprintAssignEntry]) -> Option<usize> {
        match self.selection {
            Selection::Unset => None,
            Selection::NoSprint => entries
                .iter()
                .position(|e| matches!(e, SprintAssignEntry::None)),
            Selection::Sprint(id) => entries.iter().position(|e| sprint_id_of(e) == Some(id)),
        }
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
    fn test_reset_with_sole_active_sprint_positions_cursor_but_does_not_check() {
        let now = Utc::now();
        let board = make_board();
        let sprint = active_sprint(board.id, 1, now);
        let sprints = vec![sprint.clone()];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);

        // Nothing is checked yet — the user has to press Space to place [x].
        assert_eq!(picker.selected_sprint_id(), None);
        assert_eq!(picker.raw_selection(), Selection::Unset);
        // But the cursor is parked on the sole active sprint so Space is
        // one keystroke away from assigning it.
        assert_eq!(picker.cursor_sprint_id(), Some(sprint.id));
    }

    #[test]
    fn test_arrow_moves_cursor_without_changing_selection() {
        let now = Utc::now();
        let board = make_board();
        let planning_a = Sprint::new(board.id, 1, None, None);
        let planning_b = Sprint::new(board.id, 2, None, None);
        let sprints = vec![planning_a, planning_b];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        let before_selection = picker.raw_selection();

        picker.handle_key(KeyCode::Down, &sprints, &board, now);

        assert_eq!(
            picker.raw_selection(),
            before_selection,
            "Down must not flip a [x] on; only Space places one"
        );
    }

    #[test]
    fn test_space_at_cursor_places_check_on_cursor_sprint() {
        let now = Utc::now();
        let board = make_board();
        let sprint = active_sprint(board.id, 1, now);
        let sprints = vec![sprint.clone()];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        assert_eq!(picker.selected_sprint_id(), None);

        picker.handle_key(KeyCode::Char(' '), &sprints, &board, now);

        assert_eq!(picker.selected_sprint_id(), Some(sprint.id));
    }

    #[test]
    fn test_space_after_arrow_switches_check_to_new_cursor_sprint() {
        let now = Utc::now();
        let board = make_board();
        let planning_a = Sprint::new(board.id, 1, None, None);
        let planning_b = Sprint::new(board.id, 2, None, None);
        let sprints = vec![planning_a.clone(), planning_b.clone()];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);

        // Cursor starts on (None) since neither sprint is active. Down
        // walks to the first sprint row (planning_b: higher sprint_number
        // is rendered first in the section); Space checks it.
        picker.handle_key(KeyCode::Down, &sprints, &board, now);
        picker.handle_key(KeyCode::Char(' '), &sprints, &board, now);
        let first_checked = picker.selected_sprint_id();
        assert!(first_checked.is_some());

        // Move cursor to the other sprint and press Space — the check
        // switches over instead of staying with the previous row.
        picker.handle_key(KeyCode::Down, &sprints, &board, now);
        picker.handle_key(KeyCode::Char(' '), &sprints, &board, now);
        let second_checked = picker.selected_sprint_id();
        assert!(second_checked.is_some());
        assert_ne!(
            first_checked, second_checked,
            "Space at a new cursor position must move [x] there"
        );
    }

    #[test]
    fn test_space_on_none_row_marks_none_as_explicitly_chosen() {
        let now = Utc::now();
        let board = make_board();
        let sprint = active_sprint(board.id, 1, now);
        let sprints = vec![sprint];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        // Cursor starts on the sole active sprint, walk it up onto (None).
        picker.handle_key(KeyCode::Up, &sprints, &board, now);
        picker.handle_key(KeyCode::Char(' '), &sprints, &board, now);

        assert_eq!(picker.raw_selection(), Selection::NoSprint);
        assert_eq!(picker.selected_sprint_id(), None);
    }

    #[test]
    fn test_reset_with_no_active_sprint_parks_cursor_on_none_row() {
        let now = Utc::now();
        let board = make_board();
        let sprints: Vec<Sprint> = vec![];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);

        assert_eq!(picker.selected_sprint_id(), None);
        assert_eq!(picker.raw_selection(), Selection::Unset);
        assert_eq!(picker.cursor_sprint_id(), None);
    }

    #[test]
    fn test_reset_with_multiple_active_sprints_parks_cursor_on_none_row() {
        let now = Utc::now();
        let board = make_board();
        let s1 = active_sprint(board.id, 1, now);
        let s2 = active_sprint(board.id, 2, now);
        let sprints = vec![s1, s2];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);

        assert_eq!(picker.selected_sprint_id(), None);
        assert_eq!(picker.raw_selection(), Selection::Unset);
        // Multiple active sprints — the picker can't guess which one, so
        // cursor parks on (None) and the user picks deliberately.
        assert_eq!(picker.cursor_sprint_id(), None);
    }

    #[test]
    fn test_handle_key_down_moves_cursor_returns_true() {
        let now = Utc::now();
        let board = make_board();
        let planning_a = Sprint::new(board.id, 1, None, None);
        let planning_b = Sprint::new(board.id, 2, None, None);
        let sprints = vec![planning_a, planning_b];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        let before = picker.cursor_sprint_id();

        let consumed = picker.handle_key(KeyCode::Down, &sprints, &board, now);
        let after = picker.cursor_sprint_id();

        assert!(consumed, "Down should be consumed by the picker");
        assert_ne!(before, after, "Down should advance the cursor");
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
        // Open time: sprint A is Active non-ended. The "sole active" rule
        // pre-selects A at row index `idx_open`. Confirm time: A has
        // crossed its end_date and moves into the completed/ended
        // section, shifting to a different row `idx_confirm`. An index-
        // based store would still report row `idx_open`, which at
        // confirm time points at a *different* entry (a section header,
        // or sprint B). The identity-based store reports A regardless of
        // the layout change.
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
        // Cursor parks on A; user presses Space to commit it as the
        // selection. (After decoupling, reset alone does not check
        // anything — see test_reset_with_sole_active_sprint_positions_
        // cursor_but_does_not_check.)
        picker.handle_key(KeyCode::Char(' '), &sprints, &board, now_open);
        assert_eq!(picker.selected_sprint_id(), Some(sprint_a.id));
        assert_eq!(picker.raw_selection(), Selection::Sprint(sprint_a.id));

        // Precondition: A's row index actually shifts between the two
        // times — otherwise the test wouldn't be exercising the race.
        let entries_open =
            crate::components::sprint_assign_list::build_entries(&sprints, board.id, now_open);
        let entries_confirm =
            crate::components::sprint_assign_list::build_entries(&sprints, board.id, now_confirm);
        let idx_open = entries_open
            .iter()
            .position(|e| sprint_id_of(e) == Some(sprint_a.id));
        let idx_confirm = entries_confirm
            .iter()
            .position(|e| sprint_id_of(e) == Some(sprint_a.id));
        assert!(idx_open.is_some());
        assert!(idx_confirm.is_some());
        assert_ne!(
            idx_open, idx_confirm,
            "precondition: A's row position must differ between open and confirm"
        );

        // The picker's internal row resolution at confirm time tracks A
        // by identity — it lands on A's *new* row, not on whatever entry
        // sits at the old row. Same property as selected_sprint_id, but
        // demonstrated via the navigation path that handle_key follows.
        assert_eq!(picker.current_index(&entries_confirm), idx_confirm);
        assert_eq!(picker.selected_sprint_id(), Some(sprint_a.id));
    }

    #[test]
    fn test_clear_resets_selection_to_unset_after_space_commit() {
        let now = Utc::now();
        let board = make_board();
        let sprint = active_sprint(board.id, 1, now);
        let sprints = vec![sprint.clone()];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        // Commit a selection via Space so there's something for clear() to reset.
        picker.handle_key(KeyCode::Char(' '), &sprints, &board, now);
        assert_eq!(picker.raw_selection(), Selection::Sprint(sprint.id));

        picker.clear();
        assert_eq!(picker.raw_selection(), Selection::Unset);
    }
}
