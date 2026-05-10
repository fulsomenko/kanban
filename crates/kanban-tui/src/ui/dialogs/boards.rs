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
                .model
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

pub(crate) fn render_choose_storage_file_popup(app: &App, frame: &mut Frame) {
    use crate::app::StorageBackendChoice;
    use crate::components::centered_rect;
    use ratatui::widgets::{Block, Borders, Clear};

    let area = centered_rect(70, 45, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("No board file configured")
        .borders(Borders::ALL)
        .style(popup_bg());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(4), // description
            Constraint::Length(1), // spacer
            Constraint::Length(1), // "Filename:" label
            Constraint::Length(3), // input box
            Constraint::Length(1), // resolved-path preview
            Constraint::Length(1), // spacer
            Constraint::Length(1), // format radio
            Constraint::Length(1), // spacer
            Constraint::Length(1), // hint
            Constraint::Min(0),
        ])
        .split(inner);

    let bold_normal = normal_text().add_modifier(Modifier::BOLD);
    let bold_label = label_text().add_modifier(Modifier::BOLD);

    let description = vec![
        Line::from(Span::styled(
            "Enter a filename to create a board file, or press Escape",
            normal_text(),
        )),
        Line::from(Span::styled(
            "to continue without one. Work done without a file is held",
            normal_text(),
        )),
        Line::from(Span::styled(
            "in memory and lost when you quit — you can export it at",
            normal_text(),
        )),
        Line::from(vec![
            Span::styled("any time with '", normal_text()),
            Span::styled("x", bold_normal),
            Span::styled("'.", normal_text()),
        ]),
    ];
    frame.render_widget(Paragraph::new(description), chunks[0]);

    let label = Paragraph::new("Filename:").style(highlight_text());
    frame.render_widget(label, chunks[2]);

    let input = Paragraph::new(app.input.as_str())
        .style(normal_text())
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(input, chunks[3]);

    let cursor_x = chunks[3].x + app.input.cursor_byte_offset() as u16 + 1;
    let cursor_y = chunks[3].y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));

    let resolved = resolve_dialog_path(app.input.as_str());
    let preview = Paragraph::new(Line::from(vec![
        Span::styled("Will be saved at: ", label_text()),
        Span::styled(resolved, normal_text()),
    ]));
    frame.render_widget(preview, chunks[4]);

    let radio = Line::from(vec![
        Span::styled("Format: ", highlight_text()),
        radio_marker(
            app.choose_storage_backend,
            StorageBackendChoice::Json,
            "JSON",
        ),
        Span::styled("   ", normal_text()),
        radio_marker(
            app.choose_storage_backend,
            StorageBackendChoice::Sqlite,
            "SQLite",
        ),
        Span::styled("    (", label_text()),
        Span::styled("Tab", bold_label),
        Span::styled(" to toggle)", label_text()),
    ]);
    frame.render_widget(Paragraph::new(radio), chunks[6]);

    let hint = Line::from(vec![
        Span::styled("Enter", bold_label),
        Span::styled(" — create file   ", label_text()),
        Span::styled("Esc", bold_label),
        Span::styled(" — continue in memory", label_text()),
    ]);
    frame.render_widget(Paragraph::new(hint), chunks[8]);
}

fn radio_marker(
    selected: crate::app::StorageBackendChoice,
    choice: crate::app::StorageBackendChoice,
    label: &str,
) -> Span<'static> {
    let marker = if selected == choice { "(*)" } else { "( )" };
    let style = if selected == choice {
        highlight_text()
    } else {
        normal_text()
    };
    Span::styled(format!("{} {}", marker, label), style)
}

fn resolve_dialog_path(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    let path = std::path::Path::new(input);
    if path.is_absolute() {
        return path.display().to_string();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path).display().to_string())
        .unwrap_or_else(|_| input.to_string())
}
