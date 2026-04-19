use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// ── Row types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseRow {
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
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseListRow {
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
    /// `course_authors`. Displayed on cards instead of the uploader handle.
    #[sqlx(default)]
    pub author_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseSeriesRow {
    pub series_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub role: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CourseDetailResponse {
    pub course: CourseRow,
    pub syllabus: String,
    pub authors: Vec<crate::services::author_service::Author>,
    pub sessions: Vec<CourseSessionDetail>,
    pub textbooks: Vec<CourseTextbookRow>,
    pub tags: Vec<CourseTagRow>,
    pub series: Vec<CourseSeriesRow>,
    pub skill_trees: Vec<CourseSkillTreeRow>,
    pub prerequisites: Vec<CoursePrereqRow>,
    pub rating: CourseRatingStats,
    pub reviews: Vec<CourseReviewRow>,
    pub review_count: i64,
    pub notes: Vec<CourseReviewRow>,
    pub note_count: i64,
    pub discussions: Vec<crate::models::Comment>,
    pub discussion_count: i64,
    /// The current viewer's rating for this course (1-10), if any.
    pub my_rating: Option<i16>,
    /// The current viewer's learning status + progress % for this course.
    pub my_learning_status: Option<CourseLearningStatus>,
    /// The current viewer's per-session completion records.
    pub my_session_progress: Vec<SessionProgress>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseTagRow {
    pub tag_id: String,
    pub tag_name: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseTextbookRow {
    pub book_id: String,
    pub title: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub authors: Vec<String>,
    pub cover_url: Option<String>,
    pub role: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseSkillTreeRow {
    pub tree_uri: String,
    pub title: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CoursePrereqRow {
    pub prereq_course_id: String,
    pub title: String,
    pub code: Option<String>,
    pub institution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResource {
    pub r#type: String,
    pub url: String,
    pub label: String,
}

/// A single study/lecture material entry. `kind` is an optional hint used
/// by the UI to pick an icon (reading, slides, handout, summary, notes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kind: Option<String>,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseSessionRow {
    pub id: String,
    pub course_id: String,
    pub sort_order: i32,
    pub topic: Option<String>,
    pub date: Option<String>,
    #[sqlx(default)]
    pub materials: sqlx::types::Json<Vec<Material>>,
    pub resources: sqlx::types::Json<Vec<SessionResource>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CourseSessionDetail {
    #[serde(flatten)]
    pub session: CourseSessionRow,
    pub tags: Vec<CourseTagRow>,
    pub prereqs: Vec<CourseTagRow>,
}

// ── Input types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateCourse {
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
    /// until claimed) and links them via course_authors.
    #[serde(default)]
    pub authors: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCourse {
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

pub async fn create_course(pool: &PgPool, id: &str, did: &str, input: &CreateCourse) -> crate::Result<CourseRow> {
    let lang = input.lang.as_deref().unwrap_or("zh");
    let license = input.license.as_deref().unwrap_or("CC-BY-SA-4.0");

    sqlx::query(
        "INSERT INTO courses (id, did, title, code, description, syllabus, institution, department, \
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
            "INSERT INTO course_authors (course_id, author_id, position, role) \
             VALUES ($1, $2, $3, 'instructor') ON CONFLICT DO NOTHING",
        )
        .bind(id).bind(&author_id).bind(position as i16)
        .execute(pool).await?;
    }

    get_course(pool, id).await
}

/// Replace the course's author list. Clears course_authors and re-links.
pub async fn set_course_authors(pool: &PgPool, course_id: &str, names: &[String]) -> crate::Result<()> {
    sqlx::query("DELETE FROM course_authors WHERE course_id = $1")
        .bind(course_id).execute(pool).await?;
    for (position, name) in names.iter().enumerate() {
        let author_id = super::author_service::get_or_create_author(pool, name).await?;
        sqlx::query(
            "INSERT INTO course_authors (course_id, author_id, position, role) \
             VALUES ($1, $2, $3, 'instructor') ON CONFLICT DO NOTHING",
        )
        .bind(course_id).bind(&author_id).bind(position as i16)
        .execute(pool).await?;
    }
    Ok(())
}

pub async fn list_course_authors(pool: &PgPool, course_id: &str) -> crate::Result<Vec<crate::services::author_service::Author>> {
    let rows = sqlx::query_as::<_, crate::services::author_service::Author>(
        "SELECT a.id, a.name, a.did, a.orcid, a.affiliation, a.homepage \
         FROM course_authors ca JOIN authors a ON a.id = ca.author_id \
         WHERE ca.course_id = $1 ORDER BY ca.position",
    )
    .bind(course_id).fetch_all(pool).await?;
    Ok(rows)
}

pub async fn get_course(pool: &PgPool, id: &str) -> crate::Result<CourseRow> {
    sqlx::query_as::<_, CourseRow>(
        "SELECT id, did, title, code, description, institution, department, semester, \
         lang, license, source_url, source_attribution, created_at, updated_at \
         FROM courses WHERE id = $1",
    )
    .bind(id).fetch_one(pool).await
    .map_err(|_| crate::Error::NotFound { entity: "course", id: id.to_string() })
}

pub async fn get_course_detail(pool: &PgPool, id: &str, viewer_did: Option<&str>) -> crate::Result<CourseDetailResponse> {
    let course = get_course(pool, id).await?;

    let syllabus: String = sqlx::query_scalar("SELECT syllabus FROM courses WHERE id = $1")
        .bind(id).fetch_one(pool).await.unwrap_or_default();

    // Fetch sessions with their tags and prereqs
    let session_rows = sqlx::query_as::<_, CourseSessionRow>(
        "SELECT id, course_id, sort_order, topic, date, materials, resources \
         FROM course_sessions WHERE course_id = $1 ORDER BY sort_order",
    ).bind(id).fetch_all(pool).await?;

    let mut sessions = Vec::with_capacity(session_rows.len());
    for row in session_rows {
        let tags = sqlx::query_as::<_, CourseTagRow>(
            "SELECT cst.tag_id, t.name AS tag_name \
             FROM course_session_tags cst JOIN tags t ON t.id = cst.tag_id \
             WHERE cst.session_id = $1 ORDER BY t.name",
        ).bind(&row.id).fetch_all(pool).await?;

        let prereqs = sqlx::query_as::<_, CourseTagRow>(
            "SELECT csp.tag_id, t.name AS tag_name \
             FROM course_session_prereqs csp JOIN tags t ON t.id = csp.tag_id \
             WHERE csp.session_id = $1 ORDER BY t.name",
        ).bind(&row.id).fetch_all(pool).await?;

        sessions.push(CourseSessionDetail { session: row, tags, prereqs });
    }

    let tags = sqlx::query_as::<_, CourseTagRow>(
        "SELECT ct.tag_id, t.name AS tag_name \
         FROM course_tags ct JOIN tags t ON t.id = ct.tag_id \
         WHERE ct.course_id = $1 ORDER BY t.name",
    ).bind(id).fetch_all(pool).await?;

    let textbooks = sqlx::query_as::<_, CourseTextbookRow>(
        "SELECT ct.book_id, b.title, b.authors, \
         (SELECT e.cover_url FROM book_editions e WHERE e.book_id = b.id AND e.cover_url IS NOT NULL LIMIT 1) AS cover_url, \
         ct.role, ct.sort_order \
         FROM course_textbooks ct JOIN books b ON b.id = ct.book_id \
         WHERE ct.course_id = $1 ORDER BY ct.sort_order",
    ).bind(id).fetch_all(pool).await?;

    let series = sqlx::query_as::<_, CourseSeriesRow>(
        "SELECT cs.series_id, s.title, s.summary, cs.role, cs.sort_order \
         FROM course_series cs JOIN series s ON s.id = cs.series_id \
         WHERE cs.course_id = $1 ORDER BY cs.sort_order",
    ).bind(id).fetch_all(pool).await?;

    let skill_trees = sqlx::query_as::<_, CourseSkillTreeRow>(
        "SELECT cst.tree_uri, st.title, cst.role \
         FROM course_skill_trees cst JOIN skill_trees st ON st.at_uri = cst.tree_uri \
         WHERE cst.course_id = $1",
    ).bind(id).fetch_all(pool).await?;

    let prerequisites = sqlx::query_as::<_, CoursePrereqRow>(
        "SELECT cp.prereq_course_id, c.title, c.code, c.institution \
         FROM course_prerequisites cp JOIN courses c ON c.id = cp.prereq_course_id \
         WHERE cp.course_id = $1",
    ).bind(id).fetch_all(pool).await?;

    let rating = get_rating_stats(pool, id).await?;
    let reviews = list_course_articles_by_category(pool, id, "review", 5, 0).await?;
    let review_count = count_course_articles_by_category(pool, id, "review").await?;
    let notes = list_course_articles_by_category(pool, id, "note", 5, 0).await?;
    let note_count = count_course_articles_by_category(pool, id, "note").await?;

    let course_uri = format!("course:{id}");
    let discussions = crate::services::comment_service::list_top_comments(pool, &course_uri, 5, 0).await?;
    let discussion_count = crate::services::comment_service::count_top_comments(pool, &course_uri).await?;

    let authors = list_course_authors(pool, id).await?;

    let (my_rating, my_learning_status, my_session_progress) = if let Some(did) = viewer_did {
        (
            get_user_rating(pool, id, did).await?,
            get_learning_status(pool, id, did).await?,
            list_session_progress(pool, id, did).await?,
        )
    } else {
        (None, None, vec![])
    };

    Ok(CourseDetailResponse {
        course, syllabus, authors, sessions, textbooks, tags, series, skill_trees, prerequisites,
        rating, reviews, review_count, notes, note_count, discussions, discussion_count,
        my_rating, my_learning_status, my_session_progress,
    })
}

const COURSE_LIST_SELECT: &str = "\
    SELECT c.id, c.did, p.handle AS author_handle, c.title, c.code, c.description, \
    c.institution, c.semester, c.lang, \
    (SELECT COUNT(*) FROM course_series WHERE course_id = c.id) AS series_count, \
    (SELECT COUNT(*) FROM course_sessions WHERE course_id = c.id) AS session_count, \
    COALESCE(r.avg, 0) AS avg_rating, \
    COALESCE(r.cnt, 0) AS rating_count, \
    c.created_at, \
    COALESCE( \
      (SELECT array_agg(a.name ORDER BY ca.position NULLS LAST) \
       FROM course_authors ca JOIN authors a ON a.id = ca.author_id \
       WHERE ca.course_id = c.id), \
      ARRAY[]::text[]) AS author_names \
    FROM courses c \
    LEFT JOIN profiles p ON c.did = p.did \
    LEFT JOIN (SELECT course_id, AVG(rating)::float8 AS avg, COUNT(*) AS cnt FROM course_ratings GROUP BY course_id) r ON r.course_id = c.id";

pub async fn list_courses(pool: &PgPool) -> crate::Result<Vec<CourseListRow>> {
    Ok(sqlx::query_as::<_, CourseListRow>(
        &format!("{COURSE_LIST_SELECT} ORDER BY COALESCE(r.avg, 0) * LN(COALESCE(r.cnt, 0) + 1) DESC, c.created_at DESC"),
    ).fetch_all(pool).await?)
}

pub async fn list_my_courses(pool: &PgPool, did: &str) -> crate::Result<Vec<CourseListRow>> {
    Ok(sqlx::query_as::<_, CourseListRow>(
        &format!("{COURSE_LIST_SELECT} WHERE c.did = $1 ORDER BY c.created_at DESC"),
    ).bind(did).fetch_all(pool).await?)
}

/// Snapshot a course as a flat JSON object for diffing.
fn course_to_json(c: &CourseRow, syllabus: &str) -> serde_json::Value {
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

pub async fn update_course(pool: &PgPool, id: &str, did: &str, input: &UpdateCourse, summary: &str) -> crate::Result<CourseRow> {
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM courses WHERE id = $1")
        .bind(id).fetch_optional(pool).await?;
    let owner_did = match owner {
        Some(d) => d,
        None => return Err(crate::Error::NotFound { entity: "course", id: id.to_string() }),
    };

    // Snapshot old state
    let cur = get_course(pool, id).await?;
    let old_syllabus: String = sqlx::query_scalar("SELECT syllabus FROM courses WHERE id = $1")
        .bind(id).fetch_one(pool).await.unwrap_or_default();
    let old_json = course_to_json(&cur, &old_syllabus);

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

    // Compute diff
    let ops = super::patch_service::diff(&old_json, &new_json);
    if ops.is_empty() {
        return Ok(cur); // no changes
    }

    // Record patch
    let is_owner = did == owner_did;
    let _patch = super::patch_service::create_patch(
        pool, "course", id, did, &owner_did, &ops, summary,
    ).await?;

    // If auto-applied (owner edit), materialize the update
    if is_owner {
        sqlx::query(
            "UPDATE courses SET \
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

        if let Some(ref authors) = input.authors {
            set_course_authors(pool, id, authors).await?;
        }
    }

    get_course(pool, id).await
}

pub async fn delete_course(pool: &PgPool, id: &str, did: &str) -> crate::Result<()> {
    let result = sqlx::query("DELETE FROM courses WHERE id = $1 AND did = $2")
        .bind(id).bind(did).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "course", id: id.to_string() });
    }
    Ok(())
}

// ── Session management ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateSession {
    pub topic: Option<String>,
    pub date: Option<String>,
    #[serde(default)]
    pub materials: Vec<Material>,
    #[serde(default)]
    pub resources: Vec<SessionResource>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSession {
    pub topic: Option<String>,
    pub date: Option<String>,
    pub materials: Option<Vec<Material>>,
    pub resources: Option<Vec<SessionResource>>,
    pub sort_order: Option<i32>,
}

pub async fn create_session(pool: &PgPool, session_id: &str, course_id: &str, input: &CreateSession) -> crate::Result<CourseSessionRow> {
    let sort_order = match input.sort_order {
        Some(o) => o,
        None => {
            let max: Option<i32> = sqlx::query_scalar(
                "SELECT MAX(sort_order) FROM course_sessions WHERE course_id = $1"
            ).bind(course_id).fetch_one(pool).await?;
            max.unwrap_or(0) + 1
        }
    };

    sqlx::query(
        "INSERT INTO course_sessions (id, course_id, sort_order, topic, date, materials, resources) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(session_id).bind(course_id).bind(sort_order)
    .bind(&input.topic).bind(&input.date)
    .bind(sqlx::types::Json(&input.materials))
    .bind(sqlx::types::Json(&input.resources))
    .execute(pool).await?;

    get_session(pool, session_id).await
}

pub async fn get_session(pool: &PgPool, session_id: &str) -> crate::Result<CourseSessionRow> {
    sqlx::query_as::<_, CourseSessionRow>(
        "SELECT id, course_id, sort_order, topic, date, materials, resources \
         FROM course_sessions WHERE id = $1",
    )
    .bind(session_id).fetch_one(pool).await
    .map_err(|_| crate::Error::NotFound { entity: "session", id: session_id.to_string() })
}

pub async fn update_session(pool: &PgPool, session_id: &str, input: &UpdateSession) -> crate::Result<CourseSessionRow> {
    let cur = get_session(pool, session_id).await?;

    sqlx::query(
        "UPDATE course_sessions SET \
         topic = $1, date = $2, materials = $3, resources = $4, sort_order = $5 \
         WHERE id = $6",
    )
    .bind(input.topic.as_ref().or(cur.topic.as_ref()))
    .bind(input.date.as_ref().or(cur.date.as_ref()))
    .bind(sqlx::types::Json(input.materials.as_ref().unwrap_or(&cur.materials.0)))
    .bind(sqlx::types::Json(input.resources.as_ref().unwrap_or(&cur.resources.0)))
    .bind(input.sort_order.unwrap_or(cur.sort_order))
    .bind(session_id)
    .execute(pool).await?;

    get_session(pool, session_id).await
}

pub async fn delete_session(pool: &PgPool, session_id: &str) -> crate::Result<()> {
    let result = sqlx::query("DELETE FROM course_sessions WHERE id = $1")
        .bind(session_id).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "session", id: session_id.to_string() });
    }
    Ok(())
}

pub async fn add_session_tag(pool: &PgPool, session_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query("INSERT INTO course_session_tags (session_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(session_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

pub async fn remove_session_tag(pool: &PgPool, session_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM course_session_tags WHERE session_id = $1 AND tag_id = $2")
        .bind(session_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

pub async fn add_session_prereq(pool: &PgPool, session_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query("INSERT INTO course_session_prereqs (session_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(session_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

pub async fn remove_session_prereq(pool: &PgPool, session_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM course_session_prereqs WHERE session_id = $1 AND tag_id = $2")
        .bind(session_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

// ── Relation management ─────────────────────────────────────────────────

pub async fn add_series(pool: &PgPool, course_id: &str, series_id: &str, role: &str, sort_order: i32) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO course_series (course_id, series_id, role, sort_order) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING"
    ).bind(course_id).bind(series_id).bind(role).bind(sort_order)
    .execute(pool).await?;
    Ok(())
}

pub async fn remove_series(pool: &PgPool, course_id: &str, series_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM course_series WHERE course_id = $1 AND series_id = $2")
        .bind(course_id).bind(series_id).execute(pool).await?;
    Ok(())
}

pub async fn add_skill_tree(pool: &PgPool, course_id: &str, tree_uri: &str, role: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO course_skill_trees (course_id, tree_uri, role) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"
    ).bind(course_id).bind(tree_uri).bind(role)
    .execute(pool).await?;
    Ok(())
}

pub async fn add_prerequisite(pool: &PgPool, course_id: &str, prereq_id: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO course_prerequisites (course_id, prereq_course_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    ).bind(course_id).bind(prereq_id)
    .execute(pool).await?;
    Ok(())
}

pub async fn remove_prerequisite(pool: &PgPool, course_id: &str, prereq_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM course_prerequisites WHERE course_id = $1 AND prereq_course_id = $2")
        .bind(course_id).bind(prereq_id).execute(pool).await?;
    Ok(())
}

// ── Tags ────────────────────────────────────────────────────────────────

pub async fn add_tag(pool: &PgPool, course_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query("INSERT INTO course_tags (course_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(course_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

pub async fn remove_tag(pool: &PgPool, course_id: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM course_tags WHERE course_id = $1 AND tag_id = $2")
        .bind(course_id).bind(tag_id).execute(pool).await?;
    Ok(())
}

// ── Textbooks ───────────────────────────────────────────────────────────

pub async fn add_textbook(pool: &PgPool, course_id: &str, book_id: &str, role: &str, sort_order: i32) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO course_textbooks (course_id, book_id, role, sort_order) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING"
    ).bind(course_id).bind(book_id).bind(role).bind(sort_order)
    .execute(pool).await?;
    Ok(())
}

pub async fn remove_textbook(pool: &PgPool, course_id: &str, book_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM course_textbooks WHERE course_id = $1 AND book_id = $2")
        .bind(course_id).bind(book_id).execute(pool).await?;
    Ok(())
}

// ── Ratings ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CourseRatingStats {
    pub avg_rating: f64,
    pub rating_count: i64,
}

pub async fn rate_course(pool: &PgPool, course_id: &str, user_did: &str, rating: i16) -> crate::Result<CourseRatingStats> {
    sqlx::query(
        "INSERT INTO course_ratings (course_id, user_did, rating) VALUES ($1, $2, $3) \
         ON CONFLICT (course_id, user_did) DO UPDATE SET rating = $3, updated_at = NOW()"
    ).bind(course_id).bind(user_did).bind(rating)
    .execute(pool).await?;

    get_rating_stats(pool, course_id).await
}

pub async fn unrate_course(pool: &PgPool, course_id: &str, user_did: &str) -> crate::Result<CourseRatingStats> {
    sqlx::query("DELETE FROM course_ratings WHERE course_id = $1 AND user_did = $2")
        .bind(course_id).bind(user_did)
        .execute(pool).await?;
    get_rating_stats(pool, course_id).await
}

pub async fn get_rating_stats(pool: &PgPool, course_id: &str) -> crate::Result<CourseRatingStats> {
    let row: (Option<f64>, i64) = sqlx::query_as(
        "SELECT AVG(rating::float), COUNT(*) FROM course_ratings WHERE course_id = $1"
    ).bind(course_id).fetch_one(pool).await?;
    Ok(CourseRatingStats { avg_rating: row.0.unwrap_or(0.0), rating_count: row.1 })
}

pub async fn get_user_rating(pool: &PgPool, course_id: &str, user_did: &str) -> crate::Result<Option<i16>> {
    let row: Option<(i16,)> = sqlx::query_as(
        "SELECT rating FROM course_ratings WHERE course_id = $1 AND user_did = $2"
    ).bind(course_id).bind(user_did).fetch_optional(pool).await?;
    Ok(row.map(|r| r.0))
}

// ── Reviews ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseReviewRow {
    pub at_uri: String,
    pub title: String,
    pub summary: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub author_display_name: Option<String>,
    #[sqlx(default)]
    pub course_session_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub vote_score: i64,
    pub comment_count: i64,
}

pub async fn list_course_articles_by_category(
    pool: &PgPool, course_id: &str, category: &str, limit: i64, offset: i64,
) -> crate::Result<Vec<CourseReviewRow>> {
    Ok(sqlx::query_as::<_, CourseReviewRow>(
        "SELECT a.at_uri, a.title, a.summary, a.did, \
         p.handle AS author_handle, p.display_name AS author_display_name, \
         a.course_session_id, a.created_at, \
         COALESCE((SELECT SUM(value) FROM votes WHERE target_uri = a.at_uri), 0) AS vote_score, \
         (SELECT COUNT(*) FROM comments WHERE content_uri = a.at_uri) AS comment_count \
         FROM articles a \
         LEFT JOIN profiles p ON p.did = a.did \
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

// ── Learning status & session progress ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CourseLearningStatus {
    pub course_id: String,
    pub user_did: String,
    pub status: String,
    pub progress: i16,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SessionProgress {
    pub course_id: String,
    pub session_id: String,
    pub user_did: String,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

pub async fn get_learning_status(
    pool: &PgPool, course_id: &str, user_did: &str,
) -> crate::Result<Option<CourseLearningStatus>> {
    Ok(sqlx::query_as::<_, CourseLearningStatus>(
        "SELECT * FROM course_learning_status WHERE course_id = $1 AND user_did = $2",
    ).bind(course_id).bind(user_did).fetch_optional(pool).await?)
}

pub async fn set_learning_status(
    pool: &PgPool, course_id: &str, user_did: &str, status: &str,
) -> crate::Result<CourseLearningStatus> {
    // Recompute progress from session completion counts so it can't drift
    // from whatever the user has already ticked off.
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM course_sessions WHERE course_id = $1"
    ).bind(course_id).fetch_one(pool).await?;
    let done: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM course_session_progress \
         WHERE course_id = $1 AND user_did = $2 AND completed = TRUE"
    ).bind(course_id).bind(user_did).fetch_one(pool).await?;
    let progress: i16 = if total > 0 { ((done * 100 / total) as i16).clamp(0, 100) } else { 0 };

    sqlx::query(
        "INSERT INTO course_learning_status (course_id, user_did, status, progress) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (course_id, user_did) DO UPDATE SET status = $3, progress = $4, updated_at = NOW()",
    ).bind(course_id).bind(user_did).bind(status).bind(progress)
    .execute(pool).await?;

    Ok(get_learning_status(pool, course_id, user_did).await?.expect("just inserted"))
}

pub async fn remove_learning_status(
    pool: &PgPool, course_id: &str, user_did: &str,
) -> crate::Result<()> {
    sqlx::query("DELETE FROM course_learning_status WHERE course_id = $1 AND user_did = $2")
        .bind(course_id).bind(user_did).execute(pool).await?;
    Ok(())
}

pub async fn list_session_progress(
    pool: &PgPool, course_id: &str, user_did: &str,
) -> crate::Result<Vec<SessionProgress>> {
    Ok(sqlx::query_as::<_, SessionProgress>(
        "SELECT * FROM course_session_progress WHERE course_id = $1 AND user_did = $2",
    ).bind(course_id).bind(user_did).fetch_all(pool).await?)
}

/// Upsert a session's completion for `user_did`. Mirrors book chapter
/// progress: when a session is first completed we auto-transition the
/// course learning status out of idle states (empty, want_to_learn,
/// dropped) into `learning`; progress % is recomputed from the session
/// completion count in the same transaction.
pub async fn set_session_progress(
    pool: &PgPool, course_id: &str, session_id: &str, user_did: &str, completed: bool,
) -> crate::Result<Option<CourseLearningStatus>> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO course_session_progress (course_id, session_id, user_did, completed, completed_at) \
         VALUES ($1, $2, $3, $4, CASE WHEN $4 THEN NOW() ELSE NULL END) \
         ON CONFLICT (session_id, user_did) DO UPDATE SET completed = $4, \
         completed_at = CASE WHEN $4 THEN COALESCE(course_session_progress.completed_at, NOW()) ELSE NULL END",
    ).bind(course_id).bind(session_id).bind(user_did).bind(completed)
    .execute(&mut *tx).await?;

    if completed {
        sqlx::query(
            "INSERT INTO course_learning_status (course_id, user_did, status, progress) \
             VALUES ($1, $2, 'learning', 0) \
             ON CONFLICT (course_id, user_did) DO UPDATE SET \
               status = CASE WHEN course_learning_status.status IN ('want_to_learn', 'dropped') \
                             THEN 'learning' ELSE course_learning_status.status END, \
               updated_at = NOW()",
        ).bind(course_id).bind(user_did)
        .execute(&mut *tx).await?;
    }

    let exists: Option<String> = sqlx::query_scalar(
        "SELECT status FROM course_learning_status WHERE course_id = $1 AND user_did = $2"
    ).bind(course_id).bind(user_did).fetch_optional(&mut *tx).await?;

    if exists.is_some() {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM course_sessions WHERE course_id = $1"
        ).bind(course_id).fetch_one(&mut *tx).await?;
        let done: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM course_session_progress \
             WHERE course_id = $1 AND user_did = $2 AND completed = TRUE"
        ).bind(course_id).bind(user_did).fetch_one(&mut *tx).await?;
        let progress: i16 = if total > 0 { ((done * 100 / total) as i16).clamp(0, 100) } else { 0 };
        sqlx::query(
            "UPDATE course_learning_status SET progress = $1, updated_at = NOW() \
             WHERE course_id = $2 AND user_did = $3"
        ).bind(progress).bind(course_id).bind(user_did).execute(&mut *tx).await?;
    }

    let status = sqlx::query_as::<_, CourseLearningStatus>(
        "SELECT * FROM course_learning_status WHERE course_id = $1 AND user_did = $2"
    ).bind(course_id).bind(user_did).fetch_optional(&mut *tx).await?;

    tx.commit().await?;
    Ok(status)
}
