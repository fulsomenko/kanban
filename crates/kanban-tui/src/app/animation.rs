use kanban_domain::AnimationType;
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

pub(crate) const ANIMATION_DURATION_MS: u128 = 150;

pub struct CardAnimation {
    pub animation_type: AnimationType,
    pub start_time: Instant,
}

#[derive(Default)]
pub struct AnimationState {
    pub animating: HashMap<Uuid, CardAnimation>,
    /// Column id and card position to anchor selection on after the next
    /// archive batch completes. Captured at the moment the user triggers the
    /// archive (from the focused/cursor card) so that multi-column archives
    /// land selection on the column the user was actually looking at, rather
    /// than an arbitrary archived card chosen by HashMap iteration order.
    pub archive_anchor: Option<(Uuid, i32)>,
}
