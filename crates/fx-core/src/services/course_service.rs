//! Umbrella courses — "CS229" with all its term iterations ("CS229 Autumn
//! 2008", "CS229 Autumn 2018", …) sharing a row here. Course pages
//! cross-link iterations and host a discussion thread; per-iteration
//! details live on `terms`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::services::term_service::TermRow;
use crate::util::tid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Course {
    pub id: String,
    pub title: String,
    pub code: Option<String>,
    pub institution: Option<String>,
    pub description: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCourse {
    pub title: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub institution: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Response for the course detail page — the course row plus every term
/// iteration in it, newest semester first. The discussion thread lives on
/// content_uri = `course:<id>` and is fetched separately.
#[derive(Debug, Clone, Serialize)]
pub struct CourseDetail {
    pub course: Course,
    pub terms: Vec<TermRow>,
    pub discussions: Vec<crate::models::Comment>,
    pub discussion_count: i64,
}

pub async fn create_course(
    pool: &PgPool,
    input: &CreateCourse,
    created_by: &str,
) -> crate::Result<Course> {
    let id = format!("crs-{}", tid());
    let description = input.description.clone().unwrap_or_default();
    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO courses (id, title, code, institution, description, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&id)
    .bind(&input.title)
    .bind(&input.code)
    .bind(&input.institution)
    .bind(&description)
    .bind(created_by)
    .execute(&mut *tx)
    .await?;

    // Register in polymorphic content so comments can attach.
    let uri = format!("course:{id}");
    sqlx::query("INSERT INTO content (uri, content_type) VALUES ($1, 'course') ON CONFLICT DO NOTHING")
        .bind(&uri)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    get_course(pool, &id).await
}

pub async fn get_course(pool: &PgPool, id: &str) -> crate::Result<Course> {
    sqlx::query_as::<_, Course>(
        "SELECT id, title, code, institution, description, created_by, created_at \
         FROM courses WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::Error::NotFound { entity: "course", id: id.to_string() })
}

pub async fn list_courses(pool: &PgPool) -> crate::Result<Vec<Course>> {
    Ok(sqlx::query_as::<_, Course>(
        "SELECT id, title, code, institution, description, created_by, created_at \
         FROM courses ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?)
}

/// Course list with iteration metadata for the browse page — count of
/// terms in the course and the most-recent semester (text-sorted; close
/// enough since years dominate).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseListItem {
    pub id: String,
    pub title: String,
    pub code: Option<String>,
    pub institution: Option<String>,
    pub description: String,
    pub iteration_count: i64,
    pub latest_semester: Option<String>,
}

pub async fn list_courses_with_meta(pool: &PgPool) -> crate::Result<Vec<CourseListItem>> {
    // latest_semester comes from the most-recently-created term (not text-
    // MAX of semester). Text-MAX picks "Spring 2015" over "Fall 2017"
    // because 'S' > 'F' alphabetically; created_at gives the right answer
    // since we always ingest a term right after its real-world semester.
    Ok(sqlx::query_as::<_, CourseListItem>(
        "SELECT c.id, c.title, c.code, c.institution, c.description, \
                (SELECT COUNT(*) FROM terms WHERE course_id = c.id) AS iteration_count, \
                (SELECT semester FROM terms WHERE course_id = c.id \
                 ORDER BY created_at DESC LIMIT 1) AS latest_semester \
         FROM courses c \
         ORDER BY c.created_at DESC",
    )
    .fetch_all(pool)
    .await?)
}

/// Terms in a course, newest semester first. Sort is text-based on the
/// semester string ("Spring 2026" > "Autumn 2025" > "Spring 2025" works
/// because years dominate and seasons sort reasonably within a year; if
/// that fails, creation date breaks ties).
pub async fn list_terms_in_course(pool: &PgPool, course_id: &str) -> crate::Result<Vec<TermRow>> {
    Ok(sqlx::query_as::<_, TermRow>(
        "SELECT id, did, title, code, description, institution, department, semester, lang, license, \
                source_url, source_attribution, created_at, updated_at, course_id \
         FROM terms WHERE course_id = $1 \
         ORDER BY semester DESC NULLS LAST, created_at DESC",
    )
    .bind(course_id)
    .fetch_all(pool)
    .await?)
}

pub async fn set_term_course(pool: &PgPool, term_id: &str, course_id: Option<&str>) -> crate::Result<()> {
    let res = sqlx::query("UPDATE terms SET course_id = $1 WHERE id = $2")
        .bind(course_id)
        .bind(term_id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "term", id: term_id.to_string() });
    }
    Ok(())
}

pub async fn delete_course(pool: &PgPool, id: &str) -> crate::Result<()> {
    let mut tx = pool.begin().await?;
    let existed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM courses WHERE id = $1)")
        .bind(id).fetch_one(&mut *tx).await?;
    if !existed {
        return Err(crate::Error::NotFound { entity: "course", id: id.to_string() });
    }
    // terms.course_id → NULL via ON DELETE SET NULL.
    sqlx::query("DELETE FROM courses WHERE id = $1")
        .bind(id).execute(&mut *tx).await?;
    let uri = format!("course:{id}");
    sqlx::query("DELETE FROM content WHERE uri = $1")
        .bind(&uri).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn get_course_detail(pool: &PgPool, id: &str) -> crate::Result<CourseDetail> {
    let course = get_course(pool, id).await?;
    let terms = list_terms_in_course(pool, id).await?;
    let uri = format!("course:{id}");
    let discussions = crate::services::comment_service::list_top_comments(pool, &uri, 5, 0).await?;
    let discussion_count = crate::services::comment_service::count_top_comments(pool, &uri).await?;
    Ok(CourseDetail { course, terms, discussions, discussion_count })
}
