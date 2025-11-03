use super::{Keybinding, KeybindingContext, KeybindingProvider};

pub struct SprintDetailProvider;

impl KeybindingProvider for SprintDetailProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Sprint Detail",
            vec![
                Keybinding::new("?", "help", "Show help"),
                Keybinding::new("q", "quit", "Exit sprint detail view"),
                Keybinding::new("ESC", "back", "Return to project detail"),
                Keybinding::new("a", "activate", "Activate this sprint"),
                Keybinding::new("c", "complete", "Complete this sprint"),
                Keybinding::new("p", "s-prefix", "Set sprint prefix"),
                Keybinding::new("C", "c-prefix", "Set card prefix override"),
                Keybinding::new("o", "sort", "Sort tasks by field"),
                Keybinding::new("O", "toggle order", "Toggle sort order"),
                Keybinding::new("h", "left panel", "Switch to uncompleted panel"),
                Keybinding::new("l", "right panel", "Switch to completed panel"),
                Keybinding::new("j/↓", "down", "Navigate down"),
                Keybinding::new("k/↑", "up", "Navigate up"),
                Keybinding::new("v", "select", "Select task for bulk operation"),
                Keybinding::new("n", "new", "Create new task"),
                Keybinding::new("e", "edit", "Edit selected task"),
                Keybinding::new("s", "assign", "Assign task to sprint"),
                Keybinding::new("y", "copy branch", "Copy branch name to clipboard"),
                Keybinding::new("Y", "copy cmd", "Copy git checkout command"),
            ],
        )
    }
}
