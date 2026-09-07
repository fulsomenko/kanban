#![cfg(feature = "test-helpers")]

use axum::http::StatusCode;
use kanban_domain::LoadState;
use kanban_server::test_helpers::{json_of, make_sqlite_state, make_state, send};
use kanban_service::api::{ArchivedCardResponse, Page};
use kanban_service::KanbanOperations;
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread")]
async fn test_get_board_archived_cards_lists_only_that_boards_markers() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_a_id: Uuid;
    let board_b_id: Uuid;
    let archived_a_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_a_id = ctx
            .create_board("Board A".to_string(), Some("BA".to_string()))
            .unwrap()
            .id;
        board_b_id = ctx
            .create_board("Board B".to_string(), Some("BB".to_string()))
            .unwrap()
            .id;

        let col_a_id = ctx
            .create_column(board_a_id, "Column".to_string(), None)
            .unwrap()
            .id;
        let col_b_id = ctx
            .create_column(board_b_id, "Column".to_string(), None)
            .unwrap()
            .id;

        archived_a_id = ctx
            .create_card(
                board_a_id,
                col_a_id,
                "Archived in A".to_string(),
                Default::default(),
            )
            .unwrap()
            .id;
        let archived_b_id = ctx
            .create_card(
                board_b_id,
                col_b_id,
                "Archived in B".to_string(),
                Default::default(),
            )
            .unwrap()
            .id;

        ctx.archive_card(archived_a_id).unwrap();
        ctx.archive_card(archived_b_id).unwrap();
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/archived-cards", board_a_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_of(response).await;
    let page: Page<ArchivedCardResponse> = serde_json::from_value(json)
        .expect("body should deserialize as Page<ArchivedCardResponse>");

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].entity_id, archived_a_id);
    assert_eq!(page.items[0].board_id, board_a_id);
    assert!(page.items[0].archived_at <= chrono::Utc::now());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_board_archived_cards_unknown_board_returns_404() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let random_board_id = Uuid::new_v4();

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/archived-cards", random_board_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_board_archived_cards_404s_an_unknown_board_from_the_board_tier_not_from_an_empty_marker_tier(
) {
    let dir = tempdir().unwrap();
    let state = make_sqlite_state(&dir.path().join("s.sqlite")).await;

    let random_board_id = Uuid::new_v4();

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/archived-cards", random_board_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let guard = state.ctx.lock().await;
    assert!(matches!(
        guard.model.board_id_status(random_board_id),
        LoadState::Missing
    ));
    match guard.model.board_archived_cards_state(random_board_id) {
        LoadState::Loaded(markers) => assert!(markers.is_empty()),
        other => panic!("expected Loaded([]), got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_board_archived_cards_serves_the_markers_from_the_board_scoped_model_tier() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_a_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_a_id = ctx
            .create_board("Board A".to_string(), Some("BA".to_string()))
            .unwrap()
            .id;
        let board_b_id = ctx
            .create_board("Board B".to_string(), Some("BB".to_string()))
            .unwrap()
            .id;
        let col_a_id = ctx
            .create_column(board_a_id, "Column".to_string(), None)
            .unwrap()
            .id;
        let col_b_id = ctx
            .create_column(board_b_id, "Column".to_string(), None)
            .unwrap()
            .id;
        let archived_a_id = ctx
            .create_card(
                board_a_id,
                col_a_id,
                "Archived in A".to_string(),
                Default::default(),
            )
            .unwrap()
            .id;
        let archived_b_id = ctx
            .create_card(
                board_b_id,
                col_b_id,
                "Archived in B".to_string(),
                Default::default(),
            )
            .unwrap()
            .id;
        ctx.archive_card(archived_a_id).unwrap();
        ctx.archive_card(archived_b_id).unwrap();
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/archived-cards", board_a_id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_of(response).await;
    let page: Page<ArchivedCardResponse> = serde_json::from_value(json)
        .expect("body should deserialize as Page<ArchivedCardResponse>");
    assert_eq!(page.items.len(), 1);

    let guard = state.ctx.lock().await;
    match guard.model.board_archived_cards_state(board_a_id) {
        LoadState::Loaded(markers) => assert_eq!(markers.len(), 1),
        other => panic!("expected Loaded, got {other:?}"),
    }
    assert!(matches!(
        guard.model.board_id_status(board_a_id),
        LoadState::Loaded(_)
    ));
    assert!(matches!(
        guard.model.archived_cards_state(),
        LoadState::NotLoaded
    ));
}
