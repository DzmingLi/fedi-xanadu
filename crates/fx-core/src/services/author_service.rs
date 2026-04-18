use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Author {
    pub id: String,
    pub name: String,
    pub did: Option<String>,
    pub orcid: Option<String>,
    pub affiliation: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorDetail {
    pub author: Author,
    pub books: Vec<AuthorBookEntry>,
    pub courses: Vec<AuthorCourseEntry>,
    pub article_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuthorBookEntry {
    pub book_id: String,
    pub title: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuthorCourseEntry {
    pub course_id: String,
    pub title: String,
    pub code: Option<String>,
    pub institution: Option<String>,
    pub semester: Option<String>,
}

pub async fn list_courses_by_author(
    pool: &PgPool,
    author_id: &str,
) -> crate::Result<Vec<AuthorCourseEntry>> {
    let rows = sqlx::query_as::<_, AuthorCourseEntry>(
        "SELECT c.id AS course_id, c.title, c.code, c.institution, c.semester \
         FROM course_authors ca JOIN courses c ON c.id = ca.course_id \
         WHERE ca.author_id = $1 ORDER BY ca.position, c.created_at DESC",
    )
    .bind(author_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get author by ID.
pub async fn get_author(pool: &PgPool, id: &str) -> crate::Result<Author> {
    sqlx::query_as::<_, Author>(
        "SELECT id, name, did, orcid, affiliation, homepage FROM authors WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::Error::NotFound { entity: "author", id: id.to_string() })
}

/// Get author by name (exact match).
pub async fn get_author_by_name(pool: &PgPool, name: &str) -> crate::Result<Option<Author>> {
    let row = sqlx::query_as::<_, Author>(
        "SELECT id, name, did, orcid, affiliation, homepage FROM authors WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Search authors by name prefix.
pub async fn search_authors(pool: &PgPool, query: &str, limit: i64) -> crate::Result<Vec<Author>> {
    let rows = sqlx::query_as::<_, Author>(
        "SELECT id, name, did, orcid, affiliation, homepage FROM authors \
         WHERE name ILIKE $1 ORDER BY name LIMIT $2",
    )
    .bind(format!("%{query}%"))
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List books by an author.
pub async fn list_books_by_author(pool: &PgPool, author_id: &str) -> crate::Result<Vec<AuthorBookEntry>> {
    let rows = sqlx::query_as::<_, AuthorBookEntry>(
        "SELECT b.id AS book_id, b.title, \
         (SELECT e.cover_url FROM book_editions e WHERE e.book_id = b.id AND e.cover_url IS NOT NULL LIMIT 1) AS cover_url \
         FROM book_authors ba \
         JOIN books b ON ba.book_id = b.id \
         WHERE ba.author_id = $1 \
         ORDER BY ba.position",
    )
    .bind(author_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get or create an author by name. Returns the author ID.
pub async fn get_or_create_author(pool: &PgPool, name: &str) -> crate::Result<String> {
    if let Some(existing) = get_author_by_name(pool, name).await? {
        return Ok(existing.id);
    }
    let id: String = sqlx::query_scalar(
        "INSERT INTO authors (name) VALUES ($1) RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Link an author to a book.
pub async fn link_author_to_book(
    pool: &PgPool,
    book_id: &str,
    author_id: &str,
    position: i16,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO book_authors (book_id, author_id, position) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(book_id)
    .bind(author_id)
    .bind(position)
    .execute(pool)
    .await?;
    Ok(())
}

/// Update author metadata.
pub async fn update_author(
    pool: &PgPool,
    id: &str,
    did: Option<&str>,
    orcid: Option<&str>,
    affiliation: Option<&str>,
    homepage: Option<&str>,
) -> crate::Result<()> {
    sqlx::query(
        "UPDATE authors SET did = COALESCE($2, did), orcid = COALESCE($3, orcid), \
         affiliation = COALESCE($4, affiliation), homepage = COALESCE($5, homepage) WHERE id = $1",
    )
    .bind(id)
    .bind(did)
    .bind(orcid)
    .bind(affiliation)
    .bind(homepage)
    .execute(pool)
    .await?;
    Ok(())
}

/// List authors for a book (ordered by position).
pub async fn list_book_authors(pool: &PgPool, book_id: &str) -> crate::Result<Vec<Author>> {
    let rows = sqlx::query_as::<_, Author>(
        "SELECT a.id, a.name, a.did, a.orcid, a.affiliation, a.homepage \
         FROM book_authors ba \
         JOIN authors a ON ba.author_id = a.id \
         WHERE ba.book_id = $1 \
         ORDER BY ba.position",
    )
    .bind(book_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
