use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};

use crate::error::{ApiError, ApiResult, require_owner};
use crate::state::AppState;
use super::{AuthDid, RequireAuth, UriQuery, tid};

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct SkillTreeRow {
    at_uri: String,
    did: String,
    title: String,
    description: Option<String>,
    field: Option<String>,
    forked_from: Option<String>,
    created_at: String,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct SkillTreeListRow {
    at_uri: String,
    did: String,
    author_handle: Option<String>,
    title: String,
    description: Option<String>,
    field: Option<String>,
    forked_from: Option<String>,
    created_at: String,
    score: i64,
    edge_count: i64,
    adopt_count: i64,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct SkillTreeEdgeRow {
    parent_tag: String,
    child_tag: String,
}

#[derive(serde::Serialize)]
pub struct SkillTreeDetailResponse {
    tree: SkillTreeRow,
    edges: Vec<SkillTreeEdgeRow>,
    tag_names: std::collections::HashMap<String, String>,
}

pub async fn list_skill_trees(State(state): State<AppState>) -> ApiResult<Json<Vec<SkillTreeListRow>>> {
    let rows = sqlx::query_as::<_, SkillTreeListRow>(
        "SELECT st.at_uri, st.did, p.handle AS author_handle, st.title, st.description, st.field, st.forked_from, st.created_at,
         COALESCE((SELECT SUM(CASE WHEN v.value > 0 THEN 1 WHEN v.value < 0 THEN -1 ELSE 0 END) FROM votes v WHERE v.target_uri = st.at_uri), 0) AS score,
         (SELECT COUNT(*) FROM skill_tree_edges e WHERE e.tree_uri = st.at_uri) AS edge_count,
         (SELECT COUNT(*) FROM user_active_tree ua WHERE ua.tree_uri = st.at_uri) AS adopt_count
         FROM skill_trees st LEFT JOIN profiles p ON st.did = p.did
         ORDER BY score DESC, st.created_at DESC"
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

#[derive(serde::Deserialize)]
pub struct CreateSkillTreeInput {
    title: String,
    description: Option<String>,
    field: Option<String>,
    edges: Vec<EdgeInput>,
}

#[derive(serde::Deserialize)]
pub(crate) struct EdgeInput {
    parent_tag: String,
    child_tag: String,
}

pub async fn create_skill_tree(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<CreateSkillTreeInput>,
) -> ApiResult<(StatusCode, Json<SkillTreeRow>)> {
    let at_uri = format!("at://{}/li.dzming.fedi-xanadu.skilltree/{}", did, tid());

    let mut tx = state.pool.begin().await?;

    sqlx::query("INSERT INTO skill_trees (at_uri, did, title, description, field) VALUES (?, ?, ?, ?, ?)")
        .bind(&at_uri).bind(&did).bind(&input.title).bind(&input.description).bind(&input.field)
        .execute(&mut *tx).await?;

    for edge in &input.edges {
        sqlx::query("INSERT OR IGNORE INTO tags (id, name, created_by) VALUES (?, ?, ?)")
            .bind(&edge.parent_tag).bind(&edge.parent_tag).bind(&did)
            .execute(&mut *tx).await?;
        sqlx::query("INSERT OR IGNORE INTO tags (id, name, created_by) VALUES (?, ?, ?)")
            .bind(&edge.child_tag).bind(&edge.child_tag).bind(&did)
            .execute(&mut *tx).await?;
        sqlx::query("INSERT OR IGNORE INTO skill_tree_edges (tree_uri, parent_tag, child_tag) VALUES (?, ?, ?)")
            .bind(&at_uri).bind(&edge.parent_tag).bind(&edge.child_tag)
            .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    let row = sqlx::query_as::<_, SkillTreeRow>("SELECT at_uri, did, title, description, field, forked_from, created_at FROM skill_trees WHERE at_uri = ?")
        .bind(&at_uri).fetch_one(&state.pool).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn get_skill_tree_detail(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<SkillTreeDetailResponse>> {
    let tree = sqlx::query_as::<_, SkillTreeRow>("SELECT at_uri, did, title, description, field, forked_from, created_at FROM skill_trees WHERE at_uri = ?")
        .bind(&uri).fetch_optional(&state.pool).await?
        .ok_or(ApiError::NotFound("skill tree not found".into()))?;

    let edges = sqlx::query_as::<_, SkillTreeEdgeRow>(
        "SELECT parent_tag, child_tag FROM skill_tree_edges WHERE tree_uri = ?"
    ).bind(&uri).fetch_all(&state.pool).await?;

    let tag_names = collect_tag_names(&state.pool, &edges).await;

    Ok(Json(SkillTreeDetailResponse { tree, edges, tag_names }))
}

#[derive(serde::Deserialize)]
pub(crate) struct ForkSkillTreeInput { uri: String }

pub async fn fork_skill_tree(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<ForkSkillTreeInput>,
) -> ApiResult<(StatusCode, Json<SkillTreeRow>)> {
    let source = sqlx::query_as::<_, SkillTreeRow>("SELECT at_uri, did, title, description, field, forked_from, created_at FROM skill_trees WHERE at_uri = ?")
        .bind(&input.uri).fetch_optional(&state.pool).await?
        .ok_or(ApiError::NotFound("skill tree not found".into()))?;

    let new_uri = format!("at://{}/li.dzming.fedi-xanadu.skilltree/{}", did, tid());
    let new_title = format!("Fork: {}", source.title);

    sqlx::query("INSERT INTO skill_trees (at_uri, did, title, description, field, forked_from) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&new_uri).bind(&did).bind(&new_title).bind(&source.description).bind(&source.field).bind(&input.uri)
        .execute(&state.pool).await?;

    sqlx::query("INSERT INTO skill_tree_edges (tree_uri, parent_tag, child_tag) \
                 SELECT ?, parent_tag, child_tag FROM skill_tree_edges WHERE tree_uri = ?")
        .bind(&new_uri).bind(&input.uri)
        .execute(&state.pool).await?;

    let row = sqlx::query_as::<_, SkillTreeRow>("SELECT at_uri, did, title, description, field, forked_from, created_at FROM skill_trees WHERE at_uri = ?")
        .bind(&new_uri).fetch_one(&state.pool).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

#[derive(serde::Deserialize)]
pub(crate) struct SkillTreeEdgeInput { tree_uri: String, parent_tag: String, child_tag: String }

pub async fn add_skill_tree_edge(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<SkillTreeEdgeInput>,
) -> ApiResult<StatusCode> {
    let owner: Option<String> = sqlx::query_scalar::<_, String>("SELECT did FROM skill_trees WHERE at_uri = ?")
        .bind(&input.tree_uri).fetch_optional(&state.pool).await?;
    require_owner(owner.as_deref(), &did)?;

    sqlx::query("INSERT OR IGNORE INTO tags (id, name, created_by) VALUES (?, ?, ?)")
        .bind(&input.parent_tag).bind(&input.parent_tag).bind(&did)
        .execute(&state.pool).await?;
    sqlx::query("INSERT OR IGNORE INTO tags (id, name, created_by) VALUES (?, ?, ?)")
        .bind(&input.child_tag).bind(&input.child_tag).bind(&did)
        .execute(&state.pool).await?;

    sqlx::query("INSERT OR IGNORE INTO skill_tree_edges (tree_uri, parent_tag, child_tag) VALUES (?, ?, ?)")
        .bind(&input.tree_uri).bind(&input.parent_tag).bind(&input.child_tag)
        .execute(&state.pool).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_skill_tree_edge(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<SkillTreeEdgeInput>,
) -> ApiResult<StatusCode> {
    let owner: Option<String> = sqlx::query_scalar::<_, String>("SELECT did FROM skill_trees WHERE at_uri = ?")
        .bind(&input.tree_uri).fetch_optional(&state.pool).await?;
    require_owner(owner.as_deref(), &did)?;

    sqlx::query("DELETE FROM skill_tree_edges WHERE tree_uri = ? AND parent_tag = ? AND child_tag = ?")
        .bind(&input.tree_uri).bind(&input.parent_tag).bind(&input.child_tag)
        .execute(&state.pool).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub(crate) struct AdoptTreeInput { tree_uri: String }

pub async fn adopt_skill_tree(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<AdoptTreeInput>,
) -> ApiResult<StatusCode> {
    let mut tx = state.pool.begin().await?;

    sqlx::query("INSERT INTO user_active_tree (did, tree_uri) VALUES (?, ?) ON CONFLICT(did) DO UPDATE SET tree_uri = excluded.tree_uri")
        .bind(&did).bind(&input.tree_uri)
        .execute(&mut *tx).await?;

    // Sync skill tree edges into user's personal tag tree
    sqlx::query("DELETE FROM user_tag_tree WHERE did = ?")
        .bind(&did)
        .execute(&mut *tx).await?;

    sqlx::query(
        "INSERT OR IGNORE INTO user_tag_tree (did, parent_tag, child_tag) \
         SELECT ?, parent_tag, child_tag FROM skill_tree_edges WHERE tree_uri = ?"
    )
    .bind(&did)
    .bind(&input.tree_uri)
    .execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(StatusCode::OK)
}

pub async fn get_active_tree(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
) -> ApiResult<Json<Option<SkillTreeDetailResponse>>> {
    if did == "did:plc:anonymous" { return Ok(Json(None)); }

    let tree_uri = sqlx::query_scalar::<_, String>("SELECT tree_uri FROM user_active_tree WHERE did = ?")
        .bind(&did).fetch_optional(&state.pool).await?;

    let Some(uri) = tree_uri else { return Ok(Json(None)); };

    let tree = sqlx::query_as::<_, SkillTreeRow>("SELECT at_uri, did, title, description, field, forked_from, created_at FROM skill_trees WHERE at_uri = ?")
        .bind(&uri).fetch_optional(&state.pool).await?;
    let Some(tree) = tree else { return Ok(Json(None)); };

    let edges = sqlx::query_as::<_, SkillTreeEdgeRow>(
        "SELECT parent_tag, child_tag FROM skill_tree_edges WHERE tree_uri = ?"
    ).bind(&uri).fetch_all(&state.pool).await?;

    let tag_names = collect_tag_names(&state.pool, &edges).await;

    Ok(Json(Some(SkillTreeDetailResponse { tree, edges, tag_names })))
}

// --- Helpers ---

async fn collect_tag_names(pool: &sqlx::SqlitePool, edges: &[SkillTreeEdgeRow]) -> std::collections::HashMap<String, String> {
    let mut tag_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in edges { tag_ids.insert(e.parent_tag.clone()); tag_ids.insert(e.child_tag.clone()); }
    let mut tag_names = std::collections::HashMap::new();
    for id in &tag_ids {
        if let Ok(Some(name)) = sqlx::query_scalar::<_, String>("SELECT name FROM tags WHERE id = ?")
            .bind(id).fetch_optional(pool).await {
            tag_names.insert(id.clone(), name);
        } else {
            tag_names.insert(id.clone(), id.clone());
        }
    }
    tag_names
}
