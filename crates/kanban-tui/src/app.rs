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
use kanban_domain::{Board, Card, CardStatus, Column, SortField, SortOrder, Sprint};
use serde::{Deserialize, Serialize};
use ratatui::{backend::CrosstermBackend, Terminal};
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
    pub sprints: Vec<Sprint>,
    pub sprint_selection: SelectionState,
    pub active_sprint_index: Option<usize>,
    pub active_sprint_filter: Option<uuid::Uuid>,
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Boards,
    Cards,
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
    Settings,
    Sprints,
}

pub enum CardField {
    Title,
    Description,
}

pub enum BoardField {
    Name,
    Description,
    Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BoardSettings {
    branch_prefix: Option<String>,
    sprint_duration_days: Option<u32>,
    sprint_prefix: Option<String>,
    sprint_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
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
            sprints: Vec::new(),
            sprint_selection: SelectionState::new(),
            active_sprint_index: None,
            active_sprint_filter: None,
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
        };

        if let Some(ref filename) = save_file {
            if std::path::Path::new(filename).exists() {
                if let Err(e) = app.import_board_from_file(filename) {
                    tracing::error!("Failed to load file {}: {}", filename, e);
                    app.save_file = None;
                }
            }
        }

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

        if matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) {
            self.quit();
            return false;
        }

        match self.mode {
            AppMode::Normal => match key.code {
                KeyCode::Char('n') => match self.focus {
                    Focus::Boards => self.handle_create_board_key(),
                    Focus::Cards => self.handle_create_card_key(),
                },
                KeyCode::Char('r') => self.handle_rename_board_key(),
                KeyCode::Char('e') => self.handle_edit_board_key(),
                KeyCode::Char('x') => self.handle_export_board_key(),
                KeyCode::Char('X') => self.handle_export_all_key(),
                KeyCode::Char('i') => self.handle_import_board_key(),
                KeyCode::Char('a') => self.handle_assign_to_sprint_key(),
                KeyCode::Char('c') => self.handle_toggle_card_completion(),
                KeyCode::Char('o') => self.handle_order_cards_key(),
                KeyCode::Char('O') => self.handle_toggle_sort_order_key(),
                KeyCode::Char('T') => self.handle_toggle_hide_assigned(),
                KeyCode::Char('t') => self.handle_toggle_sprint_filter(),
                KeyCode::Char('v') => self.handle_card_selection_toggle(),
                KeyCode::Char('1') => self.handle_focus_switch(Focus::Boards),
                KeyCode::Char('2') => self.handle_focus_switch(Focus::Cards),
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
            AppMode::OrderCards => {
                should_restart_events = self.handle_order_cards_popup(key.code);
            }
            AppMode::SprintDetail => self.handle_sprint_detail_key(key.code),
            AppMode::AssignCardToSprint => self.handle_assign_card_to_sprint_popup(key.code),
            AppMode::AssignMultipleCardsToSprint => {
                self.handle_assign_multiple_cards_to_sprint_popup(key.code)
            }
        }
        should_restart_events
    }


    pub fn get_board_card_count(&self, board_id: uuid::Uuid) -> usize {
        self.cards
            .iter()
            .filter(|card| {
                let in_board = self.columns
                    .iter()
                    .any(|col| col.id == card.column_id && col.board_id == board_id);

                if !in_board {
                    return false;
                }

                if let Some(filter_sprint_id) = self.active_sprint_filter {
                    if card.sprint_id != Some(filter_sprint_id) {
                        return false;
                    }
                }

                if self.hide_assigned_cards {
                    card.sprint_id.is_none()
                } else {
                    true
                }
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
                let in_board = self.columns
                    .iter()
                    .any(|col| col.id == card.column_id && col.board_id == board_id);

                if !in_board {
                    return false;
                }

                if let Some(filter_sprint_id) = self.active_sprint_filter {
                    if card.sprint_id != Some(filter_sprint_id) {
                        return false;
                    }
                }

                if self.hide_assigned_cards {
                    card.sprint_id.is_none()
                } else {
                    true
                }
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
            sprints: Vec<Sprint>,
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

                let board_sprints: Vec<Sprint> = self
                    .sprints
                    .iter()
                    .filter(|s| s.board_id == board.id)
                    .cloned()
                    .collect();

                let board_export = BoardExport {
                    board: board.clone(),
                    columns: board_columns,
                    cards: board_cards,
                    sprints: board_sprints,
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
            sprints: Vec<Sprint>,
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

            let board_sprints: Vec<Sprint> = self
                .sprints
                .iter()
                .filter(|s| s.board_id == board.id)
                .cloned()
                .collect();

            board_exports.push(BoardExport {
                board: board.clone(),
                columns: board_columns,
                cards: board_cards,
                sprints: board_sprints,
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
                sprints: Vec<Sprint>,
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

                let board_sprints: Vec<Sprint> = self
                    .sprints
                    .iter()
                    .filter(|s| s.board_id == board.id)
                    .cloned()
                    .collect();

                board_exports.push(BoardExport {
                    board: board.clone(),
                    columns: board_columns,
                    cards: board_cards,
                    sprints: board_sprints,
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

    fn check_ended_sprints(&self) {
        let ended_sprints: Vec<_> = self
            .sprints
            .iter()
            .filter(|s| s.is_ended())
            .collect();

        if !ended_sprints.is_empty() {
            tracing::warn!(
                "Found {} ended sprint(s) that need attention:",
                ended_sprints.len()
            );
            for sprint in &ended_sprints {
                if let Some(board) = self.boards.iter().find(|b| b.id == sprint.board_id) {
                    tracing::warn!(
                        "  - {} (ended: {})",
                        sprint.formatted_name(board, board.sprint_prefix.as_deref().unwrap_or("sprint")),
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
                    BoardField::Settings => {
                        let temp_file = temp_dir.join(format!("kanban-board-{}-settings.json", board.id));
                        let settings = BoardSettings {
                            branch_prefix: board.branch_prefix.clone(),
                            sprint_duration_days: board.sprint_duration_days,
                            sprint_prefix: board.sprint_prefix.clone(),
                            sprint_names: board.sprint_names.clone(),
                        };
                        let content = serde_json::to_string_pretty(&settings)
                            .unwrap_or_else(|_| "{}".to_string());
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
                            BoardField::Settings => {
                                match serde_json::from_str::<BoardSettings>(&new_content) {
                                    Ok(settings) => {
                                        board.branch_prefix = settings.branch_prefix;
                                        board.sprint_duration_days = settings.sprint_duration_days;
                                        board.sprint_prefix = settings.sprint_prefix;
                                        board.sprint_names = settings.sprint_names;
                                        board.updated_at = chrono::Utc::now();
                                        tracing::info!("Updated board settings via JSON editor");
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to parse settings JSON: {}", e);
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

    pub fn edit_card_field(
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


    pub fn import_board_from_file(&mut self, filename: &str) -> io::Result<()> {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct BoardImport {
            board: Board,
            columns: Vec<Column>,
            cards: Vec<Card>,
            #[serde(default)]
            sprints: Vec<Sprint>,
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
                    self.sprints.extend(board_data.sprints);
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

    pub fn copy_branch_name(&mut self) {
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
                            card.branch_name(board, &self.sprints, self.app_config.effective_default_prefix());
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

    pub fn copy_git_checkout_command(&mut self) {
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
                            &self.sprints,
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
