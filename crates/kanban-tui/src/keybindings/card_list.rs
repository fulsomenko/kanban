use super::{Keybinding, KeybindingAction, KeybindingContext, KeybindingProvider};

pub struct CardListProvider;

impl KeybindingProvider for CardListProvider {
    fn get_context(&self) -> KeybindingContext {
        KeybindingContext::new(
            "Normal Mode - Cards Panel",
            vec![
                Keybinding::new("?", "help", "Show help", KeybindingAction::ShowHelp),
                Keybinding::new("q", "quit", "Quit application", KeybindingAction::Escape),
                Keybinding::new(
                    "1",
                    "panel 1",
                    "Focus projects panel",
                    KeybindingAction::FocusPanel(0),
                ),
                Keybinding::new(
                    "2",
                    "panel 2",
                    "Focus tasks panel",
                    KeybindingAction::FocusPanel(1),
                ),
                Keybinding::new("n", "new", "Create new task", KeybindingAction::CreateCard),
                Keybinding::new(
                    "e",
                    "edit",
                    "Edit selected task",
                    KeybindingAction::EditCard,
                ),
                Keybinding::new(
                    "c",
                    "complete",
                    "Toggle task completion",
                    KeybindingAction::ToggleCompletion,
                ),
                Keybinding::new(
                    "d",
                    "archive",
                    "Archive selected task(s)",
                    KeybindingAction::ArchiveCard,
                ),
                Keybinding::new(
                    "D",
                    "archived",
                    "View archived tasks",
                    KeybindingAction::ToggleArchivedView,
                ),
                Keybinding::new(
                    "v",
                    "select",
                    "Select task for bulk operation",
                    KeybindingAction::ToggleCardSelection,
                ),
                Keybinding::new(
                    "V",
                    "view",
                    "Toggle task list view",
                    KeybindingAction::ToggleTaskListView,
                ),
                Keybinding::new(
                    "j/↓",
                    "down",
                    "Navigate down",
                    KeybindingAction::NavigateDown,
                ),
                Keybinding::new("k/↑", "up", "Navigate up", KeybindingAction::NavigateUp),
                Keybinding::new(
                    "h",
                    "prev col",
                    "Move to previous column",
                    KeybindingAction::NavigateLeft,
                ),
                Keybinding::new(
                    "l",
                    "next col",
                    "Move to next column",
                    KeybindingAction::NavigateRight,
                ),
                Keybinding::new(
                    "H",
                    "move left",
                    "Move card to left column",
                    KeybindingAction::MoveCardLeft,
                ),
                Keybinding::new(
                    "L",
                    "move right",
                    "Move card to right column",
                    KeybindingAction::MoveCardRight,
                ),
                Keybinding::new(
                    "o",
                    "sort",
                    "Sort tasks by field",
                    KeybindingAction::OrderCards,
                ),
                Keybinding::new(
                    "O",
                    "toggle order",
                    "Toggle sort order",
                    KeybindingAction::ToggleSortOrder,
                ),
                Keybinding::new(
                    "a",
                    "assign",
                    "Assign task to sprint",
                    KeybindingAction::AssignToSprint,
                ),
                Keybinding::new(
                    "t",
                    "filter",
                    "Toggle sprint filter",
                    KeybindingAction::ToggleFilter,
                ),
                Keybinding::new(
                    "T",
                    "options",
                    "Open filter options",
                    KeybindingAction::ToggleHideAssigned,
                ),
                Keybinding::new("/", "search", "Search tasks", KeybindingAction::Search),
                Keybinding::new(
                    "Enter/Space",
                    "detail",
                    "View task detail",
                    KeybindingAction::SelectItem,
                ),
                Keybinding::new(
                    "p",
                    "priority",
                    "Set task priority",
                    KeybindingAction::EditCard,
                ),
            ],
        )
    }
}
