use super::{Keybinding, KeybindingAction, KeybindingContext, KeybindingProvider};

pub struct SettingsViewProvider;

impl KeybindingProvider for SettingsViewProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Settings",
            vec![
                Keybinding::new(
                    "e",
                    "edit",
                    "Edit configuration in external editor",
                    KeybindingAction::EditCard,
                ),
                Keybinding::new(
                    "x",
                    "export",
                    "Export boards",
                    KeybindingAction::ExportBoards,
                ),
                Keybinding::new(
                    "q/Esc",
                    "back",
                    "Back to normal view",
                    KeybindingAction::Escape,
                ),
            ],
        )
    }
}
