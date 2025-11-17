use super::{Keybinding, KeybindingAction, KeybindingContext, KeybindingProvider};

pub struct NormalModeBoardsProvider;

impl KeybindingProvider for NormalModeBoardsProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Normal Mode - Projects Panel",
            vec![
                Keybinding::new("?", "help", "Show help", KeybindingAction::ShowHelp),
                Keybinding::new("q", "quit", "Quit application", KeybindingAction::Escape),
                Keybinding::new("1", "panel 1", "Focus projects panel", KeybindingAction::FocusPanel(0)),
                Keybinding::new("2", "panel 2", "Focus tasks panel", KeybindingAction::FocusPanel(1)),
                Keybinding::new("n", "new", "Create new project", KeybindingAction::CreateBoard),
                Keybinding::new("r", "rename", "Rename selected project", KeybindingAction::RenameBoard),
                Keybinding::new("e", "edit", "Edit selected project", KeybindingAction::EditBoard),
                Keybinding::new("x", "export", "Export selected project", KeybindingAction::ExportBoard),
                Keybinding::new("X", "export all", "Export all projects", KeybindingAction::ExportAll),
                Keybinding::new("i", "import", "Import project from file", KeybindingAction::ImportBoard),
                Keybinding::new("j/↓", "down", "Navigate down", KeybindingAction::NavigateDown),
                Keybinding::new("k/↑", "up", "Navigate up", KeybindingAction::NavigateUp),
                Keybinding::new("Enter/Space", "detail", "View project detail", KeybindingAction::SelectItem),
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
                Keybinding::new("?", "help", "Show help", KeybindingAction::ShowHelp),
                Keybinding::new("j/↓", "down", "Navigate down", KeybindingAction::NavigateDown),
                Keybinding::new("k/↑", "up", "Navigate up", KeybindingAction::NavigateUp),
                Keybinding::new("r", "restore", "Restore selected task(s)", KeybindingAction::RestoreCard),
                Keybinding::new("x", "delete", "Delete selected task(s)", KeybindingAction::DeleteCard),
                Keybinding::new("v", "select", "Select task for bulk operation", KeybindingAction::ToggleCardSelection),
                Keybinding::new("V", "view", "Toggle task list view", KeybindingAction::ToggleTaskListView),
                Keybinding::new("q/Esc", "back", "Back to normal view", KeybindingAction::Escape),
            ],
        )
    }
}
