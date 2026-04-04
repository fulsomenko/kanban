use kanban_tui::App;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_to_string<F>(draw_fn: F) -> String
where
    F: FnOnce(&mut ratatui::Frame),
{
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(draw_fn).unwrap();
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
fn test_render_manage_parents_popup_renders_without_panic() {
    let (app, _rx) = App::new(None).unwrap();
    let output = render_to_string(|frame| {
        kanban_tui::components::render_manage_parents_popup(&app, frame);
    });
    assert!(output.contains("Parents") || output.contains('─'));
}

#[test]
fn test_render_manage_children_popup_renders_without_panic() {
    let (app, _rx) = App::new(None).unwrap();
    let output = render_to_string(|frame| {
        kanban_tui::components::render_manage_children_popup(&app, frame);
    });
    assert!(output.contains("Children") || output.contains('─'));
}

#[test]
fn test_render_manage_parents_popup_shows_search_box() {
    let (app, _rx) = App::new(None).unwrap();
    let output = render_to_string(|frame| {
        kanban_tui::components::render_manage_parents_popup(&app, frame);
    });
    assert!(output.contains("Search") || output.contains("search"));
}

#[test]
fn test_render_manage_parents_popup_shows_no_cards_when_empty() {
    let (app, _rx) = App::new(None).unwrap();
    let output = render_to_string(|frame| {
        kanban_tui::components::render_manage_parents_popup(&app, frame);
    });
    assert!(output.contains("No eligible") || output.contains("eligible"));
}
