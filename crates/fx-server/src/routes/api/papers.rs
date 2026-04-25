use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use fx_core::services::paper_service::{
    self, CreatePaper, CreateVersion, Paper, PaperDetailResponse, PaperListItem, PaperVersion,
};
use fx_core::util::tid;
use serde::{Deserialize, Serialize};

use crate::auth::WriteAuth;
use crate::error::{ApiResult, AppError};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_papers(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<PaperListItem>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let rows = paper_service::list_papers(&state.pool, limit, offset).await?;
    Ok(Json(rows))
}

pub async fn get_paper(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<PaperDetailResponse>> {
    let detail = paper_service::get_paper_detail(&state.pool, &id).await?;
    Ok(Json(detail))
}

/// Create a paper. Any logged-in user can mint one — the gating is on the
/// "claim authorship" side (the author-verified flow already covers that),
/// not on adding a paper entry to the directory.
pub async fn create_paper(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreatePaper>,
) -> ApiResult<(StatusCode, Json<Paper>)> {
    let id = format!("pap-{}", tid());
    let paper = paper_service::create_paper(&state.pool, &id, &user.did, &input).await?;
    Ok((StatusCode::CREATED, Json(paper)))
}

pub async fn delete_paper(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    paper_service::delete_paper(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_version(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<CreateVersion>,
) -> ApiResult<(StatusCode, Json<PaperVersion>)> {
    let vid = format!("pv-{}", tid());
    let version = paper_service::add_version(&state.pool, &id, &vid, &input).await?;
    Ok((StatusCode::CREATED, Json(version)))
}

#[derive(Deserialize)]
pub struct DeleteVersionQuery {
    pub version_id: String,
}

pub async fn delete_version(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(_id): Path<String>,
    Query(q): Query<DeleteVersionQuery>,
) -> ApiResult<StatusCode> {
    paper_service::delete_version(&state.pool, &q.version_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct AddAuthorInput {
    pub author_id: String,
    #[serde(default)]
    pub position: i16,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String { "author".into() }

pub async fn add_author(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<AddAuthorInput>,
) -> ApiResult<StatusCode> {
    paper_service::add_author(&state.pool, &id, &input.author_id, input.position, &input.role).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── OpenAlex import ────────────────────────────────────────────────────────
//
// OpenAlex (https://api.openalex.org) is a free open-access scholarly graph.
// Pasting a DOI / arxiv id into the form fires `POST /papers/import` which
// hits OpenAlex once, builds a Paper + version rows + author matches, and
// commits everything atomically.

#[derive(Deserialize)]
pub struct ImportInput {
    /// Bare DOI ("10.1109/TIP.2024.…") or full URL ("https://doi.org/…").
    #[serde(default)]
    pub doi: Option<String>,
    /// arxiv id ("2401.12345").
    #[serde(default)]
    pub arxiv_id: Option<String>,
    /// OpenAlex work id ("W4392…"). Bypasses doi/arxiv resolution.
    #[serde(default)]
    pub openalex_id: Option<String>,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub paper: Paper,
    /// Authors that matched to existing `authors` rows (linked via ORCID or
    /// fuzzy name match). UI can show these as clickable chips immediately.
    pub matched_authors: usize,
    /// Authors that came back from OpenAlex but had no match in the local
    /// authors table — they're recorded as plain strings on `papers.authors`
    /// so display still works.
    pub unmatched_authors: usize,
    pub versions: usize,
}

pub async fn import_paper(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<ImportInput>,
) -> ApiResult<(StatusCode, Json<ImportResult>)> {
    let work_path = if let Some(id) = input.openalex_id.as_deref() {
        id.trim().to_string()
    } else if let Some(doi) = input.doi.as_deref() {
        let bare = doi.trim().trim_start_matches("https://doi.org/").trim_start_matches("http://doi.org/");
        format!("doi:{bare}")
    } else if let Some(arxiv) = input.arxiv_id.as_deref() {
        format!("arxiv:{}", arxiv.trim())
    } else {
        return Err(AppError(fx_core::Error::BadRequest(
            "one of doi, arxiv_id, or openalex_id is required".into(),
        )));
    };

    let url = format!("https://api.openalex.org/works/{work_path}");
    let client = reqwest::Client::builder()
        .user_agent("nightboat/0.1 (+https://nightbo.at)")
        .build()
        .map_err(|e| AppError(fx_core::Error::Internal(format!("build client: {e}"))))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| AppError(fx_core::Error::Internal(format!("openalex fetch: {e}"))))?;
    if !resp.status().is_success() {
        return Err(AppError(fx_core::Error::BadRequest(format!(
            "openalex returned {} for {url}", resp.status()
        ))));
    }
    let work: OpenAlexWork = resp
        .json()
        .await
        .map_err(|e| AppError(fx_core::Error::Internal(format!("openalex parse: {e}"))))?;

    // Build the Paper row from OpenAlex fields.
    let title = work.title.clone().unwrap_or_default();
    let abstract_text = reconstruct_abstract(&work.abstract_inverted_index);
    let authors_str: Vec<String> = work
        .authorships
        .iter()
        .filter_map(|a| a.author.display_name.clone())
        .collect();
    let venue = work
        .primary_location
        .as_ref()
        .and_then(|l| l.source.as_ref())
        .and_then(|s| s.display_name.clone())
        .or_else(|| work.host_venue.as_ref().and_then(|v| v.display_name.clone()));
    let venue_kind = work
        .primary_location
        .as_ref()
        .and_then(|l| l.source.as_ref())
        .and_then(|s| s.kind_for_storage());
    let doi_clean = work.doi.as_ref()
        .map(|d| d.trim_start_matches("https://doi.org/").to_string());
    let arxiv_id = extract_arxiv_id(&work);

    let create = CreatePaper {
        title: std::iter::once(("en".to_string(), title.clone())).collect(),
        abstract_: if abstract_text.is_empty() { None } else {
            Some(std::iter::once(("en".to_string(), abstract_text)).collect())
        },
        authors: authors_str,
        venue,
        venue_kind,
        year: work.publication_year.map(|y| y as i16),
        doi: doi_clean,
        arxiv_id,
        bibtex_key: None,
        accepted: true,
    };

    let pap_id = format!("pap-{}", tid());
    let paper = paper_service::create_paper(&state.pool, &pap_id, &user.did, &create).await?;

    // Versions — emit whatever URLs OpenAlex gave us. Order: preprint
    // (typically arxiv via best_oa_location) first, then published landing
    // page, then any open-access copy that's distinct.
    let mut versions = 0usize;
    if let Some(oa) = work.best_oa_location.as_ref() {
        if let Some(url) = oa.landing_page_url.clone().or_else(|| oa.pdf_url.clone()) {
            let kind = if url.contains("arxiv.org") { "preprint" } else { "published" };
            paper_service::add_version(
                &state.pool, &pap_id, &format!("pv-{}", tid()),
                &CreateVersion {
                    kind: kind.into(),
                    url: Some(url),
                    article_uri: None,
                    year: paper.year,
                    label: None,
                    sort_order: Some(0),
                },
            ).await?;
            versions += 1;
        }
    }
    if let Some(prim) = work.primary_location.as_ref().and_then(|l| l.landing_page_url.clone()) {
        if !already_seen_url(&state.pool, &pap_id, &prim).await {
            paper_service::add_version(
                &state.pool, &pap_id, &format!("pv-{}", tid()),
                &CreateVersion {
                    kind: "published".into(),
                    url: Some(prim),
                    article_uri: None,
                    year: paper.year,
                    label: None,
                    sort_order: Some(versions as i16),
                },
            ).await?;
            versions += 1;
        }
    }

    // Author matching: ORCID is the strongest signal; fall back to
    // case-insensitive name equality for authors who haven't claimed their
    // ORCID yet. Anything unmatched stays in the `papers.authors` text[]
    // (so display still works) but doesn't hyperlink to a profile.
    let mut matched = 0usize;
    let mut unmatched = 0usize;
    for (pos, ship) in work.authorships.iter().enumerate() {
        let author_id: Option<String> = if let Some(orcid) = ship.author.orcid.as_deref() {
            let orcid_clean = orcid.trim_start_matches("https://orcid.org/");
            sqlx::query_scalar::<_, String>("SELECT id FROM authors WHERE orcid = $1 LIMIT 1")
                .bind(orcid_clean)
                .fetch_optional(&state.pool)
                .await?
        } else if let Some(name) = ship.author.display_name.as_deref() {
            sqlx::query_scalar::<_, String>("SELECT id FROM authors WHERE LOWER(name) = LOWER($1) LIMIT 1")
                .bind(name)
                .fetch_optional(&state.pool)
                .await?
        } else { None };
        match author_id {
            Some(id) => {
                paper_service::add_author(&state.pool, &pap_id, &id, pos as i16, "author").await?;
                matched += 1;
            }
            None => unmatched += 1,
        }
    }

    Ok((StatusCode::CREATED, Json(ImportResult {
        paper, matched_authors: matched, unmatched_authors: unmatched, versions,
    })))
}

async fn already_seen_url(pool: &sqlx::PgPool, paper_id: &str, url: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM paper_versions WHERE paper_id = $1 AND url = $2",
    )
    .bind(paper_id)
    .bind(url)
    .fetch_one(pool)
    .await
    .map(|c| c > 0)
    .unwrap_or(false)
}

// ── OpenAlex JSON shape (subset we use) ───────────────────────────────────

#[derive(Deserialize)]
struct OpenAlexWork {
    title: Option<String>,
    abstract_inverted_index: Option<std::collections::HashMap<String, Vec<usize>>>,
    publication_year: Option<i32>,
    doi: Option<String>,
    authorships: Vec<Authorship>,
    primary_location: Option<Location>,
    best_oa_location: Option<Location>,
    host_venue: Option<HostVenue>,
    ids: Option<Ids>,
}

#[derive(Deserialize)]
struct Authorship {
    author: AuthorRef,
}

#[derive(Deserialize)]
struct AuthorRef {
    display_name: Option<String>,
    orcid: Option<String>,
}

#[derive(Deserialize)]
struct Location {
    landing_page_url: Option<String>,
    pdf_url: Option<String>,
    source: Option<Source>,
}

#[derive(Deserialize)]
struct Source {
    display_name: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
}

impl Source {
    /// Map OpenAlex's `source.type` to our `papers.venue_kind` vocabulary.
    fn kind_for_storage(&self) -> Option<String> {
        match self.kind.as_deref()? {
            "journal"      => Some("journal".into()),
            "conference"   => Some("conference".into()),
            "repository"   => Some("preprint".into()),
            "book"         => Some("book".into()),
            "book series"  => Some("book".into()),
            other          => Some(other.into()),
        }
    }
}

#[derive(Deserialize)]
struct HostVenue {
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct Ids {
    openalex: Option<String>,
}

/// Re-assemble OpenAlex's inverted-index abstract back into prose. The map
/// is { word: [positions...] } — invert it, sort by position, join.
fn reconstruct_abstract(
    inv: &Option<std::collections::HashMap<String, Vec<usize>>>,
) -> String {
    let Some(inv) = inv else { return String::new() };
    let mut by_pos: Vec<(usize, &str)> = Vec::new();
    for (word, positions) in inv {
        for &p in positions {
            by_pos.push((p, word.as_str()));
        }
    }
    by_pos.sort_by_key(|&(p, _)| p);
    by_pos.into_iter().map(|(_, w)| w).collect::<Vec<_>>().join(" ")
}

fn extract_arxiv_id(work: &OpenAlexWork) -> Option<String> {
    let oa = work.best_oa_location.as_ref()?;
    let url = oa.landing_page_url.as_deref().or(oa.pdf_url.as_deref())?;
    if !url.contains("arxiv.org") { return None }
    // arxiv.org/abs/2401.12345 or arxiv.org/pdf/2401.12345.pdf
    url.split('/').rev().find_map(|seg| {
        let s = seg.trim_end_matches(".pdf");
        if s.contains('.') && s.split('.').next()?.parse::<u32>().is_ok() {
            Some(s.to_string())
        } else { None }
    })
}
