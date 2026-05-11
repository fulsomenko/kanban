use crate::events::EventHandler;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

fn editor_env_hint() -> &'static str {
    if env::var_os("PSModulePath").is_some() {
        "$env:EDITOR"
    } else {
        "$EDITOR"
    }
}

fn fallback_editor() -> (String, Vec<String>) {
    if cfg!(target_os = "windows") {
        ("notepad".to_string(), vec![])
    } else {
        ("vi".to_string(), vec![])
    }
}

fn parse_editor(full_command: &str) -> (String, Vec<String>) {
    let parts = shell_words::split(full_command).unwrap_or_default();
    let program = parts.first().cloned().unwrap_or_default();
    let args = parts.into_iter().skip(1).collect();
    (program, args)
}

fn resolve_editor() -> (PathBuf, Vec<String>) {
    let (program, args) = match env::var("EDITOR") {
        Ok(value) => parse_editor(&value),
        Err(_) => fallback_editor(),
    };

    let path = which::which(&program).unwrap_or_else(|_| PathBuf::from(program));
    (path, args)
}

pub fn edit_in_external_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    event_handler: &EventHandler,
    temp_file: PathBuf,
    initial_content: &str,
) -> io::Result<Option<String>> {
    let (program, args) = resolve_editor();

    std::fs::write(&temp_file, initial_content)?;

    event_handler.stop();

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    io::stdout().flush()?;

    let status = Command::new(&program).args(&args).arg(&temp_file).status();

    if let Err(ref e) = status {
        tracing::error!("Failed to launch editor '{}': {}", program.display(), e);
        execute!(io::stdout(), EnterAlternateScreen)?;
        enable_raw_mode()?;
        terminal.clear()?;
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Editor '{}' not found. Please set {} environment variable.",
                program.display(),
                editor_env_hint()
            ),
        ));
    }

    let status = status.unwrap();

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::{remove_var, set_var};

    #[test]
    fn editor_env_hint_powershell() {
        set_var("PSModulePath", "mock_data");
        assert_eq!(editor_env_hint(), "$env:EDITOR");
        remove_var("PSModulePath");
    }

    #[test]
    fn editor_env_hint_non_powershell() {
        remove_var("PSModulePath");
        assert_eq!(editor_env_hint(), "$EDITOR");
    }

    #[test]
    fn parse_editor_handles_single_word() {
        let (program, args) = parse_editor("vim");
        assert_eq!(program, "vim");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_editor_splits_program_and_args() {
        let (program, args) = parse_editor("vim -u NONE");
        assert_eq!(program, "vim");
        assert_eq!(args, vec!["-u", "NONE"]);
    }

    #[test]
    fn parse_editor_handles_double_quoted_paths() {
        let (program, args) = parse_editor("\"C:/Program Files/VS Code/code.cmd\" --wait");
        assert_eq!(program, "C:/Program Files/VS Code/code.cmd");
        assert_eq!(args, vec!["--wait"]);
    }

    #[test]
    fn parse_editor_handles_single_quoted_paths() {
        let (program, args) = parse_editor("\'C:/Program Files/VS Code/code.cmd\' --wait");
        assert_eq!(program, "C:/Program Files/VS Code/code.cmd");
        assert_eq!(args, vec!["--wait"]);
    }

    #[test]
    fn resolve_editor_uses_env_var() {
        set_var("EDITOR", "vim");
        let (path, args) = resolve_editor();

        assert!(path.ends_with("vim") || path.ends_with("vim.exe"));
        assert!(args.is_empty());

        remove_var("EDITOR");
    }

    #[test]
    fn resolve_editor_handles_multiword_editor() {
        set_var("EDITOR", "code --wait");
        let (path, args) = resolve_editor();

        assert!(path.ends_with("code") || path.ends_with("code.exe") || path.ends_with("code.cmd"));
        assert_eq!(args, vec!["--wait"]);

        remove_var("EDITOR");
    }

    #[test]
    fn resolve_editor_falls_back_when_env_missing() {
        remove_var("EDITOR");
        let (path, args) = resolve_editor();

        if cfg!(target_os = "windows") {
            assert!(path.ends_with("notepad") || path.ends_with("notepad.exe"));
            assert!(args.is_empty());
        } else {
            assert!(path.ends_with("vi"));
            assert!(args.is_empty());
        }
    }

    #[test]
    fn resolve_editor_handles_nonexistent_program() {
        set_var("EDITOR", "vi_and_emacs --flag");
        let (path, args) = resolve_editor();

        assert_eq!(path, PathBuf::from("vi_and_emacs"));
        assert_eq!(args, vec!["--flag"]);

        remove_var("EDITOR");
    }
}
