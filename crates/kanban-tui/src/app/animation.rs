use kanban_domain::AnimationType;
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

pub const ANIMATION_DURATION_MS: u128 = 150;

pub struct CardAnimation {
    pub animation_type: AnimationType,
    pub start_time: Instant,
}

pub struct AnimationState {
    pub animating: HashMap<Uuid, CardAnimation>,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            animating: HashMap::new(),
        }
    }
}

impl Default for AnimationState {
    fn default() -> Self {
        Self::new()
    }
}
