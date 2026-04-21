use std::collections::HashMap;

use serde::Serialize;
use sqlx::PgPool;

use crate::models::Article;
use crate::region::{InstanceMode, visibility_filter};

/// A skill on the user's learning frontier (prereqs met, not yet mastered).
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct FrontierSkill {
    pub tag_id: String,
    pub tag_name: String,
    #[ts(type = "Record<string, string>")]
    pub tag_names: sqlx::types::Json<HashMap<String, String>>,
    pub article_count: i64,
}

/// Personalized article recommendations using a multi-signal scoring algorithm.
///
/// Signals:
///   1. Quality    — Wilson score lower bound (conservative like ratio) + engagement rate
///   2. Trending   — Recent views/votes/bookmarks/comments with time decay
///   3. Readiness  — Fraction of required prereqs satisfied by user's mastered skills
///   4. Frontier   — Article teaches a skill on user's learning frontier (highest boost)
///   5. Interest   — Article teaches a tag matching user's declared interests
///   6. Social     — Article by a followed user, or upvoted by followed users
///   7. Author rep — Author's average article quality score
///
/// For anonymous users (viewer_did = None), user-dependent signals are zero,
/// naturally degrading to quality + trending (popular content).
pub async fn get_recommendations(
    pool: &PgPool,
    mode: InstanceMode,
    viewer_did: Option<&str>,
    limit: i64,
    offset: i64,
    category: Option<&str>,
) -> crate::Result<Vec<Article>> {
    let did = viewer_did.unwrap_or("");
    let vis = visibility_filter(mode);

    // Build the category filter clause
    let cat_filter = if category.is_some() {
        "AND a.category = $5"
    } else {
        "AND ($5::VARCHAR IS NULL OR TRUE)"
    };

    let sql = format!(
        r#"
WITH
-- ══════════════════════════════════════════════════════════════
-- User context CTEs (empty for anonymous → graceful degradation)
-- ══════════════════════════════════════════════════════════════
user_mastered AS (
    -- user_skills is tag-level, so a single row covers every language
    -- label of the same concept (lit "Calculus" = lit "高等数学").
    SELECT tag_id FROM user_skills WHERE did = $1 AND status = 'mastered'
),
user_interest_descendants AS (
    WITH RECURSIVE desc_tags(tag) AS (
        SELECT tag_id FROM user_interests WHERE did = $1
        UNION
        SELECT tp.child_tag FROM tag_parents tp
        JOIN desc_tags d ON tp.parent_tag = d.tag
    )
    SELECT tag FROM desc_tags
),
user_followed AS (
    SELECT follows_did FROM user_follows WHERE did = $1
),
user_learned AS (
    SELECT article_uri FROM learned_marks WHERE did = $1
),

-- Frontier: tags whose required prereqs are ALL mastered, but tag itself is NOT mastered
frontier_tags AS (
    SELECT DISTINCT utp.to_tag AS tag_id
    FROM user_tag_prereqs utp
    WHERE utp.did = $1
      AND utp.prereq_type = 'required'
      AND NOT EXISTS (
          SELECT 1 FROM user_skills us
          WHERE us.did = $1 AND us.tag_id = utp.to_tag AND us.status = 'mastered'
      )
      AND NOT EXISTS (
          SELECT 1 FROM user_tag_prereqs blocker
          WHERE blocker.did = $1
            AND blocker.to_tag = utp.to_tag
            AND blocker.prereq_type = 'required'
            AND NOT EXISTS (
                SELECT 1 FROM user_mastered um WHERE um.tag_id = blocker.from_tag
            )
      )
),

-- Language preference
lang_settings AS (
    SELECT did, native_lang, known_langs, prefer_native, hide_unknown
    FROM user_settings WHERE did = $1
),

-- ══════════════════════════════════════════════════════════════
-- Article statistics (aggregated once)
-- ══════════════════════════════════════════════════════════════
article_stats AS (
    SELECT
        a.at_uri,
        COALESCE(v.up, 0)     AS upvotes,
        COALESCE(v.down, 0)   AS downvotes,
        COALESCE(vc.cnt, 0)   AS view_count,
        COALESCE(vc24.cnt, 0) AS views_24h,
        COALESCE(v7.cnt, 0)   AS votes_7d,
        COALESCE(bk.cnt, 0)   AS bookmark_count,
        COALESCE(bk7.cnt, 0)  AS bookmarks_7d,
        COALESCE(cm.cnt, 0)   AS comment_count,
        COALESCE(cm7.cnt, 0)  AS comments_7d
    FROM articles a
    LEFT JOIN (
        SELECT target_uri,
               COUNT(*) FILTER (WHERE value > 0) AS up,
               COUNT(*) FILTER (WHERE value < 0) AS down
        FROM votes GROUP BY target_uri
    ) v ON v.target_uri = a.at_uri
    LEFT JOIN (
        SELECT article_uri, COUNT(*) AS cnt FROM article_views GROUP BY article_uri
    ) vc ON vc.article_uri = a.at_uri
    LEFT JOIN (
        SELECT article_uri, COUNT(*) AS cnt FROM article_views
        WHERE viewed_at > NOW() - INTERVAL '24 hours'
        GROUP BY article_uri
    ) vc24 ON vc24.article_uri = a.at_uri
    LEFT JOIN (
        SELECT target_uri, COUNT(*) AS cnt FROM votes
        WHERE voted_at > NOW() - INTERVAL '7 days'
        GROUP BY target_uri
    ) v7 ON v7.target_uri = a.at_uri
    LEFT JOIN (
        SELECT article_uri, COUNT(*) AS cnt FROM user_bookmarks GROUP BY article_uri
    ) bk ON bk.article_uri = a.at_uri
    LEFT JOIN (
        SELECT article_uri, COUNT(*) AS cnt FROM user_bookmarks
        WHERE created_at > NOW() - INTERVAL '7 days'
        GROUP BY article_uri
    ) bk7 ON bk7.article_uri = a.at_uri
    LEFT JOIN (
        SELECT content_uri, COUNT(*) AS cnt FROM comments GROUP BY content_uri
    ) cm ON cm.content_uri = a.at_uri
    LEFT JOIN (
        SELECT content_uri, COUNT(*) AS cnt FROM comments
        WHERE created_at > NOW() - INTERVAL '7 days'
        GROUP BY content_uri
    ) cm7 ON cm7.content_uri = a.at_uri
    WHERE {vis} AND a.kind IN ('article', 'question', 'answer')
),

-- Per-article prereq readiness
article_readiness AS (
    SELECT
        cp.content_uri,
        COUNT(*) FILTER (WHERE cp.prereq_type = 'required') AS total_required,
        COUNT(*) FILTER (WHERE cp.prereq_type = 'required'
                         AND EXISTS (SELECT 1 FROM user_mastered um WHERE um.tag_id = cp.tag_id))
            AS satisfied_required
    FROM content_prereqs cp
    GROUP BY cp.content_uri
),

-- Author reputation: average Wilson score across their articles
author_rep AS (
    SELECT a.did,
        AVG(
            CASE WHEN (COALESCE(s.upvotes, 0) + COALESCE(s.downvotes, 0)) = 0 THEN 0.5
            ELSE
                (s.upvotes + 1.9208) / (s.upvotes + s.downvotes + 3.8416)
                - 1.96 * SQRT(
                    (s.upvotes::float * s.downvotes::float) / (s.upvotes + s.downvotes + 3.8416) + 0.9604
                ) / (s.upvotes + s.downvotes + 3.8416)
            END
        ) AS avg_wilson
    FROM articles a
    LEFT JOIN article_stats s ON s.at_uri = a.at_uri
    WHERE a.kind = 'article'
    GROUP BY a.did
),

-- Translation dedup: pick best language per translation group (articles only;
-- questions and answers fall through one-per-uri).
best_translation AS (
    SELECT DISTINCT ON (
        CASE WHEN a.kind = 'article' THEN COALESCE(a.translation_group, a.at_uri)
             ELSE a.at_uri
        END
    )
        a.at_uri
    FROM articles a
    LEFT JOIN lang_settings ls ON TRUE
    WHERE {vis} AND a.kind IN ('article', 'question', 'answer')
      -- Exclude series chapters (articles only have these)
      AND NOT EXISTS (SELECT 1 FROM series_articles sa WHERE sa.article_uri = a.at_uri)
      -- Exclude learned
      AND NOT EXISTS (SELECT 1 FROM user_learned ul WHERE ul.article_uri = a.at_uri)
      -- Exclude restricted without access
      AND (NOT a.restricted OR a.did = $1
           OR EXISTS (SELECT 1 FROM article_access_grants g
                      WHERE g.article_uri = a.at_uri AND g.grantee_did = $1)
           OR EXISTS (SELECT 1 FROM user_members m
                      WHERE m.author_did = a.did AND m.member_did = $1))
      -- Language filter: hide unknown if user requested
      AND (ls.did IS NULL
           OR NOT COALESCE(ls.hide_unknown, FALSE)
           OR a.lang = ls.native_lang
           OR a.lang = ANY(
               SELECT jsonb_array_elements_text(ls.known_langs)
           ))
    ORDER BY
        CASE WHEN a.kind = 'article' THEN COALESCE(a.translation_group, a.at_uri)
             ELSE a.at_uri
        END,
        CASE
            WHEN ls.native_lang IS NOT NULL AND a.lang = ls.native_lang THEN 0
            WHEN a.lang = ANY(SELECT jsonb_array_elements_text(COALESCE(ls.known_langs, '[]'::jsonb))) THEN 1
            ELSE 2
        END,
        a.created_at DESC
),

-- ══════════════════════════════════════════════════════════════
-- Final scoring
-- ══════════════════════════════════════════════════════════════
scored AS (
    SELECT
        s.at_uri,

        -- 1. Quality: Wilson score + engagement rate
        CASE WHEN (s.upvotes + s.downvotes) = 0 THEN 0.3
        ELSE GREATEST(0.0,
            (s.upvotes + 1.9208) / (s.upvotes + s.downvotes + 3.8416)
            - 1.96 * SQRT(
                (s.upvotes::float * s.downvotes::float) / (s.upvotes + s.downvotes + 3.8416) + 0.9604
            ) / (s.upvotes + s.downvotes + 3.8416)
        )
        END * 0.6
        + LEAST(1.0,
            CASE WHEN s.view_count = 0 THEN 0.0
            ELSE (s.upvotes + s.downvotes + s.bookmark_count + s.comment_count)::float
                 / s.view_count::float
            END
        ) * 0.4
        AS quality_score,

        -- 2. Trending: multi-window engagement velocity with time decay
        (s.views_24h * 1.0 + s.votes_7d * 3.0 + s.bookmarks_7d * 4.0 + s.comments_7d * 2.0)
        / POWER(EXTRACT(EPOCH FROM (NOW() - a.created_at)) / 3600.0 + 2.0, 1.2)
        AS trending_score,

        -- 3. Readiness: fraction of required prereqs mastered
        CASE WHEN COALESCE(ar.total_required, 0) = 0 THEN 1.0
             ELSE ar.satisfied_required::float / ar.total_required::float
        END AS readiness,

        -- 4. Frontier bonus
        CASE WHEN EXISTS (
            SELECT 1 FROM content_teaches ct
            JOIN frontier_tags ft ON ft.tag_id = ct.tag_id
            WHERE ct.content_uri = a.at_uri
        ) THEN 1.0 ELSE 0.0 END AS frontier_bonus,

        -- 5. Interest match
        CASE WHEN EXISTS (
            SELECT 1 FROM content_teaches ct
            JOIN user_interest_descendants uid ON uid.tag = ct.tag_id
            WHERE ct.content_uri = a.at_uri
        ) THEN 1.0 ELSE 0.0 END AS interest_bonus,

        -- 6. Social signal
        CASE WHEN a.did IN (SELECT follows_did FROM user_followed) THEN 0.7 ELSE 0.0 END
        + CASE WHEN EXISTS (
            SELECT 1 FROM votes vf
            JOIN user_followed uf ON vf.did = uf.follows_did
            WHERE vf.target_uri = a.at_uri AND vf.value > 0
        ) THEN 0.3 ELSE 0.0 END
        AS social_signal,

        -- 7. Author reputation
        COALESCE(arep.avg_wilson, 0.3) AS author_rep,

        -- For penalty calculation
        a.prereq_threshold

    FROM article_stats s
    JOIN articles a ON a.at_uri = s.at_uri
    JOIN best_translation bt ON bt.at_uri = a.at_uri
    LEFT JOIN article_readiness ar ON ar.content_uri = a.at_uri
    LEFT JOIN author_rep arep ON arep.did = a.did
    WHERE TRUE {cat_filter}
)

-- Final SELECT matching Article struct field order
SELECT a.at_uri, a.did, p.handle AS author_handle, p.display_name AS author_display_name, p.avatar_url AS author_avatar, COALESCE(p.reputation, 0) AS author_reputation,
       a.kind, a.title, a.summary, a.cover_url,
       pm.venue AS paper_venue, pm.year AS paper_year, pm.accepted AS paper_accepted,
       a.content_hash, a.content_format, a.lang, a.translation_group, a.license,
       a.prereq_threshold, a.question_uri, a.answer_count, a.restricted, a.category,
       a.book_id, a.edition_id, a.book_chapter_id, a.course_session_id,
       COALESCE(vs.score, 0) AS vote_score,
       COALESCE(bk.cnt, 0)  AS bookmark_count,
       COALESCE(cm.cnt, 0)  AS comment_count,
       COALESCE(fk.cnt, 0)  AS fork_count,
       a.created_at, a.updated_at
FROM scored sc
JOIN articles a ON a.at_uri = sc.at_uri
LEFT JOIN profiles p ON a.did = p.did
LEFT JOIN paper_metadata pm ON pm.article_uri = a.at_uri
LEFT JOIN (SELECT target_uri, SUM(value) AS score FROM votes GROUP BY target_uri) vs
    ON vs.target_uri = a.at_uri
LEFT JOIN (SELECT article_uri, COUNT(*) AS cnt FROM user_bookmarks GROUP BY article_uri) bk
    ON bk.article_uri = a.at_uri
LEFT JOIN (SELECT content_uri, COUNT(*) AS cnt FROM comments GROUP BY content_uri) cm
    ON cm.content_uri = a.at_uri
LEFT JOIN (SELECT source_uri, COUNT(*) AS cnt FROM forks GROUP BY source_uri) fk
    ON fk.source_uri = a.at_uri
ORDER BY
    (sc.quality_score   * 2.0)
  + (sc.trending_score  * 1.5)
  + (sc.readiness       * 3.0)
  + (sc.frontier_bonus  * 8.0)
  + (sc.interest_bonus  * 4.0)
  + (sc.social_signal   * 2.0)
  + (sc.author_rep      * 1.0)
  + (CASE WHEN sc.readiness < sc.prereq_threshold THEN -5.0 ELSE 0.0 END)
  DESC
LIMIT $3 OFFSET $4
"#,
        vis = vis,
        cat_filter = cat_filter,
    );

    let rows = sqlx::query_as::<_, Article>(&sql)
        .bind(did)           // $1: viewer DID (empty string for anonymous)
        .bind(did)           // $2: repeated for subqueries (not used, placeholder)
        .bind(limit)         // $3
        .bind(offset)        // $4
        .bind(category)      // $5
        .fetch_all(pool)
        .await?;

    Ok(rows)
}

/// Recommended questions for the sidebar.
///
/// Scoring favours questions the user can likely answer (mastered the question's
/// tags), with a boost for unanswered / low-answer questions, plus quality and
/// recency signals.  Anonymous users get trending unanswered questions.
pub async fn get_recommended_questions(
    pool: &PgPool,
    mode: InstanceMode,
    viewer_did: Option<&str>,
    limit: i64,
) -> crate::Result<Vec<Article>> {
    let did = viewer_did.unwrap_or("");
    let vis = visibility_filter(mode);

    let sql = format!(
        r#"
WITH
user_mastered AS (
    SELECT tag_id FROM user_skills WHERE did = $1 AND status = 'mastered'
),
questions AS (
    SELECT a.at_uri, a.did, p.handle AS author_handle, p.display_name AS author_display_name, p.avatar_url AS author_avatar, COALESCE(p.reputation, 0) AS author_reputation,
           a.kind, a.title, a.summary, a.cover_url,
           a.content_hash, a.content_format, a.lang, a.translation_group, a.license,
           a.prereq_threshold, a.question_uri, a.answer_count, a.restricted, a.category,
           a.book_id, a.edition_id, a.book_chapter_id, a.course_session_id,
           COALESCE(v.score, 0) AS vote_score,
           COALESCE(bk.cnt, 0) AS bookmark_count,
           COALESCE(cm.cnt, 0) AS comment_count,
           COALESCE(fk.cnt, 0) AS fork_count,
           a.created_at, a.updated_at
    FROM articles a
    LEFT JOIN profiles p ON a.did = p.did
    LEFT JOIN (SELECT target_uri, SUM(value) AS score FROM votes GROUP BY target_uri) v
        ON v.target_uri = a.at_uri
    LEFT JOIN (SELECT article_uri, COUNT(*) AS cnt FROM user_bookmarks GROUP BY article_uri) bk
        ON bk.article_uri = a.at_uri
    LEFT JOIN (SELECT content_uri, COUNT(*) AS cnt FROM comments GROUP BY content_uri) cm
        ON cm.content_uri = a.at_uri
    LEFT JOIN (SELECT source_uri, COUNT(*) AS cnt FROM forks GROUP BY source_uri) fk
        ON fk.source_uri = a.at_uri
    WHERE {vis} AND a.kind = 'question'
),
scored AS (
    SELECT q.*,
        -- 1. Answerability: fraction of question's teaches-tags that user has mastered
        CASE WHEN ct_total.cnt IS NULL OR ct_total.cnt = 0 THEN 0.5
             ELSE COALESCE(ct_mastered.cnt, 0)::float / ct_total.cnt::float
        END AS answerability,
        -- 2. Need: fewer answers → higher need (1.0 for 0 answers, decays)
        1.0 / (q.answer_count + 1.0) AS answer_need,
        -- 3. Recency: time decay
        1.0 / POWER(EXTRACT(EPOCH FROM (NOW() - q.created_at)) / 3600.0 + 2.0, 0.8) AS recency,
        -- 4. Quality: vote score (clamped)
        LEAST(1.0, GREATEST(0.0, q.vote_score::float / 10.0)) AS quality
    FROM questions q
    LEFT JOIN (
        SELECT ct.content_uri, COUNT(*) AS cnt FROM content_teaches ct GROUP BY ct.content_uri
    ) ct_total ON ct_total.content_uri = q.at_uri
    LEFT JOIN (
        SELECT ct.content_uri, COUNT(*) AS cnt FROM content_teaches ct
        JOIN user_mastered um ON um.tag_id = ct.tag_id
        GROUP BY ct.content_uri
    ) ct_mastered ON ct_mastered.content_uri = q.at_uri
    -- Exclude user's own questions
    WHERE q.did != $1
)
SELECT at_uri, did, author_handle, author_display_name, author_avatar, author_reputation, kind, title, summary, cover_url,
       content_hash, content_format, lang, translation_group, license,
       prereq_threshold, question_uri, answer_count, restricted, category,
       book_id, edition_id, vote_score, bookmark_count, comment_count, fork_count, created_at, updated_at
FROM scored
ORDER BY
    (answerability * 6.0)
  + (answer_need   * 4.0)
  + (recency       * 2.0)
  + (quality       * 1.5)
  DESC
LIMIT $2
"#,
        vis = vis,
    );

    let rows = sqlx::query_as::<_, Article>(&sql)
        .bind(did)
        .bind(limit)
        .fetch_all(pool)
        .await?;

    Ok(rows)
}

/// Returns skills on the user's learning frontier: tags whose required
/// prerequisites are all mastered, but the tag itself is not yet mastered.
pub async fn get_frontier_skills(pool: &PgPool, did: &str) -> crate::Result<Vec<FrontierSkill>> {
    let rows = sqlx::query_as::<_, FrontierSkill>(
        r#"
WITH user_mastered AS (
    SELECT tag_id FROM user_skills WHERE did = $1 AND status = 'mastered'
),
frontier_tags AS (
    SELECT DISTINCT utp.to_tag AS tag_id
    FROM user_tag_prereqs utp
    WHERE utp.did = $1
      AND utp.prereq_type = 'required'
      AND NOT EXISTS (
          SELECT 1 FROM user_skills us
          WHERE us.did = $1 AND us.tag_id = utp.to_tag AND us.status = 'mastered'
      )
      AND NOT EXISTS (
          SELECT 1 FROM user_tag_prereqs blocker
          WHERE blocker.did = $1
            AND blocker.to_tag = utp.to_tag
            AND blocker.prereq_type = 'required'
            AND NOT EXISTS (
                SELECT 1 FROM user_mastered um WHERE um.tag_id = blocker.from_tag
            )
      )
)
SELECT tag_canonical_label(ft.tag_id) AS tag_id,
       tag_canonical_label(ft.tag_id) AS tag_name,
       tag_label_map(ft.tag_id) AS tag_names,
       COALESCE(ct.cnt, 0) AS article_count
FROM frontier_tags ft
LEFT JOIN (
    SELECT tag_id, COUNT(*) AS cnt FROM content_teaches GROUP BY tag_id
) ct ON ct.tag_id = ft.tag_id
ORDER BY ct.cnt DESC
"#,
    )
    .bind(did)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
