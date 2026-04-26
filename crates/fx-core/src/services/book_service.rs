use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Book {
    pub id: String,
    #[ts(type = "Record<string, string>")]
    pub title: sqlx::types::Json<std::collections::HashMap<String, String>>,
    #[ts(type = "Record<string, string>")]
    pub subtitle: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub authors: Vec<String>,
    #[ts(type = "Record<string, string>")]
    pub description: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub abbreviation: Option<String>,
    pub default_edition_id: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    /// Self-describing exam-prep tags ('kaoyan-math-1', 'kaoyan-408', ...).
    /// NULL/empty = non-exam book. Vocabulary lives in the frontend.
    pub exam_tags: Option<Vec<String>>,
    /// Derived from editions at query time, not stored on the books table.
    #[sqlx(default)]
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateBook {
    #[ts(type = "Record<string, string>")]
    pub title: std::collections::HashMap<String, String>,
    #[serde(default)]
    #[ts(type = "Record<string, string> | undefined")]
    pub subtitle: Option<std::collections::HashMap<String, String>>,
    pub authors: Vec<String>,
    #[ts(type = "Record<string, string> | undefined")]
    pub description: Option<std::collections::HashMap<String, String>>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub prereqs: Vec<crate::models::ArticlePrereq>,
    #[serde(default)]
    pub abbreviation: Option<String>,
    #[serde(default)]
    pub exam_tags: Option<Vec<String>>,
    /// First edition is required — a book without any edition is just a
    /// shell. Past CS-textbook bulk imports left orphans like that and
    /// they're useless: no cover, no link, no chapter scope.
    pub first_edition: CreateEdition,
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
    pub edition_name: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub lang: String,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub year: Option<String>,
    pub translators: Vec<String>,

    #[ts(type = "Array<{label: string, url: string}>")]

    
    pub purchase_links: sqlx::types::Json<Vec<PurchaseLink>>,
    pub cover_url: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateEdition {
    #[serde(default)]
    pub edition_name: Option<String>,
    pub title: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    pub lang: String,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub year: Option<String>,
    pub translators: Vec<String>,
    pub purchase_links: Vec<PurchaseLink>,
    pub cover_url: Option<String>,
    /// 'draft' or 'published'. Defaults to 'published' when omitted.
    #[serde(default)]
    pub status: Option<String>,
}

pub async fn create_book(
    pool: &PgPool,
    id: &str,
    input: &CreateBook,
    created_by: &str,
) -> crate::Result<Book> {
    // ISBN uniqueness check up-front so we don't create a book and then
    // roll back a partially-inserted edition.
    if let Some(isbn) = &input.first_edition.isbn {
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT book_id FROM book_editions WHERE isbn = $1"
        ).bind(isbn).fetch_optional(pool).await?;
        if let Some(existing_book_id) = existing {
            return Err(crate::Error::BadRequest(
                format!("ISBN {isbn} already exists on book {existing_book_id}")
            ));
        }
    }

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
        "INSERT INTO books (id, title, subtitle, authors, description, created_by, abbreviation, exam_tags) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(sqlx::types::Json(&input.title))
    .bind(sqlx::types::Json(input.subtitle.as_ref().unwrap_or(&std::collections::HashMap::new())))
    .bind(&input.authors)
    .bind(sqlx::types::Json(input.description.as_ref().unwrap_or(&std::collections::HashMap::new())))
    .bind(created_by)
    .bind(&input.abbreviation)
    .bind(input.exam_tags.as_deref().filter(|t| !t.is_empty()))
    .execute(&mut *tx)
    .await?;

    // First edition is mandatory. Bundling it into the same transaction
    // means callers can't ever leave a book in the orphan state we used
    // to see (CS6110 textbook bulk-import left several Pierce/Gunter/
    // Mitchell book shells that no one could attach editions to).
    let edition_id = format!("ed-{}", crate::util::tid());
    let ed = &input.first_edition;
    let links_json = sqlx::types::Json(&ed.purchase_links);
    let status = ed.status.as_deref().unwrap_or("published");
    sqlx::query(
        "INSERT INTO book_editions (id, book_id, edition_name, title, subtitle, lang, isbn, publisher, year, translators, purchase_links, cover_url, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(&edition_id)
    .bind(id)
    .bind(&ed.edition_name)
    .bind(&ed.title)
    .bind(&ed.subtitle)
    .bind(&ed.lang)
    .bind(&ed.isbn)
    .bind(&ed.publisher)
    .bind(&ed.year)
    .bind(&ed.translators)
    .bind(&links_json)
    .bind(&ed.cover_url)
    .bind(status)
    .execute(&mut *tx)
    .await?;

    // Tags via content_teaches with uri = book:<id>. Normalize each
    // input (tag_id, label id, or new name) to a canonical tag_id.
    for input_ref in &input.tags {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, created_by).await?;
        sqlx::query(
            "INSERT INTO content_teaches (content_uri, tag_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&content_uri).bind(&tag_id)
        .execute(&mut *tx).await?;
    }

    // Prereq tags — each entry carries its own required/recommended
    // strength.
    for p in &input.prereqs {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, &p.tag_id, created_by).await?;
        sqlx::query(
            "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) \
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(&content_uri).bind(&tag_id).bind(p.prereq_type.as_str())
        .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    for (position, name) in input.authors.iter().enumerate() {
        let author_id = crate::services::author_service::get_or_create_author(pool, name).await?;
        crate::services::author_service::link_author_to_book(pool, id, &author_id, position as i16).await?;
    }

    let book = sqlx::query_as::<_, Book>(
        &format!("SELECT b.id, b.title, b.subtitle, b.authors, b.description, b.abbreviation, b.default_edition_id, b.created_by, b.created_at, b.exam_tags, {BOOK_COVER_SQL} AS cover_url FROM books b WHERE b.id = $1"),
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
        &format!("SELECT b.id, b.title, b.subtitle, b.authors, b.description, b.abbreviation, b.default_edition_id, b.created_by, b.created_at, b.exam_tags, {BOOK_COVER_SQL} AS cover_url FROM books b WHERE b.id = $1"),
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
        &format!("SELECT b.id, b.title, b.subtitle, b.authors, b.description, b.abbreviation, b.default_edition_id, b.created_by, b.created_at, b.exam_tags, {BOOK_COVER_SQL} AS cover_url FROM books b \
                  JOIN book_editions be ON be.book_id = b.id WHERE be.isbn = $1 LIMIT 1"),
    )
    .bind(isbn)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list_books(pool: &PgPool, limit: i64, offset: i64) -> crate::Result<Vec<Book>> {
    let rows = sqlx::query_as::<_, Book>(
        &format!("SELECT b.id, b.title, b.subtitle, b.authors, b.description, b.abbreviation, b.default_edition_id, b.created_by, b.created_at, b.exam_tags, {BOOK_COVER_SQL} AS cover_url FROM books b \
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
    pub subtitle: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub authors: Vec<String>,
    pub description: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub cover_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub avg_rating: f64,
    pub rating_count: i64,
    pub reader_count: i64,
    pub tags: sqlx::types::Json<Vec<String>>,
    /// Full topic closure: for each direct teach tag of the book, include
    /// every group-sibling of that tag plus every ancestor reachable via
    /// tag_parents. Used by the book-list field tabs (Math / CS / Physics
    /// / Econ): a book lives under a field if its closure contains any
    /// member of that field's group.
    pub topics: sqlx::types::Json<Vec<String>>,
    pub exam_tags: Option<Vec<String>>,
}

/// Filter options for the book list.
#[derive(Debug, Default, Clone)]
pub struct BookListFilter<'a> {
    /// If set, restrict the result by exam-prep tags:
    ///   - `"none"` → exam_tags IS NULL or empty (non-exam books)
    ///   - `"any"` → exam_tags has at least one entry (any exam-prep book)
    ///   - any other value → exam_tags contains that exact tag
    pub exam: Option<&'a str>,
}

pub async fn list_books_rich(
    pool: &PgPool,
    viewer_did: Option<&str>,
    filter: &BookListFilter<'_>,
    limit: i64,
    offset: i64,
) -> crate::Result<Vec<BookListItem>> {
    let did = viewer_did.unwrap_or("");
    // Dynamic WHERE clause for the exam filter. `$4` is bound only when
    // the filter targets a specific tag; otherwise it's a static check.
    let (exam_clause, exam_bind): (&str, &str) = match filter.exam {
        None => ("TRUE", ""),
        Some("none") => ("(b.exam_tags IS NULL OR cardinality(b.exam_tags) = 0)", ""),
        Some("any") => ("(b.exam_tags IS NOT NULL AND cardinality(b.exam_tags) > 0)", ""),
        Some(tag) => ("$4 = ANY(b.exam_tags)", tag),
    };
    let sql = format!(
        "SELECT b.id, b.title, b.subtitle, b.authors, b.description, \
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
         COALESCE((SELECT jsonb_agg(ct.tag_id) FROM content_teaches ct WHERE ct.content_uri = 'book:' || b.id), '[]'::jsonb) AS tags, \
         COALESCE(( \
             WITH RECURSIVE closure(tag) AS ( \
                 SELECT tag_id FROM content_teaches WHERE content_uri = 'book:' || b.id \
                 UNION \
                 SELECT tag_id FROM content_topics  WHERE content_uri = 'book:' || b.id \
                 UNION \
                 SELECT tp.parent_tag FROM tag_parents tp \
                 JOIN closure c ON tp.child_tag = c.tag \
             ) \
             SELECT jsonb_agg(DISTINCT tag) FROM closure \
         ), '[]'::jsonb) AS topics, \
         b.exam_tags \
         FROM books b \
         LEFT JOIN (SELECT book_id, AVG(rating)::float8 AS avg, COUNT(*) AS cnt FROM book_ratings GROUP BY book_id) r ON r.book_id = b.id \
         LEFT JOIN (SELECT book_id, COUNT(*) AS cnt FROM book_reading_status GROUP BY book_id) rd ON rd.book_id = b.id \
         WHERE {exam_clause} \
         ORDER BY COALESCE(r.avg, 0) * LN(COALESCE(r.cnt, 0) + 1) + COALESCE(rd.cnt, 0) * 0.5 DESC, b.created_at DESC \
         LIMIT $1 OFFSET $2",
    );
    let mut q = sqlx::query_as::<_, BookListItem>(&sql)
        .bind(limit)
        .bind(offset)
        .bind(did);
    if matches!(filter.exam, Some(tag) if tag != "none" && tag != "any") {
        q = q.bind(exam_bind);
    }
    let rows = q.fetch_all(pool).await?;
    Ok(rows)
}

pub async fn list_editions(pool: &PgPool, book_id: &str) -> crate::Result<Vec<BookEdition>> {
    // Newest edition first (by year). NULL years sink to the bottom, and ties
    // fall back to insertion order so a multi-volume set keeps Vol I above
    // Vol II. Year is stored as text but all current values are 4-digit years,
    // so a numeric cast is safe and gives correct chronological ordering.
    let rows = sqlx::query_as::<_, BookEdition>(
        "SELECT * FROM book_editions WHERE book_id = $1 \
         ORDER BY NULLIF(year, '')::int DESC NULLS LAST, created_at",
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
    let status = input.status.as_deref().unwrap_or("published");
    sqlx::query(
        "INSERT INTO book_editions (id, book_id, edition_name, title, subtitle, lang, isbn, publisher, year, translators, purchase_links, cover_url, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(id)
    .bind(book_id)
    .bind(&input.edition_name)
    .bind(&input.title)
    .bind(&input.subtitle)
    .bind(&input.lang)
    .bind(&input.isbn)
    .bind(&input.publisher)
    .bind(&input.year)
    .bind(&input.translators)
    .bind(&links_json)
    .bind(&input.cover_url)
    .bind(status)
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
    // cover_url is managed by POST /books/{id}/editions/{eid}/cover, not by
    // this metadata-update path. Preserve it when the request omits it so
    // an unrelated edit can't silently orphan the file on disk. status is
    // likewise preserved when the caller omits it.
    sqlx::query(
        "UPDATE book_editions SET edition_name = $1, title = $2, subtitle = $3, lang = $4, isbn = $5, publisher = $6, \
         year = $7, translators = $8, purchase_links = $9, cover_url = COALESCE($10, cover_url), \
         status = COALESCE($11, status) \
         WHERE id = $12",
    )
    .bind(&input.edition_name)
    .bind(&input.title)
    .bind(&input.subtitle)
    .bind(&input.lang)
    .bind(&input.isbn)
    .bind(&input.publisher)
    .bind(&input.year)
    .bind(&input.translators)
    .bind(&links_json)
    .bind(&input.cover_url)
    .bind(&input.status)
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

async fn get_book_articles_by_category(
    pool: &PgPool,
    book_id: &str,
    category: &str,
    limit: i64,
    offset: i64,
) -> crate::Result<Vec<crate::models::Article>> {
    let rows = sqlx::query_as::<_, crate::models::Article>(&format!(
        "{ARTICLE_BASE} \
         WHERE a.book_id = $1 AND a.category = $2 AND a.visibility = 'public' \
         ORDER BY vote_score DESC, a.created_at DESC \
         LIMIT $3 OFFSET $4",
        ARTICLE_BASE = crate::services::article_service::ARTICLE_BASE,
    ))
    .bind(book_id)
    .bind(category)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Reviews — opinions on the book itself.
pub async fn get_book_reviews(
    pool: &PgPool, book_id: &str, limit: i64, offset: i64,
) -> crate::Result<Vec<crate::models::Article>> {
    get_book_articles_by_category(pool, book_id, "review", limit, offset).await
}

/// Notes — reader thoughts, supplementary derivations, knowledge
/// contributions attached to the book or one of its chapters.
pub async fn get_book_notes(
    pool: &PgPool, book_id: &str, limit: i64, offset: i64,
) -> crate::Result<Vec<crate::models::Article>> {
    get_book_articles_by_category(pool, book_id, "note", limit, offset).await
}

/// Articles (notes or questions) scoped to a specific chapter. The
/// `kind_or_category` argument is matched against either `articles.kind`
/// ("question", "answer") or `articles.category` ("note") — whichever fits
/// the caller. Reviews never attach to a chapter so they're not a valid
/// argument here.
pub async fn get_chapter_articles(
    pool: &PgPool,
    chapter_id: &str,
    kind_or_category: &str,
    limit: i64,
    offset: i64,
) -> crate::Result<Vec<crate::models::Article>> {
    let predicate = match kind_or_category {
        "question" | "answer" => "a.kind = $2::content_kind",
        "note"                => "a.category = $2",
        _ => return Err(crate::Error::BadRequest(
            format!("get_chapter_articles: unsupported kind_or_category '{kind_or_category}'"),
        )),
    };
    let sql = format!(
        "{ARTICLE_BASE} \
         WHERE a.book_chapter_id = $1 AND {predicate} AND a.visibility = 'public' \
         ORDER BY vote_score DESC, a.created_at DESC \
         LIMIT $3 OFFSET $4",
        ARTICLE_BASE = crate::services::article_service::ARTICLE_BASE,
    );
    let rows = sqlx::query_as::<_, crate::models::Article>(&sql)
        .bind(chapter_id)
        .bind(kind_or_category)
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

pub async fn unrate_book(pool: &PgPool, book_id: &str, user_did: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM book_ratings WHERE book_id = $1 AND user_did = $2")
        .bind(book_id).bind(user_did)
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
    #[ts(type = "Record<string, string>")]
    pub title_i18n: sqlx::types::Json<std::collections::HashMap<String, String>>,
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
pub struct ChapterAuthor {
    pub author_id: String,
    pub name: String,
    /// `author`, `translator`, `editor`. Authors default to "author"; the
    /// frontend can render a non-author role inline (e.g. "tr." chip).
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct BookChapterWithTags {
    pub id: String,
    pub book_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    #[ts(type = "Record<string, string>")]
    pub title_i18n: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub order_index: i32,
    pub article_uri: Option<String>,
    pub teaches: Vec<String>,
    pub prereqs: Vec<ChapterPrereq>,
    /// Authors credited at the chapter level (for edited volumes where each
    /// chapter has its own author roster). Populated from
    /// `book_chapter_authors`; empty for single-author books.
    pub authors: Vec<ChapterAuthor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateChapter {
    pub title: String,
    #[serde(default)]
    pub title_i18n: Option<std::collections::HashMap<String, String>>,
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
        // Edges are group-canonical — one row per (content_uri, group_id).
        "SELECT ct.content_uri, ct.tag_id \
         FROM content_teaches ct \
         WHERE ct.content_uri = ANY($1) \
         ORDER BY ct.content_uri, ct.tag_id",
    )
    .bind(&uris)
    .fetch_all(pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct PrereqRow { content_uri: String, tag_id: String, prereq_type: String }
    // Edges are group-canonical — one row per (content_uri, group_id, prereq_type).
    let prereq_rows = sqlx::query_as::<_, PrereqRow>(
        "SELECT cp.content_uri, cp.tag_id, cp.prereq_type \
         FROM content_prereqs cp \
         WHERE cp.content_uri = ANY($1) \
         ORDER BY cp.content_uri, cp.prereq_type, cp.tag_id",
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

    // Pull per-chapter authors in one query keyed by chapter_id, joined to
    // `authors` for the display name. Sorted by (chapter_id, position) so
    // the frontend can render contributors in the order the book lists them.
    #[derive(sqlx::FromRow)]
    struct AuthorRow { chapter_id: String, author_id: String, name: String, role: String }
    let author_rows = sqlx::query_as::<_, AuthorRow>(
        "SELECT bca.chapter_id, bca.author_id, a.name, bca.role \
         FROM book_chapter_authors bca \
         JOIN authors a ON a.id = bca.author_id \
         WHERE bca.chapter_id = ANY($1) \
         ORDER BY bca.chapter_id, bca.position",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    let mut authors_map: std::collections::HashMap<String, Vec<ChapterAuthor>> = std::collections::HashMap::new();
    for row in author_rows {
        authors_map.entry(row.chapter_id).or_default().push(ChapterAuthor {
            author_id: row.author_id,
            name: row.name,
            role: row.role,
        });
    }

    Ok(chapters.into_iter().map(|c| {
        let uri = format!("chapter:{}", c.id);
        BookChapterWithTags {
            teaches: teaches_map.remove(&uri).unwrap_or_default(),
            prereqs: prereqs_map.remove(&uri).unwrap_or_default(),
            authors: authors_map.remove(&c.id).unwrap_or_default(),
            id: c.id,
            book_id: c.book_id,
            parent_id: c.parent_id,
            title: c.title,
            title_i18n: c.title_i18n,
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

    let title_i18n: std::collections::HashMap<String, String> = input.title_i18n.clone().unwrap_or_else(|| {
        let mut m = std::collections::HashMap::new();
        m.insert("en".to_string(), input.title.clone());
        m
    });

    sqlx::query(
        "INSERT INTO book_chapters (id, book_id, parent_id, title, title_i18n, order_index, article_uri) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id).bind(book_id).bind(&input.parent_id)
    .bind(&input.title).bind(sqlx::types::Json(&title_i18n)).bind(input.order_index).bind(&input.article_uri)
    .execute(&mut *tx).await?;

    sqlx::query("INSERT INTO content (uri, content_type) VALUES ($1, 'chapter') ON CONFLICT DO NOTHING")
        .bind(&content_uri).execute(&mut *tx).await?;

    for input_ref in &input.teaches {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, created_by).await?;
        sqlx::query(
            "INSERT INTO content_teaches (content_uri, tag_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&content_uri).bind(&tag_id).execute(&mut *tx).await?;
    }
    for p in &input.prereqs {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, &p.tag_id, created_by).await?;
        sqlx::query(
            "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) \
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(&content_uri).bind(&tag_id).bind(&p.prereq_type).execute(&mut *tx).await?;
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
    for input_ref in teaches {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, created_by).await?;
        sqlx::query(
            "INSERT INTO content_teaches (content_uri, tag_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&content_uri).bind(&tag_id).execute(&mut *tx).await?;
    }

    // Replace prereqs
    sqlx::query("DELETE FROM content_prereqs WHERE content_uri = $1")
        .bind(&content_uri).execute(&mut *tx).await?;
    for p in prereqs {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, &p.tag_id, created_by).await?;
        sqlx::query(
            "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) \
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(&content_uri).bind(&tag_id).bind(&p.prereq_type).execute(&mut *tx).await?;
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

/// Result of toggling chapter progress. Includes the final reading status and
/// the full set of chapter ids whose progress was written — the toggled chapter
/// plus all of its descendants (since toggling a parent cascades to children).
pub struct ChapterProgressResult {
    pub status: Option<ReadingStatus>,
    pub affected_chapter_ids: Vec<String>,
}

pub async fn set_chapter_progress(
    pool: &PgPool,
    book_id: &str,
    chapter_id: &str,
    user_did: &str,
    completed: bool,
) -> crate::Result<ChapterProgressResult> {
    let mut tx = pool.begin().await?;

    // Toggle the chapter AND all its descendants. A parent-level check/uncheck
    // should propagate to every sub-chapter below it in the tree.
    let affected_chapter_ids: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE subtree(id) AS ( \
             SELECT id FROM book_chapters WHERE id = $1 \
             UNION ALL \
             SELECT c.id FROM book_chapters c JOIN subtree s ON c.parent_id = s.id \
         ) SELECT id FROM subtree",
    )
    .bind(chapter_id)
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO book_chapter_progress (book_id, chapter_id, user_did, completed, completed_at) \
         SELECT $1, c.id, $3, $4, CASE WHEN $4 THEN NOW() ELSE NULL END \
         FROM unnest($2::text[]) AS c(id) \
         ON CONFLICT (chapter_id, user_did) DO UPDATE SET completed = $4, \
         completed_at = CASE WHEN $4 THEN COALESCE(book_chapter_progress.completed_at, NOW()) ELSE NULL END",
    )
    .bind(book_id).bind(&affected_chapter_ids).bind(user_did).bind(completed)
    .execute(&mut *tx).await?;

    // Auto-transition into 'reading' when user completes a chapter from an
    // idle state. 'finished' stays (user already marked the book done) and
    // 'reading' stays (already there).
    if completed {
        sqlx::query(
            "INSERT INTO book_reading_status (book_id, user_did, status, progress) \
             VALUES ($1, $2, 'reading', 0) \
             ON CONFLICT (book_id, user_did) DO UPDATE SET \
               status = CASE WHEN book_reading_status.status IN ('want_to_read', 'dropped') \
                             THEN 'reading' ELSE book_reading_status.status END, \
               updated_at = NOW()",
        )
        .bind(book_id).bind(user_did)
        .execute(&mut *tx).await?;
    }

    // Recompute progress = completed_chapters / total_chapters * 100.
    // Only touch the row if the user already has one (don't create a status
    // row from an uncheck action).
    let exists: Option<String> = sqlx::query_scalar(
        "SELECT status FROM book_reading_status WHERE book_id = $1 AND user_did = $2"
    ).bind(book_id).bind(user_did).fetch_optional(&mut *tx).await?;

    if exists.is_some() {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM book_chapters WHERE book_id = $1"
        ).bind(book_id).fetch_one(&mut *tx).await?;
        let done: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM book_chapter_progress \
             WHERE book_id = $1 AND user_did = $2 AND completed = TRUE"
        ).bind(book_id).bind(user_did).fetch_one(&mut *tx).await?;
        let progress: i16 = if total > 0 { ((done * 100 / total) as i16).clamp(0, 100) } else { 0 };
        sqlx::query(
            "UPDATE book_reading_status SET progress = $1, updated_at = NOW() \
             WHERE book_id = $2 AND user_did = $3"
        ).bind(progress).bind(book_id).bind(user_did).execute(&mut *tx).await?;
    }

    let status = sqlx::query_as::<_, ReadingStatus>(
        "SELECT * FROM book_reading_status WHERE book_id = $1 AND user_did = $2"
    ).bind(book_id).bind(user_did).fetch_optional(&mut *tx).await?;

    tx.commit().await?;
    Ok(ChapterProgressResult { status, affected_chapter_ids })
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

// ---- Delete book / edition ---------------------------------------------

/// Delete a book and cascade-remove all child rows (editions, chapters,
/// resources, ratings, reviews). Writes a final `book_edit_log` entry
/// before deletion so the audit trail survives via the FK's
/// `ON DELETE SET NULL` — callers can still query history by
/// `original_book_id` after the book row is gone.
///
/// `editor_did` is the acting user's DID, recorded on the audit row.
pub async fn delete_book(pool: &PgPool, id: &str, editor_did: &str) -> crate::Result<()> {
    let mut tx = pool.begin().await?;

    // Snapshot the book so the audit row has something to show for it.
    let book_row: Option<(serde_json::Value, Vec<String>)> = sqlx::query_as(
        "SELECT title, authors FROM books WHERE id = $1"
    ).bind(id).fetch_optional(&mut *tx).await?;
    let (title_json, authors) = book_row
        .ok_or_else(|| crate::Error::NotFound { entity: "book", id: id.to_string() })?;

    let edit_id = format!("bel-{}", crate::util::tid());
    let snapshot = serde_json::json!({
        "title": title_json,
        "authors": authors,
    });
    sqlx::query(
        "INSERT INTO book_edit_log (id, book_id, original_book_id, editor_did, old_data, new_data, summary) \
         VALUES ($1, $2, $2, $3, $4, '{}'::jsonb, $5)",
    )
    .bind(&edit_id)
    .bind(id)
    .bind(editor_did)
    .bind(&snapshot)
    .bind("deleted book")
    .execute(&mut *tx).await?;

    sqlx::query("DELETE FROM books WHERE id = $1")
        .bind(id).execute(&mut *tx).await?;
    // books.id doesn't FK into content.uri, so clear the content row too.
    let content_uri = format!("book:{id}");
    sqlx::query("DELETE FROM content WHERE uri = $1")
        .bind(&content_uri).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Delete a single edition by id. Cascades to any FK children (short
/// reviews via ON DELETE SET NULL, purchase links, covers). Refuses to
/// delete an edition if it's the only edition on the book — callers
/// should hard-delete the whole book instead.
pub async fn delete_edition(pool: &PgPool, book_id: &str, edition_id: &str) -> crate::Result<()> {
    let mut tx = pool.begin().await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM book_editions WHERE book_id = $1")
        .bind(book_id).fetch_one(&mut *tx).await?;
    if count == 0 {
        return Err(crate::Error::NotFound { entity: "book_edition", id: edition_id.to_string() });
    }
    if count == 1 {
        return Err(crate::Error::BadRequest(
            "cannot delete the only edition on a book — hard-delete the whole book instead".into(),
        ));
    }
    let res = sqlx::query("DELETE FROM book_editions WHERE id = $1 AND book_id = $2")
        .bind(edition_id).bind(book_id).execute(&mut *tx).await?;
    if res.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "book_edition", id: edition_id.to_string() });
    }
    // If we deleted the book's default_edition_id, clear it (any remaining
    // edition will be used as fallback by the display layer).
    sqlx::query("UPDATE books SET default_edition_id = NULL \
                  WHERE id = $1 AND default_edition_id = $2")
        .bind(book_id).bind(edition_id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
