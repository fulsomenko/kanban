use crate::app::{App, AppMode, DialogMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

mod board_detail;
mod card_detail;
mod dialogs;
mod main_view;
mod settings_view;
mod sprint_detail;

pub use crate::components::help_popup_viewport_height;
pub use main_view::{build_filter_title_suffix, build_tasks_panel_title};
pub use settings_view::render_settings_view;

fn render_banner(app: &App, frame: &mut Frame, area: Rect) {
    if let Some(ref banner) = app.ui_state.banner {
        banner.render(frame, area);
    }
}

pub fn render(app: &mut App, frame: &mut Frame) {
    // Check if we're in Help mode and render underlying view
    let is_help_mode = matches!(app.mode, AppMode::Help(_));

    if !is_help_mode {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(frame.area());

        // Phase 1: Render base view (from stack if in dialog mode)
        let base_mode = app.get_base_mode();
        match base_mode {
            AppMode::CardDetail => card_detail::render_card_detail_view(app, frame, chunks[0]),
            AppMode::BoardDetail => board_detail::render_board_detail_view(app, frame, chunks[0]),
            AppMode::SprintDetail => {
                sprint_detail::render_sprint_detail_view(app, frame, chunks[0])
            }
            AppMode::Settings => render_settings_view(app, frame, chunks[0]),
            _ => main_view::render_main(app, frame, chunks[0]),
        }
        crate::components::render_footer(app, frame, chunks[1]);

        // Phase 2: Render dialog overlay if active
        if let AppMode::Dialog(ref dialog) = app.mode {
            match dialog {
                // Standard dialogs
                DialogMode::CreateBoard => dialogs::render_create_board_popup(app, frame),
                DialogMode::CreateCard => dialogs::render_create_card_popup(app, frame),
                DialogMode::CreateSprint => dialogs::render_create_sprint_popup(app, frame),
                DialogMode::RenameBoard => dialogs::render_rename_board_popup(app, frame),
                DialogMode::ExportBoard => dialogs::render_export_board_popup(app, frame),
                DialogMode::ExportAll => dialogs::render_export_all_popup(app, frame),
                DialogMode::ImportBoard => dialogs::render_import_board_popup(app, frame),
                DialogMode::SetCardPoints => dialogs::render_set_card_points_popup(app, frame),
                DialogMode::SetCardPriority => dialogs::render_set_card_priority_popup(app, frame),
                DialogMode::SetMultipleCardsPriority => {
                    dialogs::render_set_multiple_cards_priority_popup(app, frame)
                }
                DialogMode::SetBranchPrefix => dialogs::render_set_branch_prefix_popup(app, frame),
                DialogMode::SetSprintPrefix => dialogs::render_set_sprint_prefix_popup(app, frame),
                DialogMode::SetSprintCardPrefix => {
                    dialogs::render_set_sprint_card_prefix_popup(app, frame)
                }
                DialogMode::OrderCards => dialogs::render_order_cards_popup(app, frame),
                DialogMode::CreateColumn => dialogs::render_create_column_popup(app, frame),
                DialogMode::RenameColumn => dialogs::render_rename_column_popup(app, frame),
                DialogMode::DeleteColumnConfirm => {
                    dialogs::render_delete_column_confirm_popup(app, frame)
                }
                DialogMode::SelectTaskListView => {
                    dialogs::render_select_task_list_view_popup(app, frame)
                }
                DialogMode::AssignCardToSprint => dialogs::render_assign_sprint_popup(app, frame),
                DialogMode::AssignMultipleCardsToSprint => {
                    dialogs::render_assign_multiple_cards_popup(app, frame)
                }
                DialogMode::CarryOverSprint => dialogs::render_carry_over_sprint_popup(app, frame),
                DialogMode::ExportBoards => dialogs::render_export_boards_popup(app, frame),
                // Component-based popups
                DialogMode::FilterOptions => {
                    crate::components::render_filter_options_popup(app, frame)
                }
                DialogMode::ConflictResolution => {
                    crate::components::render_conflict_resolution_popup(app, frame)
                }
                DialogMode::ExternalChangeDetected => {
                    crate::components::render_external_change_detected_popup(app, frame)
                }
                DialogMode::ManageParents => {
                    crate::components::render_manage_parents_popup(app, frame)
                }
                DialogMode::ManageChildren => {
                    crate::components::render_manage_children_popup(app, frame)
                }
                DialogMode::ConfirmSprintPrefixCollision => {}
            }
        }
    } else {
        // Help mode: render base view without footer, then help popup
        let base_mode = app.get_base_mode();
        match base_mode {
            AppMode::CardDetail => card_detail::render_card_detail_view(app, frame, frame.area()),
            AppMode::BoardDetail => {
                board_detail::render_board_detail_view(app, frame, frame.area())
            }
            AppMode::SprintDetail => {
                sprint_detail::render_sprint_detail_view(app, frame, frame.area())
            }
            AppMode::Settings => render_settings_view(app, frame, frame.area()),
            _ => main_view::render_main(app, frame, frame.area()),
        }
        app.view.last_frame_area = frame.area();
        crate::components::render_help_popup(app, frame);
    }

    // Render banner on top if present
    if app.ui_state.banner.is_some() {
        let banner_area = Rect {
            x: 0,
            y: 0,
            width: frame.area().width,
            height: 3,
        };
        render_banner(app, frame, banner_area);
    }
}
