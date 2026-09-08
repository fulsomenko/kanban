use crate::error::{AppError, AppJson};
use crate::handlers::boards::{create_board, create_or_replace_board};
use crate::model_read::{require_loaded, require_loaded_entity};
use crate::pagination::paginate_response;
use crate::scope::RouteScope;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use kanban_domain::{Model, NoProjections};
use kanban_service::api::{
    ArchivedBoardResponse, BoardResponse, ChangeKind, CreateBoardRequest, EntityType, Page,
    PageParams, ReplaceBoardRequest, UpdateBoardRequest,
};
use uuid::Uuid;

async fn list_boards(
    State(state): State<AppState>,
    Query(params): Query<PageParams>,
) -> Result<Json<Page<BoardResponse>>, AppError> {
    let guard = state.lock_session().await;
    let mut model = Model::default();
    guard.sync(&RouteScope::BoardList, &mut model, &mut NoProjections);
    let boards = require_loaded(model.live_boards_state(), "board list")?;
    paginate_response(
        boards.iter().map(|b| BoardResponse::from(*b)).collect(),
        &params,
    )
}

fn board_response(session: &crate::state::Session, id: Uuid) -> Result<BoardResponse, AppError> {
    let mut model = Model::default();
    session.sync(&RouteScope::Board(id), &mut model, &mut NoProjections);
    let board = require_loaded_entity(model.board_id_status(id), "Board", id)?;
    let markers = require_loaded(model.archived_boards_state(), "archived board list")?;
    let archived_at = markers
        .iter()
        .find(|marker| marker.entity_id == id)
        .map(|marker| marker.metadata.archived_at);
    Ok(BoardResponse::with_archived_at(board, archived_at))
}

async fn get_board(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<BoardResponse>, AppError> {
    let guard = state.lock_session().await;
    Ok(Json(board_response(&guard, id)?))
}

async fn list_archived_boards(
    State(state): State<AppState>,
    Query(params): Query<PageParams>,
) -> Result<Json<Page<ArchivedBoardResponse>>, AppError> {
    let guard = state.lock_session().await;
    let mut model = Model::default();
    guard.sync(
        &RouteScope::ArchivedBoardList,
        &mut model,
        &mut NoProjections,
    );
    let markers = require_loaded(model.archived_boards_state(), "archived board list")?;
    paginate_response(
        markers.iter().map(ArchivedBoardResponse::from).collect(),
        &params,
    )
}

pub fn read_router() -> Router<AppState> {
    Router::new()
        .route("/v1/boards", get(list_boards))
        .route("/v1/boards/{id}", get(get_board))
        .route("/v1/archived-boards", get(list_archived_boards))
}

async fn post_board(
    State(state): State<AppState>,
    AppJson(req): AppJson<CreateBoardRequest>,
) -> Result<(StatusCode, Json<BoardResponse>), AppError> {
    let resp = {
        let mut ctx = state.ctx.lock().await;
        let resp = create_board(&mut ctx, req).map_err(AppError::from)?;
        state
            .persist_and_broadcast(&ctx, EntityType::Board, resp.id, ChangeKind::Created)
            .await
            .map_err(|e| AppError::from(&e))?;
        resp
    };
    Ok((StatusCode::CREATED, Json(resp)))
}

async fn put_board(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    AppJson(req): AppJson<ReplaceBoardRequest>,
) -> Result<(StatusCode, Json<BoardResponse>), AppError> {
    let (resp, created) = {
        let mut ctx = state.ctx.lock().await;
        let (resp, created) = create_or_replace_board(&mut ctx, id, req).map_err(AppError::from)?;
        state
            .persist_and_broadcast(
                &ctx,
                EntityType::Board,
                id,
                ChangeKind::created_or_updated(created),
            )
            .await
            .map_err(|e| AppError::from(&e))?;
        (resp, created)
    };
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(resp)))
}

async fn patch_board(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    AppJson(req): AppJson<UpdateBoardRequest>,
) -> Result<Json<BoardResponse>, AppError> {
    let board = {
        let mut ctx = state.ctx.lock().await;
        let (board, _invalidation) =
            crate::state::mutate(&mut ctx, |c| c.update_board_impl(id, req.into()))
                .map_err(|e| AppError::from(&e))?;
        state
            .persist_and_broadcast(&ctx, EntityType::Board, id, ChangeKind::Updated)
            .await
            .map_err(|e| AppError::from(&e))?;
        board
    };
    Ok(Json(BoardResponse::from(&board)))
}

async fn delete_board(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    {
        let mut ctx = state.ctx.lock().await;
        let _invalidation = crate::state::mutate_unit(&mut ctx, |c| c.delete_board_impl(id))
            .map_err(|e| AppError::from(&e))?;
        state
            .persist_and_broadcast(&ctx, EntityType::Board, id, ChangeKind::Deleted)
            .await
            .map_err(|e| AppError::from(&e))?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn archive_board(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<BoardResponse>, AppError> {
    let mut guard = state.lock_session().await;
    let _invalidation = crate::state::mutate_unit(&mut guard, |c| c.archive_board_impl(id))
        .map_err(|e| AppError::from(&e))?;
    let response = board_response(&guard, id)?;
    state
        .persist_and_broadcast(&guard, EntityType::Board, id, ChangeKind::Updated)
        .await
        .map_err(|e| AppError::from(&e))?;
    Ok(Json(response))
}

async fn restore_board(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<BoardResponse>, AppError> {
    let mut guard = state.lock_session().await;
    let _invalidation = crate::state::mutate_unit(&mut guard, |c| c.restore_board_impl(id))
        .map_err(|e| AppError::from(&e))?;
    let response = board_response(&guard, id)?;
    state
        .persist_and_broadcast(&guard, EntityType::Board, id, ChangeKind::Updated)
        .await
        .map_err(|e| AppError::from(&e))?;
    Ok(Json(response))
}

pub fn write_router() -> Router<AppState> {
    Router::new()
        .route("/v1/boards", post(post_board))
        .route(
            "/v1/boards/{id}",
            put(put_board).patch(patch_board).delete(delete_board),
        )
        .route("/v1/boards/{id}/archive", post(archive_board))
        .route("/v1/boards/{id}/restore", post(restore_board))
}
