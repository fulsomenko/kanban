//! Search functionality for the TUI.
//!
//! Re-exports domain search types and provides TUI-specific SearchState.

use crate::input::InputState;

// Re-export domain search types
pub use kanban_domain::search::{
    BranchNameSearcher, CardSearcher, CompositeSearcher, TitleSearcher,
};

// Type aliases for backward compatibility
pub type CardTitleSearcher = TitleSearcher;
pub type CardBranchNameSearcher = BranchNameSearcher;
pub type CompositeCardSearcher = CompositeSearcher;

/// UI state for search mode.
///
/// This struct manages the search input and active state.
/// The actual search logic is in the domain layer.
pub struct SearchState {
    pub input: InputState,
    pub is_active: bool,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            input: InputState::new(),
            is_active: false,
        }
    }

    pub fn activate(&mut self) {
        self.is_active = true;
        self.input.clear();
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.input.clear();
    }

    pub fn query(&self) -> &str {
        self.input.as_str()
    }

    pub fn is_empty(&self) -> bool {
        self.input.as_str().is_empty()
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}
