use sqlx::PgPool;

use crate::models::{ArticlePrereq, Draft, SaveDraft, UpdateDraft};
use crate::Result;

pub async fn list_drafts(pool: &PgPool, did: &str) -> Result<Vec<Draft>> {
    let drafts = sqlx::query_as::<_, Draft>(
        "SELECT id, did, title, summary, content, content_format, lang, license, \
         tags, prereqs, at_uri, created_at, updated_at \
         FROM drafts WHERE did = $1 ORDER BY updated_at DESC",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(drafts)
}

pub async fn get_draft(pool: &PgPool, id: &str) -> Result<Draft> {
    sqlx::query_as::<_, Draft>(
        "SELECT id, did, title, summary, content, content_format, lang, license, \
         tags, prereqs, at_uri, created_at, updated_at \
         FROM drafts WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::Error::NotFound {
        entity: "draft",
        id: id.to_string(),
    })
}

pub async fn save_draft(
    pool: &PgPool,
    id: &str,
    did: &str,
    input: &SaveDraft,
) -> Result<Draft> {
    let tags_json = serde_json::to_string(&input.tags).unwrap_or_else(|_| "[]".into());
    let prereqs_json = serde_json::to_string(&input.prereqs).unwrap_or_else(|_| "[]".into());
    let lang = input.lang.as_deref().unwrap_or("zh");
    let license = input.license.as_deref().unwrap_or("CC-BY-SA-4.0");

    sqlx::query(
        "INSERT INTO drafts (id, did, title, summary, content, content_format, lang, license, tags, prereqs)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(id)
    .bind(did)
    .bind(&input.title)
    .bind(input.summary.as_deref().unwrap_or(""))
    .bind(&input.content)
    .bind(input.content_format)
    .bind(lang)
    .bind(license)
    .bind(&tags_json)
    .bind(&prereqs_json)
    .execute(pool)
    .await?;

    get_draft(pool, id).await
}

pub async fn update_draft(pool: &PgPool, did: &str, input: &UpdateDraft) -> Result<Draft> {
    let owner = get_draft_owner(pool, &input.id).await?;
    if owner != did {
        return Err(crate::Error::Forbidden {
            action: "update draft owned by another user",
        });
    }

    let mut tx = pool.begin().await?;

    if let Some(ref title) = input.title {
        sqlx::query("UPDATE drafts SET title = $1, updated_at = NOW() WHERE id = $2")
            .bind(title)
            .bind(&input.id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(ref desc) = input.summary {
        sqlx::query("UPDATE drafts SET summary = $1, updated_at = NOW() WHERE id = $2")
            .bind(desc)
            .bind(&input.id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(ref content) = input.content {
        sqlx::query("UPDATE drafts SET content = $1, updated_at = NOW() WHERE id = $2")
            .bind(content)
            .bind(&input.id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(fmt) = input.content_format {
        sqlx::query("UPDATE drafts SET content_format = $1, updated_at = NOW() WHERE id = $2")
            .bind(fmt)
            .bind(&input.id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(ref lang) = input.lang {
        sqlx::query("UPDATE drafts SET lang = $1, updated_at = NOW() WHERE id = $2")
            .bind(lang)
            .bind(&input.id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(ref license) = input.license {
        sqlx::query("UPDATE drafts SET license = $1, updated_at = NOW() WHERE id = $2")
            .bind(license)
            .bind(&input.id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(ref tags) = input.tags {
        let json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".into());
        sqlx::query("UPDATE drafts SET tags = $1, updated_at = NOW() WHERE id = $2")
            .bind(&json)
            .bind(&input.id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(ref prereqs) = input.prereqs {
        let json = serde_json::to_string(prereqs).unwrap_or_else(|_| "[]".into());
        sqlx::query("UPDATE drafts SET prereqs = $1, updated_at = NOW() WHERE id = $2")
            .bind(&json)
            .bind(&input.id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    get_draft(pool, &input.id).await
}

pub async fn delete_draft(pool: &PgPool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM drafts WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_draft_at_uri(pool: &PgPool, id: &str, at_uri: &str) -> Result<()> {
    sqlx::query("UPDATE drafts SET at_uri = $1 WHERE id = $2")
        .bind(at_uri)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_draft_owner(pool: &PgPool, id: &str) -> Result<String> {
    sqlx::query_scalar::<_, String>("SELECT did FROM drafts WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound {
            entity: "draft",
            id: id.to_string(),
        })
}

/// Publish a draft: create the article + tags + prereqs, delete the draft, all in one transaction.
/// Returns the tags and prereqs parsed from the draft for AT Protocol syncing.
pub async fn publish_to_article(
    pool: &PgPool,
    draft: &Draft,
    at_uri: &str,
    content_hash: &str,
    default_visibility: &str,
) -> Result<(Vec<String>, Vec<ArticlePrereq>)> {
    let tags: Vec<String> = serde_json::from_str(&draft.tags).unwrap_or_default();
    let prereqs: Vec<ArticlePrereq> = serde_json::from_str(&draft.prereqs).unwrap_or_default();

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO articles (at_uri, did, title, summary, content_hash, content_format, lang, license, prereq_threshold, visibility, kind)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0.8, $9, $10)",
    )
    .bind(at_uri)
    .bind(&draft.did)
    .bind(&draft.title)
    .bind(&draft.summary)
    .bind(content_hash)
    .bind(draft.content_format)
    .bind(&draft.lang)
    .bind(&draft.license)
    .bind(default_visibility)
    .bind(crate::content::ContentKind::Article)
    .execute(&mut *tx)
    .await?;

    for tag_id in &tags {
        sqlx::query("INSERT INTO tag_labels (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING")
            .bind(tag_id)
            .bind(tag_id)
            .bind(&draft.did)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(at_uri)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
    }

    for prereq in &prereqs {
        sqlx::query(
            "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(at_uri)
        .bind(&prereq.tag_id)
        .bind(prereq.prereq_type.as_str())
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("DELETE FROM drafts WHERE id = $1")
        .bind(&draft.id)
        .execute(&mut *tx)
        .await?;

    // Auto-bookmark
    sqlx::query(
        "INSERT INTO user_bookmarks (did, article_uri, folder_path) VALUES ($1, $2, '我的文章') ON CONFLICT DO NOTHING",
    )
    .bind(&draft.did)
    .bind(at_uri)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((tags, prereqs))
}
