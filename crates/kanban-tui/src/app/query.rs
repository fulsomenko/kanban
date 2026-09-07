use super::{App, AppMode};
use kanban_domain::{LoadState, Sprint};

impl App {
    /// One sprint tier per board-scoped feature: the scoped tier when it has
    /// resolved, otherwise the flat tier filtered to `board_id`. A scoped
    /// `Loaded` (including an empty one) is authoritative and never falls
    /// back; only `NotLoaded` triggers the fallback.
    pub(crate) fn board_sprints_view(&self, board_id: uuid::Uuid) -> LoadState<Vec<Sprint>> {
        match self.model.board_sprints_state(board_id) {
            LoadState::Loaded(sprints) => LoadState::Loaded(sprints.to_vec()),
            LoadState::NotLoaded => match self.model.sprints_state() {
                LoadState::Loaded(all) => LoadState::Loaded(
                    all.iter()
                        .filter(|s| s.board_id == board_id)
                        .cloned()
                        .collect(),
                ),
                _ => LoadState::NotLoaded,
            },
            other => other.map(|_| Vec::new()),
        }
    }

    pub fn get_current_priority_selection_index(&self) -> usize {
        if let Some(active_id) = self.selection.active_card_id {
            if let Some(card) = self.model.card_by_id_state(active_id).loaded().copied() {
                use kanban_domain::CardPriority;
                return match card.priority {
                    CardPriority::Low => 0,
                    CardPriority::Medium => 1,
                    CardPriority::High => 2,
                    CardPriority::Critical => 3,
                };
            }
        }
        0
    }

    pub fn get_current_sprint_selection_index(&self) -> usize {
        use kanban_view::sprint_assign_list::{build_entries, sprint_id_of};

        if let Some(active_id) = self.selection.active_card_id {
            if let Some(card) = self.model.card_by_id_state(active_id).loaded().copied() {
                if let Some(card_sprint_id) = card.sprint_id {
                    if let Some(board_id) = self.active_board().map(|board| board.id) {
                        if let LoadState::Loaded(sprints) = self.board_sprints_view(board_id) {
                            let entries = build_entries(&sprints, board_id, chrono::Utc::now());
                            for (idx, entry) in entries.iter().enumerate() {
                                if sprint_id_of(entry) == Some(card_sprint_id) {
                                    return idx;
                                }
                            }
                        }
                    }
                }
            }
        }
        0
    }

    pub fn get_current_sort_field_selection_index(&self) -> usize {
        self.filter
            .current_sort_field
            .map(kanban_view::selection_dialog::popup_index_of_sort_field)
            .unwrap_or(0)
    }

    /// The picker row for the projects-panel sort field: the popup index of the
    /// model's current board-list `BoardSortField`. Mirrors the card variant.
    pub fn get_current_board_sort_field_selection_index(&self) -> usize {
        let want_archived = matches!(self.get_base_mode(), AppMode::ArchivedBoardsView);
        let (field, _order) = self.controller.board_sort(want_archived);
        kanban_view::selection_dialog::popup_index_of_board_sort_field(field)
    }
}

#[cfg(test)]
mod active_card_index_regression {
    use crate::test_helpers::{load_with_card_order, setup_reload_resort_fixture};
    use crate::App;
    use kanban_domain::{CardPriority, CardUpdate, KanbanOperations};

    #[test]
    fn test_get_current_priority_selection_index_after_reload_resort_returns_originally_selected_card_priority(
    ) {
        let mut app = App::test_default();
        let fx = setup_reload_resort_fixture(&mut app);

        app.ctx
            .update_card(
                fx.a_id,
                CardUpdate {
                    priority: Some(CardPriority::Critical),
                    ..Default::default()
                },
            )
            .unwrap();
        app.ctx
            .update_card(
                fx.p_id,
                CardUpdate {
                    priority: Some(CardPriority::Low),
                    ..Default::default()
                },
            )
            .unwrap();
        load_with_card_order(&mut app, &[fx.a_id, fx.p_id, fx.b_id, fx.c_id, fx.d_id]);

        let idx = app.get_current_priority_selection_index();

        assert_eq!(
                idx, 3,
                "must return Critical's index (3) — A's priority — not Low's index (0) which is P's priority at A's stale index"
            );
    }

    #[test]
    fn test_get_current_sprint_selection_index_after_reload_resort_returns_originally_selected_card_sprint(
    ) {
        use kanban_view::sprint_assign_list::{build_entries, sprint_id_of};

        let mut app = App::test_default();
        let fx = setup_reload_resort_fixture(&mut app);

        let sprint_a = app.ctx.create_sprint(fx.board_id, None, None).unwrap();
        let sprint_p = app.ctx.create_sprint(fx.board_id, None, None).unwrap();
        app.ctx.assign_card_to_sprint(fx.a_id, sprint_a.id).unwrap();
        app.ctx.assign_card_to_sprint(fx.p_id, sprint_p.id).unwrap();
        load_with_card_order(&mut app, &[fx.a_id, fx.p_id, fx.b_id, fx.c_id, fx.d_id]);

        let sprints = app.model.sprints_state().loaded_or_empty().to_vec();
        let entries = build_entries(&sprints, fx.board_id, chrono::Utc::now());
        let expected_idx = entries
            .iter()
            .position(|e| sprint_id_of(e) == Some(sprint_a.id))
            .expect("sprint_a appears in entries");

        let idx = app.get_current_sprint_selection_index();

        assert_eq!(
            idx, expected_idx,
            "must return A's sprint index, not P's sprint index at A's stale slot"
        );
    }
}

#[cfg(test)]
mod board_columns_view_tests {
    use crate::App;
    use kanban_domain::resolved::Collection;
    use kanban_domain::{Board, Column, DependencyGraph, LoadState, Resolved};
    use std::collections::HashMap;

    fn base_resolved(board: &Board) -> Resolved {
        Resolved {
            boards: Collection {
                all: LoadState::Loaded(vec![board.clone()]),
                ..Default::default()
            },
            cards: Collection {
                all: LoadState::Loaded(vec![]),
                ..Default::default()
            },
            graph: LoadState::Loaded(DependencyGraph::default()),
            ..Default::default()
        }
    }

    #[test]
    fn test_board_columns_view_prefers_the_scoped_tier_and_falls_back_to_the_flat_one() {
        let board = Board::new("B", None::<String>);
        let other_board = Board::new("Other", None::<String>);
        let col_a = Column::new(board.id, "A", 0);
        let col_b = Column::new(board.id, "B", 1);
        let col_other = Column::new(other_board.id, "Other", 0);

        let mut app = App::test_default();
        let mut resolved = base_resolved(&board);
        resolved.columns = Collection {
            by_parent: HashMap::from([(
                board.id,
                LoadState::Loaded(vec![col_a.clone(), col_b.clone()]),
            )]),
            ..Default::default()
        };
        let _ = app.model.apply_resolved(resolved);
        match app.board_columns_view(board.id) {
            LoadState::Loaded(columns) => {
                assert_eq!(columns, vec![col_a.clone(), col_b.clone()]);
            }
            other => panic!("expected the scoped tier, got {other:?}"),
        }

        let mut app = App::test_default();
        let mut resolved = base_resolved(&board);
        resolved.columns = Collection {
            all: LoadState::Loaded(vec![col_a.clone(), col_other.clone()]),
            ..Default::default()
        };
        let _ = app.model.apply_resolved(resolved);
        match app.board_columns_view(board.id) {
            LoadState::Loaded(columns) => assert_eq!(columns, vec![col_a.clone()]),
            other => panic!("expected fallback to the flat tier, got {other:?}"),
        }

        let app = App::test_default();
        assert!(app.board_columns_view(board.id).is_not_loaded());

        let mut app = App::test_default();
        let mut resolved = base_resolved(&board);
        resolved.columns = Collection {
            by_parent: HashMap::from([(
                board.id,
                LoadState::Failed(std::sync::Arc::new(
                    kanban_domain::KanbanError::unsupported("boom"),
                )),
            )]),
            ..Default::default()
        };
        let _ = app.model.apply_resolved(resolved);
        assert!(app.board_columns_view(board.id).is_failed());
    }
}

#[cfg(test)]
mod board_sprints_view_tests {
    use crate::App;
    use kanban_domain::resolved::Collection;
    use kanban_domain::{Board, Column, DependencyGraph, LoadState, Resolved, Sprint};
    use std::collections::HashMap;

    fn base_resolved(board: &Board) -> Resolved {
        Resolved {
            boards: Collection {
                all: LoadState::Loaded(vec![board.clone()]),
                ..Default::default()
            },
            cards: Collection {
                all: LoadState::Loaded(vec![]),
                ..Default::default()
            },
            columns: Collection {
                all: LoadState::Loaded(vec![]),
                ..Default::default()
            },
            graph: LoadState::Loaded(DependencyGraph::default()),
            ..Default::default()
        }
    }

    #[test]
    fn test_board_sprints_view_prefers_the_scoped_tier_and_falls_back_to_the_flat_one() {
        let board = Board::new("B", None::<String>);
        let other_board = Board::new("Other", None::<String>);
        let s_on_board = Sprint::new(board.id, 1, None, None::<String>);
        let s_other_board = Sprint::new(other_board.id, 1, None, None::<String>);

        let mut app = App::test_default();
        let mut resolved = base_resolved(&board);
        resolved.sprints = Collection {
            all: LoadState::Loaded(vec![s_on_board.clone(), s_other_board.clone()]),
            ..Default::default()
        };
        let _ = app.model.apply_resolved(resolved);
        match app.board_sprints_view(board.id) {
            LoadState::Loaded(sprints) => {
                assert_eq!(sprints, vec![s_on_board.clone()]);
            }
            other => panic!("expected fallback to the flat tier, got {other:?}"),
        }

        let app = App::test_default();
        assert!(app.board_sprints_view(board.id).is_not_loaded());

        let mut app = App::test_default();
        let mut resolved = base_resolved(&board);
        resolved.sprints = Collection {
            by_parent: HashMap::from([(
                board.id,
                LoadState::Failed(std::sync::Arc::new(
                    kanban_domain::KanbanError::unsupported("boom"),
                )),
            )]),
            ..Default::default()
        };
        let _ = app.model.apply_resolved(resolved);
        assert!(app.board_sprints_view(board.id).is_failed());

        let mut app = App::test_default();
        let mut resolved = base_resolved(&board);
        resolved.sprints = Collection {
            all: LoadState::Loaded(vec![s_on_board.clone()]),
            by_parent: HashMap::from([(board.id, LoadState::Loaded(Vec::new()))]),
            ..Default::default()
        };
        let _ = app.model.apply_resolved(resolved);
        match app.board_sprints_view(board.id) {
            LoadState::Loaded(sprints) => assert!(sprints.is_empty()),
            other => panic!("expected Loaded(empty), got {other:?}"),
        }
    }

    #[test]
    fn test_sprint_selection_index_matches_scoped_tier_entries() {
        use kanban_view::sprint_assign_list::{build_entries, sprint_id_of};

        let board = Board::new("B", None::<String>);
        let column = Column::new(board.id, "Todo", 0);
        let mut s_a = Sprint::new(board.id, 2, None, None::<String>);
        let mut s_b = Sprint::new(board.id, 1, None, None::<String>);
        s_a.status = kanban_domain::SprintStatus::Planning;
        s_b.status = kanban_domain::SprintStatus::Planning;

        let mut app = App::test_default();
        let mut card = kanban_domain::Card::new(board.id, column.id, "Task", 0);
        card.sprint_id = Some(s_b.id);

        let mut resolved = base_resolved(&board);
        resolved.columns = Collection {
            all: LoadState::Loaded(vec![column]),
            ..Default::default()
        };
        resolved.cards = Collection {
            all: LoadState::Loaded(vec![card.clone()]),
            ..Default::default()
        };
        resolved.sprints = Collection {
            all: LoadState::Loaded(vec![s_a.clone(), s_b.clone()]),
            by_parent: HashMap::from([(board.id, LoadState::Loaded(vec![s_b.clone()]))]),
            ..Default::default()
        };
        let _ = app.model.apply_resolved(resolved);
        app.selection.active_board_id = Some(board.id);
        app.selection.active_card_id = Some(card.id);

        let sprints = app.model.sprints_state();
        let flat_entries = if let LoadState::Loaded(sprints) = sprints {
            build_entries(sprints, board.id, chrono::Utc::now())
        } else {
            panic!("expected flat tier loaded")
        };
        let flat_idx = flat_entries
            .iter()
            .position(|e| sprint_id_of(e) == Some(s_b.id))
            .unwrap();
        assert_eq!(
            flat_idx, 3,
            "sanity: the flat tier lists s_a before s_b, so s_b sits later there"
        );

        let idx = app.get_current_sprint_selection_index();

        assert_eq!(
            idx, 2,
            "must index into the scoped tier's entries ([None, Header, s_b]), not the flat tier's"
        );
    }
}
