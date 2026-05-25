mod helpers;

use chrono::{Duration, Utc};
use crossterm::event::KeyCode;
use kanban_domain::{
    field_update::FieldUpdate, CreateCardOptions, KanbanOperations, Sprint, SprintUpdate,
};
use kanban_tui::components::{build_entries, sprint_id_of};
use kanban_tui::{
    app::mode::{AppMode, DialogMode},
    components::{SelectionDialog, SprintAssignDialog},
    App,
};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use uuid::Uuid;

const TEST_BACKEND_WIDTH: u16 = 120;
const TEST_BACKEND_HEIGHT: u16 = 30;

struct DialogFixture {
    app: App,
    card_id: Uuid,
    completed_id: Uuid,
    ended_id: Uuid,
}

fn setup_app_with_sprints() -> DialogFixture {
    let mut app = App::test_default();
    let board = app.ctx.create_board("B".into(), None).unwrap();
    let column = app
        .ctx
        .create_column(board.id, "Todo".into(), None)
        .unwrap();
    let card = app
        .ctx
        .create_card(
            board.id,
            column.id,
            "Task".into(),
            CreateCardOptions::default(),
        )
        .unwrap();

    app.ctx.create_sprint(board.id, None, None).unwrap();

    let active = app.ctx.create_sprint(board.id, None, None).unwrap();
    app.ctx.activate_sprint(active.id, Some(7)).unwrap();
    let past = Utc::now() - Duration::days(1);
    app.ctx
        .update_sprint(
            active.id,
            SprintUpdate {
                end_date: FieldUpdate::Set(past),
                ..Default::default()
            },
        )
        .unwrap();

    let to_complete = app.ctx.create_sprint(board.id, None, None).unwrap();
    app.ctx.activate_sprint(to_complete.id, Some(7)).unwrap();
    app.ctx.complete_sprint(to_complete.id).unwrap();

    app.selection.active_board_index = Some(0);
    app.selection.active_card_id = Some(card.id);
    app.prepare_frame();

    DialogFixture {
        app,
        card_id: card.id,
        completed_id: to_complete.id,
        ended_id: active.id,
    }
}

fn open_assign_dialog(app: &mut App) {
    app.dialog_input.sprint_assign_selection.set(Some(0));
    app.push_mode(AppMode::Dialog(DialogMode::AssignCardToSprint));
}

fn render_dialog_to_string(app: &App) -> String {
    let backend = TestBackend::new(TEST_BACKEND_WIDTH, TEST_BACKEND_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let dialog = SprintAssignDialog;
            dialog.render(app, frame);
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

fn render_dialog_with_colors(app: &App) -> Vec<(String, Option<Color>)> {
    let backend = TestBackend::new(TEST_BACKEND_WIDTH, TEST_BACKEND_HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let dialog = SprintAssignDialog;
            dialog.render(app, frame);
        })
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut result = Vec::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = buffer.cell((x, y)).unwrap();
            result.push((cell.symbol().to_string(), cell.style().fg));
        }
    }
    result
}

fn line_color(grid: &[(String, Option<Color>)], substring: &str) -> Option<Color> {
    let width = TEST_BACKEND_WIDTH as usize;
    let height = grid.len() / width;
    for y in 0..height {
        let line: String = (0..width)
            .map(|x| grid[y * width + x].0.clone())
            .collect::<Vec<_>>()
            .join("");
        if let Some(start) = line.find(substring) {
            // Return the fg color of the first cell of the substring.
            return grid[y * width + start].1;
        }
    }
    None
}

#[test]
fn test_dialog_renders_active_planned_header_when_active_sprints_exist() {
    let fx = setup_app_with_sprints();
    let mut app = fx.app;
    open_assign_dialog(&mut app);
    let output = render_dialog_to_string(&app);
    assert!(
        output.contains("Active / Planned"),
        "expected Active / Planned header in dialog output:\n{}",
        output
    );
}

#[test]
fn test_dialog_renders_completed_ended_header_when_either_completed_or_ended_exist() {
    let fx = setup_app_with_sprints();
    let mut app = fx.app;
    open_assign_dialog(&mut app);
    let output = render_dialog_to_string(&app);
    assert!(
        output.contains("Completed / Ended"),
        "expected Completed / Ended header in dialog output:\n{}",
        output
    );
}

#[test]
fn test_dialog_renders_completed_in_green_and_ended_in_red() {
    let fx = setup_app_with_sprints();
    let mut app = fx.app;
    open_assign_dialog(&mut app);

    // Find the completed and ended sprint by their formatted names.
    let completed = app
        .model
        .sprints()
        .iter()
        .find(|s| s.id == fx.completed_id)
        .cloned()
        .unwrap();
    let ended = app
        .model
        .sprints()
        .iter()
        .find(|s| s.id == fx.ended_id)
        .cloned()
        .unwrap();
    let board = app.model.boards()[0].clone();
    let completed_name = completed.formatted_name(&board, "sprint");
    let ended_name = ended.formatted_name(&board, "sprint");

    let grid = render_dialog_with_colors(&app);
    let completed_color = line_color(&grid, &completed_name);
    let ended_color = line_color(&grid, &ended_name);

    assert_eq!(
        completed_color,
        Some(Color::Green),
        "completed sprint should be green; got {:?} for {:?}",
        completed_color,
        completed_name
    );
    assert_eq!(
        ended_color,
        Some(Color::Red),
        "ended sprint should be red; got {:?} for {:?}",
        ended_color,
        ended_name
    );
}

#[test]
fn test_down_arrow_skips_headers() {
    let fx = setup_app_with_sprints();
    let mut app = fx.app;
    open_assign_dialog(&mut app);
    // Selection starts at 0 (None entry).
    assert_eq!(app.dialog_input.sprint_assign_selection.get(), Some(0));

    // Pressing j once should land on the first selectable entry past the
    // (None) entry — the "Active / Planned" header is at index 1, so
    // selection should jump to index 2 (the first sprint).
    app.handle_assign_card_to_sprint_popup(KeyCode::Char('j'));
    assert_eq!(
        app.dialog_input.sprint_assign_selection.get(),
        Some(2),
        "down should skip the Active / Planned header"
    );
}

#[test]
fn test_assigning_to_completed_sprint_succeeds() {
    let fx = setup_app_with_sprints();
    let mut app = fx.app;
    let card_id = fx.card_id;
    let target_sprint = fx.completed_id;

    open_assign_dialog(&mut app);

    // Walk down with j until selection_idx points at the completed sprint.
    let max_steps = 20;
    let mut steps = 0;
    loop {
        let idx = app
            .dialog_input
            .sprint_assign_selection
            .get()
            .expect("selection set");
        if let Some(s) = sprint_at(&app, idx) {
            if s == target_sprint {
                break;
            }
        }
        app.handle_assign_card_to_sprint_popup(KeyCode::Char('j'));
        steps += 1;
        assert!(steps < max_steps, "could not navigate to completed sprint");
    }
    app.handle_assign_card_to_sprint_popup(KeyCode::Enter);
    let card = app.ctx.get_card(card_id).unwrap().unwrap();
    assert_eq!(
        card.sprint_id,
        Some(target_sprint),
        "card should be assigned to the completed sprint"
    );
}

#[test]
fn test_assigning_to_ended_sprint_succeeds() {
    let fx = setup_app_with_sprints();
    let mut app = fx.app;
    let card_id = fx.card_id;
    let target_sprint = fx.ended_id;

    open_assign_dialog(&mut app);

    let max_steps = 20;
    let mut steps = 0;
    loop {
        let idx = app
            .dialog_input
            .sprint_assign_selection
            .get()
            .expect("selection set");
        if let Some(s) = sprint_at(&app, idx) {
            if s == target_sprint {
                break;
            }
        }
        app.handle_assign_card_to_sprint_popup(KeyCode::Char('j'));
        steps += 1;
        assert!(steps < max_steps, "could not navigate to ended sprint");
    }
    app.handle_assign_card_to_sprint_popup(KeyCode::Enter);
    let card = app.ctx.get_card(card_id).unwrap().unwrap();
    assert_eq!(
        card.sprint_id,
        Some(target_sprint),
        "card should be assigned to the ended sprint"
    );
}

#[test]
fn test_bulk_assign_handler_supports_completed_sprint() {
    let fx = setup_app_with_sprints();
    let mut app = fx.app;
    let card_id = fx.card_id;
    let target_sprint = fx.completed_id;

    // Switch to bulk-assign flow with one selected card.
    app.multi_select.selected_cards.insert(card_id);
    app.dialog_input.sprint_assign_selection.set(Some(0));
    app.push_mode(AppMode::Dialog(DialogMode::AssignMultipleCardsToSprint));

    let max_steps = 20;
    let mut steps = 0;
    loop {
        let idx = app
            .dialog_input
            .sprint_assign_selection
            .get()
            .expect("selection set");
        if let Some(s) = sprint_at(&app, idx) {
            if s == target_sprint {
                break;
            }
        }
        app.handle_assign_multiple_cards_to_sprint_popup(KeyCode::Char('j'));
        steps += 1;
        assert!(steps < max_steps, "could not navigate to completed sprint");
    }
    app.handle_assign_multiple_cards_to_sprint_popup(KeyCode::Enter);
    let card = app.ctx.get_card(card_id).unwrap().unwrap();
    assert_eq!(
        card.sprint_id,
        Some(target_sprint),
        "bulk-assigned card should land on the completed sprint"
    );
}

#[test]
fn test_current_sprint_indicator_does_not_apply_color_override_in_completed_ended_section() {
    let fx = setup_app_with_sprints();
    let mut app = fx.app;
    let card_id = fx.card_id;
    let completed_id = fx.completed_id;

    // Pre-assign card to the completed sprint (simulates the retrospective
    // scenario where the user previously assigned to a Completed sprint).
    app.ctx
        .assign_card_to_sprint(card_id, completed_id)
        .unwrap();
    app.prepare_frame();

    open_assign_dialog(&mut app);

    let board = app.model.boards()[0].clone();
    let completed = app
        .model
        .sprints()
        .iter()
        .find(|s| s.id == completed_id)
        .cloned()
        .unwrap();
    let completed_name = completed.formatted_name(&board, "sprint");

    let grid = render_dialog_with_colors(&app);

    // The completed entry should still render in green (status colour wins),
    // not in any "current sprint" highlight that would override it.
    let color = line_color(&grid, &completed_name);
    assert_eq!(
        color,
        Some(Color::Green),
        "completed-current sprint must keep green status colour, got {:?}",
        color
    );

    // It should also carry a " (current)" suffix for identification.
    let output = render_dialog_to_string(&app);
    assert!(
        output.contains(&format!("{} (current)", completed_name)),
        "expected ' (current)' suffix on the completed-current entry; output:\n{}",
        output
    );
}

// Helper: returns the sprint id selected by the dialog at index `idx`,
// using the same entry layout the renderer/handler uses.
fn sprint_at(app: &App, idx: usize) -> Option<Uuid> {
    let board = app.model.boards().first().cloned()?;
    let sprints: Vec<Sprint> = app.model.sprints().to_vec();
    let entries = build_entries(&sprints, board.id, Utc::now());
    sprint_id_of(entries.get(idx)?)
}

#[test]
fn test_dialog_scrolls_to_keep_selected_sprint_visible_when_list_overflows() {
    // 30 planning sprints overflows the dialog's visible area at the
    // standard 120x30 test backend size.
    let mut app = App::test_default();
    let board = app.ctx.create_board("B".into(), None).unwrap();
    let column = app
        .ctx
        .create_column(board.id, "Todo".into(), None)
        .unwrap();
    let card = app
        .ctx
        .create_card(
            board.id,
            column.id,
            "Task".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    for _ in 0..30 {
        app.ctx.create_sprint(board.id, None, None).unwrap();
    }
    app.selection.active_board_index = Some(0);
    app.selection.active_card_id = Some(card.id);
    app.prepare_frame();

    open_assign_dialog(&mut app);

    // Sprints are sorted by sprint_number desc, so the oldest sprint sits at
    // the bottom of the active section — guaranteed off-screen without scroll.
    let oldest_id = app
        .model
        .sprints()
        .iter()
        .min_by_key(|s| s.sprint_number)
        .map(|s| s.id)
        .unwrap();

    let max_steps = 50;
    let mut steps = 0;
    loop {
        let idx = app
            .dialog_input
            .sprint_assign_selection
            .get()
            .expect("selection set");
        if sprint_at(&app, idx) == Some(oldest_id) {
            break;
        }
        app.handle_assign_card_to_sprint_popup(KeyCode::Char('j'));
        steps += 1;
        assert!(steps < max_steps, "could not navigate to oldest sprint");
    }

    let board_after = app.model.boards()[0].clone();
    let oldest = app
        .model
        .sprints()
        .iter()
        .find(|s| s.id == oldest_id)
        .cloned()
        .unwrap();
    let oldest_name = oldest.formatted_name(&board_after, "sprint");

    let output = render_dialog_to_string(&app);
    assert!(
        output.contains(&oldest_name),
        "selected sprint at end of long list must remain visible after scroll; output:\n{}",
        output
    );
}

#[test]
fn test_sticky_header_appears_at_top_when_scrolled_past_active_planned_header() {
    // 30 planning sprints — selected at the last entry forces the
    // Active / Planned header (at entry index 1) to scroll off the top.
    let mut app = App::test_default();
    let board = app.ctx.create_board("B".into(), None).unwrap();
    let column = app
        .ctx
        .create_column(board.id, "Todo".into(), None)
        .unwrap();
    let card = app
        .ctx
        .create_card(
            board.id,
            column.id,
            "Task".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    for _ in 0..30 {
        app.ctx.create_sprint(board.id, None, None).unwrap();
    }
    app.selection.active_board_index = Some(0);
    app.selection.active_card_id = Some(card.id);
    app.prepare_frame();

    open_assign_dialog(&mut app);

    // Navigate to the last selectable entry.
    for _ in 0..50 {
        app.handle_assign_card_to_sprint_popup(KeyCode::Char('j'));
    }

    let output = render_dialog_to_string(&app);
    assert!(
        output.contains("Active / Planned"),
        "Active / Planned header must be pinned at the top of the list area when scrolled past; output:\n{}",
        output
    );
}

#[test]
fn test_sticky_header_does_not_duplicate_when_list_fits_without_scrolling() {
    // Short list — header sits at its natural position; no overlay should
    // render so the label appears exactly once in the output.
    let fx = setup_app_with_sprints();
    let mut app = fx.app;
    open_assign_dialog(&mut app);

    let output = render_dialog_to_string(&app);
    let count = output.matches("Active / Planned").count();
    assert_eq!(
        count, 1,
        "without scroll, Active / Planned should appear exactly once (natural position); got {} in:\n{}",
        count, output
    );
}

#[test]
fn test_sticky_header_switches_to_completed_ended_when_selecting_in_lower_section() {
    // 30 planning + 30 completed sprints. With the selection on the last
    // completed entry, both section headers have scrolled off — the overlay
    // should pin the *enclosing* section's header (Completed / Ended).
    let mut app = App::test_default();
    let board = app.ctx.create_board("B".into(), None).unwrap();
    let column = app
        .ctx
        .create_column(board.id, "Todo".into(), None)
        .unwrap();
    let card = app
        .ctx
        .create_card(
            board.id,
            column.id,
            "Task".into(),
            CreateCardOptions::default(),
        )
        .unwrap();
    for _ in 0..30 {
        app.ctx.create_sprint(board.id, None, None).unwrap();
    }
    for _ in 0..30 {
        let s = app.ctx.create_sprint(board.id, None, None).unwrap();
        app.ctx.activate_sprint(s.id, Some(7)).unwrap();
        app.ctx.complete_sprint(s.id).unwrap();
    }
    app.selection.active_board_index = Some(0);
    app.selection.active_card_id = Some(card.id);
    app.prepare_frame();

    open_assign_dialog(&mut app);

    // Walk all the way to the last entry — guaranteed to be a completed sprint
    // with the lower section's header scrolled off.
    for _ in 0..200 {
        app.handle_assign_card_to_sprint_popup(KeyCode::Char('j'));
    }

    let output = render_dialog_to_string(&app);
    assert!(
        output.contains("Completed / Ended"),
        "Completed / Ended must be pinned at the top when selection is in the lower section; output:\n{}",
        output
    );
    // The upper section's header is also scrolled off but should NOT appear —
    // the overlay shows the *enclosing* section header for the selection.
    assert!(
        !output.contains("Active / Planned"),
        "Active / Planned must not appear when selection is past it and the overlay belongs to the lower section; output:\n{}",
        output
    );
}
