use super::{Keybinding, KeybindingContext, KeybindingProvider};

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

pub struct ArchivedCardsViewProvider;

impl KeybindingProvider for ArchivedCardsViewProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Archived Cards View",
            vec![
                Keybinding::new("?", "help", "Show help"),
                Keybinding::new("j/↓", "down", "Navigate down"),
                Keybinding::new("k/↑", "up", "Navigate up"),
                Keybinding::new("r", "restore", "Restore selected task(s)"),
                Keybinding::new("x/X", "delete", "Delete selected task(s)"),
                Keybinding::new("v", "select", "Select task for bulk operation"),
                Keybinding::new("V", "view", "Toggle task list view"),
                Keybinding::new("D/q/Esc", "back", "Back to normal view"),
            ],
        )
    }
}
