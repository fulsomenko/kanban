use crate::{
    card_list::{CardList, CardListId},
    card_list_component::{CardListComponent, CardListComponentConfig},
    clipboard,
    editor::edit_in_external_editor,
    events::{Event, EventHandler},
    export::{BoardExporter, BoardImporter},
    filters::FilterDialogState,
    input::InputState,
    search::SearchState,
    selection::SelectionState,
    services::{filter::CardFilter, get_sorter_for_field, BoardFilter, OrderedSorter},
    ui,
    view_strategy::{UnifiedViewStrategy, ViewRefreshContext, ViewStrategy},
};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use kanban_core::{AppConfig, Editable, KanbanResult};
use kanban_domain::{Board, Card, Column, SortField, SortOrder, Sprint};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub struct App {
    pub should_quit: bool,
    pub mode: AppMode,
    pub input: InputState,
    pub boards: Vec<Board>,
    pub board_selection: SelectionState,
    pub active_board_index: Option<usize>,
    pub active_card_index: Option<usize>,
    pub columns: Vec<Column>,
    pub cards: Vec<Card>,
    pub sprints: Vec<Sprint>,
    pub sprint_selection: SelectionState,
    pub active_sprint_index: Option<usize>,
    pub active_sprint_filters: std::collections::HashSet<uuid::Uuid>,
    pub hide_assigned_cards: bool,
    pub sprint_assign_selection: SelectionState,
    pub focus: Focus,
    pub card_focus: CardFocus,
    pub board_focus: BoardFocus,
    pub import_files: Vec<String>,
    pub import_selection: SelectionState,
    pub save_file: Option<String>,
    pub app_config: AppConfig,
    pub sort_field_selection: SelectionState,
    pub current_sort_field: Option<SortField>,
    pub current_sort_order: Option<SortOrder>,
    pub selected_cards: std::collections::HashSet<uuid::Uuid>,
    pub priority_selection: SelectionState,
    pub column_selection: SelectionState,
    pub task_list_view_selection: SelectionState,
    pub sprint_task_panel: SprintTaskPanel,
    pub sprint_uncompleted_cards: CardList,
    pub sprint_completed_cards: CardList,
    pub sprint_uncompleted_component: CardListComponent,
    pub sprint_completed_component: CardListComponent,
    pub view_strategy: Box<dyn ViewStrategy>,
    pub card_list_component: CardListComponent,
    pub search: SearchState,
    pub filter_dialog_state: Option<FilterDialogState>,
    pub viewport_height: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Boards,
    Cards,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CardFocus {
    Title,
    Metadata,
    Description,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoardFocus {
    Name,
    Description,
    Settings,
    Sprints,
    Columns,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SprintTaskPanel {
    Uncompleted,
    Completed,
}

pub enum CardField {
    Title,
    Description,
}

pub enum BoardField {
    Name,
    Description,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    CreateBoard,
    CreateCard,
    CardDetail,
    RenameBoard,
    BoardDetail,
    ExportBoard,
    ExportAll,
    ImportBoard,
    SetCardPoints,
    SetCardPriority,
    SetBranchPrefix,
    OrderCards,
    SprintDetail,
    CreateSprint,
    AssignCardToSprint,
    AssignMultipleCardsToSprint,
    CreateColumn,
    RenameColumn,
    DeleteColumnConfirm,
    SelectTaskListView,
    Search,
    SetSprintPrefix,
    SetSprintCardPrefix,
    ConfirmSprintPrefixCollision,
    FilterOptions,
    Help(Box<AppMode>),
}

impl App {
    pub fn new(save_file: Option<String>) -> Self {
        let app_config = AppConfig::load();
        let mut app = Self {
            should_quit: false,
            mode: AppMode::Normal,
            input: InputState::new(),
            boards: Vec::new(),
            board_selection: SelectionState::new(),
            active_board_index: None,
            active_card_index: None,
            columns: Vec::new(),
            cards: Vec::new(),
            sprints: Vec::new(),
            sprint_selection: SelectionState::new(),
            active_sprint_index: None,
            active_sprint_filters: std::collections::HashSet::new(),
            hide_assigned_cards: false,
            sprint_assign_selection: SelectionState::new(),
            focus: Focus::Boards,
            card_focus: CardFocus::Title,
            board_focus: BoardFocus::Name,
            import_files: Vec::new(),
            import_selection: SelectionState::new(),
            save_file: save_file.clone(),
            app_config,
            sort_field_selection: SelectionState::new(),
            current_sort_field: None,
            current_sort_order: None,
            selected_cards: std::collections::HashSet::new(),
            priority_selection: SelectionState::new(),
            column_selection: SelectionState::new(),
            task_list_view_selection: SelectionState::new(),
            sprint_task_panel: SprintTaskPanel::Uncompleted,
            sprint_uncompleted_cards: CardList::new(CardListId::All),
            sprint_completed_cards: CardList::new(CardListId::All),
            sprint_uncompleted_component: CardListComponent::new(
                CardListId::All,
                CardListComponentConfig::new()
                    .with_actions(vec![
                        crate::card_list_component::CardListActionType::Navigation,
                        crate::card_list_component::CardListActionType::Selection,
                        crate::card_list_component::CardListActionType::Editing,
                        crate::card_list_component::CardListActionType::Completion,
                        crate::card_list_component::CardListActionType::Priority,
                        crate::card_list_component::CardListActionType::Sorting,
                    ])
                    .with_movement(false),
            ),
            sprint_completed_component: CardListComponent::new(
                CardListId::All,
                CardListComponentConfig::new()
                    .with_actions(vec![
                        crate::card_list_component::CardListActionType::Navigation,
                        crate::card_list_component::CardListActionType::Selection,
                        crate::card_list_component::CardListActionType::Sorting,
                    ])
                    .with_multi_select(false),
            ),
            view_strategy: Box::new(UnifiedViewStrategy::grouped()),
            card_list_component: CardListComponent::new(
                CardListId::All,
                CardListComponentConfig::new(),
            ),
            search: SearchState::new(),
            filter_dialog_state: None,
            viewport_height: 20,
        };

        if let Some(ref filename) = save_file {
            if std::path::Path::new(filename).exists() {
                if let Err(e) = app.import_board_from_file(filename) {
                    tracing::error!("Failed to load file {}: {}", filename, e);
                    app.save_file = None;
                }
            }
        }

        app.migrate_sprint_logs();
        app.check_ended_sprints();

        app
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    fn handle_key_event(
        &mut self,
        key: crossterm::event::KeyEvent,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
    ) -> bool {
        use crossterm::event::KeyCode;
        let mut should_restart_events = false;

        let is_input_mode = matches!(
            self.mode,
            AppMode::CreateBoard
                | AppMode::CreateCard
                | AppMode::CreateSprint
                | AppMode::RenameBoard
                | AppMode::ExportBoard
                | AppMode::ExportAll
                | AppMode::SetCardPoints
                | AppMode::SetBranchPrefix
                | AppMode::CreateColumn
                | AppMode::RenameColumn
                | AppMode::Search
                | AppMode::SetSprintPrefix
                | AppMode::SetSprintCardPrefix
        );

        if matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) && !is_input_mode {
            self.quit();
            return false;
        }

        if matches!(key.code, KeyCode::Char('?'))
            && !is_input_mode
            && !matches!(self.mode, AppMode::Help(_))
        {
            let previous_mode = self.mode.clone();
            self.mode = AppMode::Help(Box::new(previous_mode));
            return false;
        }

        match self.mode {
            AppMode::Normal => match key.code {
                KeyCode::Char('/') => {
                    if self.focus == Focus::Cards {
                        self.search.activate();
                        self.mode = AppMode::Search;
                    }
                }
                KeyCode::Char('n') => match self.focus {
                    Focus::Boards => self.handle_create_board_key(),
                    Focus::Cards => self.handle_create_card_key(),
                },
                KeyCode::Char('r') => self.handle_rename_board_key(),
                KeyCode::Char('e') => match self.focus {
                    Focus::Boards => self.handle_edit_board_key(),
                    Focus::Cards => {
                        should_restart_events = self.handle_edit_card_key(terminal, event_handler);
                    }
                },
                KeyCode::Char('x') => self.handle_export_board_key(),
                KeyCode::Char('X') => self.handle_export_all_key(),
                KeyCode::Char('i') => self.handle_import_board_key(),
                KeyCode::Char('a') => self.handle_assign_to_sprint_key(),
                KeyCode::Char('c') => self.handle_toggle_card_completion(),
                KeyCode::Char('o') => self.handle_order_cards_key(),
                KeyCode::Char('O') => self.handle_toggle_sort_order_key(),
                KeyCode::Char('T') => self.handle_open_filter_dialog(),
                KeyCode::Char('t') => self.handle_toggle_sprint_filter(),
                KeyCode::Char('v') => self.handle_card_selection_toggle(),
                KeyCode::Char('V') => self.handle_toggle_task_list_view(),
                KeyCode::Char('H') => self.handle_move_card_left(),
                KeyCode::Char('L') => self.handle_move_card_right(),
                KeyCode::Char('h') => self.handle_kanban_column_left(),
                KeyCode::Char('l') => self.handle_kanban_column_right(),
                KeyCode::Char('1') => self.handle_column_or_focus_switch(0),
                KeyCode::Char('2') => self.handle_column_or_focus_switch(1),
                KeyCode::Char('3') => self.handle_column_or_focus_switch(2),
                KeyCode::Char('4') => self.handle_column_or_focus_switch(3),
                KeyCode::Char('5') => self.handle_column_or_focus_switch(4),
                KeyCode::Char('6') => self.handle_column_or_focus_switch(5),
                KeyCode::Char('7') => self.handle_column_or_focus_switch(6),
                KeyCode::Char('8') => self.handle_column_or_focus_switch(7),
                KeyCode::Char('9') => self.handle_column_or_focus_switch(8),
                KeyCode::Esc => self.handle_escape_key(),
                KeyCode::Char('j') | KeyCode::Down => self.handle_navigation_down(),
                KeyCode::Char('k') | KeyCode::Up => self.handle_navigation_up(),
                KeyCode::Enter | KeyCode::Char(' ') => self.handle_selection_activate(),
                _ => {}
            },
            AppMode::CreateBoard => self.handle_create_board_dialog(key.code),
            AppMode::CreateCard => self.handle_create_card_dialog(key.code),
            AppMode::CreateSprint => self.handle_create_sprint_dialog(key.code),
            AppMode::RenameBoard => self.handle_rename_board_dialog(key.code),
            AppMode::ExportBoard => self.handle_export_board_dialog(key.code),
            AppMode::ExportAll => self.handle_export_all_dialog(key.code),
            AppMode::ImportBoard => self.handle_import_board_popup(key.code),
            AppMode::SetCardPoints => {
                should_restart_events = self.handle_set_card_points_dialog(key.code);
            }
            AppMode::SetCardPriority => self.handle_set_card_priority_popup(key.code),
            AppMode::CardDetail => {
                should_restart_events =
                    self.handle_card_detail_key(key.code, terminal, event_handler);
            }
            AppMode::BoardDetail => {
                should_restart_events =
                    self.handle_board_detail_key(key.code, terminal, event_handler);
            }
            AppMode::SetBranchPrefix => self.handle_set_branch_prefix_dialog(key.code),
            AppMode::SetSprintPrefix => self.handle_set_sprint_prefix_dialog(key.code),
            AppMode::SetSprintCardPrefix => self.handle_set_sprint_card_prefix_dialog(key.code),
            AppMode::OrderCards => {
                should_restart_events = self.handle_order_cards_popup(key.code);
            }
            AppMode::SprintDetail => self.handle_sprint_detail_key(key.code),
            AppMode::AssignCardToSprint => self.handle_assign_card_to_sprint_popup(key.code),
            AppMode::AssignMultipleCardsToSprint => {
                self.handle_assign_multiple_cards_to_sprint_popup(key.code)
            }
            AppMode::CreateColumn => self.handle_create_column_dialog(key.code),
            AppMode::RenameColumn => self.handle_rename_column_dialog(key.code),
            AppMode::DeleteColumnConfirm => self.handle_delete_column_confirm_popup(key.code),
            AppMode::SelectTaskListView => self.handle_select_task_list_view_popup(key.code),
            AppMode::Search => self.handle_search_mode(key.code),
            AppMode::ConfirmSprintPrefixCollision => {
                self.handle_confirm_sprint_prefix_collision_popup(key.code)
            }
            AppMode::FilterOptions => self.handle_filter_options_popup(key.code),
            AppMode::Help(_) => self.handle_help_mode(key.code),
        }
        should_restart_events
    }

    fn handle_search_mode(&mut self, key_code: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        match key_code {
            KeyCode::Char(c) => {
                self.search.input.insert_char(c);
                self.refresh_view();
            }
            KeyCode::Backspace => {
                self.search.input.backspace();
                self.refresh_view();
            }
            KeyCode::Esc | KeyCode::Enter => {
                self.search.deactivate();
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_help_mode(&mut self, key_code: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;
        match key_code {
            KeyCode::Esc | KeyCode::Char('?') => {
                if let AppMode::Help(previous_mode) = &self.mode {
                    self.mode = (**previous_mode).clone();
                } else {
                    self.mode = AppMode::Normal;
                }
            }
            _ => {}
        }
    }

    pub fn get_board_card_count(&self, board_id: uuid::Uuid) -> usize {
        let board_filter = BoardFilter::new(board_id, &self.columns);

        let cards: Vec<_> = self
            .cards
            .iter()
            .filter(|c| {
                if !board_filter.matches(c) {
                    return false;
                }
                true
            })
            .collect();

        if self.active_sprint_filters.is_empty() && !self.hide_assigned_cards {
            return cards.len();
        }

        cards
            .iter()
            .filter(|c| {
                if !self.active_sprint_filters.is_empty() {
                    if let Some(sprint_id) = c.sprint_id {
                        if !self.active_sprint_filters.contains(&sprint_id) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                if self.hide_assigned_cards && c.sprint_id.is_some() {
                    return false;
                }
                true
            })
            .count()
    }

    pub fn get_sorted_board_cards(&self, board_id: uuid::Uuid) -> Vec<&Card> {
        let board = self.boards.iter().find(|b| b.id == board_id).unwrap();
        let board_filter = BoardFilter::new(board_id, &self.columns);

        let mut cards: Vec<&Card> = self
            .cards
            .iter()
            .filter(|c| {
                if !board_filter.matches(c) {
                    return false;
                }
                if !self.active_sprint_filters.is_empty() {
                    if let Some(sprint_id) = c.sprint_id {
                        if !self.active_sprint_filters.contains(&sprint_id) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                if self.hide_assigned_cards && c.sprint_id.is_some() {
                    return false;
                }
                true
            })
            .collect();

        let sorter = get_sorter_for_field(board.task_sort_field);
        let ordered_sorter = OrderedSorter::new(sorter, board.task_sort_order);
        ordered_sorter.sort(&mut cards);

        cards
    }

    pub fn get_selected_card_in_context(&self) -> Option<&Card> {
        if let Some(task_list) = self.view_strategy.get_active_task_list() {
            if let Some(card_id) = task_list.get_selected_card_id() {
                return self.cards.iter().find(|c| c.id == card_id);
            }
        }
        None
    }

    pub fn get_selected_card_id(&self) -> Option<uuid::Uuid> {
        self.view_strategy
            .get_active_task_list()
            .and_then(|list| list.get_selected_card_id())
    }

    pub fn select_card_by_id(&mut self, card_id: uuid::Uuid) {
        if let Some(task_list) = self.view_strategy.get_active_task_list_mut() {
            task_list.select_card(card_id);
        }
    }

    pub fn get_sprint_cards(&self, sprint_id: uuid::Uuid) -> Vec<&Card> {
        self.cards
            .iter()
            .filter(|card| card.sprint_id == Some(sprint_id))
            .collect()
    }

    pub fn get_sprint_completed_cards(&self, sprint_id: uuid::Uuid) -> Vec<&Card> {
        let cards: Vec<&Card> = self
            .cards
            .iter()
            .filter(|card| card.sprint_id == Some(sprint_id) && card.is_completed())
            .collect();
        tracing::debug!(
            "get_sprint_completed_cards({}): found {} cards",
            sprint_id,
            cards.len()
        );
        cards
    }

    pub fn get_sprint_uncompleted_cards(&self, sprint_id: uuid::Uuid) -> Vec<&Card> {
        let cards: Vec<&Card> = self
            .cards
            .iter()
            .filter(|card| card.sprint_id == Some(sprint_id) && !card.is_completed())
            .collect();
        tracing::debug!(
            "get_sprint_uncompleted_cards({}): found {} cards",
            sprint_id,
            cards.len()
        );
        cards
    }

    pub fn populate_sprint_task_lists(&mut self, sprint_id: uuid::Uuid) {
        let uncompleted_ids: Vec<uuid::Uuid> = self
            .cards
            .iter()
            .filter(|card| card.sprint_id == Some(sprint_id) && !card.is_completed())
            .map(|card| card.id)
            .collect();

        let completed_ids: Vec<uuid::Uuid> = self
            .cards
            .iter()
            .filter(|card| card.sprint_id == Some(sprint_id) && card.is_completed())
            .map(|card| card.id)
            .collect();

        self.sprint_uncompleted_cards.update_cards(uncompleted_ids);
        self.sprint_completed_cards.update_cards(completed_ids);

        self.sprint_uncompleted_component
            .update_cards(self.sprint_uncompleted_cards.cards.clone());
        self.sprint_completed_component
            .update_cards(self.sprint_completed_cards.cards.clone());

        // Default to uncompleted panel
        self.sprint_task_panel = SprintTaskPanel::Uncompleted;
    }

    pub fn apply_sort_to_sprint_lists(&mut self, sort_field: SortField, sort_order: SortOrder) {
        let uncompleted_card_ids: Vec<uuid::Uuid> = self.sprint_uncompleted_cards.cards.clone();
        let completed_card_ids: Vec<uuid::Uuid> = self.sprint_completed_cards.cards.clone();

        let mut uncompleted_cards: Vec<&Card> = uncompleted_card_ids
            .iter()
            .filter_map(|id| self.cards.iter().find(|c| c.id == *id))
            .collect();

        let mut completed_cards: Vec<&Card> = completed_card_ids
            .iter()
            .filter_map(|id| self.cards.iter().find(|c| c.id == *id))
            .collect();

        let sorter = get_sorter_for_field(sort_field);
        let ordered_sorter = OrderedSorter::new(sorter, sort_order);

        ordered_sorter.sort(&mut uncompleted_cards);
        ordered_sorter.sort(&mut completed_cards);

        let sorted_uncompleted_ids: Vec<uuid::Uuid> =
            uncompleted_cards.iter().map(|c| c.id).collect();
        let sorted_completed_ids: Vec<uuid::Uuid> = completed_cards.iter().map(|c| c.id).collect();

        self.sprint_uncompleted_cards
            .update_cards(sorted_uncompleted_ids);
        self.sprint_completed_cards
            .update_cards(sorted_completed_ids);

        self.sprint_uncompleted_component
            .update_cards(self.sprint_uncompleted_cards.cards.clone());
        self.sprint_completed_component
            .update_cards(self.sprint_completed_cards.cards.clone());
    }

    pub fn calculate_points(cards: &[&Card]) -> u32 {
        cards
            .iter()
            .filter_map(|card| card.points.map(|p| p as u32))
            .sum()
    }

    pub fn refresh_view(&mut self) {
        let board_idx = self.active_board_index.or(self.board_selection.get());
        if let Some(idx) = board_idx {
            if let Some(board) = self.boards.get(idx) {
                let search_query = if self.search.is_active {
                    Some(self.search.query())
                } else {
                    None
                };
                let ctx = ViewRefreshContext {
                    board,
                    all_cards: &self.cards,
                    all_columns: &self.columns,
                    all_sprints: &self.sprints,
                    active_sprint_filters: self.active_sprint_filters.clone(),
                    hide_assigned_cards: self.hide_assigned_cards,
                    search_query,
                };
                self.view_strategy.refresh_task_lists(&ctx);
            }
        }
        self.sync_card_list_component();
    }

    pub fn sync_card_list_component(&mut self) {
        if let Some(active_list) = self.view_strategy.get_active_task_list() {
            self.card_list_component
                .update_cards(active_list.cards.clone());
        }
    }

    pub fn switch_view_strategy(&mut self, task_list_view: kanban_domain::TaskListView) {
        let new_strategy: Box<dyn ViewStrategy> = match task_list_view {
            kanban_domain::TaskListView::Flat => Box::new(UnifiedViewStrategy::flat()),
            kanban_domain::TaskListView::GroupedByColumn => {
                Box::new(UnifiedViewStrategy::grouped())
            }
            kanban_domain::TaskListView::ColumnView => Box::new(UnifiedViewStrategy::kanban()),
        };

        self.view_strategy = new_strategy;
        self.refresh_view();
    }

    pub fn export_board_with_filename(&self) -> io::Result<()> {
        if let Some(board_idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get(board_idx) {
                let board_export =
                    BoardExporter::export_board(board, &self.columns, &self.cards, &self.sprints);

                let export = crate::export::AllBoardsExport {
                    boards: vec![board_export],
                };

                BoardExporter::export_to_file(&export, self.input.as_str())?;
            }
        }
        Ok(())
    }

    pub fn export_all_boards_with_filename(&self) -> io::Result<()> {
        let export = BoardExporter::export_all_boards(
            &self.boards,
            &self.columns,
            &self.cards,
            &self.sprints,
        );
        BoardExporter::export_to_file(&export, self.input.as_str())?;
        Ok(())
    }

    pub fn auto_save(&self) -> io::Result<()> {
        if let Some(ref filename) = self.save_file {
            let export = BoardExporter::export_all_boards(
                &self.boards,
                &self.columns,
                &self.cards,
                &self.sprints,
            );
            BoardExporter::export_to_file(&export, filename)?;
        }
        Ok(())
    }

    fn check_ended_sprints(&self) {
        let ended_sprints: Vec<_> = self.sprints.iter().filter(|s| s.is_ended()).collect();

        if !ended_sprints.is_empty() {
            tracing::warn!(
                "Found {} ended sprint(s) that need attention:",
                ended_sprints.len()
            );
            for sprint in &ended_sprints {
                if let Some(board) = self.boards.iter().find(|b| b.id == sprint.board_id) {
                    tracing::warn!(
                        "  - {} (ended: {})",
                        sprint.formatted_name(board, "sprint"),
                        sprint
                            .end_date
                            .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    );
                }
            }
        }
    }

    pub fn edit_board_field(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
        field: BoardField,
    ) -> io::Result<()> {
        if let Some(board_idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get(board_idx) {
                let temp_dir = std::env::temp_dir();
                let (temp_file, current_content) = match field {
                    BoardField::Name => {
                        let temp_file = temp_dir.join(format!("kanban-board-{}-name.md", board.id));
                        (temp_file, board.name.clone())
                    }
                    BoardField::Description => {
                        let temp_file =
                            temp_dir.join(format!("kanban-board-{}-description.md", board.id));
                        let content = board.description.as_deref().unwrap_or("").to_string();
                        (temp_file, content)
                    }
                };

                if let Some(new_content) =
                    edit_in_external_editor(terminal, event_handler, temp_file, &current_content)?
                {
                    let board_id = board.id;
                    if let Some(board) = self.boards.iter_mut().find(|b| b.id == board_id) {
                        match field {
                            BoardField::Name => {
                                if !new_content.trim().is_empty() {
                                    board.update_name(new_content.trim().to_string());
                                }
                            }
                            BoardField::Description => {
                                let desc = if new_content.trim().is_empty() {
                                    None
                                } else {
                                    Some(new_content)
                                };
                                board.update_description(desc);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn edit_card_field(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
        field: CardField,
    ) -> io::Result<()> {
        if let Some(card_idx) = self.active_card_index {
            if let Some(card) = self.cards.get(card_idx) {
                let temp_dir = std::env::temp_dir();
                let (temp_file, current_content) = match field {
                    CardField::Title => {
                        let temp_file = temp_dir.join(format!("kanban-card-{}-title.md", card.id));
                        (temp_file, card.title.clone())
                    }
                    CardField::Description => {
                        let temp_file =
                            temp_dir.join(format!("kanban-card-{}-description.md", card.id));
                        let content = card.description.as_deref().unwrap_or("").to_string();
                        (temp_file, content)
                    }
                };

                if let Some(new_content) =
                    edit_in_external_editor(terminal, event_handler, temp_file, &current_content)?
                {
                    let card_id = card.id;
                    if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) {
                        match field {
                            CardField::Title => {
                                if !new_content.trim().is_empty() {
                                    card.update_title(new_content.trim().to_string());
                                }
                            }
                            CardField::Description => {
                                let desc = if new_content.trim().is_empty() {
                                    None
                                } else {
                                    Some(new_content)
                                };
                                card.update_description(desc);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn edit_entity_json_impl<T: Editable<E>, E>(
        entity: &mut E,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
        temp_file: std::path::PathBuf,
    ) -> io::Result<()> {
        let dto = T::from_entity(entity);
        let current_content =
            serde_json::to_string_pretty(&dto).unwrap_or_else(|_| "{}".to_string());

        if let Some(new_content) =
            edit_in_external_editor(terminal, event_handler, temp_file, &current_content)?
        {
            match serde_json::from_str::<T>(&new_content) {
                Ok(updated_dto) => {
                    updated_dto.apply_to(entity);
                    tracing::info!("Updated entity via JSON editor");
                }
                Err(e) => {
                    tracing::error!("Failed to parse JSON: {}", e);
                }
            }
        }

        Ok(())
    }

    pub async fn run(&mut self) -> KanbanResult<()> {
        let mut terminal = setup_terminal()?;

        while !self.should_quit {
            let mut events = EventHandler::new();

            loop {
                terminal.draw(|frame| ui::render(self, frame))?;

                if let Some(event) = events.next().await {
                    match event {
                        Event::Key(key) => {
                            let should_restart = self.handle_key_event(key, &mut terminal, &events);
                            if should_restart {
                                break;
                            }
                        }
                        Event::Tick => {}
                    }
                }

                if self.should_quit {
                    break;
                }
            }

            if self.should_quit {
                break;
            }
        }

        if let Err(e) = self.auto_save() {
            tracing::error!("Failed to auto-save: {}", e);
        }

        restore_terminal(&mut terminal)?;
        Ok(())
    }

    pub fn import_board_from_file(&mut self, filename: &str) -> io::Result<()> {
        let first_new_index = self.boards.len();
        let import = BoardImporter::import_from_file(filename)?;
        let (boards, columns, cards, sprints) = BoardImporter::extract_entities(import);

        self.boards.extend(boards);
        self.columns.extend(columns);
        self.cards.extend(cards);
        self.sprints.extend(sprints);

        self.board_selection.set(Some(first_new_index));

        self.switch_view_strategy(kanban_domain::TaskListView::GroupedByColumn);

        Ok(())
    }

    fn migrate_sprint_logs(&mut self) {
        let mut migrated_count = 0;

        for card in &mut self.cards {
            if let Some(sprint_id) = card.sprint_id {
                if card.sprint_logs.is_empty() {
                    if let Some(sprint) = self.sprints.iter().find(|s| s.id == sprint_id) {
                        let sprint_log = kanban_domain::SprintLog::new(
                            sprint_id,
                            sprint.sprint_number,
                            sprint.name_index.and_then(|idx| {
                                self.boards
                                    .iter()
                                    .find(|b| b.id == sprint.board_id)
                                    .and_then(|board| board.sprint_names.get(idx).cloned())
                            }),
                            format!("{:?}", sprint.status),
                        );
                        card.sprint_logs.push(sprint_log);
                        migrated_count += 1;
                    }
                }
            }
        }

        if migrated_count > 0 {
            tracing::info!("Migrated sprint logs for {} cards", migrated_count);
        }
    }

    /// Generic handler for copying card outputs to clipboard
    fn copy_card_output<F>(&mut self, output_type: &str, get_output: F)
    where
        F: Fn(&Card, &Board, &[Sprint], &str) -> String,
    {
        if let Some(card_idx) = self.active_card_index {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.boards.get(board_idx) {
                    if let Some(card) = self.cards.get(card_idx) {
                        let output = get_output(
                            card,
                            board,
                            &self.sprints,
                            self.app_config.effective_default_card_prefix(),
                        );
                        if let Err(e) = clipboard::copy_to_clipboard(&output) {
                            tracing::error!("Failed to copy to clipboard: {}", e);
                        } else {
                            tracing::info!("Copied {}: {}", output_type, output);
                        }
                    }
                }
            }
        }
    }

    pub fn copy_branch_name(&mut self) {
        self.copy_card_output("branch name", |card, board, sprints, prefix| {
            card.branch_name(board, sprints, prefix)
        });
    }

    pub fn copy_git_checkout_command(&mut self) {
        self.copy_card_output("command", |card, board, sprints, prefix| {
            card.git_checkout_command(board, sprints, prefix)
        });
    }

    pub fn get_current_priority_selection_index(&self) -> usize {
        if let Some(card_idx) = self.active_card_index {
            if let Some(card) = self.cards.get(card_idx) {
                use kanban_domain::CardPriority;
                return match card.priority {
                    CardPriority::Low => 0,
                    CardPriority::Medium => 1,
                    CardPriority::High => 2,
                    CardPriority::Critical => 3,
                };
            }
        }
        0
    }

    pub fn get_current_sprint_selection_index(&self) -> usize {
        if let Some(card_idx) = self.active_card_index {
            if let Some(card) = self.cards.get(card_idx) {
                if let Some(card_sprint_id) = card.sprint_id {
                    if let Some(board_idx) = self.active_board_index {
                        if let Some(board) = self.boards.get(board_idx) {
                            let board_sprints: Vec<_> = self
                                .sprints
                                .iter()
                                .filter(|s| s.board_id == board.id)
                                .collect();
                            for (idx, sprint) in board_sprints.iter().enumerate() {
                                if sprint.id == card_sprint_id {
                                    return idx + 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        0
    }

    pub fn get_current_sort_field_selection_index(&self) -> usize {
        if let Some(sort_field) = self.current_sort_field {
            return match sort_field {
                SortField::Points => 0,
                SortField::Priority => 1,
                SortField::CreatedAt => 2,
                SortField::UpdatedAt => 3,
                SortField::Status => 4,
                SortField::Default => 5,
            };
        }
        0
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

impl Default for App {
    fn default() -> Self {
        Self::new(None)
    }
}
