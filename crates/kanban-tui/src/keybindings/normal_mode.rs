use super::{Keybinding, KeybindingContext, KeybindingProvider};

pub struct NormalModeCardsProvider;

impl KeybindingProvider for NormalModeCardsProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Normal Mode - Cards Panel",
            vec![
                Keybinding::new("?", "Show help"),
                Keybinding::new("q", "Quit"),
                Keybinding::new("1", "Focus projects panel"),
                Keybinding::new("2", "Focus tasks panel"),
                Keybinding::new("n", "New task"),
                Keybinding::new("e", "Edit task"),
                Keybinding::new("c", "Toggle task completion"),
                Keybinding::new("v", "Select task"),
                Keybinding::new("V", "Toggle task list view"),
                Keybinding::new("j/↓", "Navigate down"),
                Keybinding::new("k/↑", "Navigate up"),
                Keybinding::new("h", "Previous column"),
                Keybinding::new("l", "Next column"),
                Keybinding::new("H", "Move card left"),
                Keybinding::new("L", "Move card right"),
                Keybinding::new("o", "Sort tasks"),
                Keybinding::new("O", "Toggle sort order"),
                Keybinding::new("a", "Assign to sprint"),
                Keybinding::new("t", "Toggle sprint filter"),
                Keybinding::new("T", "Filter options"),
                Keybinding::new("/", "Search tasks"),
                Keybinding::new("Enter/Space", "View task detail"),
            ],
        )
    }
}

pub struct NormalModeBoardsProvider;

impl KeybindingProvider for NormalModeBoardsProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Normal Mode - Projects Panel",
            vec![
                Keybinding::new("?", "Show help"),
                Keybinding::new("q", "Quit"),
                Keybinding::new("1", "Focus projects panel"),
                Keybinding::new("2", "Focus tasks panel"),
                Keybinding::new("n", "New project"),
                Keybinding::new("r", "Rename project"),
                Keybinding::new("e", "Edit project"),
                Keybinding::new("x", "Export project"),
                Keybinding::new("X", "Export all projects"),
                Keybinding::new("i", "Import project"),
                Keybinding::new("j/↓", "Navigate down"),
                Keybinding::new("k/↑", "Navigate up"),
                Keybinding::new("Enter/Space", "View project detail"),
            ],
        )
    }
}
