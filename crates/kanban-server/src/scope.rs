#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use kanban_domain::resolved::Collection;
    use kanban_domain::{Board, Column, KanbanError, LoadState, Model, Resolved};
    use uuid::Uuid;

    use super::*;
    use kanban_service::FetchPlan;

    #[test]
    fn test_route_scope_for_board_list_requests_only_the_board_list() {
        let round = RouteScope::BoardList.next_round(&Model::default());

        assert_eq!(
            round,
            FetchRound {
                board_list: true,
                ..Default::default()
            }
        );
        assert!(!round.archived_board_list);
        assert!(round.boards.is_empty());
    }

    #[test]
    fn test_route_scope_for_single_board_requests_the_board_by_id_and_archived_markers() {
        let id = Uuid::new_v4();
        let round = RouteScope::Board(id).next_round(&Model::default());

        assert_eq!(round.boards, vec![id]);
        assert!(round.archived_board_list);
        assert!(!round.board_list);
    }

    #[test]
    fn test_route_scope_for_board_cards_requests_only_board_scoped_tiers() {
        let board_id = Uuid::new_v4();
        let round = RouteScope::BoardCards {
            board_id,
            column_id: None,
        }
        .next_round(&Model::default());

        assert_eq!(round.boards, vec![board_id]);
        assert_eq!(round.columns_by_board, vec![board_id]);
        assert_eq!(round.archived_cards_by_board, vec![board_id]);
        assert!(!round.card_list);
        assert!(!round.board_list);
        assert!(!round.column_list);
        assert!(round.cards_by_column.is_empty());
    }

    #[test]
    fn test_route_scope_for_board_cards_walks_loaded_columns_into_a_second_round_then_halts() {
        let board = Board::new("Kanban", None::<String>);
        let board_id = board.id;
        let column = Column::new(board_id, "TODO", 0);
        let column_id = column.id;

        let mut model = Model::default();
        let _ = model.apply_resolved(Resolved {
            boards: Collection {
                by_id: [(board_id, LoadState::Loaded(board))].into(),
                ..Default::default()
            },
            columns: Collection {
                by_parent: [(board_id, LoadState::Loaded(vec![column]))].into(),
                ..Default::default()
            },
            archived_cards: Collection {
                by_parent: [(board_id, LoadState::Loaded(vec![]))].into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let scope = RouteScope::BoardCards {
            board_id,
            column_id: None,
        };
        let round2 = scope.next_round(&model);
        assert_eq!(
            round2,
            FetchRound {
                cards_by_column: vec![column_id],
                ..Default::default()
            }
        );

        let mut model2 = model;
        let _ = model2.apply_resolved(Resolved {
            cards: Collection {
                by_parent: [(column_id, LoadState::Loaded(vec![]))].into(),
                ..Default::default()
            },
            ..Default::default()
        });
        let round3 = scope.next_round(&model2);
        assert!(round3.is_empty());
    }

    #[test]
    fn test_route_scope_for_board_cards_filtered_by_column_requests_that_column_in_the_first_round()
     {
        let board_id = Uuid::new_v4();
        let column_id = Uuid::new_v4();
        let round = RouteScope::BoardCards {
            board_id,
            column_id: Some(column_id),
        }
        .next_round(&Model::default());

        assert_eq!(round.cards_by_column, vec![column_id]);
        assert_eq!(round.boards, vec![board_id]);
        assert_eq!(round.columns_by_board, vec![board_id]);
    }

    #[test]
    fn test_route_scope_for_board_archived_cards_requests_the_board_and_its_markers() {
        let board_id = Uuid::new_v4();
        let round = RouteScope::BoardArchivedCards(board_id).next_round(&Model::default());

        assert_eq!(round.boards, vec![board_id]);
        assert_eq!(round.archived_cards_by_board, vec![board_id]);
        assert!(!round.board_list);
        assert!(!round.archived_card_list);
    }

    #[test]
    fn test_route_scope_for_board_columns_and_board_sprints_request_the_board_and_its_scope() {
        let board_id = Uuid::new_v4();

        let columns_round = RouteScope::BoardColumns(board_id).next_round(&Model::default());
        assert_eq!(columns_round.boards, vec![board_id]);
        assert_eq!(columns_round.columns_by_board, vec![board_id]);
        assert_eq!(
            columns_round,
            FetchRound {
                boards: vec![board_id],
                columns_by_board: vec![board_id],
                ..Default::default()
            }
        );

        let sprints_round = RouteScope::BoardSprints(board_id).next_round(&Model::default());
        assert_eq!(
            sprints_round,
            FetchRound {
                boards: vec![board_id],
                sprints_by_board: vec![board_id],
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_route_scope_for_a_nested_sprint_requests_its_board_but_the_flat_form_cannot() {
        let board_id = Uuid::new_v4();
        let sprint_id = Uuid::new_v4();

        let nested_round = RouteScope::Sprint {
            board_id: Some(board_id),
            sprint_id,
        }
        .next_round(&Model::default());
        assert_eq!(nested_round.sprints, vec![sprint_id]);
        assert_eq!(nested_round.boards, vec![board_id]);

        let flat_round = RouteScope::Sprint {
            board_id: None,
            sprint_id,
        }
        .next_round(&Model::default());
        assert_eq!(flat_round.sprints, vec![sprint_id]);
        assert!(flat_round.boards.is_empty());
        assert!(!flat_round.board_list);
    }

    #[test]
    fn test_route_scope_for_a_card_or_column_requests_only_that_entitys_per_id_tier() {
        let card_id = Uuid::new_v4();
        let card_round = RouteScope::Card(card_id).next_round(&Model::default());
        assert_eq!(card_round.cards, vec![card_id]);
        assert!(!card_round.is_empty());
        assert!(!card_round.card_list);
        assert!(!card_round.board_list);
        assert!(card_round.boards.is_empty());
        assert!(card_round.columns_by_board.is_empty());

        let column_id = Uuid::new_v4();
        let column_round = RouteScope::Column(column_id).next_round(&Model::default());
        assert_eq!(column_round.columns, vec![column_id]);
    }

    #[test]
    fn test_route_scope_for_card_graph_requests_the_graph_and_the_card() {
        let card_id = Uuid::new_v4();
        let round = RouteScope::CardGraph(card_id).next_round(&Model::default());

        assert!(round.graph);
        assert_eq!(round.cards, vec![card_id]);
        assert!(!round.card_list);
    }

    #[test]
    fn test_route_scope_retries_a_failed_tier_but_not_a_missing_one() {
        let id = Uuid::new_v4();

        let mut missing_model = Model::default();
        let _ = missing_model.apply_resolved(Resolved {
            boards: Collection {
                by_id: [(id, LoadState::Missing)].into(),
                ..Default::default()
            },
            ..Default::default()
        });
        let missing_round = RouteScope::Board(id).next_round(&missing_model);
        assert!(missing_round.boards.is_empty());

        let mut failed_model = Model::default();
        let _ = failed_model.apply_resolved(Resolved {
            boards: Collection {
                by_id: [(
                    id,
                    LoadState::Failed(Arc::new(KanbanError::Database("boom".into()))),
                )]
                .into(),
                ..Default::default()
            },
            ..Default::default()
        });
        let failed_round = RouteScope::Board(id).next_round(&failed_model);
        assert_eq!(failed_round.boards, vec![id]);
    }
}
