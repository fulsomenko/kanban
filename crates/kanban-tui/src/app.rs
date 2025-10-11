use crate::{
    clipboard,
    dialog::{handle_dialog_input, DialogAction},
    editor::edit_in_external_editor,
    events::{Event, EventHandler},
    input::InputState,
    selection::SelectionState,
    ui,
};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use kanban_core::{AppConfig, KanbanResult};
use kanban_domain::{Board, Card, CardStatus, Column, SortField, SortOrder};
use ratatui::{backend::CrosstermBackend, Terminal};
use serde::Serialize;
use std::io;

pub struct App {
    pub should_quit: bool,
    pub mode: AppMode,
    pub input: InputState,
    pub boards: Vec<Board>,
    pub board_selection: SelectionState,
    pub active_board_index: Option<usize>,
    pub card_selection: SelectionState,
    pub active_card_index: Option<usize>,
    pub columns: Vec<Column>,
    pub cards: Vec<Card>,
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Projects,
    Tasks,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CardFocus {
    Title,
    Metadata,
    Description,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoardFocus {
    Name,
    Description,
}

enum CardField {
    Title,
    Description,
}

enum BoardField {
    Name,
    Description,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    CreateBoard,
    CreateCard,
    CardDetail,
    RenameBoard,
    BoardDetail,
    BoardSettings,
    ExportBoard,
    ExportAll,
    ImportBoard,
    SetCardPoints,
    SetBranchPrefix,
    OrderTasks,
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
            card_selection: SelectionState::new(),
            active_card_index: None,
            columns: Vec::new(),
            cards: Vec::new(),
            focus: Focus::Projects,
            card_focus: CardFocus::Title,
            board_focus: BoardFocus::Name,
            import_files: Vec::new(),
            import_selection: SelectionState::new(),
            save_file: save_file.clone(),
            app_config,
            sort_field_selection: SelectionState::new(),
            current_sort_field: None,
            current_sort_order: None,
        };

        if let Some(ref filename) = save_file {
            if std::path::Path::new(filename).exists() {
                if let Err(e) = app.import_board_from_file(filename) {
                    tracing::error!("Failed to load file {}: {}", filename, e);
                    app.save_file = None;
                }
            }
        }

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

        if matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) {
            self.quit();
            return false;
        }

        match self.mode {
            AppMode::Normal => match key.code {
                KeyCode::Char('n') => match self.focus {
                    Focus::Projects => {
                        self.mode = AppMode::CreateBoard;
                        self.input.clear();
                    }
                    Focus::Tasks => {
                        if self.active_board_index.is_some() {
                            self.mode = AppMode::CreateCard;
                            self.input.clear();
                        }
                    }
                },
                KeyCode::Char('r') => {
                    if self.focus == Focus::Projects && self.board_selection.get().is_some() {
                        if let Some(board_idx) = self.board_selection.get() {
                            if let Some(board) = self.boards.get(board_idx) {
                                self.input.set(board.name.clone());
                                self.mode = AppMode::RenameBoard;
                            }
                        }
                    }
                }
                KeyCode::Char('e') => {
                    if self.focus == Focus::Projects && self.board_selection.get().is_some() {
                        self.mode = AppMode::BoardDetail;
                        self.board_focus = BoardFocus::Name;
                    }
                }
                KeyCode::Char('s') => {
                    if self.focus == Focus::Projects && self.board_selection.get().is_some() {
                        self.mode = AppMode::BoardSettings;
                    }
                }
                KeyCode::Char('x') => {
                    if self.focus == Focus::Projects && self.board_selection.get().is_some() {
                        if let Some(board_idx) = self.board_selection.get() {
                            if let Some(board) = self.boards.get(board_idx) {
                                let filename = format!(
                                    "{}-{}.json",
                                    board.name.replace(" ", "-").to_lowercase(),
                                    chrono::Utc::now().format("%Y%m%d-%H%M%S")
                                );
                                self.input.set(filename);
                                self.mode = AppMode::ExportBoard;
                            }
                        }
                    }
                }
                KeyCode::Char('X') => {
                    if self.focus == Focus::Projects && !self.boards.is_empty() {
                        let filename = format!(
                            "kanban-all-{}.json",
                            chrono::Utc::now().format("%Y%m%d-%H%M%S")
                        );
                        self.input.set(filename);
                        self.mode = AppMode::ExportAll;
                    }
                }
                KeyCode::Char('i') => {
                    if self.focus == Focus::Projects {
                        self.scan_import_files();
                        if !self.import_files.is_empty() {
                            self.import_selection.set(Some(0));
                            self.mode = AppMode::ImportBoard;
                        }
                    }
                }
                KeyCode::Char('c') => {
                    if self.focus == Focus::Tasks && self.card_selection.get().is_some() {
                        self.toggle_card_completion();
                    }
                }
                KeyCode::Char('o') => {
                    if self.focus == Focus::Tasks && self.active_board_index.is_some() {
                        self.sort_field_selection.set(Some(0));
                        self.mode = AppMode::OrderTasks;
                    }
                }
                KeyCode::Char('O') => {
                    if self.focus == Focus::Tasks && self.active_board_index.is_some() {
                        if let Some(current_order) = self.current_sort_order {
                            let new_order = match current_order {
                                SortOrder::Ascending => SortOrder::Descending,
                                SortOrder::Descending => SortOrder::Ascending,
                            };
                            self.current_sort_order = Some(new_order);

                            if let Some(board_idx) = self.active_board_index {
                                if let Some(board) = self.boards.get_mut(board_idx) {
                                    if let Some(field) = self.current_sort_field {
                                        board.update_task_sort(field, new_order);
                                    }
                                }
                            }

                            tracing::info!("Toggled sort order to: {:?}", new_order);
                        }
                    }
                }
                KeyCode::Char('1') => self.focus = Focus::Projects,
                KeyCode::Char('2') => {
                    if self.active_board_index.is_some() {
                        self.focus = Focus::Tasks;
                    }
                }
                KeyCode::Esc => {
                    if self.active_board_index.is_some() {
                        self.active_board_index = None;
                        self.card_selection.clear();
                        self.focus = Focus::Projects;
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => match self.focus {
                    Focus::Projects => {
                        self.board_selection.next(self.boards.len());
                    }
                    Focus::Tasks => {
                        if let Some(board_idx) = self.active_board_index {
                            if let Some(board) = self.boards.get(board_idx) {
                                let card_count = self.get_board_card_count(board.id);
                                self.card_selection.next(card_count);
                            }
                        }
                    }
                },
                KeyCode::Char('k') | KeyCode::Up => match self.focus {
                    Focus::Projects => {
                        self.board_selection.prev();
                    }
                    Focus::Tasks => {
                        self.card_selection.prev();
                    }
                },
                KeyCode::Enter | KeyCode::Char(' ') => match self.focus {
                    Focus::Projects => {
                        if self.board_selection.get().is_some() {
                            self.active_board_index = self.board_selection.get();
                            self.card_selection.clear();

                            if let Some(board_idx) = self.active_board_index {
                                if let Some(board) = self.boards.get(board_idx) {
                                    self.current_sort_field = Some(board.task_sort_field);
                                    self.current_sort_order = Some(board.task_sort_order);

                                    let card_count = self.get_board_card_count(board.id);
                                    if card_count > 0 {
                                        self.card_selection.set(Some(0));
                                    }
                                }
                            }

                            self.focus = Focus::Tasks;
                        }
                    }
                    Focus::Tasks => {
                        if let Some(sorted_idx) = self.card_selection.get() {
                            if let Some(board_idx) = self.active_board_index {
                                if let Some(board) = self.boards.get(board_idx) {
                                    let sorted_cards = self.get_sorted_board_cards(board.id);
                                    if let Some(selected_card) = sorted_cards.get(sorted_idx) {
                                        let card_id = selected_card.id;
                                        let actual_idx =
                                            self.cards.iter().position(|c| c.id == card_id);
                                        self.active_card_index = actual_idx;
                                        self.mode = AppMode::CardDetail;
                                    }
                                }
                            }
                        }
                    }
                },
                _ => {}
            },
            AppMode::CreateBoard => match handle_dialog_input(&mut self.input, key.code) {
                DialogAction::Confirm => {
                    self.create_board();
                    self.mode = AppMode::Normal;
                    self.input.clear();
                }
                DialogAction::Cancel => {
                    self.mode = AppMode::Normal;
                    self.input.clear();
                }
                DialogAction::None => {}
            },
            AppMode::CreateCard => match handle_dialog_input(&mut self.input, key.code) {
                DialogAction::Confirm => {
                    self.create_task();
                    self.mode = AppMode::Normal;
                    self.input.clear();
                }
                DialogAction::Cancel => {
                    self.mode = AppMode::Normal;
                    self.input.clear();
                }
                DialogAction::None => {}
            },
            AppMode::RenameBoard => match handle_dialog_input(&mut self.input, key.code) {
                DialogAction::Confirm => {
                    self.rename_board();
                    self.mode = AppMode::Normal;
                    self.input.clear();
                }
                DialogAction::Cancel => {
                    self.mode = AppMode::Normal;
                    self.input.clear();
                }
                DialogAction::None => {}
            },
            AppMode::ExportBoard => match handle_dialog_input(&mut self.input, key.code) {
                DialogAction::Confirm => {
                    if let Err(e) = self.export_board_with_filename() {
                        tracing::error!("Failed to export board: {}", e);
                    }
                    self.mode = AppMode::Normal;
                    self.input.clear();
                }
                DialogAction::Cancel => {
                    self.mode = AppMode::Normal;
                    self.input.clear();
                }
                DialogAction::None => {}
            },
            AppMode::ExportAll => match handle_dialog_input(&mut self.input, key.code) {
                DialogAction::Confirm => {
                    if let Err(e) = self.export_all_boards_with_filename() {
                        tracing::error!("Failed to export all boards: {}", e);
                    }
                    self.mode = AppMode::Normal;
                    self.input.clear();
                }
                DialogAction::Cancel => {
                    self.mode = AppMode::Normal;
                    self.input.clear();
                }
                DialogAction::None => {}
            },
            AppMode::ImportBoard => match key.code {
                KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                    self.import_selection.clear();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.import_selection.next(self.import_files.len());
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.import_selection.prev();
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(idx) = self.import_selection.get() {
                        if let Some(filename) = self.import_files.get(idx).cloned() {
                            if let Err(e) = self.import_board_from_file(&filename) {
                                tracing::error!("Failed to import board: {}", e);
                            }
                        }
                    }
                    self.mode = AppMode::Normal;
                    self.import_selection.clear();
                }
                _ => {}
            },
            AppMode::SetCardPoints => match handle_dialog_input(&mut self.input, key.code) {
                DialogAction::Confirm => {
                    let input_str = self.input.as_str().trim();
                    let points = if input_str.is_empty() {
                        None
                    } else if let Ok(p) = input_str.parse::<u8>() {
                        if (1..=5).contains(&p) {
                            Some(p)
                        } else {
                            tracing::error!("Points must be between 1-5");
                            self.mode = AppMode::CardDetail;
                            self.input.clear();
                            return should_restart_events;
                        }
                    } else {
                        tracing::error!("Invalid points value");
                        self.mode = AppMode::CardDetail;
                        self.input.clear();
                        return should_restart_events;
                    };

                    if let Some(card_idx) = self.active_card_index {
                        if let Some(board_idx) = self.active_board_index {
                            if let Some(board) = self.boards.get(board_idx) {
                                let board_cards: Vec<_> = self
                                    .cards
                                    .iter()
                                    .filter(|card| {
                                        self.columns.iter().any(|col| {
                                            col.id == card.column_id && col.board_id == board.id
                                        })
                                    })
                                    .collect();

                                if let Some(card) = board_cards.get(card_idx) {
                                    let card_id = card.id;
                                    if let Some(card) =
                                        self.cards.iter_mut().find(|c| c.id == card_id)
                                    {
                                        card.set_points(points);
                                        tracing::info!("Set points to: {:?}", points);
                                    }
                                }
                            }
                        }
                    }
                    self.mode = AppMode::CardDetail;
                    self.input.clear();
                }
                DialogAction::Cancel => {
                    self.mode = AppMode::CardDetail;
                    self.input.clear();
                }
                DialogAction::None => {}
            },
            AppMode::CardDetail => match key.code {
                KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                    self.active_card_index = None;
                    self.card_focus = CardFocus::Title;
                }
                KeyCode::Char('1') => {
                    self.card_focus = CardFocus::Title;
                }
                KeyCode::Char('2') => {
                    self.card_focus = CardFocus::Metadata;
                }
                KeyCode::Char('3') => {
                    self.card_focus = CardFocus::Description;
                }
                KeyCode::Char('y') => {
                    self.copy_branch_name();
                }
                KeyCode::Char('Y') => {
                    self.copy_git_checkout_command();
                }
                KeyCode::Char('e') => match self.card_focus {
                    CardFocus::Title => {
                        if let Err(e) =
                            self.edit_card_field(terminal, event_handler, CardField::Title)
                        {
                            tracing::error!("Failed to edit title: {}", e);
                        }
                        should_restart_events = true;
                    }
                    CardFocus::Description => {
                        if let Err(e) =
                            self.edit_card_field(terminal, event_handler, CardField::Description)
                        {
                            tracing::error!("Failed to edit description: {}", e);
                        }
                        should_restart_events = true;
                    }
                    CardFocus::Metadata => {
                        self.input.clear();
                        self.mode = AppMode::SetCardPoints;
                    }
                },
                _ => {}
            },
            AppMode::BoardDetail => match key.code {
                KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                    self.board_focus = BoardFocus::Name;
                }
                KeyCode::Char('1') => {
                    self.board_focus = BoardFocus::Name;
                }
                KeyCode::Char('2') => {
                    self.board_focus = BoardFocus::Description;
                }
                KeyCode::Char('e') => match self.board_focus {
                    BoardFocus::Name => {
                        if let Err(e) =
                            self.edit_board_field(terminal, event_handler, BoardField::Name)
                        {
                            tracing::error!("Failed to edit board name: {}", e);
                        }
                        should_restart_events = true;
                    }
                    BoardFocus::Description => {
                        if let Err(e) =
                            self.edit_board_field(terminal, event_handler, BoardField::Description)
                        {
                            tracing::error!("Failed to edit board description: {}", e);
                        }
                        should_restart_events = true;
                    }
                },
                _ => {}
            },
            AppMode::BoardSettings => match key.code {
                KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                }
                KeyCode::Char('p') => {
                    if let Some(board_idx) = self.board_selection.get() {
                        if let Some(board) = self.boards.get(board_idx) {
                            let current_prefix =
                                board.branch_prefix.clone().unwrap_or_else(String::new);
                            self.input.set(current_prefix);
                            self.mode = AppMode::SetBranchPrefix;
                        }
                    }
                }
                _ => {}
            },
            AppMode::SetBranchPrefix => match handle_dialog_input(&mut self.input, key.code) {
                DialogAction::Confirm => {
                    let prefix_str = self.input.as_str().trim();
                    if prefix_str.is_empty() {
                        if let Some(board_idx) = self.board_selection.get() {
                            if let Some(board) = self.boards.get_mut(board_idx) {
                                board.update_branch_prefix(None);
                                tracing::info!("Cleared branch prefix");
                            }
                        }
                    } else if Card::validate_branch_prefix(prefix_str) {
                        if let Some(board_idx) = self.board_selection.get() {
                            if let Some(board) = self.boards.get_mut(board_idx) {
                                board.update_branch_prefix(Some(prefix_str.to_string()));
                                tracing::info!("Set branch prefix to: {}", prefix_str);
                            }
                        }
                    } else {
                        tracing::error!(
                            "Invalid prefix: use alphanumeric, hyphens, underscores only"
                        );
                    }
                    self.mode = AppMode::BoardSettings;
                    self.input.clear();
                }
                DialogAction::Cancel => {
                    self.mode = AppMode::BoardSettings;
                    self.input.clear();
                }
                DialogAction::None => {}
            },
            AppMode::OrderTasks => match key.code {
                KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                    self.sort_field_selection.clear();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.sort_field_selection.next(6);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.sort_field_selection.prev();
                }
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('a') | KeyCode::Char('d') => {
                    if let Some(field_idx) = self.sort_field_selection.get() {
                        let field = match field_idx {
                            0 => SortField::Points,
                            1 => SortField::Priority,
                            2 => SortField::CreatedAt,
                            3 => SortField::UpdatedAt,
                            4 => SortField::Status,
                            5 => SortField::Default,
                            _ => return should_restart_events,
                        };

                        let order = if self.current_sort_field == Some(field)
                            && matches!(key.code, KeyCode::Enter | KeyCode::Char(' '))
                        {
                            match self.current_sort_order {
                                Some(SortOrder::Ascending) => SortOrder::Descending,
                                Some(SortOrder::Descending) => SortOrder::Ascending,
                                None => SortOrder::Ascending,
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('d') => SortOrder::Descending,
                                _ => SortOrder::Ascending,
                            }
                        };

                        self.current_sort_field = Some(field);
                        self.current_sort_order = Some(order);

                        if let Some(board_idx) = self.active_board_index {
                            if let Some(board) = self.boards.get_mut(board_idx) {
                                board.update_task_sort(field, order);
                            }
                        }

                        self.mode = AppMode::Normal;
                        self.sort_field_selection.clear();

                        tracing::info!("Sorting by {:?} ({:?})", field, order);
                    }
                }
                _ => {}
            },
        }
        should_restart_events
    }

    fn create_board(&mut self) {
        let board = Board::new(self.input.as_str().to_string(), None);
        tracing::info!("Creating project: {} (id: {})", board.name, board.id);
        self.boards.push(board);
        let new_index = self.boards.len() - 1;
        self.board_selection.set(Some(new_index));
    }

    fn rename_board(&mut self) {
        if let Some(idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get_mut(idx) {
                board.update_name(self.input.as_str().to_string());
                tracing::info!("Renamed project to: {}", board.name);
            }
        }
    }

    fn toggle_card_completion(&mut self) {
        if let Some(sorted_idx) = self.card_selection.get() {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.boards.get(board_idx) {
                    let sorted_cards = self.get_sorted_board_cards(board.id);

                    if let Some(card) = sorted_cards.get(sorted_idx) {
                        let card_id = card.id;
                        let new_status = if card.status == CardStatus::Done {
                            CardStatus::Todo
                        } else {
                            CardStatus::Done
                        };

                        if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) {
                            card.update_status(new_status);
                            tracing::info!(
                                "Toggled card '{}' to status: {:?}",
                                card.title,
                                new_status
                            );
                        }
                    }
                }
            }
        }
    }

    fn create_task(&mut self) {
        if let Some(idx) = self.active_board_index {
            if let Some(board) = self.boards.get_mut(idx) {
                let column = self
                    .columns
                    .iter()
                    .find(|col| col.board_id == board.id)
                    .cloned();

                let column = match column {
                    Some(col) => col,
                    None => {
                        let new_column = Column::new(board.id, "Todo".to_string(), 0);
                        self.columns.push(new_column.clone());
                        new_column
                    }
                };

                let position = self
                    .cards
                    .iter()
                    .filter(|c| c.column_id == column.id)
                    .count() as i32;
                let card = Card::new(board, column.id, self.input.as_str().to_string(), position);
                let board_id = board.id;
                tracing::info!("Creating card: {} (id: {})", card.title, card.id);
                self.cards.push(card);

                let card_count = self.get_board_card_count(board_id);
                let new_card_index = card_count.saturating_sub(1);
                self.card_selection.set(Some(new_card_index));
            }
        }
    }

    fn get_board_card_count(&self, board_id: uuid::Uuid) -> usize {
        self.cards
            .iter()
            .filter(|card| {
                self.columns
                    .iter()
                    .any(|col| col.id == card.column_id && col.board_id == board_id)
            })
            .count()
    }

    pub fn get_sorted_board_cards(&self, board_id: uuid::Uuid) -> Vec<&Card> {
        let board = self.boards.iter().find(|b| b.id == board_id).unwrap();
        let sort_field = board.task_sort_field;
        let sort_order = board.task_sort_order;

        let mut cards: Vec<&Card> = self
            .cards
            .iter()
            .filter(|card| {
                self.columns
                    .iter()
                    .any(|col| col.id == card.column_id && col.board_id == board_id)
            })
            .collect();

        cards.sort_by(|a, b| {
            use std::cmp::Ordering;
            let cmp = match sort_field {
                SortField::Points => match (a.points, b.points) {
                    (Some(ap), Some(bp)) => ap.cmp(&bp),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                },
                SortField::Priority => {
                    let a_val = match a.priority {
                        kanban_domain::CardPriority::Critical => 3,
                        kanban_domain::CardPriority::High => 2,
                        kanban_domain::CardPriority::Medium => 1,
                        kanban_domain::CardPriority::Low => 0,
                    };
                    let b_val = match b.priority {
                        kanban_domain::CardPriority::Critical => 3,
                        kanban_domain::CardPriority::High => 2,
                        kanban_domain::CardPriority::Medium => 1,
                        kanban_domain::CardPriority::Low => 0,
                    };
                    a_val.cmp(&b_val)
                }
                SortField::CreatedAt => a.created_at.cmp(&b.created_at),
                SortField::UpdatedAt => a.updated_at.cmp(&b.updated_at),
                SortField::Status => {
                    let a_val = match a.status {
                        CardStatus::Done => 3,
                        CardStatus::InProgress => 2,
                        CardStatus::Blocked => 1,
                        CardStatus::Todo => 0,
                    };
                    let b_val = match b.status {
                        CardStatus::Done => 3,
                        CardStatus::InProgress => 2,
                        CardStatus::Blocked => 1,
                        CardStatus::Todo => 0,
                    };
                    a_val.cmp(&b_val)
                }
                SortField::Default => a.card_number.cmp(&b.card_number),
            };

            match sort_order {
                SortOrder::Ascending => cmp,
                SortOrder::Descending => cmp.reverse(),
            }
        });

        cards
    }

    pub fn export_board_with_filename(&self) -> io::Result<()> {
        #[derive(Serialize)]
        struct BoardExport {
            board: Board,
            columns: Vec<Column>,
            cards: Vec<Card>,
        }

        #[derive(Serialize)]
        struct AllBoardsExport {
            boards: Vec<BoardExport>,
        }

        if let Some(board_idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get(board_idx) {
                let board_columns: Vec<Column> = self
                    .columns
                    .iter()
                    .filter(|col| col.board_id == board.id)
                    .cloned()
                    .collect();

                let column_ids: Vec<uuid::Uuid> = board_columns.iter().map(|c| c.id).collect();

                let board_cards: Vec<Card> = self
                    .cards
                    .iter()
                    .filter(|card| column_ids.contains(&card.column_id))
                    .cloned()
                    .collect();

                let board_export = BoardExport {
                    board: board.clone(),
                    columns: board_columns,
                    cards: board_cards,
                };

                let export = AllBoardsExport {
                    boards: vec![board_export],
                };

                let json = serde_json::to_string_pretty(&export).map_err(io::Error::other)?;

                std::fs::write(self.input.as_str(), json)?;
                tracing::info!("Exported board to: {}", self.input.as_str());
            }
        }
        Ok(())
    }

    pub fn export_all_boards_with_filename(&self) -> io::Result<()> {
        #[derive(Serialize)]
        struct BoardExport {
            board: Board,
            columns: Vec<Column>,
            cards: Vec<Card>,
        }

        #[derive(Serialize)]
        struct AllBoardsExport {
            boards: Vec<BoardExport>,
        }

        let mut board_exports = Vec::new();

        for board in &self.boards {
            let board_columns: Vec<Column> = self
                .columns
                .iter()
                .filter(|col| col.board_id == board.id)
                .cloned()
                .collect();

            let column_ids: Vec<uuid::Uuid> = board_columns.iter().map(|c| c.id).collect();

            let board_cards: Vec<Card> = self
                .cards
                .iter()
                .filter(|card| column_ids.contains(&card.column_id))
                .cloned()
                .collect();

            board_exports.push(BoardExport {
                board: board.clone(),
                columns: board_columns,
                cards: board_cards,
            });
        }

        let export = AllBoardsExport {
            boards: board_exports,
        };

        let json = serde_json::to_string_pretty(&export).map_err(io::Error::other)?;

        std::fs::write(self.input.as_str(), json)?;
        tracing::info!("Exported all boards to: {}", self.input.as_str());

        Ok(())
    }

    pub fn auto_save(&self) -> io::Result<()> {
        if let Some(ref filename) = self.save_file {
            #[derive(Serialize)]
            struct BoardExport {
                board: Board,
                columns: Vec<Column>,
                cards: Vec<Card>,
            }

            #[derive(Serialize)]
            struct AllBoardsExport {
                boards: Vec<BoardExport>,
            }

            let mut board_exports = Vec::new();

            for board in &self.boards {
                let board_columns: Vec<Column> = self
                    .columns
                    .iter()
                    .filter(|col| col.board_id == board.id)
                    .cloned()
                    .collect();

                let column_ids: Vec<uuid::Uuid> = board_columns.iter().map(|c| c.id).collect();

                let board_cards: Vec<Card> = self
                    .cards
                    .iter()
                    .filter(|card| column_ids.contains(&card.column_id))
                    .cloned()
                    .collect();

                board_exports.push(BoardExport {
                    board: board.clone(),
                    columns: board_columns,
                    cards: board_cards,
                });
            }

            let export = AllBoardsExport {
                boards: board_exports,
            };

            let json = serde_json::to_string_pretty(&export).map_err(io::Error::other)?;

            std::fs::write(filename, json)?;
            tracing::info!("Auto-saved {} boards to: {}", self.boards.len(), filename);
        }
        Ok(())
    }

    fn edit_board_field(
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

    fn edit_card_field(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
        field: CardField,
    ) -> io::Result<()> {
        if let Some(card_idx) = self.active_card_index {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.boards.get(board_idx) {
                    let board_cards: Vec<_> = self
                        .cards
                        .iter()
                        .filter(|card| {
                            self.columns
                                .iter()
                                .any(|col| col.id == card.column_id && col.board_id == board.id)
                        })
                        .collect();

                    if let Some(card) = board_cards.get(card_idx) {
                        let temp_dir = std::env::temp_dir();
                        let (temp_file, current_content) = match field {
                            CardField::Title => {
                                let temp_file =
                                    temp_dir.join(format!("kanban-card-{}-title.md", card.id));
                                (temp_file, card.title.clone())
                            }
                            CardField::Description => {
                                let temp_file = temp_dir
                                    .join(format!("kanban-card-{}-description.md", card.id));
                                let content = card.description.as_deref().unwrap_or("").to_string();
                                (temp_file, content)
                            }
                        };

                        if let Some(new_content) = edit_in_external_editor(
                            terminal,
                            event_handler,
                            temp_file,
                            &current_content,
                        )? {
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

    fn scan_import_files(&mut self) {
        self.import_files.clear();
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        if let Some(filename) = entry.file_name().to_str() {
                            if filename.ends_with(".json") {
                                self.import_files.push(filename.to_string());
                            }
                        }
                    }
                }
            }
        }
        self.import_files.sort();
    }

    pub fn import_board_from_file(&mut self, filename: &str) -> io::Result<()> {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct BoardImport {
            board: Board,
            columns: Vec<Column>,
            cards: Vec<Card>,
        }

        #[derive(Deserialize)]
        struct AllBoardsImport {
            boards: Vec<BoardImport>,
        }

        let content = std::fs::read_to_string(filename)?;
        let first_new_index = self.boards.len();

        match serde_json::from_str::<AllBoardsImport>(&content) {
            Ok(import) => {
                let count = import.boards.len();
                for board_data in import.boards {
                    self.boards.push(board_data.board);
                    self.columns.extend(board_data.columns);
                    self.cards.extend(board_data.cards);
                }
                tracing::info!("Imported {} boards from: {}", count, filename);
            }
            Err(err) => {
                tracing::error!("Import error: {}", err);
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Invalid JSON format. Expected {{\"boards\": [...]}} structure. Error: {}",
                        err
                    ),
                ));
            }
        }

        self.board_selection.set(Some(first_new_index));
        Ok(())
    }

    fn copy_branch_name(&mut self) {
        if let Some(card_idx) = self.active_card_index {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.boards.get(board_idx) {
                    let board_cards: Vec<_> = self
                        .cards
                        .iter()
                        .filter(|card| {
                            self.columns
                                .iter()
                                .any(|col| col.id == card.column_id && col.board_id == board.id)
                        })
                        .collect();

                    if let Some(card) = board_cards.get(card_idx) {
                        let branch_name =
                            card.branch_name(board, self.app_config.effective_default_prefix());
                        if let Err(e) = clipboard::copy_to_clipboard(&branch_name) {
                            tracing::error!("Failed to copy to clipboard: {}", e);
                        } else {
                            tracing::info!("Copied branch name: {}", branch_name);
                        }
                    }
                }
            }
        }
    }

    fn copy_git_checkout_command(&mut self) {
        if let Some(card_idx) = self.active_card_index {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.boards.get(board_idx) {
                    let board_cards: Vec<_> = self
                        .cards
                        .iter()
                        .filter(|card| {
                            self.columns
                                .iter()
                                .any(|col| col.id == card.column_id && col.board_id == board.id)
                        })
                        .collect();

                    if let Some(card) = board_cards.get(card_idx) {
                        let command = card.git_checkout_command(
                            board,
                            self.app_config.effective_default_prefix(),
                        );
                        if let Err(e) = clipboard::copy_to_clipboard(&command) {
                            tracing::error!("Failed to copy to clipboard: {}", e);
                        } else {
                            tracing::info!("Copied command: {}", command);
                        }
                    }
                }
            }
        }
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
