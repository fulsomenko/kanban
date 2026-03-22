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
}

