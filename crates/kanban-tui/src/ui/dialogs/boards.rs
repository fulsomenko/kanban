use crate::app::App;
use crate::components::*;
use crate::theme::*;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub(crate) fn render_export_boards_popup(app: &App, frame: &mut Frame) {
    use crate::app::{ExportFormat, ExportStep};

    let Some(ref dialog) = app.export_dialog else {
        return;
    };

    match dialog.step {
        ExportStep::SelectBoards => {
            let inner = render_popup_with_block(frame, "Select Boards to Export", 60, 60);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(inner);

            let items: Vec<Line> = app
                .ctx
                .boards()
                .iter()
                .enumerate()
                .map(|(i, board)| {
                    let checkbox = if dialog.board_selections.get(i).copied().unwrap_or(false) {
                        "[x] "
                    } else {
                        "[ ] "
                    };
                    let style = if i == dialog.cursor {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Line::from(Span::styled(format!("{}{}", checkbox, board.name), style))
                })
                .collect();

            let list = Paragraph::new(items);
            frame.render_widget(list, chunks[0]);

            let hint = Paragraph::new(Line::from(vec![Span::styled(
                "Space: toggle | a: all | Enter: next | Esc: cancel",
                Style::default().fg(Color::DarkGray),
            )]));
            frame.render_widget(hint, chunks[1]);
        }
        ExportStep::ExportOptions => {
            let inner = render_popup_with_block(frame, "Export Options", 60, 30);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(inner);

            let filename_label = Paragraph::new(Line::from(vec![
                Span::styled("Filename: ", Style::default().fg(Color::Cyan)),
                Span::styled(&dialog.filename, Style::default().fg(Color::White)),
                Span::styled("_", Style::default().fg(Color::Yellow)),
            ]));
            frame.render_widget(filename_label, chunks[0]);

            frame.render_widget(Paragraph::new(""), chunks[1]);

            let json_style = if dialog.format == ExportFormat::Json {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let sqlite_style = if dialog.format == ExportFormat::Sqlite {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let json_radio = if dialog.format == ExportFormat::Json {
                "(*)"
            } else {
                "( )"
            };
            let sqlite_radio = if dialog.format == ExportFormat::Sqlite {
                "(*)"
            } else {
                "( )"
            };

            let format_line = Paragraph::new(Line::from(vec![
                Span::styled("Format: ", Style::default().fg(Color::Cyan)),
                Span::styled(format!("{} JSON  ", json_radio), json_style),
                Span::styled(format!("{} SQLite", sqlite_radio), sqlite_style),
            ]));
            frame.render_widget(format_line, chunks[2]);

            let hint = Paragraph::new(Line::from(vec![Span::styled(
                "Tab: format | Enter: export | Esc: back",
                Style::default().fg(Color::DarkGray),
            )]));
            frame.render_widget(hint, chunks[3]);
        }
    }
}

pub(crate) fn render_create_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Create New Project",
        "Project Name:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(crate) fn render_rename_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Rename Project",
        "New Project Name:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(crate) fn render_export_board_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Export Project",
        "Filename:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(crate) fn render_export_all_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Export All Projects",
        "Filename:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}

pub(crate) fn render_import_board_popup(app: &App, frame: &mut Frame) {
    let inner = render_popup_with_block(frame, "Import Projects", 60, 50);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let label = Paragraph::new("Select a JSON file to import:").style(highlight_text());
    frame.render_widget(label, chunks[0]);

    if app.dialog_input.import_files.is_empty() {
        let empty_msg =
            Paragraph::new("No JSON files found in current directory").style(label_text());
        frame.render_widget(empty_msg, chunks[1]);
    } else {
        let mut lines = vec![];
        for (idx, filename) in app.dialog_input.import_files.iter().enumerate() {
            let config = ListItemConfig::new()
                .selected(app.dialog_input.import_selection.get() == Some(idx))
                .focused(true);
            lines.push(styled_list_item(filename, &config));
        }
        let list = Paragraph::new(lines);
        frame.render_widget(list, chunks[1]);
    }
}

pub(crate) fn render_set_branch_prefix_popup(app: &App, frame: &mut Frame) {
    render_input_popup(
        frame,
        "Set Branch Prefix",
        "Branch Prefix:",
        app.input.as_str(),
        app.input.cursor_byte_offset(),
    );
}
