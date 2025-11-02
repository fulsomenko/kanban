use super::{Keybinding, KeybindingContext, KeybindingProvider};

pub struct SearchModeProvider;

impl KeybindingProvider for SearchModeProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Search Mode",
            vec![
                Keybinding::new("ESC", "Exit search"),
                Keybinding::new("Enter", "Confirm search"),
                Keybinding::new("Type", "Enter search query"),
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
                Keybinding::new("ESC", "Cancel"),
                Keybinding::new("Enter", "Confirm"),
                Keybinding::new("Type", "Enter text"),
                Keybinding::new("Backspace", "Delete character"),
                Keybinding::new("←/→", "Move cursor"),
                Keybinding::new("Home/End", "Jump to start/end"),
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
                Keybinding::new("ESC", "Cancel"),
                Keybinding::new("j/↓", "Navigate down"),
                Keybinding::new("k/↑", "Navigate up"),
                Keybinding::new("Enter/Space", "Select and confirm"),
            ],
        )
    }
}

pub struct DeleteConfirmProvider {
    what: String,
}

impl DeleteConfirmProvider {
    pub fn new(what: impl Into<String>) -> Self {
        Self {
            what: what.into(),
        }
    }
}

impl KeybindingProvider for DeleteConfirmProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            format!("Delete {} - Confirm", self.what),
            vec![
                Keybinding::new("ESC", "Cancel"),
                Keybinding::new("n", "Cancel"),
                Keybinding::new("y", "Confirm delete"),
                Keybinding::new("Enter", "Confirm delete"),
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
                Keybinding::new("ESC", "Cancel"),
                Keybinding::new("j/↓", "Navigate down"),
                Keybinding::new("k/↑", "Navigate up"),
                Keybinding::new("Space", "Toggle filter"),
                Keybinding::new("Enter", "Apply filters"),
            ],
        )
    }
}
