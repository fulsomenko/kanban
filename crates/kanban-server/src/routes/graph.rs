use crate::error::AppError;
use crate::model_read::{require_loaded, require_loaded_entity};
use crate::scope::RouteScope;
use crate::state::{AppState, Session};
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use kanban_domain::NoProjections;
use kanban_service::api::CardGraphResponse;
use uuid::Uuid;

async fn get_card_graph(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CardGraphResponse>, AppError> {
    let mut guard = state.lock_session().await;
    {
        let Session { ctx, model } = &mut *guard;
        ctx.sync(&RouteScope::CardGraph(id), model, &mut NoProjections);
    }
    require_loaded_entity(guard.model.card_id_status(id), "Card", id)?;
    let graph = require_loaded(guard.model.graph_state().as_ref(), "card graph")?;
    Ok(Json(CardGraphResponse::from_graph(id, graph)))
}

pub fn read_router() -> Router<AppState> {
    Router::new().route("/v1/cards/{id}/graph", get(get_card_graph))
}
