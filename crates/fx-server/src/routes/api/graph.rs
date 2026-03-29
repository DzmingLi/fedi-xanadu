use axum::{
    Json,
    extract::State,
};
use fx_core::services::graph_service;

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::MaybeAuth;

#[utoipa::path(get, path = "/api/v1/graph", responses((status = 200, body = graph_service::GraphData)))]
pub async fn get_graph(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
) -> ApiResult<Json<graph_service::GraphData>> {
    let did = user.as_ref().map(|u| u.did.as_str());
    let graph = graph_service::build_knowledge_graph(&state.pool, did).await?;
    Ok(Json(graph))
}
