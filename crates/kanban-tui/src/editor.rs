use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use crate::events::EventHandler;

pub fn edit_in_external_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    event_handler: &EventHandler,
    temp_file: PathBuf,
    initial_content: &str,
) -> io::Result<Option<String>> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    std::fs::write(&temp_file, initial_content)?;

    event_handler.stop();

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    io::stdout().flush()?;

    let status = Command::new(&editor).arg(&temp_file).status()?;

    while crossterm::event::poll(std::time::Duration::from_millis(0))? {
        let _ = crossterm::event::read()?;
    }

    execute!(io::stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    terminal.clear()?;

    let result = if status.success() {
        let content = std::fs::read_to_string(&temp_file)?;
        Some(content)
    } else {
        None
    };

    let _ = std::fs::remove_file(&temp_file);

    Ok(result)
}
