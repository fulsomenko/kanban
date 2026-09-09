use crate::client_ident::ClientIdent;
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
    filter_and_sort_cards, ArchivedFilter, Card, CardListFilter, Model, NoProjections, Sprint,
};
use kanban_service::api::ArchivedCardResponse;
use kanban_service::api::ArchivedFilterDto;
use kanban_service::api::CardResponse;
use kanban_service::api::{CardStatusDto, SortFieldDto, SortOrderDto};
use kanban_service::api::{
    ChangeKind, CreateCardRequest, EntityType, Page, PageParams, UpdateCardRequest,
};
use kanban_service::{CardUpdate, KanbanError, KanbanOperations};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Comma-separated UUID list; blank segments are skipped, an all-blank value
/// yields `None`.
fn uuid_csv<'de, D>(deserializer: D) -> Result<Option<HashSet<Uuid>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    let mut ids = HashSet::new();
    for part in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        ids.insert(Uuid::parse_str(part).map_err(serde::de::Error::custom)?);
    }
    Ok(if ids.is_empty() { None } else { Some(ids) })
}

#[derive(Debug, Default, Deserialize)]
pub struct CardQuery {
    pub column_id: Option<Uuid>,
    pub sprint_id: Option<Uuid>,
    #[serde(default, deserialize_with = "uuid_csv")]
    pub sprint_ids: Option<HashSet<Uuid>>,
    #[serde(default)]
    pub hide_assigned: bool,
    pub status: Option<CardStatusDto>,
    pub search: Option<String>,
    pub sort: Option<SortFieldDto>,
    pub sort_order: Option<SortOrderDto>,
    #[serde(default)]
    pub archived: ArchivedFilterDto,
}

fn card_query_filter(q: &CardQuery, board_id: Uuid, archived: ArchivedFilter) -> CardListFilter {
    let mut sprint_ids = q.sprint_ids.clone();
    if let Some(sprint_id) = q.sprint_id {
        sprint_ids
            .get_or_insert_with(HashSet::new)
            .insert(sprint_id);
    }
    CardListFilter {
        board_id: if archived == ArchivedFilter::LiveOnly {
            Some(board_id)
        } else {
            None
        },
        column_id: q.column_id,
        sprint_ids,
        hide_assigned: q.hide_assigned,
        status: q.status.map(Into::into),
        search: q.search.clone(),
        sort: q.sort.map(Into::into),
        sort_order: q.sort_order.map(Into::into),
        archived,
    }
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
    let searching = q.search.as_deref().is_some_and(|s| !s.is_empty());
    let scope = RouteScope::BoardCards {
        board_id,
        column_id: q.column_id,
        archived,
        search: searching,
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

    let filter = card_query_filter(&q, board_id, archived);
    let sprints: &[Sprint] = if searching {
        require_loaded(model.board_sprints_state(board_id), "sprints of board")?
    } else {
        &[]
    };
    let filtered = filter_and_sort_cards(&cards, columns, sprints, Some(board), &[], &filter);
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
    ClientIdent(client): ClientIdent,
    AppJson(req): AppJson<CreateCardRequest>,
) -> Result<(StatusCode, Json<CardResponse>), AppError> {
    let (resp, created) = {
        let mut ctx = state.lock_for_write(client).await;
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
    ClientIdent(client): ClientIdent,
    AppJson(req): AppJson<CreateCardRequest>,
) -> Result<(StatusCode, Json<CardResponse>), AppError> {
    let (resp, created) = {
        let mut ctx = state.lock_for_write(client).await;
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
    ClientIdent(client): ClientIdent,
    AppJson(req): AppJson<UpdateCardRequest>,
) -> Result<Json<CardResponse>, AppError> {
    let updates = CardUpdate::try_from(req).map_err(|e| AppError::from(&e))?;
    let card = {
        let mut ctx = state.lock_for_write(client).await;
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
    ClientIdent(client): ClientIdent,
) -> Result<StatusCode, AppError> {
    {
        let mut ctx = state.lock_for_write(client).await;
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
    ClientIdent(client): ClientIdent,
    AppJson(req): AppJson<UpdateCardRequest>,
) -> Result<Json<CardResponse>, AppError> {
    let updates = CardUpdate::try_from(req).map_err(|e| AppError::from(&e))?;
    let card = {
        let mut ctx = state.lock_for_write(client).await;
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
    ClientIdent(client): ClientIdent,
) -> Result<StatusCode, AppError> {
    {
        let mut ctx = state.lock_for_write(client).await;
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
    ClientIdent(client): ClientIdent,
) -> Result<Json<CardResponse>, AppError> {
    let (card, archived_at) = {
        let mut ctx = state.lock_for_write(client).await;
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
    ClientIdent(client): ClientIdent,
) -> Result<Json<CardResponse>, AppError> {
    let card = {
        let mut ctx = state.lock_for_write(client).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_domain::{CardStatus, SortField, SortOrder};

    #[test]
    fn test_card_query_filter_maps_every_field() {
        let board_id = Uuid::new_v4();
        let sprint_id = Uuid::new_v4();
        let other_sprint_id = Uuid::new_v4();
        let q = CardQuery {
            column_id: Some(Uuid::new_v4()),
            sprint_id: Some(sprint_id),
            sprint_ids: Some(HashSet::from([other_sprint_id])),
            hide_assigned: true,
            status: Some(CardStatusDto::Done),
            search: Some("q".to_string()),
            sort: Some(SortFieldDto::Priority),
            sort_order: Some(SortOrderDto::Descending),
            archived: ArchivedFilterDto::LiveOnly,
        };

        let filter = card_query_filter(&q, board_id, ArchivedFilter::LiveOnly);

        assert_eq!(filter.board_id, Some(board_id));
        assert_eq!(filter.column_id, q.column_id);
        assert_eq!(
            filter.sprint_ids,
            Some(HashSet::from([sprint_id, other_sprint_id]))
        );
        assert!(filter.hide_assigned);
        assert_eq!(filter.status, Some(CardStatus::Done));
        assert_eq!(filter.search, Some("q".to_string()));
        assert_eq!(filter.sort, Some(SortField::Priority));
        assert_eq!(filter.sort_order, Some(SortOrder::Descending));
        assert_eq!(filter.archived, ArchivedFilter::LiveOnly);
    }

    #[test]
    fn test_card_query_filter_clears_board_id_for_non_live_only() {
        let board_id = Uuid::new_v4();
        let q = CardQuery::default();

        for archived in [ArchivedFilter::Include, ArchivedFilter::ArchivedOnly] {
            let filter = card_query_filter(&q, board_id, archived);
            assert_eq!(filter.board_id, None);
        }

        let filter = card_query_filter(&q, board_id, ArchivedFilter::LiveOnly);
        assert_eq!(filter.board_id, Some(board_id));
    }

    #[test]
    fn test_card_query_filter_unions_sprint_id_into_sprint_ids() {
        let board_id = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let only_sprint_id = CardQuery {
            sprint_id: Some(a),
            ..Default::default()
        };
        assert_eq!(
            card_query_filter(&only_sprint_id, board_id, ArchivedFilter::LiveOnly).sprint_ids,
            Some(HashSet::from([a]))
        );

        let only_sprint_ids = CardQuery {
            sprint_ids: Some(HashSet::from([a, b])),
            ..Default::default()
        };
        assert_eq!(
            card_query_filter(&only_sprint_ids, board_id, ArchivedFilter::LiveOnly).sprint_ids,
            Some(HashSet::from([a, b]))
        );

        let both = CardQuery {
            sprint_id: Some(a),
            sprint_ids: Some(HashSet::from([b])),
            ..Default::default()
        };
        assert_eq!(
            card_query_filter(&both, board_id, ArchivedFilter::LiveOnly).sprint_ids,
            Some(HashSet::from([a, b]))
        );

        let neither = CardQuery::default();
        assert_eq!(
            card_query_filter(&neither, board_id, ArchivedFilter::LiveOnly).sprint_ids,
            None
        );
    }

    #[test]
    fn test_uuid_csv_parses_a_list_and_rejects_garbage() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let q: CardQuery =
            serde_json::from_value(serde_json::json!({ "sprint_ids": format!("{a},{b}") }))
                .unwrap();
        assert_eq!(q.sprint_ids, Some(HashSet::from([a, b])));

        let err =
            serde_json::from_value::<CardQuery>(serde_json::json!({ "sprint_ids": "notauuid" }));
        assert!(err.is_err());

        let q: CardQuery =
            serde_json::from_value(serde_json::json!({ "sprint_ids": format!("{a}, {b} ") }))
                .unwrap();
        assert_eq!(q.sprint_ids, Some(HashSet::from([a, b])));
    }

    #[test]
    fn test_uuid_csv_empty_value_yields_no_filter() {
        let q: CardQuery = serde_json::from_value(serde_json::json!({ "sprint_ids": "" })).unwrap();
        assert_eq!(q.sprint_ids, None);

        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let q: CardQuery =
            serde_json::from_value(serde_json::json!({ "sprint_ids": format!("{a},,{b}") }))
                .unwrap();
        assert_eq!(q.sprint_ids, Some(HashSet::from([a, b])));
    }
}
