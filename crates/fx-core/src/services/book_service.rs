use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Book {
    pub id: String,
    #[ts(type = "Record<string, string>")]
    pub title: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub authors: Vec<String>,
    #[ts(type = "Record<string, string>")]
    pub description: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub abbreviation: Option<String>,
    pub default_edition_id: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    /// Derived from editions at query time, not stored on the books table.
    #[sqlx(default)]
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateBook {
    #[ts(type = "Record<string, string>")]
    pub title: std::collections::HashMap<String, String>,
    pub authors: Vec<String>,
    #[ts(type = "Record<string, string> | undefined")]
    pub description: Option<std::collections::HashMap<String, String>>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub prereqs: Vec<String>,
    #[serde(default)]
    pub abbreviation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PurchaseLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookEdition {
    pub id: String,
    pub book_id: String,
    pub title: String,
    pub lang: String,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub year: Option<String>,
    pub translators: Vec<String>,
    
    #[ts(type = "Array<{label: string, url: string}>")]

    
    pub purchase_links: sqlx::types::Json<Vec<PurchaseLink>>,
    pub cover_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateEdition {
    pub title: String,
    pub lang: String,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub year: Option<String>,
    pub translators: Vec<String>,
    pub purchase_links: Vec<PurchaseLink>,
    pub cover_url: Option<String>,
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
        "INSERT INTO books (id, title, authors, description, created_by, abbreviation) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(sqlx::types::Json(&input.title))
    .bind(&input.authors)
    .bind(sqlx::types::Json(input.description.as_ref().unwrap_or(&std::collections::HashMap::new())))
    .bind(created_by)
    .bind(&input.abbreviation)
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

    let book = sqlx::query_as::<_, Book>(
        &format!("SELECT b.id, b.title, b.authors, b.description, b.abbreviation, b.default_edition_id, b.created_by, b.created_at, {BOOK_COVER_SQL} AS cover_url FROM books b WHERE b.id = $1"),
    )
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(book)
}

/// Derived cover subquery: pick from editions by most-marked popularity.
const BOOK_COVER_SQL: &str = "\
(SELECT e.cover_url FROM book_editions e \
 LEFT JOIN (SELECT preferred_edition_id, COUNT(*) AS cnt FROM book_reading_status \
            WHERE book_id = b.id AND preferred_edition_id IS NOT NULL \
            GROUP BY preferred_edition_id) pop ON pop.preferred_edition_id = e.id \
 WHERE e.book_id = b.id AND e.cover_url IS NOT NULL \
 ORDER BY COALESCE(pop.cnt, 0) DESC, e.created_at LIMIT 1)";

pub async fn get_book(pool: &PgPool, id: &str) -> crate::Result<Book> {
    sqlx::query_as::<_, Book>(
        &format!("SELECT b.id, b.title, b.authors, b.description, b.abbreviation, b.default_edition_id, b.created_by, b.created_at, {BOOK_COVER_SQL} AS cover_url FROM books b WHERE b.id = $1"),
    )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "book", id: id.to_string() })
}

/// Viewer-aware cover: prefer user's selected edition, then most popular.
pub async fn get_book_for_viewer(pool: &PgPool, id: &str, viewer_did: &str) -> crate::Result<Book> {
    sqlx::query_as::<_, Book>(
        &format!(
            "SELECT b.*, \
             COALESCE(\
               (SELECT e.cover_url FROM book_editions e \
                JOIN book_reading_status rs ON rs.book_id = b.id AND rs.user_did = $2 AND rs.preferred_edition_id = e.id \
                WHERE e.book_id = b.id AND e.cover_url IS NOT NULL LIMIT 1), \
               {BOOK_COVER_SQL}\
             ) AS cover_url \
             FROM books b WHERE b.id = $1",
        ),
    )
    .bind(id)
    .bind(viewer_did)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::Error::NotFound { entity: "book", id: id.to_string() })
}

/// Find a book by ISBN (searches across all editions).
pub async fn find_book_by_isbn(pool: &PgPool, isbn: &str) -> crate::Result<Option<Book>> {
    let row = sqlx::query_as::<_, Book>(
        &format!("SELECT b.id, b.title, b.authors, b.description, b.abbreviation, b.default_edition_id, b.created_by, b.created_at, {BOOK_COVER_SQL} AS cover_url FROM books b \
                  JOIN book_editions be ON be.book_id = b.id WHERE be.isbn = $1 LIMIT 1"),
    )
    .bind(isbn)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list_books(pool: &PgPool, limit: i64, offset: i64) -> crate::Result<Vec<Book>> {
    let rows = sqlx::query_as::<_, Book>(
        &format!("SELECT b.id, b.title, b.authors, b.description, b.abbreviation, b.default_edition_id, b.created_by, b.created_at, {BOOK_COVER_SQL} AS cover_url FROM books b \
                  ORDER BY b.created_at DESC LIMIT $1 OFFSET $2"),
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Book with rating stats and tags, for list display.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BookListItem {
    pub id: String,
    pub title: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub authors: Vec<String>,
    pub description: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub cover_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub avg_rating: f64,
    pub rating_count: i64,
    pub reader_count: i64,
    pub tags: sqlx::types::Json<Vec<String>>,
}

pub async fn list_books_rich(pool: &PgPool, viewer_did: Option<&str>, limit: i64, offset: i64) -> crate::Result<Vec<BookListItem>> {
    let did = viewer_did.unwrap_or("");
    let rows = sqlx::query_as::<_, BookListItem>(
        "SELECT b.id, b.title, b.authors, b.description, \
         COALESCE(\
           (SELECT e.cover_url FROM book_editions e \
            JOIN book_reading_status rs ON rs.book_id = b.id AND rs.user_did = $3 AND rs.preferred_edition_id = e.id \
            WHERE e.book_id = b.id AND e.cover_url IS NOT NULL LIMIT 1), \
           (SELECT e.cover_url FROM book_editions e \
            LEFT JOIN (SELECT preferred_edition_id, COUNT(*) AS cnt FROM book_reading_status WHERE book_id = b.id AND preferred_edition_id IS NOT NULL GROUP BY preferred_edition_id) pop ON pop.preferred_edition_id = e.id \
            WHERE e.book_id = b.id AND e.cover_url IS NOT NULL \
            ORDER BY COALESCE(pop.cnt, 0) DESC, e.created_at LIMIT 1)\
         ) AS cover_url, \
         b.created_at, \
         COALESCE(r.avg, 0) AS avg_rating, \
         COALESCE(r.cnt, 0) AS rating_count, \
         COALESCE(rd.cnt, 0) AS reader_count, \
         COALESCE((SELECT jsonb_agg(ct.tag_id) FROM content_teaches ct WHERE ct.content_uri = 'book:' || b.id), '[]'::jsonb) AS tags \
         FROM books b \
         LEFT JOIN (SELECT book_id, AVG(rating)::float8 AS avg, COUNT(*) AS cnt FROM book_ratings GROUP BY book_id) r ON r.book_id = b.id \
         LEFT JOIN (SELECT book_id, COUNT(*) AS cnt FROM book_reading_status GROUP BY book_id) rd ON rd.book_id = b.id \
         ORDER BY COALESCE(r.avg, 0) * LN(COALESCE(r.cnt, 0) + 1) + COALESCE(rd.cnt, 0) * 0.5 DESC, b.created_at DESC \
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .bind(did)
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
    // Reject duplicate ISBN
    if let Some(isbn) = &input.isbn {
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT book_id FROM book_editions WHERE isbn = $1"
        ).bind(isbn).fetch_optional(pool).await?;
        if let Some(existing_book_id) = existing {
            return Err(crate::Error::BadRequest(
                format!("ISBN {isbn} already exists on book {existing_book_id}")
            ));
        }
    }

    let links_json = sqlx::types::Json(&input.purchase_links);
    sqlx::query(
        "INSERT INTO book_editions (id, book_id, title, lang, isbn, publisher, year, translators, purchase_links, cover_url) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
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
    .bind(&input.cover_url)
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

pub async fn update_edition(
    pool: &PgPool,
    edition_id: &str,
    input: &CreateEdition,
) -> crate::Result<BookEdition> {
    let links_json = sqlx::types::Json(&input.purchase_links);
    sqlx::query(
        "UPDATE book_editions SET title = $1, lang = $2, isbn = $3, publisher = $4, \
         year = $5, translators = $6, purchase_links = $7, cover_url = $8 \
         WHERE id = $9",
    )
    .bind(&input.title)
    .bind(&input.lang)
    .bind(&input.isbn)
    .bind(&input.publisher)
    .bind(&input.year)
    .bind(&input.translators)
    .bind(&links_json)
    .bind(&input.cover_url)
    .bind(edition_id)
    .execute(pool)
    .await?;

    let edition = sqlx::query_as::<_, BookEdition>(
        "SELECT * FROM book_editions WHERE id = $1",
    )
    .bind(edition_id)
    .fetch_one(pool)
    .await?;
    Ok(edition)
}

pub async fn update_book(
    pool: &PgPool,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
) -> crate::Result<()> {
    if let Some(t) = title {
        sqlx::query("UPDATE books SET title = $1 WHERE id = $2")
            .bind(t).bind(id).execute(pool).await?;
    }
    if let Some(d) = description {
        sqlx::query("UPDATE books SET description = $1 WHERE id = $2")
            .bind(d).bind(id).execute(pool).await?;
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
        "SELECT a.at_uri, a.did, p.handle AS author_handle, COALESCE(p.reputation, 0) AS author_reputation, \
         a.kind, a.title, a.description, \
         a.content_hash, a.content_format, a.lang, a.translation_group, a.license, a.prereq_threshold, \
         a.question_uri, a.answer_count, a.restricted, a.category, a.book_id, a.edition_id, \
         COALESCE(v.vote_score, 0) AS vote_score, \
         COALESCE(b.bookmark_count, 0) AS bookmark_count, \
         COALESCE(cm.comment_count, 0) AS comment_count, \
         COALESCE(fk.fork_count, 0) AS fork_count, \
         a.created_at, a.updated_at \
         FROM articles a \
         LEFT JOIN profiles p ON a.did = p.did \
         LEFT JOIN (SELECT target_uri, SUM(value) AS vote_score FROM votes GROUP BY target_uri) v ON v.target_uri = a.at_uri \
         LEFT JOIN (SELECT article_uri, COUNT(*) AS bookmark_count FROM user_bookmarks GROUP BY article_uri) b ON b.article_uri = a.at_uri \
         LEFT JOIN (SELECT content_uri, COUNT(*) AS comment_count FROM comments GROUP BY content_uri) cm ON cm.content_uri = a.at_uri \
         LEFT JOIN (SELECT source_uri, COUNT(*) AS fork_count FROM forks GROUP BY source_uri) fk ON fk.source_uri = a.at_uri \
         WHERE a.book_id = $1 AND a.category = 'review' AND a.visibility = 'public' \
         ORDER BY vote_score DESC, a.created_at DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(book_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ---- Ratings ----

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookRatingStats {
    pub avg_rating: f64,
    pub rating_count: i64,
}

pub async fn rate_book(pool: &PgPool, book_id: &str, user_did: &str, rating: i16) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO book_ratings (book_id, user_did, rating) VALUES ($1, $2, $3) \
         ON CONFLICT (book_id, user_did) DO UPDATE SET rating = $3, updated_at = NOW()",
    )
    .bind(book_id).bind(user_did).bind(rating)
    .execute(pool).await?;
    Ok(())
}

pub async fn get_user_rating(pool: &PgPool, book_id: &str, user_did: &str) -> crate::Result<Option<i16>> {
    let row = sqlx::query_scalar::<_, i16>(
        "SELECT rating FROM book_ratings WHERE book_id = $1 AND user_did = $2",
    )
    .bind(book_id).bind(user_did)
    .fetch_optional(pool).await?;
    Ok(row)
}

pub async fn get_rating_stats(pool: &PgPool, book_id: &str) -> crate::Result<BookRatingStats> {
    let row = sqlx::query_as::<_, (Option<f64>, i64)>(
        "SELECT AVG(rating::float), COUNT(*) FROM book_ratings WHERE book_id = $1",
    )
    .bind(book_id)
    .fetch_one(pool).await?;
    Ok(BookRatingStats {
        avg_rating: row.0.unwrap_or(0.0),
        rating_count: row.1,
    })
}

// ---- Reading status ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ReadingStatus {
    pub book_id: String,
    pub user_did: String,
    pub status: String,
    pub progress: i16,
    pub updated_at: DateTime<Utc>,
}

pub async fn set_reading_status(
    pool: &PgPool, book_id: &str, user_did: &str, status: &str, progress: i16,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO book_reading_status (book_id, user_did, status, progress) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (book_id, user_did) DO UPDATE SET status = $3, progress = $4, updated_at = NOW()",
    )
    .bind(book_id).bind(user_did).bind(status).bind(progress)
    .execute(pool).await?;
    Ok(())
}

pub async fn get_reading_status(pool: &PgPool, book_id: &str, user_did: &str) -> crate::Result<Option<ReadingStatus>> {
    sqlx::query_as::<_, ReadingStatus>(
        "SELECT * FROM book_reading_status WHERE book_id = $1 AND user_did = $2",
    )
    .bind(book_id).bind(user_did)
    .fetch_optional(pool).await.map_err(Into::into)
}

pub async fn remove_reading_status(pool: &PgPool, book_id: &str, user_did: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM book_reading_status WHERE book_id = $1 AND user_did = $2")
        .bind(book_id).bind(user_did)
        .execute(pool).await?;
    Ok(())
}

// ---- Chapters ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookChapter {
    pub id: String,
    pub book_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub order_index: i32,
    pub article_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ChapterPrereq {
    pub tag_id: String,
    pub prereq_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookChapterWithTags {
    pub id: String,
    pub book_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub order_index: i32,
    pub article_uri: Option<String>,
    pub teaches: Vec<String>,
    pub prereqs: Vec<ChapterPrereq>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateChapter {
    pub title: String,
    pub parent_id: Option<String>,
    pub order_index: i32,
    pub article_uri: Option<String>,
    #[serde(default)]
    pub teaches: Vec<String>,
    #[serde(default)]
    pub prereqs: Vec<ChapterPrereq>,
}

pub async fn list_chapters(pool: &PgPool, book_id: &str) -> crate::Result<Vec<BookChapter>> {
    let rows = sqlx::query_as::<_, BookChapter>(
        "SELECT * FROM book_chapters WHERE book_id = $1 ORDER BY order_index",
    )
    .bind(book_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_chapters_with_tags(pool: &PgPool, book_id: &str) -> crate::Result<Vec<BookChapterWithTags>> {
    let chapters = list_chapters(pool, book_id).await?;
    if chapters.is_empty() {
        return Ok(vec![]);
    }

    let ids: Vec<&str> = chapters.iter().map(|c| c.id.as_str()).collect();
    let uris: Vec<String> = ids.iter().map(|id| format!("chapter:{id}")).collect();

    #[derive(sqlx::FromRow)]
    struct TeachRow { content_uri: String, tag_id: String }
    let teaches_rows = sqlx::query_as::<_, TeachRow>(
        "SELECT content_uri, tag_id FROM content_teaches WHERE content_uri = ANY($1) ORDER BY tag_id",
    )
    .bind(&uris)
    .fetch_all(pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct PrereqRow { content_uri: String, tag_id: String, prereq_type: String }
    let prereq_rows = sqlx::query_as::<_, PrereqRow>(
        "SELECT content_uri, tag_id, prereq_type FROM content_prereqs WHERE content_uri = ANY($1) ORDER BY tag_id",
    )
    .bind(&uris)
    .fetch_all(pool)
    .await?;

    let mut teaches_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for row in teaches_rows {
        teaches_map.entry(row.content_uri).or_default().push(row.tag_id);
    }
    let mut prereqs_map: std::collections::HashMap<String, Vec<ChapterPrereq>> = std::collections::HashMap::new();
    for row in prereq_rows {
        prereqs_map.entry(row.content_uri).or_default().push(ChapterPrereq { tag_id: row.tag_id, prereq_type: row.prereq_type });
    }

    Ok(chapters.into_iter().map(|c| {
        let uri = format!("chapter:{}", c.id);
        BookChapterWithTags {
            teaches: teaches_map.remove(&uri).unwrap_or_default(),
            prereqs: prereqs_map.remove(&uri).unwrap_or_default(),
            id: c.id,
            book_id: c.book_id,
            parent_id: c.parent_id,
            title: c.title,
            order_index: c.order_index,
            article_uri: c.article_uri,
        }
    }).collect())
}

pub async fn create_chapter(
    pool: &PgPool,
    id: &str,
    book_id: &str,
    created_by: &str,
    input: &CreateChapter,
) -> crate::Result<BookChapter> {
    let mut tx = pool.begin().await?;
    let content_uri = format!("chapter:{id}");

    sqlx::query(
        "INSERT INTO book_chapters (id, book_id, parent_id, title, order_index, article_uri) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id).bind(book_id).bind(&input.parent_id)
    .bind(&input.title).bind(input.order_index).bind(&input.article_uri)
    .execute(&mut *tx).await?;

    sqlx::query("INSERT INTO content (uri, content_type) VALUES ($1, 'chapter') ON CONFLICT DO NOTHING")
        .bind(&content_uri).execute(&mut *tx).await?;

    for tag_id in &input.teaches {
        sqlx::query("INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING")
            .bind(tag_id).bind(tag_id).bind(created_by).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(&content_uri).bind(tag_id).execute(&mut *tx).await?;
    }
    for p in &input.prereqs {
        sqlx::query("INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING")
            .bind(&p.tag_id).bind(&p.tag_id).bind(created_by).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
            .bind(&content_uri).bind(&p.tag_id).bind(&p.prereq_type).execute(&mut *tx).await?;
    }

    tx.commit().await?;

    let ch = sqlx::query_as::<_, BookChapter>("SELECT * FROM book_chapters WHERE id = $1")
        .bind(id).fetch_one(pool).await?;
    Ok(ch)
}

pub async fn set_chapter_tags(
    pool: &PgPool,
    chapter_id: &str,
    created_by: &str,
    teaches: &[String],
    prereqs: &[ChapterPrereq],
) -> crate::Result<()> {
    let content_uri = format!("chapter:{chapter_id}");
    let mut tx = pool.begin().await?;

    // Ensure content row exists
    sqlx::query("INSERT INTO content (uri, content_type) VALUES ($1, 'chapter') ON CONFLICT DO NOTHING")
        .bind(&content_uri).execute(&mut *tx).await?;

    // Replace teaches
    sqlx::query("DELETE FROM content_teaches WHERE content_uri = $1")
        .bind(&content_uri).execute(&mut *tx).await?;
    for tag_id in teaches {
        sqlx::query("INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING")
            .bind(tag_id).bind(tag_id).bind(created_by).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(&content_uri).bind(tag_id).execute(&mut *tx).await?;
    }

    // Replace prereqs
    sqlx::query("DELETE FROM content_prereqs WHERE content_uri = $1")
        .bind(&content_uri).execute(&mut *tx).await?;
    for p in prereqs {
        sqlx::query("INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING")
            .bind(&p.tag_id).bind(&p.tag_id).bind(created_by).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
            .bind(&content_uri).bind(&p.tag_id).bind(&p.prereq_type).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn delete_chapter(pool: &PgPool, id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM book_chapters WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---- Chapter progress ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ChapterProgress {
    pub book_id: String,
    pub chapter_id: String,
    pub user_did: String,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

pub async fn set_chapter_progress(
    pool: &PgPool,
    book_id: &str,
    chapter_id: &str,
    user_did: &str,
    completed: bool,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO book_chapter_progress (book_id, chapter_id, user_did, completed, completed_at) \
         VALUES ($1, $2, $3, $4, CASE WHEN $4 THEN NOW() ELSE NULL END) \
         ON CONFLICT (chapter_id, user_did) DO UPDATE SET completed = $4, \
         completed_at = CASE WHEN $4 THEN COALESCE(book_chapter_progress.completed_at, NOW()) ELSE NULL END",
    )
    .bind(book_id)
    .bind(chapter_id)
    .bind(user_did)
    .bind(completed)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_chapter_progress(
    pool: &PgPool,
    book_id: &str,
    user_did: &str,
) -> crate::Result<Vec<ChapterProgress>> {
    let rows = sqlx::query_as::<_, ChapterProgress>(
        "SELECT * FROM book_chapter_progress WHERE book_id = $1 AND user_did = $2",
    )
    .bind(book_id)
    .bind(user_did)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ---- Book resources (supplementary materials) ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookResource {
    pub id: String,
    pub book_id: String,
    pub edition_id: Option<String>,
    pub kind: String,
    pub label: String,
    pub url: String,
    pub position: i16,
}

pub async fn list_book_resources(pool: &PgPool, book_id: &str) -> crate::Result<Vec<BookResource>> {
    let rows = sqlx::query_as::<_, BookResource>(
        "SELECT id, book_id, edition_id, kind, label, url, position \
         FROM book_resources WHERE book_id = $1 ORDER BY kind, position",
    )
    .bind(book_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn add_book_resource(
    pool: &PgPool,
    book_id: &str,
    edition_id: Option<&str>,
    kind: &str,
    label: &str,
    url: &str,
    position: i16,
    created_by: &str,
) -> crate::Result<String> {
    let id: String = sqlx::query_scalar(
        "INSERT INTO book_resources (book_id, edition_id, kind, label, url, position, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(book_id)
    .bind(edition_id)
    .bind(kind)
    .bind(label)
    .bind(url)
    .bind(position)
    .bind(created_by)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn delete_book_resource(pool: &PgPool, id: &str) -> crate::Result<bool> {
    let result = sqlx::query("DELETE FROM book_resources WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
