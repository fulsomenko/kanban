use super::{Keybinding, KeybindingContext, KeybindingProvider};

pub struct SprintDetailProvider;

impl KeybindingProvider for SprintDetailProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Sprint Detail",
            vec![
                Keybinding::new("?", "Show help"),
                Keybinding::new("q", "Quit sprint detail"),
                Keybinding::new("ESC", "Back to project detail"),
                Keybinding::new("a", "Activate sprint"),
                Keybinding::new("c", "Complete sprint"),
                Keybinding::new("p", "Set sprint prefix"),
                Keybinding::new("C", "Set card prefix override"),
                Keybinding::new("o", "Sort tasks"),
                Keybinding::new("O", "Toggle sort order"),
                Keybinding::new("h", "Switch to uncompleted panel"),
                Keybinding::new("l", "Switch to completed panel"),
                Keybinding::new("j/↓", "Navigate down"),
                Keybinding::new("k/↑", "Navigate up"),
                Keybinding::new("v", "Select task"),
                Keybinding::new("n", "New task"),
                Keybinding::new("e", "Edit task"),
                Keybinding::new("s", "Assign to sprint"),
                Keybinding::new("y", "Copy branch name"),
                Keybinding::new("Y", "Copy git checkout command"),
            ],
        )
    }
}
