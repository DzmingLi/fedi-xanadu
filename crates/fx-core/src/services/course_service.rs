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
    pub is_published: bool,
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
    pub is_published: bool,
    pub series_count: i64,
    pub staff_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseSeriesRow {
    pub series_id: String,
    pub title: String,
    pub description: Option<String>,
    pub role: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseStaffRow {
    pub user_did: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CourseDetailResponse {
    pub course: CourseRow,
    pub syllabus: String,
    pub schedule: Vec<CourseSession>,
    pub textbooks: Vec<CourseTextbookRow>,
    pub tags: Vec<CourseTagRow>,
    pub series: Vec<CourseSeriesRow>,
    pub staff: Vec<CourseStaffRow>,
    pub skill_trees: Vec<CourseSkillTreeRow>,
    pub prerequisites: Vec<CoursePrereqRow>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseTagRow {
    pub tag_id: String,
    pub tag_name: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseTextbookRow {
    pub book_id: String,
    pub title: String,
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
pub struct CourseSession {
    pub session: i32,
    pub topic: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub video_url: Option<String>,
    #[serde(default)]
    pub notes_url: Option<String>,
    #[serde(default)]
    pub assignment_url: Option<String>,
    #[serde(default)]
    pub readings: Option<String>,
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
}

#[derive(Debug, Deserialize)]
pub struct UpdateCourse {
    pub title: Option<String>,
    pub code: Option<String>,
    pub description: Option<String>,
    pub syllabus: Option<String>,
    pub schedule: Option<Vec<CourseSession>>,
    pub institution: Option<String>,
    pub department: Option<String>,
    pub semester: Option<String>,
    pub lang: Option<String>,
    pub license: Option<String>,
    pub source_url: Option<String>,
    pub source_attribution: Option<String>,
    pub is_published: Option<bool>,
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

    get_course(pool, id).await
}

pub async fn get_course(pool: &PgPool, id: &str) -> crate::Result<CourseRow> {
    sqlx::query_as::<_, CourseRow>(
        "SELECT id, did, title, code, description, institution, department, semester, \
         lang, license, source_url, source_attribution, is_published, created_at, updated_at \
         FROM courses WHERE id = $1",
    )
    .bind(id).fetch_one(pool).await
    .map_err(|_| crate::Error::NotFound { entity: "course", id: id.to_string() })
}

pub async fn get_course_detail(pool: &PgPool, id: &str) -> crate::Result<CourseDetailResponse> {
    let course = get_course(pool, id).await?;

    let syllabus: String = sqlx::query_scalar("SELECT syllabus FROM courses WHERE id = $1")
        .bind(id).fetch_one(pool).await.unwrap_or_default();

    let schedule_json: serde_json::Value = sqlx::query_scalar("SELECT schedule FROM courses WHERE id = $1")
        .bind(id).fetch_one(pool).await.unwrap_or(serde_json::json!([]));
    let schedule: Vec<CourseSession> = serde_json::from_value(schedule_json).unwrap_or_default();

    let tags = sqlx::query_as::<_, CourseTagRow>(
        "SELECT ct.tag_id, t.name AS tag_name \
         FROM course_tags ct JOIN tags t ON t.id = ct.tag_id \
         WHERE ct.course_id = $1 ORDER BY t.name",
    ).bind(id).fetch_all(pool).await?;

    let textbooks = sqlx::query_as::<_, CourseTextbookRow>(
        "SELECT ct.book_id, b.title, b.authors, b.cover_url, ct.role, ct.sort_order \
         FROM course_textbooks ct JOIN books b ON b.id = ct.book_id \
         WHERE ct.course_id = $1 ORDER BY ct.sort_order",
    ).bind(id).fetch_all(pool).await?;

    let series = sqlx::query_as::<_, CourseSeriesRow>(
        "SELECT cs.series_id, s.title, s.description, cs.role, cs.sort_order \
         FROM course_series cs JOIN series s ON s.id = cs.series_id \
         WHERE cs.course_id = $1 ORDER BY cs.sort_order",
    ).bind(id).fetch_all(pool).await?;

    let staff = sqlx::query_as::<_, CourseStaffRow>(
        "SELECT cs.user_did, p.handle, p.display_name, p.avatar_url, cs.role, cs.sort_order \
         FROM course_staff cs LEFT JOIN profiles p ON p.did = cs.user_did \
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

    Ok(CourseDetailResponse { course, syllabus, schedule, textbooks, tags, series, staff, skill_trees, prerequisites })
}

pub async fn list_courses(pool: &PgPool) -> crate::Result<Vec<CourseListRow>> {
    Ok(sqlx::query_as::<_, CourseListRow>(
        "SELECT c.id, c.did, p.handle AS author_handle, c.title, c.code, c.description, \
         c.institution, c.semester, c.lang, c.is_published, \
         (SELECT COUNT(*) FROM course_series WHERE course_id = c.id) AS series_count, \
         (SELECT COUNT(*) FROM course_staff WHERE course_id = c.id) AS staff_count, \
         c.created_at \
         FROM courses c LEFT JOIN profiles p ON c.did = p.did \
         WHERE c.is_published = true \
         ORDER BY c.created_at DESC",
    ).fetch_all(pool).await?)
}

pub async fn list_my_courses(pool: &PgPool, did: &str) -> crate::Result<Vec<CourseListRow>> {
    Ok(sqlx::query_as::<_, CourseListRow>(
        "SELECT c.id, c.did, p.handle AS author_handle, c.title, c.code, c.description, \
         c.institution, c.semester, c.lang, c.is_published, \
         (SELECT COUNT(*) FROM course_series WHERE course_id = c.id) AS series_count, \
         (SELECT COUNT(*) FROM course_staff WHERE course_id = c.id) AS staff_count, \
         c.created_at \
         FROM courses c LEFT JOIN profiles p ON c.did = p.did \
         WHERE c.did = $1 \
         ORDER BY c.created_at DESC",
    ).bind(did).fetch_all(pool).await?)
}

pub async fn update_course(pool: &PgPool, id: &str, did: &str, input: &UpdateCourse) -> crate::Result<CourseRow> {
    let owner: Option<String> = sqlx::query_scalar("SELECT did FROM courses WHERE id = $1")
        .bind(id).fetch_optional(pool).await?;
    match owner {
        Some(ref d) if d != did => return Err(crate::Error::Forbidden { action: "edit course" }),
        None => return Err(crate::Error::NotFound { entity: "course", id: id.to_string() }),
        _ => {}
    }

    // Fetch current values, merge with input
    let cur = get_course(pool, id).await?;
    let schedule_json = input.schedule.as_ref()
        .map(|s| serde_json::to_value(s).unwrap_or(serde_json::json!([])));

    sqlx::query(
        "UPDATE courses SET \
         title = $1, code = $2, description = $3, syllabus = COALESCE($4, syllabus), \
         institution = $5, department = $6, semester = $7, lang = $8, license = $9, \
         source_url = $10, source_attribution = $11, is_published = $12, \
         schedule = COALESCE($13, schedule), updated_at = NOW() \
         WHERE id = $14",
    )
    .bind(input.title.as_deref().unwrap_or(&cur.title))
    .bind(input.code.as_ref().or(cur.code.as_ref()))
    .bind(input.description.as_deref().unwrap_or(&cur.description))
    .bind(input.syllabus.as_deref())
    .bind(input.institution.as_ref().or(cur.institution.as_ref()))
    .bind(input.department.as_ref().or(cur.department.as_ref()))
    .bind(input.semester.as_ref().or(cur.semester.as_ref()))
    .bind(input.lang.as_deref().unwrap_or(&cur.lang))
    .bind(input.license.as_deref().unwrap_or(&cur.license))
    .bind(input.source_url.as_ref().or(cur.source_url.as_ref()))
    .bind(input.source_attribution.as_ref().or(cur.source_attribution.as_ref()))
    .bind(input.is_published.unwrap_or(cur.is_published))
    .bind(schedule_json)
    .bind(id)
    .execute(pool).await?;

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

pub async fn add_staff(pool: &PgPool, course_id: &str, user_did: &str, role: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO course_staff (course_id, user_did, role) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"
    ).bind(course_id).bind(user_did).bind(role)
    .execute(pool).await?;
    Ok(())
}

pub async fn remove_staff(pool: &PgPool, course_id: &str, user_did: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM course_staff WHERE course_id = $1 AND user_did = $2")
        .bind(course_id).bind(user_did).execute(pool).await?;
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
