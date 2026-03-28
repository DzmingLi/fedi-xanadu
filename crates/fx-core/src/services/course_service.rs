use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Course {
    pub id: String,
    pub title: String,
    pub description: String,
    pub instructor_did: String,
    pub cover_url: Option<String>,
    pub schedule_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CourseUnit {
    pub id: String,
    pub course_id: String,
    pub sort_order: i32,
    pub title: String,
    pub description: String,
    pub available_from: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CourseItem {
    pub id: String,
    pub unit_id: String,
    pub sort_order: i32,
    pub role: String,
    pub target_uri: Option<String>,
    pub external_url: Option<String>,
    pub title: String,
    pub note: String,
    pub due_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCourse {
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub schedule_type: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitWithItems {
    pub unit: CourseUnit,
    pub items: Vec<CourseItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseDetail {
    pub course: Course,
    pub units: Vec<UnitWithItems>,
}

// ---- CRUD ----

pub async fn create_course(
    pool: &PgPool,
    id: &str,
    input: &CreateCourse,
    instructor_did: &str,
) -> crate::Result<Course> {
    let mut tx = pool.begin().await?;

    let content_uri = format!("course:{id}");
    sqlx::query("INSERT INTO content (uri, content_type) VALUES ($1, 'course') ON CONFLICT DO NOTHING")
        .bind(&content_uri)
        .execute(&mut *tx).await?;

    let schedule = input.schedule_type.as_deref().unwrap_or("weekly");
    sqlx::query(
        "INSERT INTO courses (id, title, description, instructor_did, cover_url, schedule_type) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(&input.title)
    .bind(input.description.as_deref().unwrap_or(""))
    .bind(instructor_did)
    .bind(&input.cover_url)
    .bind(schedule)
    .execute(&mut *tx).await?;

    for tag_id in &input.tags {
        sqlx::query("INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING")
            .bind(tag_id).bind(tag_id).bind(instructor_did)
            .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(&content_uri).bind(tag_id)
            .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    sqlx::query_as::<_, Course>("SELECT * FROM courses WHERE id = $1")
        .bind(id).fetch_one(pool).await.map_err(Into::into)
}

pub async fn get_course(pool: &PgPool, id: &str) -> crate::Result<CourseDetail> {
    let course = sqlx::query_as::<_, Course>("SELECT * FROM courses WHERE id = $1")
        .bind(id).fetch_optional(pool).await?
        .ok_or_else(|| crate::Error::NotFound { entity: "course", id: id.to_string() })?;

    let units = sqlx::query_as::<_, CourseUnit>(
        "SELECT * FROM course_units WHERE course_id = $1 ORDER BY sort_order",
    ).bind(id).fetch_all(pool).await?;

    let unit_ids: Vec<&str> = units.iter().map(|u| u.id.as_str()).collect();
    let items = if unit_ids.is_empty() {
        vec![]
    } else {
        sqlx::query_as::<_, CourseItem>(
            "SELECT * FROM course_items WHERE unit_id = ANY($1) ORDER BY sort_order",
        ).bind(&unit_ids).fetch_all(pool).await?
    };

    let units_with_items = units.into_iter().map(|u| {
        let unit_items: Vec<CourseItem> = items.iter()
            .filter(|i| i.unit_id == u.id)
            .cloned()
            .collect();
        UnitWithItems { unit: u, items: unit_items }
    }).collect();

    Ok(CourseDetail { course, units: units_with_items })
}

pub async fn list_courses(pool: &PgPool, limit: i64, offset: i64) -> crate::Result<Vec<Course>> {
    sqlx::query_as::<_, Course>(
        "SELECT * FROM courses ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    ).bind(limit).bind(offset).fetch_all(pool).await.map_err(Into::into)
}

pub async fn update_course(
    pool: &PgPool, id: &str,
    title: Option<&str>, description: Option<&str>,
    cover_url: Option<&str>, schedule_type: Option<&str>,
) -> crate::Result<()> {
    if let Some(v) = title {
        sqlx::query("UPDATE courses SET title = $1, updated_at = NOW() WHERE id = $2")
            .bind(v).bind(id).execute(pool).await?;
    }
    if let Some(v) = description {
        sqlx::query("UPDATE courses SET description = $1, updated_at = NOW() WHERE id = $2")
            .bind(v).bind(id).execute(pool).await?;
    }
    if let Some(v) = cover_url {
        sqlx::query("UPDATE courses SET cover_url = $1, updated_at = NOW() WHERE id = $2")
            .bind(v).bind(id).execute(pool).await?;
    }
    if let Some(v) = schedule_type {
        sqlx::query("UPDATE courses SET schedule_type = $1, updated_at = NOW() WHERE id = $2")
            .bind(v).bind(id).execute(pool).await?;
    }
    Ok(())
}

pub async fn delete_course(pool: &PgPool, id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM content WHERE uri = $1")
        .bind(&format!("course:{id}")).execute(pool).await?;
    sqlx::query("DELETE FROM courses WHERE id = $1")
        .bind(id).execute(pool).await?;
    Ok(())
}

pub async fn get_course_owner(pool: &PgPool, id: &str) -> crate::Result<String> {
    sqlx::query_scalar::<_, String>("SELECT instructor_did FROM courses WHERE id = $1")
        .bind(id).fetch_optional(pool).await?
        .ok_or_else(|| crate::Error::NotFound { entity: "course", id: id.to_string() })
}

// ---- Units ----

pub async fn create_unit(
    pool: &PgPool, id: &str, course_id: &str, title: &str, description: &str, available_from: Option<NaiveDate>,
) -> crate::Result<CourseUnit> {
    let order: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(sort_order) FROM course_units WHERE course_id = $1",
    ).bind(course_id).fetch_one(pool).await?;

    sqlx::query(
        "INSERT INTO course_units (id, course_id, sort_order, title, description, available_from) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id).bind(course_id).bind(order.unwrap_or(-1) + 1)
    .bind(title).bind(description).bind(available_from)
    .execute(pool).await?;

    sqlx::query_as::<_, CourseUnit>("SELECT * FROM course_units WHERE id = $1")
        .bind(id).fetch_one(pool).await.map_err(Into::into)
}

pub async fn update_unit(
    pool: &PgPool, id: &str, title: Option<&str>, description: Option<&str>, available_from: Option<Option<NaiveDate>>,
) -> crate::Result<()> {
    if let Some(v) = title {
        sqlx::query("UPDATE course_units SET title = $1 WHERE id = $2")
            .bind(v).bind(id).execute(pool).await?;
    }
    if let Some(v) = description {
        sqlx::query("UPDATE course_units SET description = $1 WHERE id = $2")
            .bind(v).bind(id).execute(pool).await?;
    }
    if let Some(v) = available_from {
        sqlx::query("UPDATE course_units SET available_from = $1 WHERE id = $2")
            .bind(v).bind(id).execute(pool).await?;
    }
    Ok(())
}

pub async fn delete_unit(pool: &PgPool, id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM course_units WHERE id = $1").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn get_unit_course_id(pool: &PgPool, unit_id: &str) -> crate::Result<String> {
    sqlx::query_scalar::<_, String>("SELECT course_id FROM course_units WHERE id = $1")
        .bind(unit_id).fetch_optional(pool).await?
        .ok_or_else(|| crate::Error::NotFound { entity: "course_unit", id: unit_id.to_string() })
}

pub async fn reorder_units(pool: &PgPool, course_id: &str, unit_ids: &[String]) -> crate::Result<()> {
    for (i, uid) in unit_ids.iter().enumerate() {
        sqlx::query("UPDATE course_units SET sort_order = $1 WHERE id = $2 AND course_id = $3")
            .bind(i as i32).bind(uid).bind(course_id)
            .execute(pool).await?;
    }
    Ok(())
}

// ---- Items ----

pub async fn create_item(
    pool: &PgPool, id: &str, unit_id: &str,
    role: &str, target_uri: Option<&str>, external_url: Option<&str>,
    title: &str, note: &str, due_date: Option<NaiveDate>,
) -> crate::Result<CourseItem> {
    let order: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(sort_order) FROM course_items WHERE unit_id = $1",
    ).bind(unit_id).fetch_one(pool).await?;

    sqlx::query(
        "INSERT INTO course_items (id, unit_id, sort_order, role, target_uri, external_url, title, note, due_date) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(id).bind(unit_id).bind(order.unwrap_or(-1) + 1)
    .bind(role).bind(target_uri).bind(external_url)
    .bind(title).bind(note).bind(due_date)
    .execute(pool).await?;

    sqlx::query_as::<_, CourseItem>("SELECT * FROM course_items WHERE id = $1")
        .bind(id).fetch_one(pool).await.map_err(Into::into)
}

pub async fn update_item(
    pool: &PgPool, id: &str,
    role: Option<&str>, target_uri: Option<Option<&str>>, external_url: Option<Option<&str>>,
    title: Option<&str>, note: Option<&str>, due_date: Option<Option<NaiveDate>>,
) -> crate::Result<()> {
    if let Some(v) = role {
        sqlx::query("UPDATE course_items SET role = $1 WHERE id = $2").bind(v).bind(id).execute(pool).await?;
    }
    if let Some(v) = target_uri {
        sqlx::query("UPDATE course_items SET target_uri = $1 WHERE id = $2").bind(v).bind(id).execute(pool).await?;
    }
    if let Some(v) = external_url {
        sqlx::query("UPDATE course_items SET external_url = $1 WHERE id = $2").bind(v).bind(id).execute(pool).await?;
    }
    if let Some(v) = title {
        sqlx::query("UPDATE course_items SET title = $1 WHERE id = $2").bind(v).bind(id).execute(pool).await?;
    }
    if let Some(v) = note {
        sqlx::query("UPDATE course_items SET note = $1 WHERE id = $2").bind(v).bind(id).execute(pool).await?;
    }
    if let Some(v) = due_date {
        sqlx::query("UPDATE course_items SET due_date = $1 WHERE id = $2").bind(v).bind(id).execute(pool).await?;
    }
    Ok(())
}

pub async fn delete_item(pool: &PgPool, id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM course_items WHERE id = $1").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn get_item_unit_id(pool: &PgPool, item_id: &str) -> crate::Result<String> {
    sqlx::query_scalar::<_, String>("SELECT unit_id FROM course_items WHERE id = $1")
        .bind(item_id).fetch_optional(pool).await?
        .ok_or_else(|| crate::Error::NotFound { entity: "course_item", id: item_id.to_string() })
}

pub async fn reorder_items(pool: &PgPool, unit_id: &str, item_ids: &[String]) -> crate::Result<()> {
    for (i, iid) in item_ids.iter().enumerate() {
        sqlx::query("UPDATE course_items SET sort_order = $1 WHERE id = $2 AND unit_id = $3")
            .bind(i as i32).bind(iid).bind(unit_id)
            .execute(pool).await?;
    }
    Ok(())
}
