use crate::app::App;
use crate::components::centered_rect;
use crate::error_log::LogLevel;
use kanban_persistence::PersistenceMetadata;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Build the header text shown above the log entries in the Diagnostics
/// popup. Pure: takes only the inputs it renders so it can be unit-tested
/// without spinning up an `App`.
///
/// Returns one `(label, value)` row per fact. Renders to plain Strings; the
/// caller is responsible for any per-row styling.
fn diagnostics_rows(
    save_file: Option<&str>,
    metadata: Option<&PersistenceMetadata>,
) -> Vec<(&'static str, String)> {
    let mut rows: Vec<(&'static str, String)> = Vec::new();

    rows.push((
        "File",
        save_file.map(str::to_string).unwrap_or_else(|| "(in-memory)".to_string()),
    ));

    let format = metadata
        .and_then(|m| m.format_version)
        .map(|v| format!("v{v}"))
        .unwrap_or_else(|| "unknown".to_string());
    rows.push(("Format", format));

    let writer = match metadata {
        Some(m) => match (m.writer_version.as_deref(), m.writer_commit.as_deref()) {
            (Some(v), Some(c)) => format!("kanban {v} ({})", short_commit(c)),
            (Some(v), None) => format!("kanban {v}"),
            (None, _) => "unknown (pre-stamp file)".to_string(),
        },
        None => "(no metadata)".to_string(),
    };
    rows.push(("Writer", writer));

    rows.push((
        "Binary",
        format!(
            "kanban {} ({})",
            kanban_core::KANBAN_VERSION,
            short_commit(kanban_core::KANBAN_COMMIT)
        ),
    ));

    if let Some(m) = metadata {
        rows.push(("Saved at", m.saved_at.to_rfc3339()));
    }

    rows
}

fn short_commit(c: &str) -> String {
    if c == "unknown" || c.is_empty() {
        c.to_string()
    } else {
        c.chars().take(8).collect()
    }
}

pub fn render_error_log_popup(app: &App, frame: &mut Frame) {
    let area = centered_rect(85, 75, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Diagnostics [F12] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = diagnostics_rows(
        app.persistence.save_file.as_deref(),
        app.ctx.persistence_metadata().as_ref(),
    );
    let total = app.with_error_log(|log| log.entries.len());
    let header_height = (rows.len() as u16) + 2; // rows + blank line + "{n} entries"

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .horizontal_margin(1)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(inner);

    let mut header_lines: Vec<Line> = rows
        .into_iter()
        .map(|(label, value)| {
            Line::from(vec![
                Span::styled(
                    format!("{label:<10}"),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(value),
            ])
        })
        .collect();
    header_lines.push(Line::from(""));
    header_lines.push(Line::from(Span::styled(
        format!("{total} entries"),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(Paragraph::new(header_lines), chunks[0]);

    let viewport_height = chunks[1].height as usize;
    let total_entries = total;

    let scroll_offset = app.ui_state.error_log_list.get_scroll_offset();
    let visible: Vec<Line> = app.with_error_log(|log| {
        log.entries
            .iter()
            .rev()
            .skip(scroll_offset)
            .take(viewport_height)
            .map(|entry| {
                let (label, color) = match entry.level {
                    LogLevel::Error => ("[ERROR]", Color::Red),
                    LogLevel::Warn => (" [WARN]", Color::Yellow),
                };
                let ts = entry.timestamp.format("%H:%M:%S").to_string();
                Line::from(vec![
                    Span::styled(
                        label,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(" {} {} ", ts, entry.target)),
                    Span::raw(entry.message.clone()),
                ])
            })
            .collect()
    });

    frame.render_widget(Paragraph::new(visible), chunks[1]);

    let items_above = scroll_offset.min(total_entries);
    let items_below = total_entries.saturating_sub(scroll_offset + viewport_height);
    let mut footer_lines: Vec<Line> = Vec::new();
    if items_above > 0 || items_below > 0 {
        footer_lines.push(Line::from(Span::styled(
            format!("↑ {items_above} above  ↓ {items_below} below"),
            Style::default().fg(Color::DarkGray),
        )));
    }
    footer_lines.push(Line::from(Span::styled(
        "ESC/q: close | j/k: scroll",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    )));
    frame.render_widget(Paragraph::new(footer_lines), chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn rows_to_map(rows: Vec<(&'static str, String)>) -> std::collections::HashMap<&'static str, String> {
        rows.into_iter().collect()
    }

    #[test]
    fn test_diagnostics_rows_with_no_save_file_and_no_metadata() {
        let rows = rows_to_map(diagnostics_rows(None, None));
        assert_eq!(rows.get("File").unwrap(), "(in-memory)");
        assert_eq!(rows.get("Format").unwrap(), "unknown");
        assert_eq!(rows.get("Writer").unwrap(), "(no metadata)");
        assert!(rows.contains_key("Binary"));
        assert!(!rows.contains_key("Saved at"));
    }

    #[test]
    fn test_diagnostics_rows_with_full_metadata() {
        let meta = PersistenceMetadata {
            instance_id: Uuid::nil(),
            saved_at: Utc.with_ymd_and_hms(2026, 5, 23, 12, 0, 0).unwrap(),
            writer_version: Some("0.6.0".into()),
            writer_commit: Some("18e98c4810dca9f69c7a894aa7a9ba009740cb1e".into()),
            format_version: Some(6),
        };
        let rows = rows_to_map(diagnostics_rows(Some("/tmp/board.json"), Some(&meta)));
        assert_eq!(rows.get("File").unwrap(), "/tmp/board.json");
        assert_eq!(rows.get("Format").unwrap(), "v6");
        let writer = rows.get("Writer").unwrap();
        assert!(writer.contains("0.6.0"), "writer line: {writer}");
        assert!(writer.contains("18e98c48"), "must show short commit: {writer}");
        assert!(rows.contains_key("Saved at"));
    }

    #[test]
    fn test_diagnostics_rows_with_legacy_metadata_missing_writer_stamp() {
        let meta = PersistenceMetadata {
            instance_id: Uuid::nil(),
            saved_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            writer_version: None,
            writer_commit: None,
            format_version: Some(6),
        };
        let rows = rows_to_map(diagnostics_rows(Some("/tmp/legacy.json"), Some(&meta)));
        assert_eq!(rows.get("Writer").unwrap(), "unknown (pre-stamp file)");
        assert_eq!(rows.get("Format").unwrap(), "v6");
    }

    #[test]
    fn test_short_commit_truncates_long_hashes() {
        assert_eq!(short_commit("18e98c4810dca9f69c7a"), "18e98c48");
    }

    #[test]
    fn test_short_commit_preserves_unknown_marker() {
        assert_eq!(short_commit("unknown"), "unknown");
    }

    #[test]
    fn test_short_commit_preserves_empty() {
        assert_eq!(short_commit(""), "");
    }
}
