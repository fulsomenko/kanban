mod helpers;

use kanban_tui::App;

fn render_help_popup_to_string(app: &mut App) -> String {
    helpers::render_widget_to_string(120, 40, |frame| {
        kanban_tui::components::render_help_popup(app, frame);
    })
}

#[test]
fn test_render_help_popup_renders_without_panic() {
    let mut app = App::test_default();
    let output = render_help_popup_to_string(&mut app);
    assert!(output.contains("Help") || output.contains('─'));
}

#[test]
fn test_help_popup_viewport_height_is_nonzero() {
    use ratatui::layout::Rect;
    let area = Rect::new(0, 0, 120, 40);
    let height = kanban_tui::components::help_popup_viewport_height(area);
    assert!(
        height > 0,
        "Viewport height should be non-zero for a reasonable terminal size"
    );
}

#[test]
fn test_help_popup_viewport_height_scales_with_area() {
    use ratatui::layout::Rect;
    let small = kanban_tui::components::help_popup_viewport_height(Rect::new(0, 0, 80, 24));
    let large = kanban_tui::components::help_popup_viewport_height(Rect::new(0, 0, 200, 60));
    assert!(
        large > small,
        "Larger terminal should yield larger viewport height"
    );
}
