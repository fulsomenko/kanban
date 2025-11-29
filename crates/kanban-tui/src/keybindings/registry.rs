use super::{
    board_detail::BoardDetailProvider,
    card_detail::CardDetailProvider,
    card_list::CardListProvider,
    dialog_modes::{
        DeleteConfirmProvider, DialogInputProvider, DialogSelectionProvider, FilterOptionsProvider,
        SearchModeProvider,
    },
    normal_mode::{ArchivedCardsViewProvider, NormalModeBoardsProvider},
    sprint_detail::SprintDetailProvider,
    KeybindingProvider,
};
use crate::app::{App, AppMode, Focus};

pub struct KeybindingRegistry;

impl KeybindingRegistry {
    pub fn get_provider(app: &App) -> Box<dyn KeybindingProvider> {
        Self::get_provider_for_mode(
            &app.mode,
            app.focus.clone(),
            app.card_focus,
            app.board_focus,
        )
    }

    fn get_provider_for_mode(
        mode: &AppMode,
        focus: Focus,
        card_focus: crate::app::CardFocus,
        board_focus: crate::app::BoardFocus,
    ) -> Box<dyn KeybindingProvider> {
        match mode {
            AppMode::Normal => match focus {
                Focus::Cards => Box::new(CardListProvider),
                Focus::Boards => Box::new(NormalModeBoardsProvider),
            },
            AppMode::CardDetail => Box::new(CardDetailProvider::new(card_focus)),
            AppMode::BoardDetail => Box::new(BoardDetailProvider::new(board_focus)),
            AppMode::SprintDetail => Box::new(SprintDetailProvider),
            AppMode::Search => Box::new(SearchModeProvider),
            AppMode::CreateBoard => Box::new(DialogInputProvider::new("Create Project")),
            AppMode::CreateCard => Box::new(DialogInputProvider::new("Create Task")),
            AppMode::CreateSprint => Box::new(DialogInputProvider::new("Create Sprint")),
            AppMode::RenameBoard => Box::new(DialogInputProvider::new("Rename Project")),
            AppMode::RenameColumn => Box::new(DialogInputProvider::new("Rename Column")),
            AppMode::CreateColumn => Box::new(DialogInputProvider::new("Create Column")),
            AppMode::ExportBoard => Box::new(DialogInputProvider::new("Export Project")),
            AppMode::ExportAll => Box::new(DialogInputProvider::new("Export All Projects")),
            AppMode::SetCardPoints => Box::new(DialogInputProvider::new("Set Points")),
            AppMode::SetBranchPrefix => Box::new(DialogInputProvider::new("Set Branch Prefix")),
            AppMode::SetSprintPrefix => Box::new(DialogInputProvider::new("Set Sprint Prefix")),
            AppMode::SetSprintCardPrefix => Box::new(DialogInputProvider::new("Set Card Prefix")),
            AppMode::ImportBoard => Box::new(DialogSelectionProvider::new("Import Project")),
            AppMode::SetCardPriority => Box::new(DialogSelectionProvider::new("Set Priority")),
            AppMode::OrderCards => Box::new(DialogSelectionProvider::new("Sort Tasks")),
            AppMode::AssignCardToSprint => {
                Box::new(DialogSelectionProvider::new("Assign to Sprint"))
            }
            AppMode::AssignMultipleCardsToSprint => {
                Box::new(DialogSelectionProvider::new("Assign Cards to Sprint"))
            }
            AppMode::SelectTaskListView => {
                Box::new(DialogSelectionProvider::new("Select Task View"))
            }
            AppMode::DeleteColumnConfirm => Box::new(DeleteConfirmProvider::new("Column")),
            AppMode::ConfirmSprintPrefixCollision => {
                Box::new(DialogSelectionProvider::new("Confirm Action"))
            }
            AppMode::FilterOptions => Box::new(FilterOptionsProvider),
            AppMode::ArchivedCardsView => Box::new(ArchivedCardsViewProvider),
            AppMode::ConflictResolution => {
                Box::new(DialogSelectionProvider::new("Resolve Conflict"))
            }
            AppMode::Help(previous_mode) => {
                Self::get_provider_for_mode(previous_mode, focus, card_focus, board_focus)
            }
        }
    }
}
