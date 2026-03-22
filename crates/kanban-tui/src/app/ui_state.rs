use crate::components::{generic_list::ListComponent, Banner};
use crate::keybindings::KeybindingAction;
use std::time::Instant;

pub struct UiState {
    pub banner: Option<Banner>,
    pub help_list: ListComponent,
    pub help_pending_action: Option<(Instant, KeybindingAction)>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            banner: None,
            help_list: ListComponent::new(false),
            help_pending_action: None,
        }
    }
}
