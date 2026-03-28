use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Book {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub description: String,
    pub cover_url: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBook {
    pub title: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub prereqs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BookEdition {
    pub id: String,
    pub book_id: String,
    pub title: String,
    pub lang: String,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub year: Option<String>,
    pub translators: Vec<String>,
    pub purchase_links: sqlx::types::Json<Vec<PurchaseLink>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEdition {
    pub title: String,
    pub lang: String,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub year: Option<String>,
    pub translators: Vec<String>,
    pub purchase_links: Vec<PurchaseLink>,
}

pub async fn create_book(
    pool: &PgPool,
    id: &str,
    input: &CreateBook,
    created_by: &str,
) -> crate::Result<Book> {
    let mut tx = pool.begin().await?;

    // Insert into content table so content_teaches FK works
    let content_uri = format!("book:{id}");
    sqlx::query(
        "INSERT INTO content (uri, content_type) VALUES ($1, 'book') ON CONFLICT DO NOTHING",
    )
    .bind(&content_uri)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO books (id, title, authors, description, cover_url, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(&input.title)
    .bind(&input.authors)
    .bind(input.description.as_deref().unwrap_or(""))
    .bind(&input.cover_url)
    .bind(created_by)
    .execute(&mut *tx)
    .await?;

    // Tags via content_teaches with uri = book:<id>
    for tag_id in &input.tags {
        sqlx::query(
            "INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
        )
        .bind(tag_id).bind(tag_id).bind(created_by)
        .execute(&mut *tx).await?;

        sqlx::query(
            "INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&content_uri).bind(tag_id)
        .execute(&mut *tx).await?;
    }

    // Prereq tags
    for tag_id in &input.prereqs {
        sqlx::query(
            "INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
        )
        .bind(tag_id).bind(tag_id).bind(created_by)
        .execute(&mut *tx).await?;

        sqlx::query(
            "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) VALUES ($1, $2, 'required') ON CONFLICT DO NOTHING",
        )
        .bind(&content_uri).bind(tag_id)
        .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    let book = sqlx::query_as::<_, Book>("SELECT * FROM books WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(book)
}

pub async fn get_book(pool: &PgPool, id: &str) -> crate::Result<Book> {
    sqlx::query_as::<_, Book>("SELECT * FROM books WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "book", id: id.to_string() })
}

pub async fn list_books(pool: &PgPool, limit: i64, offset: i64) -> crate::Result<Vec<Book>> {
    let rows = sqlx::query_as::<_, Book>(
        "SELECT * FROM books ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_editions(pool: &PgPool, book_id: &str) -> crate::Result<Vec<BookEdition>> {
    let rows = sqlx::query_as::<_, BookEdition>(
        "SELECT * FROM book_editions WHERE book_id = $1 ORDER BY created_at",
    )
    .bind(book_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn create_edition(
    pool: &PgPool,
    id: &str,
    book_id: &str,
    input: &CreateEdition,
) -> crate::Result<BookEdition> {
    let links_json = sqlx::types::Json(&input.purchase_links);
    sqlx::query(
        "INSERT INTO book_editions (id, book_id, title, lang, isbn, publisher, year, translators, purchase_links) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(id)
    .bind(book_id)
    .bind(&input.title)
    .bind(&input.lang)
    .bind(&input.isbn)
    .bind(&input.publisher)
    .bind(&input.year)
    .bind(&input.translators)
    .bind(&links_json)
    .execute(pool)
    .await?;

    let edition = sqlx::query_as::<_, BookEdition>(
        "SELECT * FROM book_editions WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(edition)
}

pub async fn update_book(
    pool: &PgPool,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    cover_url: Option<&str>,
) -> crate::Result<()> {
    if let Some(t) = title {
        sqlx::query("UPDATE books SET title = $1 WHERE id = $2")
            .bind(t).bind(id).execute(pool).await?;
    }
    if let Some(d) = description {
        sqlx::query("UPDATE books SET description = $1 WHERE id = $2")
            .bind(d).bind(id).execute(pool).await?;
    }
    if let Some(c) = cover_url {
        sqlx::query("UPDATE books SET cover_url = $1 WHERE id = $2")
            .bind(c).bind(id).execute(pool).await?;
    }
    Ok(())
}

pub async fn delete_edition(pool: &PgPool, id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM book_editions WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get reviews (articles with category='review' and book_id) for a book.
pub async fn get_book_reviews(
    pool: &PgPool,
    book_id: &str,
    limit: i64,
    offset: i64,
) -> crate::Result<Vec<crate::models::Article>> {
    let rows = sqlx::query_as::<_, crate::models::Article>(
        "SELECT a.*, p.handle AS author_handle \
         FROM articles a \
         LEFT JOIN profiles p ON a.did = p.did \
         WHERE a.book_id = $1 AND a.category = 'review' AND a.visibility = 'public' \
         ORDER BY a.vote_score DESC, a.created_at DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(book_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
