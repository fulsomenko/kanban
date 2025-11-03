use super::{Keybinding, KeybindingContext, KeybindingProvider};
use crate::app::CardFocus;

pub struct CardDetailProvider {
    focus: CardFocus,
}

impl CardDetailProvider {
    pub fn new(focus: CardFocus) -> Self {
        Self { focus }
    }
}

impl KeybindingProvider for CardDetailProvider {
    fn get_context(&self) -> KeybindingContext {
        let focus_name = match self.focus {
            CardFocus::Title => "Title",
            CardFocus::Metadata => "Metadata",
            CardFocus::Description => "Description",
        };

        KeybindingContext::new(
            format!("Card Detail - {} Panel", focus_name),
            vec![
                Keybinding::new("?", "help", "Show help"),
                Keybinding::new("q", "quit", "Exit card detail view"),
                Keybinding::new("ESC", "back", "Return to task list"),
                Keybinding::new("1", "panel 1", "Focus task title panel"),
                Keybinding::new("2", "panel 2", "Focus metadata panel"),
                Keybinding::new("3", "panel 3", "Focus description panel"),
                Keybinding::new("e", "edit", "Edit current panel"),
                Keybinding::new("y", "copy branch", "Copy branch name to clipboard"),
                Keybinding::new("Y", "copy cmd", "Copy git checkout command"),
                Keybinding::new("a", "assign", "Assign task to sprint"),
            ],
        )
    }
}
