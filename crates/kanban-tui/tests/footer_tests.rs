use kanban_tui::app::focus::Focus;
use kanban_tui::App;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

fn render_footer_to_string(app: &App) -> String {
    let backend = TestBackend::new(120, 3);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 120, 3);
            kanban_tui::components::render_footer(app, frame, area);
        })
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut result = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            result.push_str(buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
        }
        result.push('\n');
    }
    result
}

#[test]
fn test_render_footer_normal_mode_renders_without_panic() {
    let (app, _rx) = App::new(None).unwrap();
    let output = render_footer_to_string(&app);
    assert!(output.contains('─') || output.contains('│') || output.contains('┌'));
}

#[test]
fn test_render_footer_search_active_shows_search_query() {
    let (mut app, _rx) = App::new(None).unwrap();
    app.focus.active = Focus::Boards;
    app.filter.search.is_active = true;
    for c in "hello".chars() {
        app.filter.search.input.insert_char(c);
    }
    let output = render_footer_to_string(&app);
    assert!(output.contains("/hello"), "Footer should show search query");
}

#[test]
fn test_render_footer_search_mode_renders_without_panic() {
    use kanban_tui::app::mode::AppMode;
    let (mut app, _rx) = App::new(None).unwrap();
    app.focus.active = Focus::Boards;
    app.push_mode(AppMode::Search);
    let output = render_footer_to_string(&app);
    assert!(!output.trim().is_empty());
}
