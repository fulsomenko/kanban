use kanban_tui::app::mode::{AppMode, DialogMode};
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

fn setup_app_with_filter_dialog() -> App {
    use kanban_tui::filters::FilterDialogState;
    use kanban_domain::CardFilters;
    let (mut app, _rx) = App::new(None).unwrap();
    app.push_mode(AppMode::Dialog(DialogMode::FilterOptions));
    app.filter.dialog_state = Some(FilterDialogState::new(CardFilters::default()));
    app
}

#[test]
fn test_render_filter_options_popup_renders_without_dialog_state() {
    let (app, _rx) = App::new(None).unwrap();
    let output = render_to_string(|frame| {
        kanban_tui::components::render_filter_options_popup(&app, frame);
    });
    // Without dialog state active, the popup renders but shows nothing inside
    assert!(!output.trim().is_empty());
}

#[test]
fn test_render_filter_options_popup_shows_sprints_section() {
    let app = setup_app_with_filter_dialog();
    let output = render_to_string(|frame| {
        kanban_tui::components::render_filter_options_popup(&app, frame);
    });
    assert!(output.contains("Sprints") || output.contains("sprint"));
}

#[test]
fn test_render_filter_options_popup_shows_date_range_section() {
    let app = setup_app_with_filter_dialog();
    let output = render_to_string(|frame| {
        kanban_tui::components::render_filter_options_popup(&app, frame);
    });
    assert!(output.contains("Date") || output.contains("Range"));
}

#[test]
fn test_render_filter_options_popup_shows_tags_section() {
    let app = setup_app_with_filter_dialog();
    let output = render_to_string(|frame| {
        kanban_tui::components::render_filter_options_popup(&app, frame);
    });
    assert!(output.contains("Tags") || output.contains("tags"));
}
