use super::App;
use kanban_domain::LoadState;
use uuid::Uuid;

impl App {
    /// Ended sprints on the ACTIVE board only, or `None` while no board is
    /// active or its scoped sprint tier has not loaded.
    pub(in crate::app) fn check_ended_sprints(&self) -> Option<Vec<Uuid>> {
        let board_id = self.scope_board_id()?;
        let LoadState::Loaded(sprints) = self.board_sprints_view(board_id) else {
            return None;
        };
        let ended_sprints: Vec<_> = sprints
            .iter()
            .filter(|s| s.is_ended(chrono::Utc::now()))
            .collect();

        if !ended_sprints.is_empty() {
            tracing::warn!(
                "Found {} ended sprint(s) that need attention:",
                ended_sprints.len()
            );
            if let LoadState::Loaded(boards) = self.model.boards_state() {
                for sprint in &ended_sprints {
                    if let Some(board) = boards.iter().find(|b| b.id == sprint.board_id) {
                        tracing::warn!(
                            "  - {} (ended: {})",
                            sprint.formatted_name(board, None),
                            sprint
                                .end_date
                                .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
                                .unwrap_or_else(|| "unknown".to_string())
                        );
                    }
                }
            }
        }

        Some(ended_sprints.into_iter().map(|s| s.id).collect())
    }

    pub(in crate::app) fn migrate_sprint_logs(&mut self) -> usize {
        match self.ctx.migrate_sprint_logs() {
            Ok(n) => n,
            Err(e) => {
                tracing::error!("Failed to migrate sprint logs: {}", e);
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::App;
    use kanban_domain::{
        EntityIds, FieldUpdate, Invalidation, KanbanOperations, LoadState, SprintStatus,
        SprintUpdate,
    };

    fn seed_ended_sprint(app: &mut App) -> (uuid::Uuid, uuid::Uuid) {
        let board = app.ctx.create_board("Board".into(), None).unwrap();
        let sprint = app.ctx.create_sprint(board.id, None, None).unwrap();
        app.ctx
            .update_sprint(
                sprint.id,
                SprintUpdate {
                    status: Some(SprintStatus::Active),
                    end_date: FieldUpdate::Set(chrono::Utc::now() - chrono::Duration::days(1)),
                    ..Default::default()
                },
            )
            .unwrap();
        (board.id, sprint.id)
    }

    #[test]
    fn test_check_ended_sprints_does_not_scan_an_unloaded_sprint_tier() {
        let mut app = App::test_default();
        let (board_id, _sprint_id) = seed_ended_sprint(&mut app);
        assert!(app.model.board_sprints_state(board_id).is_not_loaded());

        let ended = app.check_ended_sprints();

        assert_eq!(
            ended, None,
            "a NotLoaded sprint tier must decline to scan, not report zero ended sprints"
        );
    }

    #[test]
    fn test_check_ended_sprints_reports_ended_sprints_in_the_loaded_tier() {
        let mut app = App::test_default();
        let (board_id, sprint_id) = seed_ended_sprint(&mut app);
        let snap = kanban_service::read_full_snapshot(app.ctx.data_store()).unwrap();
        let _ = app.model.load_from_snapshot(snap);
        app.selection.active_board_id = Some(board_id);
        assert!(app.model.board_sprints_state(board_id).is_loaded());

        let ended = app.check_ended_sprints();

        assert_eq!(ended, Some(vec![sprint_id]));
    }

    #[test]
    fn test_check_ended_sprints_with_a_not_loaded_boards_tier_skips_the_board_name_lookup() {
        let mut app = App::test_default();
        let (board_id, sprint_id) = seed_ended_sprint(&mut app);
        let snap = kanban_service::read_full_snapshot(app.ctx.data_store()).unwrap();
        let _ = app.model.load_from_snapshot(snap);
        app.selection.active_board_id = Some(board_id);
        assert!(app.model.board_sprints_state(board_id).is_loaded());

        let _ = app
            .model
            .invalidate(Invalidation::Entities(EntityIds::boards([
                uuid::Uuid::new_v4(),
            ])));
        assert!(matches!(app.model.boards_state(), LoadState::NotLoaded));

        let ended = app.check_ended_sprints();

        assert_eq!(
            ended,
            Some(vec![sprint_id]),
            "the sprint-tier scan must be unaffected by a NotLoaded boards tier"
        );
    }

    #[test]
    fn test_check_ended_sprints_scopes_to_the_active_board() {
        let mut app = App::test_default();
        let (board_a, sprint_a) = seed_ended_sprint(&mut app);
        let (_board_b, _sprint_b) = seed_ended_sprint(&mut app);
        let snap = kanban_service::read_full_snapshot(app.ctx.data_store()).unwrap();
        let _ = app.model.load_from_snapshot(snap);

        app.selection.active_board_id = Some(board_a);
        assert_eq!(app.check_ended_sprints(), Some(vec![sprint_a]));

        app.selection.active_board_id = None;
        assert_eq!(app.check_ended_sprints(), None);
    }
}
