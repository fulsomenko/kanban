#![cfg(feature = "test-helpers")]

//! Pins that the sprint read routes (`list_sprints`, `get_sprint`,
//! `get_sprint_flat`) populate the session Model rather than reading the
//! context directly.

use axum::http::StatusCode;
use kanban_domain::LoadState;
use kanban_server::test_helpers::{json_of, make_sqlite_state, make_state, send};
use kanban_service::KanbanOperations;
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread")]
async fn test_list_sprints_loads_the_board_head_and_the_scoped_sprint_tier_into_the_session_model()
{
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Board A".to_string(), Some("BA".to_string()))
            .unwrap()
            .id;
        ctx.create_sprint(board_id, Some("SPR".to_string()), Some("Alpha".to_string()))
            .unwrap();
        ctx.create_sprint(board_id, Some("SPR".to_string()), Some("Beta".to_string()))
            .unwrap();
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/sprints", board_id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_of(response).await;
    let arr = json["items"].as_array().expect("items should be an array");
    let names: std::collections::HashSet<_> = arr
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        names,
        ["Alpha".to_string(), "Beta".to_string()]
            .into_iter()
            .collect()
    );

    let guard = state.ctx.lock().await;
    assert!(
        matches!(guard.model.board_sprints_state(board_id), LoadState::Loaded(s) if s.len() == 2),
        "expected board_sprints_state to be Loaded with 2 sprints, got {:?}",
        guard.model.board_sprints_state(board_id)
    );
    assert!(
        guard.model.board_id_status(board_id).is_loaded(),
        "expected board_id_status to be Loaded, got {:?}",
        guard.model.board_id_status(board_id)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_sprint_loads_the_sprint_and_its_board_head_into_the_session_model() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    let sprint_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Board A".to_string(), Some("BA".to_string()))
            .unwrap()
            .id;
        sprint_id = ctx
            .create_sprint(board_id, Some("SPR".to_string()), Some("Alpha".to_string()))
            .unwrap()
            .id;
    }

    let response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/sprints/{}", board_id, sprint_id),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_of(response).await;
    assert_eq!(json["name"].as_str().unwrap(), "Alpha");
    assert_eq!(json["board_id"].as_str().unwrap(), board_id.to_string());

    let guard = state.ctx.lock().await;
    assert!(
        guard.model.sprint_id_status(sprint_id).is_loaded(),
        "expected sprint_id_status to be Loaded, got {:?}",
        guard.model.sprint_id_status(sprint_id)
    );
    assert!(
        guard.model.board_id_status(board_id).is_loaded(),
        "expected board_id_status to be Loaded, got {:?}",
        guard.model.board_id_status(board_id)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_the_flat_sprint_route_chains_a_second_round_for_its_owning_board_head() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    let sprint_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Board A".to_string(), Some("BA".to_string()))
            .unwrap()
            .id;
        sprint_id = ctx
            .create_sprint(board_id, Some("SPR".to_string()), Some("Alpha".to_string()))
            .unwrap()
            .id;
    }

    let response = send(&state, "GET", &format!("/v1/sprints/{}", sprint_id), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_of(response).await;
    assert_eq!(json["name"].as_str().unwrap(), "Alpha");

    let guard = state.ctx.lock().await;
    assert!(
        guard.model.sprint_id_status(sprint_id).is_loaded(),
        "expected sprint_id_status to be Loaded, got {:?}",
        guard.model.sprint_id_status(sprint_id)
    );
    assert!(
        guard.model.board_id_status(board_id).is_loaded(),
        "expected board_id_status to be Loaded, got {:?}",
        guard.model.board_id_status(board_id)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sprint_reads_of_an_archived_board_resolve_the_name_and_load_its_head() {
    let dir = tempdir().unwrap();
    let state = make_state(&dir.path().join("s.json"));

    let board_id: Uuid;
    let sprint_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Board A".to_string(), Some("BA".to_string()))
            .unwrap()
            .id;
        sprint_id = ctx
            .create_sprint(board_id, Some("SPR".to_string()), Some("Alpha".to_string()))
            .unwrap()
            .id;
        ctx.archive_board(board_id).unwrap();
    }

    let list_response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/sprints", board_id),
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = json_of(list_response).await;
    let arr = list_json["items"]
        .as_array()
        .expect("items should be an array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"].as_str().unwrap(), "Alpha");

    let get_response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/sprints/{}", board_id, sprint_id),
        None,
    )
    .await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let get_json = json_of(get_response).await;
    assert_eq!(get_json["name"].as_str().unwrap(), "Alpha");

    let flat_response = send(&state, "GET", &format!("/v1/sprints/{}", sprint_id), None).await;
    assert_eq!(flat_response.status(), StatusCode::OK);
    let flat_json = json_of(flat_response).await;
    assert_eq!(flat_json["name"].as_str().unwrap(), "Alpha");

    let guard = state.ctx.lock().await;
    assert!(
        guard.model.board_id_status(board_id).is_loaded(),
        "expected board_id_status to be Loaded, got {:?}",
        guard.model.board_id_status(board_id)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sprint_reads_populate_the_session_model_on_a_sqlite_locator_and_reset_between_requests(
) {
    let dir = tempdir().unwrap();
    let state = make_sqlite_state(&dir.path().join("s.sqlite")).await;

    let board_id: Uuid;
    let sprint_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Board A".to_string(), Some("BA".to_string()))
            .unwrap()
            .id;
        sprint_id = ctx
            .create_sprint(board_id, Some("SPR".to_string()), Some("Alpha".to_string()))
            .unwrap()
            .id;
    }

    let get_response = send(&state, "GET", &format!("/v1/sprints/{}", sprint_id), None).await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let get_json = json_of(get_response).await;
    assert_eq!(get_json["name"].as_str().unwrap(), "Alpha");

    {
        let guard = state.ctx.lock().await;
        assert!(
            guard.model.sprint_id_status(sprint_id).is_loaded(),
            "expected sprint_id_status to be Loaded, got {:?}",
            guard.model.sprint_id_status(sprint_id)
        );
        assert!(
            guard.model.board_id_status(board_id).is_loaded(),
            "expected board_id_status to be Loaded, got {:?}",
            guard.model.board_id_status(board_id)
        );
    }

    let list_response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/sprints", board_id),
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = json_of(list_response).await;
    let arr = list_json["items"]
        .as_array()
        .expect("items should be an array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"].as_str().unwrap(), "Alpha");

    let guard = state.ctx.lock().await;
    assert!(
        matches!(guard.model.board_sprints_state(board_id), LoadState::Loaded(s) if s.len() == 1),
        "expected board_sprints_state to be Loaded with 1 sprint, got {:?}",
        guard.model.board_sprints_state(board_id)
    );
    assert!(
        guard.model.board_id_status(board_id).is_loaded(),
        "expected board_id_status to be Loaded, got {:?}",
        guard.model.board_id_status(board_id)
    );
    assert!(
        guard.model.sprint_id_status(sprint_id).is_not_loaded(),
        "expected sprint_id_status to be reset to NotLoaded after lock_session, got {:?}",
        guard.model.sprint_id_status(sprint_id)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sprint_reads_of_an_archived_board_resolve_on_a_sqlite_locator() {
    let dir = tempdir().unwrap();
    let state = make_sqlite_state(&dir.path().join("s.sqlite")).await;

    let board_id: Uuid;
    let sprint_id: Uuid;
    {
        let mut ctx = state.ctx.lock().await;
        board_id = ctx
            .create_board("Board A".to_string(), Some("BA".to_string()))
            .unwrap()
            .id;
        sprint_id = ctx
            .create_sprint(board_id, Some("SPR".to_string()), Some("Alpha".to_string()))
            .unwrap()
            .id;
        ctx.archive_board(board_id).unwrap();
    }

    let list_response = send(
        &state,
        "GET",
        &format!("/v1/boards/{}/sprints", board_id),
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = json_of(list_response).await;
    let arr = list_json["items"]
        .as_array()
        .expect("items should be an array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"].as_str().unwrap(), "Alpha");

    let flat_response = send(&state, "GET", &format!("/v1/sprints/{}", sprint_id), None).await;
    assert_eq!(flat_response.status(), StatusCode::OK);
    let flat_json = json_of(flat_response).await;
    assert_eq!(flat_json["name"].as_str().unwrap(), "Alpha");

    let guard = state.ctx.lock().await;
    assert!(
        guard.model.board_id_status(board_id).is_loaded(),
        "expected board_id_status to be Loaded, got {:?}",
        guard.model.board_id_status(board_id)
    );
}
