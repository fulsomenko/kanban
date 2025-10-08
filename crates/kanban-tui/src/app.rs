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
use std::io;
use crate::{events::{EventHandler, Event}, ui, selection::SelectionState};

pub struct App {
    pub should_quit: bool,
    pub mode: AppMode,
    pub input_buffer: String,
    pub projects: Vec<Board>,
    pub project_selection: SelectionState,
    pub active_project_index: Option<usize>,
    pub task_selection: SelectionState,
    pub active_task_index: Option<usize>,
    pub columns: Vec<Column>,
    pub cards: Vec<Card>,
    pub focus: Focus,
    pub task_focus: TaskFocus,
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
pub enum AppMode {
    Normal,
    CreateProject,
    CreateTask,
    TaskDetail,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            mode: AppMode::Normal,
            input_buffer: String::new(),
            projects: Vec::new(),
            project_selection: SelectionState::new(),
            active_project_index: None,
            task_selection: SelectionState::new(),
            active_task_index: None,
            columns: Vec::new(),
            cards: Vec::new(),
            focus: Focus::Projects,
            task_focus: TaskFocus::Title,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        match self.mode {
            AppMode::Normal => {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => self.quit(),
                    KeyCode::Char('n') => {
                        match self.focus {
                            Focus::Projects => {
                                self.mode = AppMode::CreateProject;
                                self.input_buffer.clear();
                            }
                            Focus::Tasks => {
                                if self.active_project_index.is_some() {
                                    self.mode = AppMode::CreateTask;
                                    self.input_buffer.clear();
                                }
                            }
                        }
                    }
                    KeyCode::Char('1') => self.focus = Focus::Projects,
                    KeyCode::Char('2') => {
                        if self.active_project_index.is_some() {
                            self.focus = Focus::Tasks;
                        }
                    }
                    KeyCode::Esc => {
                        if self.active_project_index.is_some() {
                            self.active_project_index = None;
                            self.task_selection.clear();
                            self.focus = Focus::Projects;
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        match self.focus {
                            Focus::Projects => {
                                self.project_selection.next(self.projects.len());
                            }
                            Focus::Tasks => {
                                if let Some(project_idx) = self.active_project_index {
                                    if let Some(project) = self.projects.get(project_idx) {
                                        let task_count = self.get_project_task_count(project.id);
                                        self.task_selection.next(task_count);
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        match self.focus {
                            Focus::Projects => {
                                self.project_selection.prev();
                            }
                            Focus::Tasks => {
                                self.task_selection.prev();
                            }
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        match self.focus {
                            Focus::Projects => {
                                self.active_project_index = self.project_selection.get();
                                self.task_selection.clear();

                                if let Some(project_idx) = self.active_project_index {
                                    if let Some(project) = self.projects.get(project_idx) {
                                        let task_count = self.get_project_task_count(project.id);
                                        if task_count > 0 {
                                            self.task_selection.set(Some(0));
                                        }
                                    }
                                }

                                self.focus = Focus::Tasks;
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
                        self.input_buffer.clear();
                    }
                    KeyCode::Enter => {
                        if !self.input_buffer.is_empty() {
                            self.create_project();
                            self.mode = AppMode::Normal;
                            self.input_buffer.clear();
                        }
                    }
                    KeyCode::Char(c) => {
                        if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.input_buffer.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                    }
                    _ => {}
                }
            }
            AppMode::CreateTask => {
                match key.code {
                    KeyCode::Esc => {
                        self.mode = AppMode::Normal;
                        self.input_buffer.clear();
                    }
                    KeyCode::Enter => {
                        if !self.input_buffer.is_empty() {
                            self.create_task();
                            self.mode = AppMode::Normal;
                            self.input_buffer.clear();
                        }
                    }
                    KeyCode::Char(c) => {
                        if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.input_buffer.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                    }
                    _ => {}
                }
            }
            AppMode::TaskDetail => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
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
                    _ => {}
                }
            }
        }
    }

    fn create_project(&mut self) {
        let project = Board::new(self.input_buffer.clone(), None);
        tracing::info!("Creating project: {} (id: {})", project.name, project.id);
        self.projects.push(project);
        let new_index = self.projects.len() - 1;
        self.project_selection.set(Some(new_index));
    }

    fn create_task(&mut self) {
        if let Some(idx) = self.active_project_index {
            if let Some(project) = self.projects.get(idx) {
                let column = self.columns.iter()
                    .find(|col| col.board_id == project.id)
                    .cloned();

                let column = match column {
                    Some(col) => col,
                    None => {
                        let new_column = Column::new(project.id, "Todo".to_string(), 0);
                        self.columns.push(new_column.clone());
                        new_column
                    }
                };

                let position = self.cards.iter().filter(|c| c.column_id == column.id).count() as i32;
                let card = Card::new(column.id, self.input_buffer.clone(), position);
                tracing::info!("Creating task: {} (id: {})", card.title, card.id);
                self.cards.push(card);

                let task_count = self.get_project_task_count(project.id);
                let new_task_index = task_count.saturating_sub(1);
                self.task_selection.set(Some(new_task_index));
            }
        }
    }

    fn get_project_task_count(&self, board_id: uuid::Uuid) -> usize {
        self.cards.iter()
            .filter(|card| {
                self.columns.iter()
                    .any(|col| col.id == card.column_id && col.board_id == board_id)
            })
            .count()
    }

    pub async fn run(&mut self) -> KanbanResult<()> {
        let mut terminal = setup_terminal()?;
        let mut events = EventHandler::new();

        while !self.should_quit {
            terminal.draw(|frame| ui::render(self, frame))?;

            if let Some(event) = events.next().await {
                match event {
                    Event::Key(key) => {
                        self.handle_key_event(key);
                    }
                    Event::Tick => {}
                }
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
