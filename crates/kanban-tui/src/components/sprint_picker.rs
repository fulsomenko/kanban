use crate::components::sprint_assign_list::{
    build_entries, build_entries_active_only, next_selectable, prev_selectable, sprint_id_of,
    SprintAssignEntry,
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

/// Which subset of sprints the picker shows. Create-card flows hide
/// finished sprints (you can't bind a brand-new card to one), while
/// assign-to-existing-card flows let the user pick from everything so
/// they can look back at the card's history.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SprintFilter {
    /// Only the Active/Planned section; no Completed/Ended entries.
    #[default]
    ActiveOnly,
    /// Active/Planned and Completed/Ended sections both shown.
    All,
}

/// Stateful sprint picker shared by every sprint-selection dialog. The
/// picker keeps two independent identities:
///
/// - `cursor` is where Up/Down/j/k arrow keys move; it is what the
///   user is currently *pointing at*.
/// - `selection` is what gets the `[x]` checkbox and is what the host
///   reads back via `selected_sprint_id`. Only Space writes to it.
///
/// The same struct drives both the create-card picker (built with
/// `SprintFilter::ActiveOnly`) and the assign-to-card picker (built
/// with `SprintFilter::All`); the filter only changes which entries
/// the picker considers, the navigation and toggle model is identical.
///
/// Composition: `SprintPicker` -> `SprintPickerView` (stateless adapter
/// that turns sprints into ListItem rows) -> `RadioList<Option<Uuid>>`
/// (generic radio-list primitive).
#[derive(Default)]
pub struct SprintPicker {
    selection: Selection,
    cursor: Cursor,
    filter: SprintFilter,
}

impl SprintPicker {
    /// Convenience: a picker for the create-card dialog
    /// (`SprintFilter::ActiveOnly`).
    pub fn new() -> Self {
        Self {
            filter: SprintFilter::ActiveOnly,
            ..Default::default()
        }
    }

    /// Construct a picker with an explicit filter. Use
    /// `SprintFilter::All` for the assign-to-existing-card dialogs so
    /// the user can also pick Completed/Ended sprints; use
    /// `SprintFilter::ActiveOnly` for create-card flows.
    pub fn with_filter(filter: SprintFilter) -> Self {
        Self {
            filter,
            ..Default::default()
        }
    }

    /// Reset for a fresh create-card dialog. The cursor lands on the
    /// sole Active non-ended sprint when there is exactly one — and in
    /// that case the sprint is also pre-checked. Otherwise the cursor
    /// parks on (None) and selection starts `Unset`.
    pub fn reset_for_board(&mut self, sprints: &[Sprint], board: &Board, now: DateTime<Utc>) {
        let view = SprintPickerView::for_new_card(sprints, board, now);
        let entries = self.build_entries(sprints, board, now);
        self.cursor = view
            .initial_selection()
            .and_then(|idx| entries.get(idx).map(Self::entry_to_cursor))
            .unwrap_or(Cursor::NoSprint);
        self.selection = match self.cursor {
            Cursor::Sprint(id) => Selection::Sprint(id),
            Cursor::NoSprint => Selection::Unset,
        };
    }

    /// Reset for an assign-to-card dialog where the card already has a
    /// `current_sprint_id` (or `None` if it's unassigned). Cursor +
    /// selection both follow that current id, so the dialog opens with
    /// `[x]` already on whatever the card is bound to.
    pub fn reset_for_card_assignment(
        &mut self,
        current_sprint_id: Option<Uuid>,
        sprints: &[Sprint],
        board: &Board,
        now: DateTime<Utc>,
    ) {
        let entries = self.build_entries(sprints, board, now);
        let (cursor, selection) = match current_sprint_id {
            Some(id) => {
                // Only consider it pre-checked if the sprint is in the
                // current entry list (the filter might exclude it).
                if entries.iter().any(|e| sprint_id_of(e) == Some(id)) {
                    (Cursor::Sprint(id), Selection::Sprint(id))
                } else {
                    (Cursor::NoSprint, Selection::Unset)
                }
            }
            None => (Cursor::NoSprint, Selection::NoSprint),
        };
        self.cursor = cursor;
        self.selection = selection;
    }

    pub fn clear(&mut self) {
        self.selection = Selection::Unset;
        self.cursor = Cursor::NoSprint;
    }

    /// Try to consume a key. Returns true when the picker handled it:
    /// Up/Down/j/k move the cursor; Space toggles `[x]` at the cursor
    /// (places or moves it, or removes it from a row that already has
    /// it). Other keys fall through to the host (text input, etc.).
    pub fn handle_key(
        &mut self,
        key_code: KeyCode,
        sprints: &[Sprint],
        board: &Board,
        now: DateTime<Utc>,
    ) -> bool {
        if matches!(key_code, KeyCode::Char(' ')) {
            let cursor_as_selection = match self.cursor {
                Cursor::NoSprint => Selection::NoSprint,
                Cursor::Sprint(id) => Selection::Sprint(id),
            };
            self.selection = if self.selection == cursor_as_selection {
                Selection::Unset
            } else {
                cursor_as_selection
            };
            return true;
        }
        let entries = self.build_entries(sprints, board, now);
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

    /// True when the user has explicitly committed a "(None)" choice
    /// via Space. Used by the assign-to-existing-card flow to
    /// distinguish "unassign the card" from "user didn't touch the
    /// picker so leave the card alone".
    pub fn explicitly_unassigned(&self) -> bool {
        matches!(self.selection, Selection::NoSprint)
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        sprints: &[Sprint],
        board: &Board,
        now: DateTime<Utc>,
    ) {
        let view = self.build_view(sprints, board, now);
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

    fn build_entries<'a>(
        &self,
        sprints: &'a [Sprint],
        board: &Board,
        now: DateTime<Utc>,
    ) -> Vec<SprintAssignEntry<'a>> {
        match self.filter {
            SprintFilter::ActiveOnly => build_entries_active_only(sprints, board.id, now),
            SprintFilter::All => build_entries(sprints, board.id, now),
        }
    }

    fn build_view<'a>(
        &self,
        sprints: &'a [Sprint],
        board: &'a Board,
        now: DateTime<Utc>,
    ) -> SprintPickerView<'a> {
        match self.filter {
            SprintFilter::ActiveOnly => SprintPickerView::for_new_card(sprints, board, now),
            SprintFilter::All => SprintPickerView::for_card_assignment(sprints, board, None, now),
        }
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
    fn test_reset_with_sole_active_sprint_pre_checks_it() {
        let now = Utc::now();
        let board = make_board();
        let sprint = active_sprint(board.id, 1, now);
        let sprints = vec![sprint.clone()];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);

        // Exactly one active non-ended sprint → both cursor and selection
        // land on it, so the dialog opens with [x] already there and the
        // user can Enter immediately to confirm.
        assert_eq!(picker.selected_sprint_id(), Some(sprint.id));
        assert_eq!(picker.raw_selection(), Selection::Sprint(sprint.id));
        assert_eq!(picker.cursor_sprint_id(), Some(sprint.id));
    }

    #[test]
    fn test_space_on_already_checked_row_unchecks_it() {
        let now = Utc::now();
        let board = make_board();
        let sprint = active_sprint(board.id, 1, now);
        let sprints = vec![sprint.clone()];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        assert_eq!(picker.raw_selection(), Selection::Sprint(sprint.id));

        // Cursor already on the checked sprint — Space toggles off.
        picker.handle_key(KeyCode::Char(' '), &sprints, &board, now);
        assert_eq!(picker.raw_selection(), Selection::Unset);
        assert_eq!(picker.selected_sprint_id(), None);

        // And toggles back on.
        picker.handle_key(KeyCode::Char(' '), &sprints, &board, now);
        assert_eq!(picker.raw_selection(), Selection::Sprint(sprint.id));
    }

    #[test]
    fn test_space_on_none_row_toggles_between_nosprint_and_unset() {
        let now = Utc::now();
        let board = make_board();
        let sprints: Vec<Sprint> = vec![];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        // No active sprint → cursor parks on (None), selection starts Unset.
        assert_eq!(picker.raw_selection(), Selection::Unset);

        picker.handle_key(KeyCode::Char(' '), &sprints, &board, now);
        assert_eq!(picker.raw_selection(), Selection::NoSprint);

        picker.handle_key(KeyCode::Char(' '), &sprints, &board, now);
        assert_eq!(picker.raw_selection(), Selection::Unset);
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
    fn test_space_at_uncrossed_sprint_cursor_places_check_there() {
        let now = Utc::now();
        let board = make_board();
        // Two planning sprints — no active, so the picker opens with
        // selection Unset and cursor on (None). Down walks the cursor
        // onto a sprint; Space then checks it.
        let p_a = Sprint::new(board.id, 1, None, None);
        let p_b = Sprint::new(board.id, 2, None, None);
        let sprints = vec![p_a, p_b];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        assert_eq!(picker.selected_sprint_id(), None);
        assert_eq!(picker.raw_selection(), Selection::Unset);

        picker.handle_key(KeyCode::Down, &sprints, &board, now);
        let cursor_id = picker.cursor_sprint_id().expect("cursor on sprint");
        picker.handle_key(KeyCode::Char(' '), &sprints, &board, now);

        assert_eq!(picker.selected_sprint_id(), Some(cursor_id));
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
        // Sole active sprint at open time → A is pre-checked, no Space
        // needed.
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
    fn test_picker_skips_completed_and_ended_sprints_during_navigation() {
        // Active and Planning sprints are valid targets in the create-card
        // picker; Completed and Ended ones are intentionally hidden so the
        // user can't accidentally bind a brand-new card to a sprint that's
        // already wrapped up. Arrowing past the active section should
        // clamp instead of walking into a Completed row.
        let now = Utc::now();
        let board = make_board();
        let active = active_sprint(board.id, 1, now);

        // Sprint that ended in the past — would normally appear in the
        // Completed/Ended section of build_entries.
        let mut completed = Sprint::new(board.id, 2, None, None);
        completed.status = SprintStatus::Completed;
        completed.start_date = Some(now - Duration::days(30));
        completed.end_date = Some(now - Duration::days(15));

        let sprints = vec![active.clone(), completed.clone()];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        // Cursor parks on the sole active sprint (the only candidate).
        assert_eq!(picker.cursor_sprint_id(), Some(active.id));

        // Walk down — completed sprint must not be reachable.
        for _ in 0..3 {
            picker.handle_key(KeyCode::Down, &sprints, &board, now);
        }
        assert_ne!(
            picker.cursor_sprint_id(),
            Some(completed.id),
            "Down must not reach a completed sprint in the new-card picker"
        );
    }

    #[test]
    fn test_clear_resets_selection_to_unset() {
        let now = Utc::now();
        let board = make_board();
        let sprint = active_sprint(board.id, 1, now);
        let sprints = vec![sprint.clone()];

        let mut picker = SprintPicker::new();
        picker.reset_for_board(&sprints, &board, now);
        // Sole active sprint → pre-checked.
        assert_eq!(picker.raw_selection(), Selection::Sprint(sprint.id));

        picker.clear();
        assert_eq!(picker.raw_selection(), Selection::Unset);
    }
}
