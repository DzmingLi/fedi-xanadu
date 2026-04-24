//! Groups of related course iterations — "CS229 (Autumn 2008)" and
//! hypothetical future "CS229 (Autumn 2018)" live in the `courses` table
//! but share a row here. Group pages cross-link iterations and host a
//! discussion thread; per-iteration details still live on `courses`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::services::course_service::CourseRow;
use crate::util::tid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourseGroup {
    pub id: String,
    pub title: String,
    pub code: Option<String>,
    pub institution: Option<String>,
    pub description: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCourseGroup {
    pub title: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub institution: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Response for the group detail page — the group row plus every course
/// iteration in it, newest semester first. The discussion thread lives on
/// content_uri = `coursegroup:<id>` and is fetched separately.
#[derive(Debug, Clone, Serialize)]
pub struct CourseGroupDetail {
    pub group: CourseGroup,
    pub courses: Vec<CourseRow>,
    pub discussions: Vec<crate::models::Comment>,
    pub discussion_count: i64,
}

pub async fn create_course_group(
    pool: &PgPool,
    input: &CreateCourseGroup,
    created_by: &str,
) -> crate::Result<CourseGroup> {
    let id = format!("cg-{}", tid());
    let description = input.description.clone().unwrap_or_default();
    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO course_groups (id, title, code, institution, description, created_by) \
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
    let uri = format!("coursegroup:{id}");
    sqlx::query("INSERT INTO content (uri, content_type) VALUES ($1, 'coursegroup') ON CONFLICT DO NOTHING")
        .bind(&uri)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    get_course_group(pool, &id).await
}

pub async fn get_course_group(pool: &PgPool, id: &str) -> crate::Result<CourseGroup> {
    sqlx::query_as::<_, CourseGroup>(
        "SELECT id, title, code, institution, description, created_by, created_at \
         FROM course_groups WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::Error::NotFound { entity: "course_group", id: id.to_string() })
}

pub async fn list_course_groups(pool: &PgPool) -> crate::Result<Vec<CourseGroup>> {
    Ok(sqlx::query_as::<_, CourseGroup>(
        "SELECT id, title, code, institution, description, created_by, created_at \
         FROM course_groups ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?)
}

/// Courses in a group, newest semester first. Sort is text-based on the
/// semester string ("Spring 2026" > "Autumn 2025" > "Spring 2025" works
/// because years dominate and seasons sort reasonably within a year; if
/// that fails, creation date breaks ties).
pub async fn list_courses_in_group(pool: &PgPool, group_id: &str) -> crate::Result<Vec<CourseRow>> {
    Ok(sqlx::query_as::<_, CourseRow>(
        "SELECT id, did, title, code, description, institution, department, semester, lang, license, \
                source_url, source_attribution, created_at, updated_at, group_id \
         FROM courses WHERE group_id = $1 \
         ORDER BY semester DESC NULLS LAST, created_at DESC",
    )
    .bind(group_id)
    .fetch_all(pool)
    .await?)
}

pub async fn set_course_group(pool: &PgPool, course_id: &str, group_id: Option<&str>) -> crate::Result<()> {
    let res = sqlx::query("UPDATE courses SET group_id = $1 WHERE id = $2")
        .bind(group_id)
        .bind(course_id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "course", id: course_id.to_string() });
    }
    Ok(())
}

pub async fn delete_course_group(pool: &PgPool, id: &str) -> crate::Result<()> {
    let mut tx = pool.begin().await?;
    let existed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM course_groups WHERE id = $1)")
        .bind(id).fetch_one(&mut *tx).await?;
    if !existed {
        return Err(crate::Error::NotFound { entity: "course_group", id: id.to_string() });
    }
    // courses.group_id → NULL via ON DELETE SET NULL.
    sqlx::query("DELETE FROM course_groups WHERE id = $1")
        .bind(id).execute(&mut *tx).await?;
    let uri = format!("coursegroup:{id}");
    sqlx::query("DELETE FROM content WHERE uri = $1")
        .bind(&uri).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn get_course_group_detail(pool: &PgPool, id: &str) -> crate::Result<CourseGroupDetail> {
    let group = get_course_group(pool, id).await?;
    let courses = list_courses_in_group(pool, id).await?;
    let uri = format!("coursegroup:{id}");
    let discussions = crate::services::comment_service::list_top_comments(pool, &uri, 5, 0).await?;
    let discussion_count = crate::services::comment_service::count_top_comments(pool, &uri).await?;
    Ok(CourseGroupDetail { group, courses, discussions, discussion_count })
}
