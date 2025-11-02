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
                Keybinding::new("?", "Show help"),
                Keybinding::new("q", "Quit card detail"),
                Keybinding::new("ESC", "Back to task list"),
                Keybinding::new("1", "Focus task title"),
                Keybinding::new("2", "Focus metadata"),
                Keybinding::new("3", "Focus description"),
                Keybinding::new("e", "Edit current panel"),
                Keybinding::new("y", "Copy branch name"),
                Keybinding::new("Y", "Copy git checkout command"),
                Keybinding::new("s", "Assign to sprint"),
            ],
        )
    }
}
