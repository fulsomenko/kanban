use super::{Keybinding, KeybindingContext, KeybindingProvider};

pub struct NormalModeCardsProvider;

impl KeybindingProvider for NormalModeCardsProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Normal Mode - Cards Panel",
            vec![
                Keybinding::new("?", "help", "Show help"),
                Keybinding::new("q", "quit", "Quit application"),
                Keybinding::new("1", "panel 1", "Focus projects panel"),
                Keybinding::new("2", "panel 2", "Focus tasks panel"),
                Keybinding::new("n", "new", "Create new task"),
                Keybinding::new("e", "edit", "Edit selected task"),
                Keybinding::new("c", "complete", "Toggle task completion"),
                Keybinding::new("v", "select", "Select task for bulk operation"),
                Keybinding::new("V", "view", "Toggle task list view"),
                Keybinding::new("j/↓", "down", "Navigate down"),
                Keybinding::new("k/↑", "up", "Navigate up"),
                Keybinding::new("h", "prev col", "Move to previous column"),
                Keybinding::new("l", "next col", "Move to next column"),
                Keybinding::new("H", "move left", "Move card to left column"),
                Keybinding::new("L", "move right", "Move card to right column"),
                Keybinding::new("o", "sort", "Sort tasks by field"),
                Keybinding::new("O", "toggle order", "Toggle sort order"),
                Keybinding::new("a", "assign", "Assign task to sprint"),
                Keybinding::new("t", "filter", "Toggle sprint filter"),
                Keybinding::new("T", "options", "Open filter options"),
                Keybinding::new("/", "search", "Search tasks"),
                Keybinding::new("Enter/Space", "detail", "View task detail"),
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
                Keybinding::new("?", "help", "Show help"),
                Keybinding::new("q", "quit", "Quit application"),
                Keybinding::new("1", "panel 1", "Focus projects panel"),
                Keybinding::new("2", "panel 2", "Focus tasks panel"),
                Keybinding::new("n", "new", "Create new project"),
                Keybinding::new("r", "rename", "Rename selected project"),
                Keybinding::new("e", "edit", "Edit selected project"),
                Keybinding::new("x", "export", "Export selected project"),
                Keybinding::new("X", "export all", "Export all projects"),
                Keybinding::new("i", "import", "Import project from file"),
                Keybinding::new("j/↓", "down", "Navigate down"),
                Keybinding::new("k/↑", "up", "Navigate up"),
                Keybinding::new("Enter/Space", "detail", "View project detail"),
            ],
        )
    }
}
