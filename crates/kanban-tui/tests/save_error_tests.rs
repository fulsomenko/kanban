mod helpers;

use kanban_tui::App;

#[test]
fn test_app_has_save_error_field_initially_none() {
    let app = App::test_default();
    assert!(
        app.save_error.is_none(),
        "save_error must be None on a fresh App"
    );
}

#[test]
fn test_set_save_error_stores_message() {
    let mut app = App::test_default();
    app.set_save_error("disk full".to_string());
    assert_eq!(
        app.save_error.as_deref(),
        Some("disk full"),
        "set_save_error must store the message"
    );
}

#[test]
fn test_clear_save_error_resets_to_none() {
    let mut app = App::test_default();
    app.set_save_error("some error".to_string());
    app.clear_save_error();
    assert!(
        app.save_error.is_none(),
        "clear_save_error must reset save_error to None"
    );
}

#[test]
fn test_render_save_error_banner_visible_when_set() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = App::test_default();
    app.set_save_error("disk full".to_string());

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            kanban_tui::ui::render(&mut app, frame);
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let mut output = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            output.push_str(buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
        }
    }

    assert!(
        output.contains("Save error") || output.contains("disk full"),
        "UI must display the save error banner when save_error is Some; got: {output}"
    );
}

#[test]
fn test_render_no_save_error_banner_when_none() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = App::test_default();

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            kanban_tui::ui::render(&mut app, frame);
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let mut output = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            output.push_str(buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
        }
    }

    assert!(
        !output.contains("Save error"),
        "UI must not display save error banner when save_error is None"
    );
}
