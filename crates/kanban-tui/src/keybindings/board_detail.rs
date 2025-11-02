use super::{Keybinding, KeybindingContext, KeybindingProvider};
use crate::app::BoardFocus;

pub struct BoardDetailProvider {
    focus: BoardFocus,
}

impl BoardDetailProvider {
    pub fn new(focus: BoardFocus) -> Self {
        Self { focus }
    }
}

impl KeybindingProvider for BoardDetailProvider {
    fn get_context(&self) -> KeybindingContext {
        let focus_name = match self.focus {
            BoardFocus::Name => "Name",
            BoardFocus::Description => "Description",
            BoardFocus::Settings => "Settings",
            BoardFocus::Sprints => "Sprints",
            BoardFocus::Columns => "Columns",
        };

        let mut bindings = vec![
            Keybinding::new("?", "Show help"),
            Keybinding::new("q", "Quit project detail"),
            Keybinding::new("ESC", "Back to project list"),
            Keybinding::new("1", "Focus project name"),
            Keybinding::new("2", "Focus description"),
            Keybinding::new("3", "Focus settings"),
            Keybinding::new("4", "Focus sprints"),
            Keybinding::new("5", "Focus columns"),
            Keybinding::new("e", "Edit current panel"),
            Keybinding::new("p", "Set branch prefix"),
        ];

        match self.focus {
            BoardFocus::Sprints => {
                bindings.extend(vec![
                    Keybinding::new("n", "New sprint"),
                    Keybinding::new("j/↓", "Navigate down"),
                    Keybinding::new("k/↑", "Navigate up"),
                    Keybinding::new("Enter/Space", "Open sprint detail"),
                ]);
            }
            BoardFocus::Columns => {
                bindings.extend(vec![
                    Keybinding::new("n", "New column"),
                    Keybinding::new("r", "Rename column"),
                    Keybinding::new("d", "Delete column"),
                    Keybinding::new("j/↓", "Navigate down"),
                    Keybinding::new("k/↑", "Navigate up"),
                    Keybinding::new("J", "Reorder column up"),
                    Keybinding::new("K", "Reorder column down"),
                ]);
            }
            _ => {}
        }

        KeybindingContext::new(format!("Project Detail - {} Panel", focus_name), bindings)
    }
}
