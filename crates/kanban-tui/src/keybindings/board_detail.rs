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
            Keybinding::new("?", "help", "Show help"),
            Keybinding::new("q", "quit", "Exit project detail view"),
            Keybinding::new("ESC", "back", "Return to project list"),
            Keybinding::new("1", "panel 1", "Focus project name panel"),
            Keybinding::new("2", "panel 2", "Focus description panel"),
            Keybinding::new("3", "panel 3", "Focus settings panel"),
            Keybinding::new("4", "panel 4", "Focus sprints panel"),
            Keybinding::new("5", "panel 5", "Focus columns panel"),
            Keybinding::new("e", "edit", "Edit current panel"),
            Keybinding::new("p", "prefix", "Set branch prefix"),
        ];

        match self.focus {
            BoardFocus::Sprints => {
                bindings.extend(vec![
                    Keybinding::new("n", "new", "Create new sprint"),
                    Keybinding::new("j/↓", "down", "Navigate down"),
                    Keybinding::new("k/↑", "up", "Navigate up"),
                    Keybinding::new("Enter/Space", "detail", "Open sprint detail"),
                ]);
            }
            BoardFocus::Columns => {
                bindings.extend(vec![
                    Keybinding::new("n", "new", "Create new column"),
                    Keybinding::new("r", "rename", "Rename selected column"),
                    Keybinding::new("d", "delete", "Delete selected column"),
                    Keybinding::new("j/↓", "down", "Navigate down"),
                    Keybinding::new("k/↑", "up", "Navigate up"),
                    Keybinding::new("J", "move up", "Reorder column up"),
                    Keybinding::new("K", "move down", "Reorder column down"),
                ]);
            }
            _ => {}
        }

        KeybindingContext::new(format!("Project Detail - {} Panel", focus_name), bindings)
    }
}
