use crate::app::{App, AppMode, Focus};
use crate::events::EventHandler;
use crossterm::event::KeyCode;
use kanban_core::AppConfigDto;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

impl App {
    pub fn handle_open_settings(&mut self) {
        if self.focus.active == Focus::Boards {
            self.push_mode(AppMode::Settings);
        }
    }

    pub fn handle_settings_key(
        &mut self,
        key: KeyCode,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_handler: &EventHandler,
    ) -> bool {
        match key {
            KeyCode::Char('e') => {
                let mut config = self.app_config.clone();
                let temp_file =
                    std::env::temp_dir().join("kanban_config_edit.json");
                if let Err(e) = App::edit_entity_json_impl::<AppConfigDto, _>(
                    &mut config,
                    terminal,
                    event_handler,
                    temp_file,
                ) {
                    self.set_error(format!("Failed to edit config: {}", e));
                    return true;
                }
                self.app_config = config;
                if let Err(e) = self.app_config.save() {
                    self.set_error(format!("Failed to save config: {}", e));
                }
                true
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.pop_mode();
                false
            }
            _ => false,
        }
    }
}
