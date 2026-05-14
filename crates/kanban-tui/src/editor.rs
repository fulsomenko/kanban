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

fn editor_env_hint(is_powershell: bool) -> &'static str {
    if is_powershell {
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
    match shell_words::split(full_command) {
        Ok(mut parts) if !parts.is_empty() => {
            let program = parts.remove(0);
            (program, parts)
        }
        _ => (full_command.to_string(), vec![]),
    }
}

fn resolve_editor_with(editor_env: Option<&str>) -> (PathBuf, Vec<String>) {
    let (program, args) = match editor_env {
        Some(value) => parse_editor(value),
        None => fallback_editor(),
    };
    let path = which::which(&program).unwrap_or_else(|_| PathBuf::from(program));
    (path, args)
}

fn resolve_editor() -> (PathBuf, Vec<String>) {
    resolve_editor_with(env::var("EDITOR").ok().as_deref())
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
                editor_env_hint(env::var_os("PSModulePath").is_some()),
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

    #[test]
    fn editor_env_hint_powershell() {
        assert_eq!(editor_env_hint(true), "$env:EDITOR");
    }

    #[test]
    fn editor_env_hint_non_powershell() {
        assert_eq!(editor_env_hint(false), "$EDITOR");
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
    fn parse_editor_handles_malformed_single_word_command() {
        let (program, args) = parse_editor("'vim");
        assert_eq!(program, "'vim");
        assert!(args.is_empty())
    }

    #[test]
    fn parse_editor_handles_malformed_multiword_command() {
        let (program, args) = parse_editor("'vim -u NONE");
        assert_eq!(program, "'vim -u NONE");
        assert!(args.is_empty())
    }

    #[test]
    fn resolve_editor_with_handles_simple_editor_path_no_args() {
        let (path, args) = resolve_editor_with(Some("cargo"));
        assert!(path.is_absolute());
        assert!(path.ends_with("cargo") || path.ends_with("cargo.exe"));
        assert!(args.is_empty());
    }

    #[test]
    fn resolve_editor_with_handles_simple_editor_path_with_args() {
        let (path, args) = resolve_editor_with(Some("cargo version"));
        assert!(path.is_absolute());
        assert!(path.ends_with("cargo") || path.ends_with("cargo.exe"));
        assert_eq!(args, vec!["version"]);
    }

    #[test]
    fn resolve_editor_with_falls_back_when_env_missing() {
        let (path, args) = resolve_editor_with(None);

        if cfg!(target_os = "windows") {
            assert!(path.ends_with("notepad") || path.ends_with("notepad.exe"));
        } else {
            assert!(path.ends_with("vi"));
        }
        assert!(args.is_empty());
    }

    #[test]
    fn resolve_editor_with_handles_nonexistent_program() {
        let (path, args) = resolve_editor_with(Some("vi_and_emacs --flag"));
        assert_eq!(path, PathBuf::from("vi_and_emacs"));
        assert_eq!(args, vec!["--flag"]);
    }
}
