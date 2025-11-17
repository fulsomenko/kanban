use super::{Keybinding, KeybindingAction, KeybindingContext, KeybindingProvider};

pub struct SearchModeProvider;

impl KeybindingProvider for SearchModeProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Search Mode",
            vec![
                Keybinding::new("ESC", "exit", "Exit search mode", KeybindingAction::Escape),
                Keybinding::new("Enter", "confirm", "Confirm search and filter", KeybindingAction::SelectItem),
                Keybinding::new("Type", "query", "Enter search query", KeybindingAction::Search),
            ],
        )
    }
}

pub struct DialogInputProvider {
    dialog_name: String,
}

impl DialogInputProvider {
    pub fn new(dialog_name: impl Into<String>) -> Self {
        Self {
            dialog_name: dialog_name.into(),
        }
    }
}

impl KeybindingProvider for DialogInputProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            format!("{} - Input Dialog", self.dialog_name),
            vec![
                Keybinding::new("ESC", "cancel", "Cancel and close dialog", KeybindingAction::Escape),
                Keybinding::new("Enter", "confirm", "Confirm and apply", KeybindingAction::SelectItem),
                Keybinding::new("Type", "input", "Enter text", KeybindingAction::EditCard),
                Keybinding::new("Backspace", "delete", "Delete previous character", KeybindingAction::EditCard),
                Keybinding::new("←/→", "move", "Move cursor left/right", KeybindingAction::NavigateLeft),
                Keybinding::new("Home/End", "jump", "Jump to start/end of line", KeybindingAction::NavigateLeft),
            ],
        )
    }
}

pub struct DialogSelectionProvider {
    dialog_name: String,
}

impl DialogSelectionProvider {
    pub fn new(dialog_name: impl Into<String>) -> Self {
        Self {
            dialog_name: dialog_name.into(),
        }
    }
}

impl KeybindingProvider for DialogSelectionProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            format!("{} - Selection Dialog", self.dialog_name),
            vec![
                Keybinding::new("ESC", "cancel", "Cancel and close dialog", KeybindingAction::Escape),
                Keybinding::new("j/↓", "down", "Navigate down", KeybindingAction::NavigateDown),
                Keybinding::new("k/↑", "up", "Navigate up", KeybindingAction::NavigateUp),
                Keybinding::new("Enter/Space", "select", "Select and confirm", KeybindingAction::SelectItem),
            ],
        )
    }
}

pub struct DeleteConfirmProvider {
    what: String,
}

impl DeleteConfirmProvider {
    pub fn new(what: impl Into<String>) -> Self {
        Self { what: what.into() }
    }
}

impl KeybindingProvider for DeleteConfirmProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            format!("Delete {} - Confirm", self.what),
            vec![
                Keybinding::new("ESC", "cancel", "Cancel deletion", KeybindingAction::Escape),
                Keybinding::new("n", "no", "Do not delete", KeybindingAction::Escape),
                Keybinding::new("y", "yes", "Confirm deletion", KeybindingAction::SelectItem),
                Keybinding::new("Enter", "yes", "Confirm deletion", KeybindingAction::SelectItem),
            ],
        )
    }
}

pub struct FilterOptionsProvider;

impl KeybindingProvider for FilterOptionsProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Filter Options",
            vec![
                Keybinding::new("ESC", "cancel", "Cancel and close filters", KeybindingAction::Escape),
                Keybinding::new("j/↓", "down", "Navigate down", KeybindingAction::NavigateDown),
                Keybinding::new("k/↑", "up", "Navigate up", KeybindingAction::NavigateUp),
                Keybinding::new("Space", "toggle", "Toggle filter option", KeybindingAction::ToggleFilter),
                Keybinding::new("Enter", "apply", "Apply selected filters", KeybindingAction::SelectItem),
            ],
        )
    }
}
