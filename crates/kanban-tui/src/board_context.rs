use kanban_domain::{Board, Sprint};

/// Get the active sprint's card prefix override if one exists.
/// Returns the sprint's card_prefix if the board has an active sprint
/// that has a card_prefix override set.
pub fn get_active_sprint_card_prefix_override<'a>(
    board: &'a Board,
    sprints: &'a [Sprint],
) -> Option<&'a str> {
    board.active_sprint_id.and_then(|sprint_id| {
        sprints
            .iter()
            .find(|s| s.id == sprint_id)
            .and_then(|sprint| sprint.card_prefix.as_deref())
    })
}

/// Get the active sprint's sprint prefix override if one exists.
/// Returns the sprint's prefix if the board has an active sprint
/// that has a sprint prefix override set.
pub fn get_active_sprint_prefix_override<'a>(
    board: &'a Board,
    sprints: &'a [Sprint],
) -> Option<&'a str> {
    board.active_sprint_id.and_then(|sprint_id| {
        sprints
            .iter()
            .find(|s| s.id == sprint_id)
            .and_then(|sprint| sprint.prefix.as_deref())
    })
}
