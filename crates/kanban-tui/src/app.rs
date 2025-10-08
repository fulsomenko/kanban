use kanban_core::KanbanResult;
use kanban_domain::{Board, Column, Card};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io::{self, Write};
use std::process::Command;
use serde::Serialize;
use crate::{events::{EventHandler, Event}, ui, selection::SelectionState};

pub struct App {
    pub should_quit: bool,
    pub mode: AppMode,
    pub input_buffer: String,
    pub input_cursor: usize,
    pub boards: Vec<Board>,
    pub board_selection: SelectionState,
    pub active_board_index: Option<usize>,
    pub task_selection: SelectionState,
    pub active_task_index: Option<usize>,
    pub columns: Vec<Column>,
    pub cards: Vec<Card>,
    pub focus: Focus,
    pub task_focus: TaskFocus,
    pub board_focus: BoardFocus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Projects,
    Tasks,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskFocus {
    Title,
    Metadata,
    Description,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoardFocus {
    Name,
    Description,
}

enum TaskField {
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
    CreateProject,
    CreateTask,
    TaskDetail,
    RenameProject,
    BoardDetail,
    ExportBoard,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            mode: AppMode::Normal,
            input_buffer: String::new(),
            input_cursor: 0,
            boards: Vec::new(),
            board_selection: SelectionState::new(),
            active_board_index: None,
            task_selection: SelectionState::new(),
            active_task_index: None,
            columns: Vec::new(),
            cards: Vec::new(),
            focus: Focus::Projects,
            task_focus: TaskFocus::Title,
            board_focus: BoardFocus::Name,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    fn handle_input_char(&mut self, c: char) {
        self.input_buffer.insert(self.input_cursor, c);
        self.input_cursor += 1;
    }

    fn handle_input_backspace(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor -= 1;
            self.input_buffer.remove(self.input_cursor);
        }
    }

    fn handle_input_delete(&mut self) {
        if self.input_cursor < self.input_buffer.len() {
            self.input_buffer.remove(self.input_cursor);
        }
    }

    fn handle_input_left(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor -= 1;
        }
    }

    fn handle_input_right(&mut self) {
        if self.input_cursor < self.input_buffer.len() {
            self.input_cursor += 1;
        }
    }

    fn handle_input_home(&mut self) {
        self.input_cursor = 0;
    }

    fn handle_input_end(&mut self) {
        self.input_cursor = self.input_buffer.len();
    }

    fn clear_input(&mut self) {
        self.input_buffer.clear();
        self.input_cursor = 0;
    }

    fn set_input(&mut self, text: String) {
        self.input_buffer = text;
        self.input_cursor = self.input_buffer.len();
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, event_handler: &EventHandler) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};
        let mut should_restart_events = false;

        if matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) {
            self.quit();
            return false;
        }

        match self.mode {
            AppMode::Normal => {
                match key.code {
                    KeyCode::Char('n') => {
                        match self.focus {
                            Focus::Projects => {
                                self.mode = AppMode::CreateProject;
                                self.clear_input();
                            }
                            Focus::Tasks => {
                                if self.active_board_index.is_some() {
                                    self.mode = AppMode::CreateTask;
                                    self.clear_input();
                                }
                            }
                        }
                    }
                    KeyCode::Char('r') => {
                        if self.focus == Focus::Projects && self.board_selection.get().is_some() {
                            if let Some(board_idx) = self.board_selection.get() {
                                if let Some(board) = self.boards.get(board_idx) {
                                    self.set_input(board.name.clone());
                                    self.mode = AppMode::RenameProject;
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
                    KeyCode::Char('x') => {
                        if self.focus == Focus::Projects && self.board_selection.get().is_some() {
                            if let Some(board_idx) = self.board_selection.get() {
                                if let Some(board) = self.boards.get(board_idx) {
                                    let filename = format!("{}-{}.json",
                                        board.name.replace(" ", "-").to_lowercase(),
                                        chrono::Utc::now().format("%Y%m%d-%H%M%S")
                                    );
                                    self.set_input(filename);
                                    self.mode = AppMode::ExportBoard;
                                }
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
                            self.task_selection.clear();
                            self.focus = Focus::Projects;
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        match self.focus {
                            Focus::Projects => {
                                self.board_selection.next(self.boards.len());
                            }
                            Focus::Tasks => {
                                if let Some(board_idx) = self.active_board_index {
                                    if let Some(board) = self.boards.get(board_idx) {
                                        let task_count = self.get_board_task_count(board.id);
                                        self.task_selection.next(task_count);
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        match self.focus {
                            Focus::Projects => {
                                self.board_selection.prev();
                            }
                            Focus::Tasks => {
                                self.task_selection.prev();
                            }
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        match self.focus {
                            Focus::Projects => {
                                if self.board_selection.get().is_some() {
                                    self.active_board_index = self.board_selection.get();
                                    self.task_selection.clear();

                                    if let Some(board_idx) = self.active_board_index {
                                        if let Some(board) = self.boards.get(board_idx) {
                                            let task_count = self.get_board_task_count(board.id);
                                            if task_count > 0 {
                                                self.task_selection.set(Some(0));
                                            }
                                        }
                                    }

                                    self.focus = Focus::Tasks;
                                }
                            }
                            Focus::Tasks => {
                                if self.task_selection.get().is_some() {
                                    self.active_task_index = self.task_selection.get();
                                    self.mode = AppMode::TaskDetail;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            AppMode::CreateProject => {
                match key.code {
                    KeyCode::Esc => {
                        self.mode = AppMode::Normal;
                        self.clear_input();
                    }
                    KeyCode::Enter => {
                        if !self.input_buffer.is_empty() {
                            self.create_board();
                            self.mode = AppMode::Normal;
                            self.clear_input();
                        }
                    }
                    KeyCode::Char(c) => {
                        if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.handle_input_char(c);
                        }
                    }
                    KeyCode::Backspace => self.handle_input_backspace(),
                    KeyCode::Delete => self.handle_input_delete(),
                    KeyCode::Left => self.handle_input_left(),
                    KeyCode::Right => self.handle_input_right(),
                    KeyCode::Home => self.handle_input_home(),
                    KeyCode::End => self.handle_input_end(),
                    _ => {}
                }
            }
            AppMode::CreateTask => {
                match key.code {
                    KeyCode::Esc => {
                        self.mode = AppMode::Normal;
                        self.clear_input();
                    }
                    KeyCode::Enter => {
                        if !self.input_buffer.is_empty() {
                            self.create_task();
                            self.mode = AppMode::Normal;
                            self.clear_input();
                        }
                    }
                    KeyCode::Char(c) => {
                        if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.handle_input_char(c);
                        }
                    }
                    KeyCode::Backspace => self.handle_input_backspace(),
                    KeyCode::Delete => self.handle_input_delete(),
                    KeyCode::Left => self.handle_input_left(),
                    KeyCode::Right => self.handle_input_right(),
                    KeyCode::Home => self.handle_input_home(),
                    KeyCode::End => self.handle_input_end(),
                    _ => {}
                }
            }
            AppMode::RenameProject => {
                match key.code {
                    KeyCode::Esc => {
                        self.mode = AppMode::Normal;
                        self.clear_input();
                    }
                    KeyCode::Enter => {
                        if !self.input_buffer.is_empty() {
                            self.rename_board();
                            self.mode = AppMode::Normal;
                            self.clear_input();
                        }
                    }
                    KeyCode::Char(c) => {
                        if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.handle_input_char(c);
                        }
                    }
                    KeyCode::Backspace => self.handle_input_backspace(),
                    KeyCode::Delete => self.handle_input_delete(),
                    KeyCode::Left => self.handle_input_left(),
                    KeyCode::Right => self.handle_input_right(),
                    KeyCode::Home => self.handle_input_home(),
                    KeyCode::End => self.handle_input_end(),
                    _ => {}
                }
            }
            AppMode::ExportBoard => {
                match key.code {
                    KeyCode::Esc => {
                        self.mode = AppMode::Normal;
                        self.clear_input();
                    }
                    KeyCode::Enter => {
                        if !self.input_buffer.is_empty() {
                            if let Err(e) = self.export_board_with_filename() {
                                tracing::error!("Failed to export board: {}", e);
                            }
                            self.mode = AppMode::Normal;
                            self.clear_input();
                        }
                    }
                    KeyCode::Char(c) => {
                        if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.handle_input_char(c);
                        }
                    }
                    KeyCode::Backspace => self.handle_input_backspace(),
                    KeyCode::Delete => self.handle_input_delete(),
                    KeyCode::Left => self.handle_input_left(),
                    KeyCode::Right => self.handle_input_right(),
                    KeyCode::Home => self.handle_input_home(),
                    KeyCode::End => self.handle_input_end(),
                    _ => {}
                }
            }
            AppMode::TaskDetail => {
                match key.code {
                    KeyCode::Esc => {
                        self.mode = AppMode::Normal;
                        self.active_task_index = None;
                        self.task_focus = TaskFocus::Title;
                    }
                    KeyCode::Char('1') => {
                        self.task_focus = TaskFocus::Title;
                    }
                    KeyCode::Char('2') => {
                        self.task_focus = TaskFocus::Metadata;
                    }
                    KeyCode::Char('3') => {
                        self.task_focus = TaskFocus::Description;
                    }
                    KeyCode::Char('e') => {
                        match self.task_focus {
                            TaskFocus::Title => {
                                if let Err(e) = self.edit_task_field(terminal, event_handler, TaskField::Title) {
                                    tracing::error!("Failed to edit title: {}", e);
                                }
                                should_restart_events = true;
                            }
                            TaskFocus::Description => {
                                if let Err(e) = self.edit_task_field(terminal, event_handler, TaskField::Description) {
                                    tracing::error!("Failed to edit description: {}", e);
                                }
                                should_restart_events = true;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            AppMode::BoardDetail => {
                match key.code {
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
                    KeyCode::Char('e') => {
                        match self.board_focus {
                            BoardFocus::Name => {
                                if let Err(e) = self.edit_board_field(terminal, event_handler, BoardField::Name) {
                                    tracing::error!("Failed to edit board name: {}", e);
                                }
                                should_restart_events = true;
                            }
                            BoardFocus::Description => {
                                if let Err(e) = self.edit_board_field(terminal, event_handler, BoardField::Description) {
                                    tracing::error!("Failed to edit board description: {}", e);
                                }
                                should_restart_events = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        should_restart_events
    }

    fn create_board(&mut self) {
        let board = Board::new(self.input_buffer.clone(), None);
        tracing::info!("Creating project: {} (id: {})", board.name, board.id);
        self.boards.push(board);
        let new_index = self.boards.len() - 1;
        self.board_selection.set(Some(new_index));
    }

    fn rename_board(&mut self) {
        if let Some(idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get_mut(idx) {
                board.update_name(self.input_buffer.clone());
                tracing::info!("Renamed project to: {}", board.name);
            }
        }
    }

    fn create_task(&mut self) {
        if let Some(idx) = self.active_board_index {
            if let Some(board) = self.boards.get(idx) {
                let column = self.columns.iter()
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

                let position = self.cards.iter().filter(|c| c.column_id == column.id).count() as i32;
                let card = Card::new(column.id, self.input_buffer.clone(), position);
                tracing::info!("Creating task: {} (id: {})", card.title, card.id);
                self.cards.push(card);

                let task_count = self.get_board_task_count(board.id);
                let new_task_index = task_count.saturating_sub(1);
                self.task_selection.set(Some(new_task_index));
            }
        }
    }

    fn get_board_task_count(&self, board_id: uuid::Uuid) -> usize {
        self.cards.iter()
            .filter(|card| {
                self.columns.iter()
                    .any(|col| col.id == card.column_id && col.board_id == board_id)
            })
            .count()
    }

    fn export_board_with_filename(&self) -> io::Result<()> {
        if let Some(board_idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get(board_idx) {
                let board_columns: Vec<Column> = self.columns.iter()
                    .filter(|col| col.board_id == board.id)
                    .cloned()
                    .collect();

                let column_ids: Vec<uuid::Uuid> = board_columns.iter().map(|c| c.id).collect();

                let board_cards: Vec<Card> = self.cards.iter()
                    .filter(|card| column_ids.contains(&card.column_id))
                    .cloned()
                    .collect();

                #[derive(Serialize)]
                struct BoardExport {
                    board: Board,
                    columns: Vec<Column>,
                    tasks: Vec<Card>,
                }

                let export = BoardExport {
                    board: board.clone(),
                    columns: board_columns,
                    tasks: board_cards,
                };

                let json = serde_json::to_string_pretty(&export)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                std::fs::write(&self.input_buffer, json)?;
                tracing::info!("Exported board to: {}", self.input_buffer);
            }
        }
        Ok(())
    }

    fn edit_board_field(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, event_handler: &EventHandler, field: BoardField) -> io::Result<()> {
        if let Some(board_idx) = self.board_selection.get() {
            if let Some(board) = self.boards.get(board_idx) {
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

                let temp_dir = std::env::temp_dir();
                let (temp_file, current_content) = match field {
                    BoardField::Name => {
                        let temp_file = temp_dir.join(format!("kanban-board-{}-name.md", board.id));
                        (temp_file, board.name.clone())
                    }
                    BoardField::Description => {
                        let temp_file = temp_dir.join(format!("kanban-board-{}-description.md", board.id));
                        let content = board.description.as_ref().map(|s| s.as_str()).unwrap_or("").to_string();
                        (temp_file, content)
                    }
                };

                std::fs::write(&temp_file, current_content)?;

                event_handler.stop();

                disable_raw_mode()?;
                execute!(io::stdout(), LeaveAlternateScreen)?;
                io::stdout().flush()?;

                let status = Command::new(&editor)
                    .arg(&temp_file)
                    .status()?;

                while crossterm::event::poll(std::time::Duration::from_millis(0))? {
                    let _ = crossterm::event::read()?;
                }

                execute!(io::stdout(), EnterAlternateScreen)?;
                enable_raw_mode()?;

                terminal.clear()?;

                if status.success() {
                    let new_content = std::fs::read_to_string(&temp_file)?;

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

                let _ = std::fs::remove_file(&temp_file);
            }
        }
        Ok(())
    }

    fn edit_task_field(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, event_handler: &EventHandler, field: TaskField) -> io::Result<()> {
        if let Some(task_idx) = self.active_task_index {
            if let Some(board_idx) = self.active_board_index {
                if let Some(board) = self.boards.get(board_idx) {
                    let board_tasks: Vec<_> = self.cards.iter()
                        .filter(|card| {
                            self.columns.iter()
                                .any(|col| col.id == card.column_id && col.board_id == board.id)
                        })
                        .collect();

                    if let Some(task) = board_tasks.get(task_idx) {
                        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

                        let temp_dir = std::env::temp_dir();
                        let (temp_file, current_content) = match field {
                            TaskField::Title => {
                                let temp_file = temp_dir.join(format!("kanban-task-{}-title.md", task.id));
                                (temp_file, task.title.clone())
                            }
                            TaskField::Description => {
                                let temp_file = temp_dir.join(format!("kanban-task-{}-description.md", task.id));
                                let content = task.description.as_ref().map(|s| s.as_str()).unwrap_or("").to_string();
                                (temp_file, content)
                            }
                        };

                        std::fs::write(&temp_file, current_content)?;

                        event_handler.stop();

                        disable_raw_mode()?;
                        execute!(io::stdout(), LeaveAlternateScreen)?;
                        io::stdout().flush()?;

                        let status = Command::new(&editor)
                            .arg(&temp_file)
                            .status()?;

                        while crossterm::event::poll(std::time::Duration::from_millis(0))? {
                            let _ = crossterm::event::read()?;
                        }

                        execute!(io::stdout(), EnterAlternateScreen)?;
                        enable_raw_mode()?;

                        terminal.clear()?;

                        if status.success() {
                            let new_content = std::fs::read_to_string(&temp_file)?;

                            let task_id = task.id;
                            if let Some(card) = self.cards.iter_mut().find(|c| c.id == task_id) {
                                match field {
                                    TaskField::Title => {
                                        if !new_content.trim().is_empty() {
                                            card.update_title(new_content.trim().to_string());
                                        }
                                    }
                                    TaskField::Description => {
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

                        let _ = std::fs::remove_file(&temp_file);
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

        restore_terminal(&mut terminal)?;
        Ok(())
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
