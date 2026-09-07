#![cfg(feature = "test-helpers")]

//! Column read routes (GET /v1/boards/{board_id}/columns, GET /v1/boards/{board_id}/columns/{id}).
//! Read-only, no mutation, no event broadcast. Established via `tower::ServiceExt::oneshot`
//! against the router directly, with no real TCP socket.

use axum::http::StatusCode;
use kanban_domain::LoadState;
use kanban_server::test_helpers::{json_of, make_sqlite_state, make_state, send};
use kanban_service::KanbanOperations;
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread")]
async fn test_list_columns_returns_board_columns_in_position_order() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Test Board".to_string(), Some("TB".to_string()))
            .unwrap()
            .id;
        let _ = ctx
            .create_column(board_id, "Column 3".to_string(), Some(2))
            .unwrap()
            .id;
        let _ = ctx
            .create_column(board_id, "Column 1".to_string(), Some(0))
            .unwrap()
            .id;
        let _ = ctx
            .create_column(board_id, "Column 2".to_string(), Some(1))
            .unwrap()
            .id;
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/columns", board_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_of(response).await;

    let arr = json["items"].as_array().expect("items should be an array");
    assert_eq!(arr.len(), 3, "should have 3 columns");

    assert_eq!(arr[0]["position"], 0, "first should be position 0");
    assert_eq!(arr[1]["position"], 1, "second should be position 1");
    assert_eq!(arr[2]["position"], 2, "third should be position 2");

    for col in arr {
        assert_eq!(
            col["board_id"],
            board_id.to_string(),
            "all columns should have matching board_id"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_list_columns_empty_board_returns_200_empty_page() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Empty Board".to_string(), Some("EB".to_string()))
            .unwrap()
            .id;
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/columns", board_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_of(response).await;
    assert_eq!(json["items"], serde_json::json!([]));
    assert_eq!(json["total"], 0);
    assert_eq!(json["total_pages"], 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_list_columns_unknown_board_returns_404() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let random_board_id = Uuid::new_v4();

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/columns", random_board_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = json_of(response).await;
    assert_eq!(json["code"], "NOT_FOUND");
    assert_eq!(
        json["message"],
        format!("Board {random_board_id} not found")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_list_columns_archived_board_returns_200_with_its_columns() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Archived Board".to_string(), Some("AB".to_string()))
            .unwrap()
            .id;
        ctx.create_column(board_id, "Column 1".to_string(), Some(0))
            .unwrap();
        ctx.archive_board(board_id).unwrap();
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/columns", board_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_of(response).await;
    let arr = json["items"].as_array().expect("items should be an array");
    assert_eq!(
        arr.len(),
        1,
        "archived board should still serve its columns"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_column_returns_column_response_for_existing_id() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    let column_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Test Board".to_string(), Some("TB".to_string()))
            .unwrap()
            .id;
        column_id = ctx
            .create_column(board_id, "Test Column".to_string(), None)
            .unwrap()
            .id;
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/columns/{}", board_id, column_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_of(response).await;

    assert_eq!(json["id"], column_id.to_string());
    assert_eq!(json["board_id"], board_id.to_string());
    assert_eq!(json["name"], "Test Column");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_column_unknown_id_returns_404() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Test Board".to_string(), Some("TB".to_string()))
            .unwrap()
            .id;
    }

    let random_column_id = Uuid::new_v4();

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/columns/{}", board_id, random_column_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = json_of(response).await;
    assert_eq!(json["code"], "NOT_FOUND");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_column_wrong_board_returns_404() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_a_id: Uuid;
    let board_b_id: Uuid;
    let column_id: Uuid;
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
        column_id = ctx
            .create_column(board_a_id, "Column in A".to_string(), None)
            .unwrap()
            .id;
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/columns/{}", board_b_id, column_id),
        None,
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "column from a different board should 404"
    );
    let json = json_of(response).await;
    assert_eq!(json["code"], "NOT_FOUND");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_list_columns_populates_the_session_model_board_and_column_tiers() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Test Board".to_string(), Some("TB".to_string()))
            .unwrap()
            .id;
        ctx.create_column(board_id, "Column 1".to_string(), Some(0))
            .unwrap();
        ctx.create_column(board_id, "Column 2".to_string(), Some(1))
            .unwrap();
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/columns", board_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_of(response).await;
    let arr = json["items"].as_array().expect("items should be an array");
    assert_eq!(arr.len(), 2);

    let guard = state.ctx.lock().await;
    assert!(
        matches!(guard.model.board_id_status(board_id), LoadState::Loaded(_)),
        "board tier should be populated after list_columns"
    );
    match guard.model.board_columns_state(board_id) {
        LoadState::Loaded(cols) => assert_eq!(cols.len(), 2),
        other => panic!("expected columns tier to be Loaded, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_column_populates_the_session_models_per_id_column_tier() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    let column_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Test Board".to_string(), Some("TB".to_string()))
            .unwrap()
            .id;
        column_id = ctx
            .create_column(board_id, "Test Column".to_string(), None)
            .unwrap()
            .id;
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/columns/{}", board_id, column_id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let guard = state.ctx.lock().await;
    match guard.model.column_id_status(column_id) {
        LoadState::Loaded(c) => {
            assert_eq!(c.id, column_id);
            assert_eq!(c.board_id, board_id);
        }
        other => panic!("expected column tier to be Loaded, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_column_flat_populates_the_session_models_per_id_column_tier() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    let column_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Test Board".to_string(), Some("TB".to_string()))
            .unwrap()
            .id;
        column_id = ctx
            .create_column(board_id, "Test Column".to_string(), None)
            .unwrap()
            .id;
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/columns/{}", column_id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let guard = state.ctx.lock().await;
    assert!(matches!(
        guard.model.column_id_status(column_id),
        LoadState::Loaded(_)
    ));
    assert!(
        matches!(guard.model.board_id_status(board_id), LoadState::NotLoaded),
        "the flat route has no board in its path and must not plan a board tier"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_list_columns_on_sqlite_serves_an_archived_boards_columns_from_the_model() {
    let dir = tempdir().unwrap();
    let state = make_sqlite_state(&dir.path().join("board.sqlite")).await;

    let board_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Archived Board".to_string(), Some("AB".to_string()))
            .unwrap()
            .id;
        ctx.create_column(board_id, "Column 1".to_string(), Some(0))
            .unwrap();
        ctx.archive_board(board_id).unwrap();
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/columns", board_id),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_of(response).await;
    let arr = json["items"].as_array().expect("items should be an array");
    assert_eq!(arr.len(), 1);

    let guard = state.ctx.lock().await;
    assert!(matches!(
        guard.model.board_id_status(board_id),
        LoadState::Loaded(_)
    ));
    match guard.model.board_columns_state(board_id) {
        LoadState::Loaded(cols) => assert_eq!(cols.len(), 1),
        other => panic!("expected columns tier to be Loaded, got {other:?}"),
    }
}
