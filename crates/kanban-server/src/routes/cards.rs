use crate::error::{AppError, AppJson};
use crate::model_read::{require_loaded, require_loaded_entity};
use crate::pagination::paginate_response;
use crate::scope::RouteScope;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use kanban_domain::{
    filter_and_sort_cards, ArchivedFilter, Card, CardListFilter, Model, NoProjections,
};
use kanban_service::api::ArchivedCardResponse;
use kanban_service::api::ArchivedFilterDto;
use kanban_service::api::CardResponse;
use kanban_service::api::{
    ChangeKind, CreateCardRequest, EntityType, Page, PageParams, UpdateCardRequest,
};
use kanban_service::{CardUpdate, KanbanError, KanbanOperations};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CardQuery {
    pub column_id: Option<Uuid>,
    pub sprint_id: Option<Uuid>,
    #[serde(default)]
    pub archived: ArchivedFilterDto,
}

#[derive(Debug, Deserialize)]
pub struct RestoreCardQuery {
    pub column_id: Option<Uuid>,
}

fn optional_card<'a>(
    state: kanban_domain::LoadState<&'a Card>,
    what: &str,
) -> Result<Option<&'a Card>, AppError> {
    match state {
        kanban_domain::LoadState::Missing => Ok(None),
        other => require_loaded(other, what).map(Some),
    }
}

async fn list_cards(
    State(state): State<AppState>,
    Path(board_id): Path<Uuid>,
    Query(q): Query<CardQuery>,
    Query(params): Query<PageParams>,
) -> Result<Json<Page<CardResponse>>, AppError> {
    let archived: ArchivedFilter = q.archived.into();
    let scope = RouteScope::BoardCards {
        board_id,
        column_id: q.column_id,
        archived,
    };

    let guard = state.lock_session().await;
    let mut model = Model::default();
    guard.sync(&scope, &mut model, &mut NoProjections);

    let board = require_loaded_entity(model.board_id_status(board_id), "Board", board_id)?;
    let columns = require_loaded(model.board_columns_state(board_id), "columns of board")?;

    let source_column_ids: Vec<Uuid> = match q.column_id {
        Some(cid) => {
            if columns.iter().any(|c| c.id == cid) {
                vec![cid]
            } else {
                Vec::new()
            }
        }
        None => columns.iter().map(|c| c.id).collect(),
    };

    let mut live: Vec<Card> = Vec::new();
    for column_id in &source_column_ids {
        let cards = require_loaded(model.column_cards_state(*column_id), "cards of column")?;
        live.extend(cards.iter().cloned());
    }

    let mut cards = if archived == ArchivedFilter::ArchivedOnly {
        Vec::new()
    } else {
        live
    };
    let mut archived_at: HashMap<Uuid, DateTime<Utc>> = HashMap::new();
    if archived != ArchivedFilter::LiveOnly {
        let markers = require_loaded(model.archived_cards_state(), "archived cards")?;
        for marker in markers.iter().filter(|m| m.context.board_id == board_id) {
            if let Some(card) = optional_card(model.card_by_id_state(marker.entity_id), "Card")? {
                cards.push(card.clone());
                archived_at.insert(marker.entity_id, marker.metadata.archived_at);
            }
        }
    }

    let filter = CardListFilter {
        board_id: if archived == ArchivedFilter::LiveOnly {
            Some(board_id)
        } else {
            None
        },
        column_id: q.column_id,
        sprint_ids: q.sprint_id.map(|id| HashSet::from([id])),
        archived,
        ..Default::default()
    };
    let filtered = filter_and_sort_cards(&cards, columns, &[], Some(board), &[], &filter);
    let responses: Vec<CardResponse> = filtered
        .iter()
        .map(|c| CardResponse::with_archived_at(c, archived_at.get(&c.id).copied()))
        .collect();
    paginate_response(responses, &params)
}

async fn get_card(
    State(state): State<AppState>,
    Path((board_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<CardResponse>, AppError> {
    let scope = RouteScope::Card(id);
    let guard = state.lock_session().await;
    let mut model = Model::default();
    guard.sync(&scope, &mut model, &mut NoProjections);

    let card = require_loaded_entity(model.card_by_id_state(id), "Card", id)?;
    if card.board_id != board_id {
        return Err(AppError::from(&KanbanError::not_found("Card", id)));
    }
    Ok(Json(CardResponse::from(card)))
}

async fn list_archived_cards(
    State(state): State<AppState>,
    Path(board_id): Path<Uuid>,
    Query(params): Query<PageParams>,
) -> Result<Json<Page<ArchivedCardResponse>>, AppError> {
    let scope = RouteScope::BoardArchivedCards(board_id);
    let guard = state.lock_session().await;
    let mut model = Model::default();
    guard.sync(&scope, &mut model, &mut NoProjections);

    require_loaded_entity(model.board_id_status(board_id), "Board", board_id)?;
    let markers = require_loaded(
        model.board_archived_cards_state(board_id),
        "archived cards of board",
    )?;
    let responses: Vec<ArchivedCardResponse> =
        markers.iter().map(ArchivedCardResponse::from).collect();
    paginate_response(responses, &params)
}

pub fn read_router() -> Router<AppState> {
    Router::new()
        .route("/v1/boards/{board_id}/cards", get(list_cards))
        .route("/v1/boards/{board_id}/cards/{id}", get(get_card))
        .route(
            "/v1/boards/{board_id}/archived-cards",
            get(list_archived_cards),
        )
}

fn created_status(created: bool) -> StatusCode {
    if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    }
}

fn do_update_card(
    ctx: &mut crate::state::Session,
    id: Uuid,
    updates: CardUpdate,
) -> Result<Card, AppError> {
    crate::state::mutate(ctx, |c| c.update_card_impl(id, updates))
        .map(|(value, _invalidation)| value)
        .map_err(|e| AppError::from(&e))
}

fn do_delete_card(ctx: &mut crate::state::Session, id: Uuid) -> Result<(), AppError> {
    crate::state::mutate_unit(ctx, |c| c.delete_card_impl(id))
        .map(|_invalidation| ())
        .map_err(|e| AppError::from(&e))
}

fn do_archive_card(ctx: &mut crate::state::Session, id: Uuid) -> Result<(), AppError> {
    crate::state::mutate(ctx, |c| c.archive_card_impl(id))
        .map(|((), _invalidation)| ())
        .map_err(|e| AppError::from(&e))
}

fn do_restore_card(
    ctx: &mut crate::state::Session,
    id: Uuid,
    column_id: Option<Uuid>,
) -> Result<Card, AppError> {
    crate::state::mutate(ctx, |c| c.restore_card_impl(id, column_id))
        .map(|(card, _invalidation)| card)
        .map_err(|e| AppError::from(&e))
}

/// Fetch a card and 404 unless it belongs to `board_id`, since
/// `KanbanOperations::{update_card, delete_card}` key on the global card id
/// alone with no board scoping of their own.
fn require_card_in_board(
    ctx: &crate::state::Session,
    board_id: Uuid,
    id: Uuid,
) -> Result<(), AppError> {
    ctx.get_card(id)
        .map_err(|e| AppError::from(&e))?
        .filter(|c| c.board_id == board_id)
        .ok_or_else(|| AppError::from(&KanbanError::not_found("Card", id)))?;
    Ok(())
}

async fn create_card_route(
    State(state): State<AppState>,
    Path(column_id): Path<Uuid>,
    AppJson(req): AppJson<CreateCardRequest>,
) -> Result<(StatusCode, Json<CardResponse>), AppError> {
    let (resp, created) = {
        let mut ctx = state.ctx.lock().await;
        let result = crate::handlers::cards::create_card(&mut ctx, column_id, req)
            .map_err(AppError::from)?;
        state
            .persist_and_broadcast(
                &ctx,
                EntityType::Card,
                result.0.id,
                ChangeKind::created_or_updated(result.1),
            )
            .await
            .map_err(|e| AppError::from(&e))?;
        result
    };
    Ok((created_status(created), Json(resp)))
}

async fn put_card_route(
    State(state): State<AppState>,
    Path((column_id, id)): Path<(Uuid, Uuid)>,
    AppJson(req): AppJson<CreateCardRequest>,
) -> Result<(StatusCode, Json<CardResponse>), AppError> {
    let (resp, created) = {
        let mut ctx = state.ctx.lock().await;
        let result = crate::handlers::cards::create_or_replace_card(&mut ctx, column_id, id, req)
            .map_err(AppError::from)?;
        state
            .persist_and_broadcast(
                &ctx,
                EntityType::Card,
                id,
                ChangeKind::created_or_updated(result.1),
            )
            .await
            .map_err(|e| AppError::from(&e))?;
        result
    };
    Ok((created_status(created), Json(resp)))
}

async fn update_card_route(
    State(state): State<AppState>,
    Path((board_id, id)): Path<(Uuid, Uuid)>,
    AppJson(req): AppJson<UpdateCardRequest>,
) -> Result<Json<CardResponse>, AppError> {
    let updates = CardUpdate::try_from(req).map_err(|e| AppError::from(&e))?;
    let card = {
        let mut ctx = state.ctx.lock().await;
        require_card_in_board(&ctx, board_id, id)?;
        let card = do_update_card(&mut ctx, id, updates)?;
        state
            .persist_and_broadcast(&ctx, EntityType::Card, id, ChangeKind::Updated)
            .await
            .map_err(|e| AppError::from(&e))?;
        card
    };
    Ok(Json(CardResponse::from(&card)))
}

async fn delete_card_route(
    State(state): State<AppState>,
    Path((board_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    {
        let mut ctx = state.ctx.lock().await;
        require_card_in_board(&ctx, board_id, id)?;
        do_delete_card(&mut ctx, id)?;
        state
            .persist_and_broadcast(&ctx, EntityType::Card, id, ChangeKind::Deleted)
            .await
            .map_err(|e| AppError::from(&e))?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub fn write_router() -> Router<AppState> {
    Router::new()
        .route("/v1/columns/{column_id}/cards", post(create_card_route))
        .route("/v1/columns/{column_id}/cards/{id}", put(put_card_route))
        .route(
            "/v1/boards/{board_id}/cards/{id}",
            patch(update_card_route).delete(delete_card_route),
        )
}

async fn get_card_flat(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CardResponse>, AppError> {
    let scope = RouteScope::Card(id);
    let guard = state.lock_session().await;
    let mut model = Model::default();
    guard.sync(&scope, &mut model, &mut NoProjections);

    let card = require_loaded_entity(model.card_by_id_state(id), "Card", id)?;
    Ok(Json(CardResponse::from(card)))
}

async fn update_card_route_flat(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    AppJson(req): AppJson<UpdateCardRequest>,
) -> Result<Json<CardResponse>, AppError> {
    let updates = CardUpdate::try_from(req).map_err(|e| AppError::from(&e))?;
    let card = {
        let mut ctx = state.ctx.lock().await;
        let card = do_update_card(&mut ctx, id, updates)?;
        state
            .persist_and_broadcast(&ctx, EntityType::Card, id, ChangeKind::Updated)
            .await
            .map_err(|e| AppError::from(&e))?;
        card
    };
    Ok(Json(CardResponse::from(&card)))
}

async fn delete_card_route_flat(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    {
        let mut ctx = state.ctx.lock().await;
        do_delete_card(&mut ctx, id)?;
        state
            .persist_and_broadcast(&ctx, EntityType::Card, id, ChangeKind::Deleted)
            .await
            .map_err(|e| AppError::from(&e))?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn archive_card_route(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CardResponse>, AppError> {
    let (card, archived_at) = {
        let mut ctx = state.ctx.lock().await;
        do_archive_card(&mut ctx, id)?;
        let card = ctx
            .get_card(id)
            .map_err(|e| AppError::from(&e))?
            .ok_or_else(|| AppError::from(&KanbanError::not_found("Card", id)))?;
        let archived_at = ctx.card_archived_at(id).map_err(|e| AppError::from(&e))?;
        state
            .persist_and_broadcast(&ctx, EntityType::Card, id, ChangeKind::Updated)
            .await
            .map_err(|e| AppError::from(&e))?;
        (card, archived_at)
    };
    Ok(Json(CardResponse::with_archived_at(&card, archived_at)))
}

async fn restore_card_route(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<RestoreCardQuery>,
) -> Result<Json<CardResponse>, AppError> {
    let card = {
        let mut ctx = state.ctx.lock().await;
        let card = do_restore_card(&mut ctx, id, q.column_id)?;
        state
            .persist_and_broadcast(&ctx, EntityType::Card, id, ChangeKind::Updated)
            .await
            .map_err(|e| AppError::from(&e))?;
        card
    };
    Ok(Json(CardResponse::from(&card)))
}

pub fn flat_read_router() -> Router<AppState> {
    Router::new().route("/v1/cards/{id}", get(get_card_flat))
}

pub fn flat_write_router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/cards/{id}",
            patch(update_card_route_flat).delete(delete_card_route_flat),
        )
        .route("/v1/cards/{id}/archive", post(archive_card_route))
        .route("/v1/cards/{id}/restore", post(restore_card_route))
}
