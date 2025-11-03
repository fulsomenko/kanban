use super::{Keybinding, KeybindingContext, KeybindingProvider};

pub struct SearchModeProvider;

impl KeybindingProvider for SearchModeProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Search Mode",
            vec![
                Keybinding::new("ESC", "exit", "Exit search mode"),
                Keybinding::new("Enter", "confirm", "Confirm search and filter"),
                Keybinding::new("Type", "query", "Enter search query"),
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
                Keybinding::new("ESC", "cancel", "Cancel and close dialog"),
                Keybinding::new("Enter", "confirm", "Confirm and apply"),
                Keybinding::new("Type", "input", "Enter text"),
                Keybinding::new("Backspace", "delete", "Delete previous character"),
                Keybinding::new("←/→", "move", "Move cursor left/right"),
                Keybinding::new("Home/End", "jump", "Jump to start/end of line"),
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
                Keybinding::new("ESC", "cancel", "Cancel and close dialog"),
                Keybinding::new("j/↓", "down", "Navigate down"),
                Keybinding::new("k/↑", "up", "Navigate up"),
                Keybinding::new("Enter/Space", "select", "Select and confirm"),
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
                Keybinding::new("ESC", "cancel", "Cancel deletion"),
                Keybinding::new("n", "no", "Do not delete"),
                Keybinding::new("y", "yes", "Confirm deletion"),
                Keybinding::new("Enter", "yes", "Confirm deletion"),
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
                Keybinding::new("ESC", "cancel", "Cancel and close filters"),
                Keybinding::new("j/↓", "down", "Navigate down"),
                Keybinding::new("k/↑", "up", "Navigate up"),
                Keybinding::new("Space", "toggle", "Toggle filter option"),
                Keybinding::new("Enter", "apply", "Apply selected filters"),
            ],
        )
    }
}
