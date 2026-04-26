use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// ── Row types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TermRow {
    pub id: String,
    pub did: String,
    pub title: String,
    pub code: Option<String>,
    pub description: String,
    pub institution: Option<String>,
    pub department: Option<String>,
    pub semester: Option<String>,
    pub lang: String,
    pub license: String,
    pub source_url: Option<String>,
    pub source_attribution: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Course group this iteration belongs to. When two courses share a
    /// `course_id`, they're different iterations of the same offering and
    /// the UI can render them as sibling tabs.
    #[sqlx(default)]
    pub course_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TermListRow {
    pub id: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub title: String,
    pub code: Option<String>,
    pub description: String,
    pub institution: Option<String>,
    pub semester: Option<String>,
    pub lang: String,
    pub series_count: i64,
    pub session_count: i64,
    pub avg_rating: f64,
    pub rating_count: i64,
    pub created_at: DateTime<Utc>,
    /// Names of the course's real authors (professors), joined from
    /// `term_authors`. Displayed on cards instead of the uploader handle.
    #[sqlx(default)]
    pub author_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TermSeriesRow {
    pub series_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub role: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TermDetailResponse {
    pub term: TermRow,
    pub syllabus: String,
    pub authors: Vec<crate::services::author_service::Author>,
    pub sessions: Vec<TermSessionDetail>,
    pub textbooks: Vec<TermTextbookRow>,
    pub tags: Vec<TermTagRow>,
    pub series: Vec<TermSeriesRow>,
    pub skill_trees: Vec<TermSkillTreeRow>,
    pub prerequisites: Vec<TermPrereqRow>,
    pub rating: TermRatingStats,
    pub reviews: Vec<TermReviewRow>,
    pub review_count: i64,
    pub notes: Vec<TermReviewRow>,
    pub note_count: i64,
    pub discussions: Vec<crate::models::Comment>,
    pub discussion_count: i64,
    /// The current viewer's rating for this course (1-10), if any.
    pub my_rating: Option<i16>,
    /// The current viewer's learning status + progress % for this course.
    pub my_learning_status: Option<TermLearningStatus>,
    /// The current viewer's per-session completion records.
    pub my_session_progress: Vec<SessionProgress>,
    /// Course-level supplementary resources (software pages, tools, etc.).
    pub resources: Vec<TermResource>,
    /// Other iterations of the same course (same `course_id`), newest
    /// semester first. Empty when the course isn't in a group. The
    /// viewed course itself is excluded.
    pub siblings: Vec<TermSibling>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TermSibling {
    pub id: String,
    pub title: String,
    pub semester: Option<String>,
    pub institution: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TermTagRow {
    pub tag_id: String,
    pub tag_name: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TermTextbookRow {
    pub book_id: String,
    pub title: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub authors: Vec<String>,
    pub cover_url: Option<String>,
    pub role: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TermSkillTreeRow {
    pub tree_uri: String,
    pub title: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TermPrereqRow {
    pub prereq_term_id: String,
    pub title: String,
    pub code: Option<String>,
    pub institution: Option<String>,
}

/// Anything attached to a session — video, paper, slide deck, problem
/// set. The previous schema split this into `materials` and `resources`
/// arrays based on display preference; now it's one flat array and the
/// frontend decides how to group / render based on `kind`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKind {
    Video,
    Slides,
    Notes,
    Recitation,
    Reading,
    Code,
    Homework,
    Discussion,
    Outline,
    Summary,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub kind: AttachmentKind,
    pub label: String,
    /// URL is optional — a textbook citation like "Pierce Ch 5.1-2" is a
    /// valid label-only attachment that renders as plain text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Required reading vs. supplementary / further-reading. The UI uses
    /// this to dim or section off optional entries. Defaults to `true`
    /// so callers that don't care get the sensible default.
    #[serde(default = "default_required")]
    pub required: bool,
}

fn default_required() -> bool { true }

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TermSessionRow {
    pub id: String,
    pub term_id: String,
    pub sort_order: i32,
    pub topic: Option<String>,
    pub date: Option<String>,
    #[sqlx(default)]
    pub attachments: sqlx::types::Json<Vec<Attachment>>,
    /// Discriminator: `lecture` (default), `section` (thematic header
    /// row spanning the whole calendar width — no attachments, not
    /// counted in lecture numbering), or `exam`. Frontends decide how
    /// to render based on this.
    #[serde(default = "default_session_kind")]
    #[sqlx(default)]
    pub kind: String,
}

fn default_session_kind() -> String { "lecture".to_string() }

#[derive(Debug, Clone, Serialize)]
pub struct TermSessionDetail {
    #[serde(flatten)]
    pub session: TermSessionRow,
    pub tags: Vec<TermTagRow>,
    pub prereqs: Vec<TermTagRow>,
}

// ── Input types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateTerm {
    pub title: String,
    pub code: Option<String>,
    pub description: Option<String>,
    pub syllabus: Option<String>,
    pub institution: Option<String>,
    pub department: Option<String>,
    pub semester: Option<String>,
    pub lang: Option<String>,
    pub license: Option<String>,
    pub source_url: Option<String>,
    pub source_attribution: Option<String>,
    /// Author/instructor names — creates author entities (with did=NULL
    /// until claimed) and links them via term_authors.
    #[serde(default)]
    pub authors: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTerm {
    pub title: Option<String>,
    pub code: Option<String>,
    pub description: Option<String>,
    pub syllabus: Option<String>,
    pub institution: Option<String>,
    pub department: Option<String>,
    pub semester: Option<String>,
    pub lang: Option<String>,
    pub license: Option<String>,
    pub source_url: Option<String>,
    pub source_attribution: Option<String>,
    #[serde(default)]
    pub authors: Option<Vec<String>>,
    #[serde(default)]
    pub summary: Option<String>,
}

// ── Service functions ───────────────────────────────────────────────────

pub async fn create_term(pool: &PgPool, id: &str, did: &str, input: &CreateTerm) -> crate::Result<TermRow> {
    let lang = input.lang.as_deref().unwrap_or("zh");
    let license = input.license.as_deref().unwrap_or("CC-BY-SA-4.0");

    sqlx::query(
        "INSERT INTO terms (id, did, title, code, description, syllabus, institution, department, \
         semester, lang, license, source_url, source_attribution) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(id).bind(did).bind(&input.title).bind(&input.code)
    .bind(input.description.as_deref().unwrap_or(""))
    .bind(input.syllabus.as_deref().unwrap_or(""))
    .bind(&input.institution).bind(&input.department)
    .bind(&input.semester).bind(lang).bind(license)
    .bind(&input.source_url).bind(&input.source_attribution)
    .execute(pool).await?;

    for (position, name) in input.authors.iter().enumerate() {
        let author_id = super::author_service::get_or_create_author(pool, name).await?;
        sqlx::query(
            "INSERT INTO term_authors (term_id, author_id, position, role) \
             VALUES ($1, $2, $3, 'instructor') ON CONFLICT DO NOTHING",
        )
        .bind(id).bind(&author_id).bind(position as i16)
        .execute(pool).await?;
    }

    get_term(pool, id).await
}

/// Replace the course's author list. Clears term_authors and re-links.
pub async fn set_term_authors(pool: &PgPool, term_id: &str, names: &[String]) -> crate::Result<()> {
    sqlx::query("DELETE FROM term_authors WHERE term_id = $1")
        .bind(term_id).execute(pool).await?;
    for (position, name) in names.iter().enumerate() {
        let author_id = super::author_service::get_or_create_author(pool, name).await?;
        sqlx::query(
            "INSERT INTO term_authors (term_id, author_id, position, role) \
             VALUES ($1, $2, $3, 'instructor') ON CONFLICT DO NOTHING",
        )
        .bind(term_id).bind(&author_id).bind(position as i16)
        .execute(pool).await?;
    }
    Ok(())
}

pub async fn list_term_authors(pool: &PgPool, term_id: &str) -> crate::Result<Vec<crate::services::author_service::Author>> {
    let rows = sqlx::query_as::<_, crate::services::author_service::Author>(
        "SELECT a.id, a.name, a.did, a.orcid, a.affiliation, a.homepage, \
                a.original_names, a.official_translations, a.translations \
         FROM term_authors ca JOIN authors a ON a.id = ca.author_id \
         WHERE ca.term_id = $1 ORDER BY ca.position",
    )
    .bind(term_id).fetch_all(pool).await?;
    Ok(rows)
}

pub async fn get_term(pool: &PgPool, id: &str) -> crate::Result<TermRow> {
    sqlx::query_as::<_, TermRow>(
        "SELECT id, did, title, code, description, institution, department, semester, \
         lang, license, source_url, source_attribution, created_at, updated_at, course_id \
         FROM terms WHERE id = $1",
    )
    .bind(id).fetch_one(pool).await
    .map_err(|_| crate::Error::NotFound { entity: "term", id: id.to_string() })
}

pub async fn get_term_detail(pool: &PgPool, id: &str, viewer_did: Option<&str>) -> crate::Result<TermDetailResponse> {
    let term = get_term(pool, id).await?;

    let syllabus: String = sqlx::query_scalar("SELECT syllabus FROM terms WHERE id = $1")
        .bind(id).fetch_one(pool).await.unwrap_or_default();

    // Fetch sessions with their tags and prereqs
    let session_rows = sqlx::query_as::<_, TermSessionRow>(
        "SELECT id, term_id, sort_order, topic, date, attachments, kind \
         FROM term_sessions WHERE term_id = $1 ORDER BY sort_order",
    ).bind(id).fetch_all(pool).await?;

    let mut sessions = Vec::with_capacity(session_rows.len());
    for row in session_rows {
        let tags = sqlx::query_as::<_, TermTagRow>(
            "SELECT cst.tag_id, tag_canonical_label(cst.tag_id) AS tag_name \
             FROM term_session_tags cst \
             WHERE cst.session_id = $1 ORDER BY tag_name",
        ).bind(&row.id).fetch_all(pool).await?;

        let prereqs = sqlx::query_as::<_, TermTagRow>(
            "SELECT csp.tag_id, tag_canonical_label(csp.tag_id) AS tag_name \
             FROM term_session_prereqs csp \
             WHERE csp.session_id = $1 ORDER BY tag_name",
        ).bind(&row.id).fetch_all(pool).await?;

        sessions.push(TermSessionDetail { session: row, tags, prereqs });
    }

    let tags = sqlx::query_as::<_, TermTagRow>(
        "SELECT ct.tag_id, tag_canonical_label(ct.tag_id) AS tag_name \
         FROM term_tags ct \
         WHERE ct.term_id = $1 ORDER BY tag_name",
    ).bind(id).fetch_all(pool).await?;

    let textbooks = sqlx::query_as::<_, TermTextbookRow>(
        "SELECT ct.book_id, b.title, b.authors, \
         (SELECT e.cover_url FROM book_editions e WHERE e.book_id = b.id AND e.cover_url IS NOT NULL LIMIT 1) AS cover_url, \
         ct.role, ct.sort_order \
         FROM term_textbooks ct JOIN books b ON b.id = ct.book_id \
         WHERE ct.term_id = $1 ORDER BY ct.sort_order",
    ).bind(id).fetch_all(pool).await?;

    let series = sqlx::query_as::<_, TermSeriesRow>(
        "SELECT cs.series_id, s.title, s.summary, cs.role, cs.sort_order \
         FROM term_series cs JOIN series s ON s.id = cs.series_id \
         WHERE cs.term_id = $1 ORDER BY cs.sort_order",
    ).bind(id).fetch_all(pool).await?;

    let skill_trees = sqlx::query_as::<_, TermSkillTreeRow>(
        "SELECT cst.tree_uri, st.title, cst.role \
         FROM term_skill_trees cst JOIN skill_trees st ON st.at_uri = cst.tree_uri \
         WHERE cst.term_id = $1",
    ).bind(id).fetch_all(pool).await?;

    let prerequisites = sqlx::query_as::<_, TermPrereqRow>(
        "SELECT cp.prereq_term_id, c.title, c.code, c.institution \
         FROM term_prerequisites cp JOIN terms c ON c.id = cp.prereq_term_id \
         WHERE cp.term_id = $1",
    ).bind(id).fetch_all(pool).await?;

    let rating = get_rating_stats(pool, id).await?;
    let reviews = list_term_articles_by_category(pool, id, "review", 5, 0).await?;
    let review_count = count_term_articles_by_category(pool, id, "review").await?;
    let notes = list_term_articles_by_category(pool, id, "note", 5, 0).await?;
    let note_count = count_term_articles_by_category(pool, id, "note").await?;

    let term_uri = format!("term:{id}");
    let discussions = crate::services::comment_service::list_top_comments(pool, &term_uri, 5, 0).await?;
    let discussion_count = crate::services::comment_service::count_top_comments(pool, &term_uri).await?;

    let authors = list_term_authors(pool, id).await?;

    let (my_rating, my_learning_status, my_session_progress) = if let Some(did) = viewer_did {
        (
            get_user_rating(pool, id, did).await?,
            get_learning_status(pool, id, did).await?,
            list_session_progress(pool, id, did).await?,
        )
    } else {
        (None, None, vec![])
    };

    let resources = list_term_resources(pool, id).await?;

    let siblings: Vec<TermSibling> = if let Some(ref cid) = term.course_id {
        sqlx::query_as::<_, TermSibling>(
            "SELECT id, title, semester, institution FROM terms \
             WHERE course_id = $1 AND id <> $2 \
             ORDER BY semester DESC NULLS LAST, created_at DESC",
        ).bind(cid).bind(id).fetch_all(pool).await?
    } else {
        Vec::new()
    };

    Ok(TermDetailResponse {
        term, syllabus, authors, sessions, textbooks, tags, series, skill_trees, prerequisites,
        rating, reviews, review_count, notes, note_count, discussions, discussion_count,
        my_rating, my_learning_status, my_session_progress, resources, siblings,
    })
}

const TERM_LIST_SELECT: &str = "\
    SELECT c.id, c.did, p.handle AS author_handle, c.title, c.code, c.description, \
    c.institution, c.semester, c.lang, \
    (SELECT COUNT(*) FROM term_series WHERE term_id = c.id) AS series_count, \
    (SELECT COUNT(*) FROM term_sessions WHERE term_id = c.id) AS session_count, \
    COALESCE(r.avg, 0) AS avg_rating, \
    COALESCE(r.cnt, 0) AS rating_count, \
    c.created_at, \
    COALESCE( \
      (SELECT array_agg(a.name ORDER BY ca.position NULLS LAST) \
       FROM term_authors ca JOIN authors a ON a.id = ca.author_id \
       WHERE ca.term_id = c.id), \
      ARRAY[]::text[]) AS author_names \
    FROM terms c \
    LEFT JOIN profiles p ON c.did = p.did \
    LEFT JOIN (SELECT term_id, AVG(rating)::float8 AS avg, COUNT(*) AS cnt FROM term_ratings GROUP BY term_id) r ON r.term_id = c.id";

pub async fn list_terms(pool: &PgPool) -> crate::Result<Vec<TermListRow>> {
    Ok(sqlx::query_as::<_, TermListRow>(
        &format!("{TERM_LIST_SELECT} ORDER BY COALESCE(r.avg, 0) * LN(COALESCE(r.cnt, 0) + 1) DESC, c.created_at DESC"),
    ).fetch_all(pool).await?)
}

pub async fn list_my_terms(pool: &PgPool, did: &str) -> crate::Result<Vec<TermListRow>> {
    Ok(sqlx::query_as::<_, TermListRow>(
        &format!("{TERM_LIST_SELECT} WHERE c.did = $1 ORDER BY c.created_at DESC"),
    ).bind(did).fetch_all(pool).await?)
}

/// Snapshot a course as a flat JSON object for diffing.
fn term_to_json(c: &TermRow, syllabus: &str) -> serde_json::Value {
    serde_json::json!({
        "title": c.title,
        "code": c.code,
        "description": c.description,
        "syllabus": syllabus,
        "institution": c.institution,
        "department": c.department,
        "semester": c.semester,
        "lang": c.lang,
        "license": c.license,
        "source_url": c.source_url,
        "source_attribution": c.source_attribution,
    })
}

pub async fn update_term(pool: &PgPool, id: &str, did: &str, input: &UpdateTerm, summary: &str) -> crate::Result<TermRow> {
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM terms WHERE id = $1")
        .bind(id).fetch_optional(pool).await?;
    let owner_did = match owner {
        Some(d) => d,
        None => return Err(crate::Error::NotFound { entity: "term", id: id.to_string() }),
    };

    // Snapshot old state
    let cur = get_term(pool, id).await?;
    let old_syllabus: String = sqlx::query_scalar("SELECT syllabus FROM terms WHERE id = $1")
        .bind(id).fetch_one(pool).await.unwrap_or_default();
    let old_json = term_to_json(&cur, &old_syllabus);

    // Build new state
    let new_syllabus = input.syllabus.as_deref().unwrap_or(&old_syllabus);

    let new_json = serde_json::json!({
        "title": input.title.as_deref().unwrap_or(&cur.title),
        "code": input.code.as_ref().or(cur.code.as_ref()),
        "description": input.description.as_deref().unwrap_or(&cur.description),
        "syllabus": new_syllabus,
        "institution": input.institution.as_ref().or(cur.institution.as_ref()),
        "department": input.department.as_ref().or(cur.department.as_ref()),
        "semester": input.semester.as_ref().or(cur.semester.as_ref()),
        "lang": input.lang.as_deref().unwrap_or(&cur.lang),
        "license": input.license.as_deref().unwrap_or(&cur.license),
        "source_url": input.source_url.as_ref().or(cur.source_url.as_ref()),
        "source_attribution": input.source_attribution.as_ref().or(cur.source_attribution.as_ref()),
    });

    // Authors live outside the patchable diff (managed via term_authors,
    // not the courses row), so an authors-only update would be silently
    // dropped if we only checked `ops`. Treat an explicit `authors` payload
    // as a real change.
    let ops = super::patch_service::diff(&old_json, &new_json);
    let authors_update = input.authors.is_some();
    if ops.is_empty() && !authors_update {
        return Ok(cur); // no changes
    }

    let is_owner = did == owner_did;

    // Record patch only when there are diffable field changes — author edits
    // skip the patch table since they don't fit the field-diff model.
    if !ops.is_empty() {
        let _patch = super::patch_service::create_patch(
            pool, "term", id, did, &owner_did, &ops, summary,
        ).await?;
    }

    // If auto-applied (owner edit), materialize the update
    if is_owner {
        if !ops.is_empty() {
            sqlx::query(
                "UPDATE terms SET \
                 title = $1, code = $2, description = $3, syllabus = $4, \
                 institution = $5, department = $6, semester = $7, lang = $8, license = $9, \
                 source_url = $10, source_attribution = $11, \
                 updated_at = NOW() \
                 WHERE id = $12",
            )
            .bind(new_json["title"].as_str())
            .bind(new_json["code"].as_str())
            .bind(new_json["description"].as_str())
            .bind(new_json["syllabus"].as_str())
            .bind(new_json["institution"].as_str())
            .bind(new_json["department"].as_str())
            .bind(new_json["semester"].as_str())
            .bind(new_json["lang"].as_str())
            .bind(new_json["license"].as_str())
            .bind(new_json["source_url"].as_str())
            .bind(new_json["source_attribution"].as_str())
            .bind(id)
            .execute(pool).await?;
        }

        if let Some(ref authors) = input.authors {
            set_term_authors(pool, id, authors).await?;
        }
    }

    get_term(pool, id).await
}

pub async fn delete_term(pool: &PgPool, id: &str, did: &str) -> crate::Result<()> {
    let result = sqlx::query("DELETE FROM terms WHERE id = $1 AND did = $2")
        .bind(id).bind(did).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "term", id: id.to_string() });
    }
    Ok(())
}

// ── Session management ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateSession {
    pub topic: Option<String>,
    pub date: Option<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    pub sort_order: Option<i32>,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSession {
    pub topic: Option<String>,
    pub date: Option<String>,
    pub attachments: Option<Vec<Attachment>>,
    pub sort_order: Option<i32>,
    #[serde(default)]
    pub kind: Option<String>,
}

pub async fn create_session(pool: &PgPool, session_id: &str, term_id: &str, input: &CreateSession) -> crate::Result<TermSessionRow> {
    let sort_order = match input.sort_order {
        Some(o) => o,
        None => {
            let max: Option<i32> = sqlx::query_scalar(
                "SELECT MAX(sort_order) FROM term_sessions WHERE term_id = $1"
            ).bind(term_id).fetch_one(pool).await?;
            max.unwrap_or(0) + 1
        }
    };

    let kind = input.kind.as_deref().unwrap_or("lecture");
    sqlx::query(
        "INSERT INTO term_sessions (id, term_id, sort_order, topic, date, attachments, kind) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(session_id).bind(term_id).bind(sort_order)
    .bind(&input.topic).bind(&input.date)
    .bind(sqlx::types::Json(&input.attachments))
    .bind(kind)
    .execute(pool).await?;

    get_session(pool, session_id).await
}

pub async fn get_session(pool: &PgPool, session_id: &str) -> crate::Result<TermSessionRow> {
    sqlx::query_as::<_, TermSessionRow>(
        "SELECT id, term_id, sort_order, topic, date, attachments, kind \
         FROM term_sessions WHERE id = $1",
    )
    .bind(session_id).fetch_one(pool).await
    .map_err(|_| crate::Error::NotFound { entity: "session", id: session_id.to_string() })
}

pub async fn update_session(pool: &PgPool, session_id: &str, input: &UpdateSession) -> crate::Result<TermSessionRow> {
    let cur = get_session(pool, session_id).await?;

    sqlx::query(
        "UPDATE term_sessions SET \
         topic = $1, date = $2, attachments = $3, sort_order = $4, kind = $5 \
         WHERE id = $6",
    )
    .bind(input.topic.as_ref().or(cur.topic.as_ref()))
    .bind(input.date.as_ref().or(cur.date.as_ref()))
    .bind(sqlx::types::Json(input.attachments.as_ref().unwrap_or(&cur.attachments.0)))
    .bind(input.sort_order.unwrap_or(cur.sort_order))
    .bind(input.kind.as_deref().unwrap_or(&cur.kind))
    .bind(session_id)
    .execute(pool).await?;

    get_session(pool, session_id).await
}

pub async fn delete_session(pool: &PgPool, session_id: &str) -> crate::Result<()> {
    let result = sqlx::query("DELETE FROM term_sessions WHERE id = $1")
        .bind(session_id).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "session", id: session_id.to_string() });
    }
    Ok(())
}

pub async fn add_session_tag(pool: &PgPool, session_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO term_session_tags (session_id, tag_id) \
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
        .bind(session_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

pub async fn remove_session_tag(pool: &PgPool, session_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query(
        "DELETE FROM term_session_tags WHERE session_id = $1 AND tag_id = $2",
    )
        .bind(session_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

pub async fn add_session_prereq(pool: &PgPool, session_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO term_session_prereqs (session_id, tag_id) \
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
        .bind(session_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

pub async fn remove_session_prereq(pool: &PgPool, session_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query(
        "DELETE FROM term_session_prereqs WHERE session_id = $1 AND tag_id = $2",
    )
        .bind(session_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

// ── Relation management ─────────────────────────────────────────────────

pub async fn add_series(pool: &PgPool, term_id: &str, series_id: &str, role: &str, sort_order: i32) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO term_series (term_id, series_id, role, sort_order) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING"
    ).bind(term_id).bind(series_id).bind(role).bind(sort_order)
    .execute(pool).await?;
    Ok(())
}

pub async fn remove_series(pool: &PgPool, term_id: &str, series_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM term_series WHERE term_id = $1 AND series_id = $2")
        .bind(term_id).bind(series_id).execute(pool).await?;
    Ok(())
}

pub async fn add_skill_tree(pool: &PgPool, term_id: &str, tree_uri: &str, role: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO term_skill_trees (term_id, tree_uri, role) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"
    ).bind(term_id).bind(tree_uri).bind(role)
    .execute(pool).await?;
    Ok(())
}

pub async fn add_prerequisite(pool: &PgPool, term_id: &str, prereq_id: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO term_prerequisites (term_id, prereq_term_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    ).bind(term_id).bind(prereq_id)
    .execute(pool).await?;
    Ok(())
}

pub async fn remove_prerequisite(pool: &PgPool, term_id: &str, prereq_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM term_prerequisites WHERE term_id = $1 AND prereq_term_id = $2")
        .bind(term_id).bind(prereq_id).execute(pool).await?;
    Ok(())
}

// ── Tags ────────────────────────────────────────────────────────────────

pub async fn add_tag(pool: &PgPool, term_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO term_tags (term_id, tag_id) \
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
        .bind(term_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

pub async fn remove_tag(pool: &PgPool, term_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query(
        "DELETE FROM term_tags WHERE term_id = $1 AND tag_id = $2",
    )
        .bind(term_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

// ── Textbooks ───────────────────────────────────────────────────────────

pub async fn add_textbook(pool: &PgPool, term_id: &str, book_id: &str, role: &str, sort_order: i32) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO term_textbooks (term_id, book_id, role, sort_order) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING"
    ).bind(term_id).bind(book_id).bind(role).bind(sort_order)
    .execute(pool).await?;
    Ok(())
}

pub async fn remove_textbook(pool: &PgPool, term_id: &str, book_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM term_textbooks WHERE term_id = $1 AND book_id = $2")
        .bind(term_id).bind(book_id).execute(pool).await?;
    Ok(())
}

// ── Ratings ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct TermRatingStats {
    pub avg_rating: f64,
    pub rating_count: i64,
}

pub async fn rate_term(pool: &PgPool, term_id: &str, user_did: &str, rating: i16) -> crate::Result<TermRatingStats> {
    sqlx::query(
        "INSERT INTO term_ratings (term_id, user_did, rating) VALUES ($1, $2, $3) \
         ON CONFLICT (term_id, user_did) DO UPDATE SET rating = $3, updated_at = NOW()"
    ).bind(term_id).bind(user_did).bind(rating)
    .execute(pool).await?;

    get_rating_stats(pool, term_id).await
}

pub async fn unrate_term(pool: &PgPool, term_id: &str, user_did: &str) -> crate::Result<TermRatingStats> {
    sqlx::query("DELETE FROM term_ratings WHERE term_id = $1 AND user_did = $2")
        .bind(term_id).bind(user_did)
        .execute(pool).await?;
    get_rating_stats(pool, term_id).await
}

pub async fn get_rating_stats(pool: &PgPool, term_id: &str) -> crate::Result<TermRatingStats> {
    let row: (Option<f64>, i64) = sqlx::query_as(
        "SELECT AVG(rating::float), COUNT(*) FROM term_ratings WHERE term_id = $1"
    ).bind(term_id).fetch_one(pool).await?;
    Ok(TermRatingStats { avg_rating: row.0.unwrap_or(0.0), rating_count: row.1 })
}

pub async fn get_user_rating(pool: &PgPool, term_id: &str, user_did: &str) -> crate::Result<Option<i16>> {
    let row: Option<(i16,)> = sqlx::query_as(
        "SELECT rating FROM term_ratings WHERE term_id = $1 AND user_did = $2"
    ).bind(term_id).bind(user_did).fetch_optional(pool).await?;
    Ok(row.map(|r| r.0))
}

// ── Reviews ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TermReviewRow {
    pub repo_uri: String,
    pub source_path: String,
    pub at_uri: Option<String>,
    pub title: String,
    pub summary: String,
    pub author_did: String,
    pub author_handle: Option<String>,
    pub author_display_name: Option<String>,
    /// Optional iteration tag. On per-term listings this echoes the path
    /// param; on course-level listings it lets the UI render an
    /// "took in {semester}" chip alongside each row.
    #[sqlx(default)]
    pub term_id: Option<String>,
    /// Joined `terms.semester` (e.g. "Fall 2017") when `term_id` is set —
    /// purely for chip rendering. Null on rows whose term has been
    /// detached or had its semester cleared.
    #[sqlx(default)]
    pub term_semester: Option<String>,
    #[sqlx(default)]
    pub term_session_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub vote_score: i64,
    pub comment_count: i64,
}

pub async fn list_term_articles_by_category(
    pool: &PgPool, term_id: &str, category: &str, limit: i64, offset: i64,
) -> crate::Result<Vec<TermReviewRow>> {
    Ok(sqlx::query_as::<_, TermReviewRow>(
        "SELECT a.repo_uri, a.source_path, l.at_uri, l.title, l.summary, a.author_did, \
         p.handle AS author_handle, p.display_name AS author_display_name, \
         a.term_id, t.semester AS term_semester, a.term_session_id, a.created_at, \
         COALESCE((SELECT SUM(value) FROM votes \
                   WHERE target_uri = article_uri(a.repo_uri, a.source_path)), 0) AS vote_score, \
         (SELECT COUNT(*) FROM comments \
          WHERE content_uri = article_uri(a.repo_uri, a.source_path)) AS comment_count \
         FROM articles a \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
            AND l.file_path = a.source_path \
         LEFT JOIN profiles p ON p.did = a.author_did \
         LEFT JOIN terms t ON t.id = a.term_id \
         WHERE a.term_id = $1 AND a.category = $2 \
         ORDER BY vote_score DESC, a.created_at DESC \
         LIMIT $3 OFFSET $4"
    ).bind(term_id).bind(category).bind(limit).bind(offset).fetch_all(pool).await?)
}

/// List reviews/notes anchored to an umbrella course (across every
/// iteration). Mirrors `list_term_articles_by_category` but keys on
/// `articles.course_id` so the course-detail page can surface every
/// review/note in one query. The optional iteration tag (`term_id` +
/// joined `term_semester`) lets the UI render a per-row chip.
pub async fn list_course_articles_by_category(
    pool: &PgPool, course_id: &str, category: &str, limit: i64, offset: i64,
) -> crate::Result<Vec<TermReviewRow>> {
    Ok(sqlx::query_as::<_, TermReviewRow>(
        "SELECT a.repo_uri, a.source_path, l.at_uri, l.title, l.summary, a.author_did, \
         p.handle AS author_handle, p.display_name AS author_display_name, \
         a.term_id, t.semester AS term_semester, a.term_session_id, a.created_at, \
         COALESCE((SELECT SUM(value) FROM votes \
                   WHERE target_uri = article_uri(a.repo_uri, a.source_path)), 0) AS vote_score, \
         (SELECT COUNT(*) FROM comments \
          WHERE content_uri = article_uri(a.repo_uri, a.source_path)) AS comment_count \
         FROM articles a \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
            AND l.file_path = a.source_path \
         LEFT JOIN profiles p ON p.did = a.author_did \
         LEFT JOIN terms t ON t.id = a.term_id \
         WHERE a.course_id = $1 AND a.category = $2 \
         ORDER BY vote_score DESC, a.created_at DESC \
         LIMIT $3 OFFSET $4"
    ).bind(course_id).bind(category).bind(limit).bind(offset).fetch_all(pool).await?)
}

pub async fn count_course_articles_by_category(
    pool: &PgPool, course_id: &str, category: &str,
) -> crate::Result<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM articles WHERE course_id = $1 AND category = $2",
    ).bind(course_id).bind(category).fetch_one(pool).await?)
}

pub async fn count_term_articles_by_category(
    pool: &PgPool, term_id: &str, category: &str,
) -> crate::Result<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM articles WHERE term_id = $1 AND category = $2",
    ).bind(term_id).bind(category).fetch_one(pool).await?)
}

// ── Learning status & session progress ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TermLearningStatus {
    pub term_id: String,
    pub user_did: String,
    pub status: String,
    pub progress: i16,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SessionProgress {
    pub term_id: String,
    pub session_id: String,
    pub user_did: String,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

pub async fn get_learning_status(
    pool: &PgPool, term_id: &str, user_did: &str,
) -> crate::Result<Option<TermLearningStatus>> {
    Ok(sqlx::query_as::<_, TermLearningStatus>(
        "SELECT * FROM term_learning_status WHERE term_id = $1 AND user_did = $2",
    ).bind(term_id).bind(user_did).fetch_optional(pool).await?)
}

pub async fn set_learning_status(
    pool: &PgPool, term_id: &str, user_did: &str, status: &str,
) -> crate::Result<TermLearningStatus> {
    // Recompute progress from session completion counts so it can't drift
    // from whatever the user has already ticked off.
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM term_sessions WHERE term_id = $1"
    ).bind(term_id).fetch_one(pool).await?;
    let done: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM term_session_progress \
         WHERE term_id = $1 AND user_did = $2 AND completed = TRUE"
    ).bind(term_id).bind(user_did).fetch_one(pool).await?;
    let progress: i16 = if total > 0 { ((done * 100 / total) as i16).clamp(0, 100) } else { 0 };

    sqlx::query(
        "INSERT INTO term_learning_status (term_id, user_did, status, progress) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (term_id, user_did) DO UPDATE SET status = $3, progress = $4, updated_at = NOW()",
    ).bind(term_id).bind(user_did).bind(status).bind(progress)
    .execute(pool).await?;

    Ok(get_learning_status(pool, term_id, user_did).await?.expect("just inserted"))
}

pub async fn remove_learning_status(
    pool: &PgPool, term_id: &str, user_did: &str,
) -> crate::Result<()> {
    sqlx::query("DELETE FROM term_learning_status WHERE term_id = $1 AND user_did = $2")
        .bind(term_id).bind(user_did).execute(pool).await?;
    Ok(())
}

pub async fn list_session_progress(
    pool: &PgPool, term_id: &str, user_did: &str,
) -> crate::Result<Vec<SessionProgress>> {
    Ok(sqlx::query_as::<_, SessionProgress>(
        "SELECT * FROM term_session_progress WHERE term_id = $1 AND user_did = $2",
    ).bind(term_id).bind(user_did).fetch_all(pool).await?)
}

/// Upsert a session's completion for `user_did`. Mirrors book chapter
/// progress: when a session is first completed we auto-transition the
/// course learning status out of idle states (empty, want_to_learn,
/// dropped) into `learning`; progress % is recomputed from the session
/// completion count in the same transaction.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TermResource {
    pub id: String,
    pub term_id: String,
    pub kind: String,
    pub label: String,
    pub url: String,
    pub position: i16,
}

pub async fn list_term_resources(pool: &PgPool, term_id: &str) -> crate::Result<Vec<TermResource>> {
    Ok(sqlx::query_as::<_, TermResource>(
        "SELECT id, term_id, kind, label, url, position \
         FROM term_resources WHERE term_id = $1 ORDER BY position, created_at"
    ).bind(term_id).fetch_all(pool).await?)
}

pub async fn add_term_resource(
    pool: &PgPool, term_id: &str, kind: &str, label: &str, url: &str,
    position: i16, created_by: &str,
) -> crate::Result<String> {
    let id: String = sqlx::query_scalar(
        "INSERT INTO term_resources (term_id, kind, label, url, position, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id"
    ).bind(term_id).bind(kind).bind(label).bind(url).bind(position).bind(created_by)
    .fetch_one(pool).await?;
    Ok(id)
}

pub async fn delete_term_resource(pool: &PgPool, id: &str) -> crate::Result<bool> {
    let result = sqlx::query("DELETE FROM term_resources WHERE id = $1")
        .bind(id).execute(pool).await?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_session_progress(
    pool: &PgPool, term_id: &str, session_id: &str, user_did: &str, completed: bool,
) -> crate::Result<Option<TermLearningStatus>> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO term_session_progress (term_id, session_id, user_did, completed, completed_at) \
         VALUES ($1, $2, $3, $4, CASE WHEN $4 THEN NOW() ELSE NULL END) \
         ON CONFLICT (session_id, user_did) DO UPDATE SET completed = $4, \
         completed_at = CASE WHEN $4 THEN COALESCE(term_session_progress.completed_at, NOW()) ELSE NULL END",
    ).bind(term_id).bind(session_id).bind(user_did).bind(completed)
    .execute(&mut *tx).await?;

    if completed {
        sqlx::query(
            "INSERT INTO term_learning_status (term_id, user_did, status, progress) \
             VALUES ($1, $2, 'learning', 0) \
             ON CONFLICT (term_id, user_did) DO UPDATE SET \
               status = CASE WHEN term_learning_status.status IN ('want_to_learn', 'dropped') \
                             THEN 'learning' ELSE term_learning_status.status END, \
               updated_at = NOW()",
        ).bind(term_id).bind(user_did)
        .execute(&mut *tx).await?;
    }

    let exists: Option<String> = sqlx::query_scalar(
        "SELECT status FROM term_learning_status WHERE term_id = $1 AND user_did = $2"
    ).bind(term_id).bind(user_did).fetch_optional(&mut *tx).await?;

    if exists.is_some() {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM term_sessions WHERE term_id = $1"
        ).bind(term_id).fetch_one(&mut *tx).await?;
        let done: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM term_session_progress \
             WHERE term_id = $1 AND user_did = $2 AND completed = TRUE"
        ).bind(term_id).bind(user_did).fetch_one(&mut *tx).await?;
        let progress: i16 = if total > 0 { ((done * 100 / total) as i16).clamp(0, 100) } else { 0 };
        sqlx::query(
            "UPDATE term_learning_status SET progress = $1, updated_at = NOW() \
             WHERE term_id = $2 AND user_did = $3"
        ).bind(progress).bind(term_id).bind(user_did).execute(&mut *tx).await?;
    }

    let status = sqlx::query_as::<_, TermLearningStatus>(
        "SELECT * FROM term_learning_status WHERE term_id = $1 AND user_did = $2"
    ).bind(term_id).bind(user_did).fetch_optional(&mut *tx).await?;

    tx.commit().await?;
    Ok(status)
}
