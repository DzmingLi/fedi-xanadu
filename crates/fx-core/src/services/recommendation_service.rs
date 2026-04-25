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
user_mastered AS (
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
    SELECT repo_uri, source_path FROM learned_marks WHERE did = $1
),
frontier_tags AS (
    SELECT DISTINCT utp.to_tag AS tag_id
    FROM user_tag_prereqs utp
    WHERE utp.did = $1 AND utp.prereq_type = 'required'
      AND NOT EXISTS (
          SELECT 1 FROM user_skills us
          WHERE us.did = $1 AND us.tag_id = utp.to_tag AND us.status = 'mastered'
      )
      AND NOT EXISTS (
          SELECT 1 FROM user_tag_prereqs blocker
          WHERE blocker.did = $1 AND blocker.to_tag = utp.to_tag
            AND blocker.prereq_type = 'required'
            AND NOT EXISTS (
                SELECT 1 FROM user_mastered um WHERE um.tag_id = blocker.from_tag
            )
      )
),
lang_settings AS (
    SELECT did, native_lang, known_langs, prefer_native, hide_unknown
    FROM user_settings WHERE did = $1
),

-- Article statistics keyed by composite article key + synthetic URI
article_stats AS (
    SELECT
        a.repo_uri, a.source_path,
        article_uri(a.repo_uri, a.source_path) AS synth_uri,
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
    ) v ON v.target_uri = article_uri(a.repo_uri, a.source_path)
    LEFT JOIN (
        SELECT repo_uri, source_path, COUNT(*) AS cnt FROM article_views
        GROUP BY repo_uri, source_path
    ) vc ON vc.repo_uri = a.repo_uri AND vc.source_path = a.source_path
    LEFT JOIN (
        SELECT repo_uri, source_path, COUNT(*) AS cnt FROM article_views
        WHERE viewed_at > NOW() - INTERVAL '24 hours'
        GROUP BY repo_uri, source_path
    ) vc24 ON vc24.repo_uri = a.repo_uri AND vc24.source_path = a.source_path
    LEFT JOIN (
        SELECT target_uri, COUNT(*) AS cnt FROM votes
        WHERE voted_at > NOW() - INTERVAL '7 days'
        GROUP BY target_uri
    ) v7 ON v7.target_uri = article_uri(a.repo_uri, a.source_path)
    LEFT JOIN (
        SELECT repo_uri, source_path, COUNT(*) AS cnt FROM user_bookmarks
        GROUP BY repo_uri, source_path
    ) bk ON bk.repo_uri = a.repo_uri AND bk.source_path = a.source_path
    LEFT JOIN (
        SELECT repo_uri, source_path, COUNT(*) AS cnt FROM user_bookmarks
        WHERE created_at > NOW() - INTERVAL '7 days'
        GROUP BY repo_uri, source_path
    ) bk7 ON bk7.repo_uri = a.repo_uri AND bk7.source_path = a.source_path
    LEFT JOIN (
        SELECT content_uri, COUNT(*) AS cnt FROM comments GROUP BY content_uri
    ) cm ON cm.content_uri = article_uri(a.repo_uri, a.source_path)
    LEFT JOIN (
        SELECT content_uri, COUNT(*) AS cnt FROM comments
        WHERE created_at > NOW() - INTERVAL '7 days'
        GROUP BY content_uri
    ) cm7 ON cm7.content_uri = article_uri(a.repo_uri, a.source_path)
    WHERE {vis} AND a.kind IN ('article', 'answer')
),

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

author_rep AS (
    SELECT a.author_did,
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
    LEFT JOIN article_stats s ON s.repo_uri = a.repo_uri AND s.source_path = a.source_path
    WHERE a.kind = 'article'
    GROUP BY a.author_did
),

-- Eligible articles: visible, not a series chapter, not yet learned,
-- access-allowed, language-filter satisfied. In the new schema each article
-- has exactly one row in `articles` so no translation-group dedup is needed.
eligible AS (
    SELECT a.repo_uri, a.source_path
    FROM articles a
    JOIN article_localizations l
        ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path
       AND l.file_path = a.source_path
    LEFT JOIN lang_settings ls ON TRUE
    WHERE {vis} AND a.kind IN ('article', 'answer')
      AND NOT EXISTS (
          SELECT 1 FROM series_articles sa
          WHERE sa.repo_uri = a.repo_uri AND sa.source_path = a.source_path
      )
      AND NOT EXISTS (
          SELECT 1 FROM user_learned ul
          WHERE ul.repo_uri = a.repo_uri AND ul.source_path = a.source_path
      )
      AND (NOT a.restricted OR a.author_did = $1
           OR EXISTS (SELECT 1 FROM article_access_grants g
                      WHERE g.repo_uri = a.repo_uri AND g.source_path = a.source_path
                        AND g.grantee_did = $1)
           OR EXISTS (SELECT 1 FROM user_members m
                      WHERE m.author_did = a.author_did AND m.member_did = $1))
      AND (ls.did IS NULL
           OR NOT COALESCE(ls.hide_unknown, FALSE)
           OR l.lang = ls.native_lang
           OR l.lang = ANY(SELECT jsonb_array_elements_text(ls.known_langs)))
),

scored AS (
    SELECT
        a.repo_uri, a.source_path,
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

        (s.views_24h * 1.0 + s.votes_7d * 3.0 + s.bookmarks_7d * 4.0 + s.comments_7d * 2.0)
        / POWER(EXTRACT(EPOCH FROM (NOW() - a.created_at)) / 3600.0 + 2.0, 1.2)
        AS trending_score,

        CASE WHEN COALESCE(ar.total_required, 0) = 0 THEN 1.0
             ELSE ar.satisfied_required::float / ar.total_required::float
        END AS readiness,

        CASE WHEN EXISTS (
            SELECT 1 FROM content_teaches ct
            JOIN frontier_tags ft ON ft.tag_id = ct.tag_id
            WHERE ct.content_uri = s.synth_uri
        ) THEN 1.0 ELSE 0.0 END AS frontier_bonus,

        CASE WHEN EXISTS (
            SELECT 1 FROM content_teaches ct
            JOIN user_interest_descendants uid ON uid.tag = ct.tag_id
            WHERE ct.content_uri = s.synth_uri
        ) THEN 1.0 ELSE 0.0 END AS interest_bonus,

        CASE WHEN a.author_did IN (SELECT follows_did FROM user_followed) THEN 0.7 ELSE 0.0 END
        + CASE WHEN EXISTS (
            SELECT 1 FROM votes vf
            JOIN user_followed uf ON vf.did = uf.follows_did
            WHERE vf.target_uri = s.synth_uri AND vf.value > 0
        ) THEN 0.3 ELSE 0.0 END
        AS social_signal,

        COALESCE(arep.avg_wilson, 0.3) AS author_rep,
        a.prereq_threshold
    FROM article_stats s
    JOIN articles a ON a.repo_uri = s.repo_uri AND a.source_path = s.source_path
    JOIN eligible e  ON e.repo_uri = a.repo_uri AND e.source_path = a.source_path
    LEFT JOIN article_readiness ar ON ar.content_uri = s.synth_uri
    LEFT JOIN author_rep arep ON arep.author_did = a.author_did
    WHERE TRUE {cat_filter}
)

SELECT a.repo_uri, a.source_path, a.author_did,
       p.handle AS author_handle, p.display_name AS author_display_name,
       p.avatar_url AS author_avatar, COALESCE(p.reputation, 0) AS author_reputation,
       a.kind, a.category, a.license, a.prereq_threshold, a.restricted, a.answer_count,
       a.cover_url, a.cover_file,
       a.question_repo_uri, a.question_source_path,
       a.book_id, a.edition_id, a.course_id, a.book_chapter_id, a.course_session_id,
       l.lang, l.at_uri, l.file_path, l.title, l.summary, l.summary_html,
       l.content_hash, l.content_format, l.translator_did,
       pm.venue AS paper_venue, pm.year AS paper_year, pm.accepted AS paper_accepted,
       COALESCE(vs.score, 0) AS vote_score,
       COALESCE(bk.cnt,   0) AS bookmark_count,
       COALESCE(cm.cnt,   0) AS comment_count,
       a.created_at, a.updated_at
FROM scored sc
JOIN articles a ON a.repo_uri = sc.repo_uri AND a.source_path = sc.source_path
JOIN article_localizations l
    ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path
   AND l.file_path = a.source_path
LEFT JOIN profiles p ON p.did = a.author_did
LEFT JOIN paper_metadata pm
    ON pm.repo_uri = a.repo_uri AND pm.source_path = a.source_path
LEFT JOIN (SELECT target_uri, SUM(value) AS score FROM votes GROUP BY target_uri) vs
    ON vs.target_uri = article_uri(a.repo_uri, a.source_path)
LEFT JOIN (SELECT repo_uri, source_path, COUNT(*) AS cnt FROM user_bookmarks GROUP BY repo_uri, source_path) bk
    ON bk.repo_uri = a.repo_uri AND bk.source_path = a.source_path
LEFT JOIN (SELECT content_uri, COUNT(*) AS cnt FROM comments GROUP BY content_uri) cm
    ON cm.content_uri = article_uri(a.repo_uri, a.source_path)
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
    SELECT a.repo_uri, a.source_path, a.author_did,
           a.kind, a.category, a.license, a.prereq_threshold, a.restricted, a.answer_count,
           a.cover_url, a.cover_file,
           a.question_repo_uri, a.question_source_path,
           a.book_id, a.edition_id, a.course_id, a.book_chapter_id, a.course_session_id,
           l.lang, l.at_uri, l.file_path, l.title, l.summary, l.summary_html,
           l.content_hash, l.content_format, l.translator_did,
           p.handle AS author_handle, p.display_name AS author_display_name,
           p.avatar_url AS author_avatar, COALESCE(p.reputation, 0) AS author_reputation,
           pm.venue AS paper_venue, pm.year AS paper_year, pm.accepted AS paper_accepted,
           COALESCE(v.score, 0) AS vote_score,
           COALESCE(bk.cnt,   0) AS bookmark_count,
           COALESCE(cm.cnt,   0) AS comment_count,
           a.created_at, a.updated_at,
           article_uri(a.repo_uri, a.source_path) AS synth_uri
    FROM articles a
    JOIN article_localizations l
        ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path
       AND l.file_path = a.source_path
    LEFT JOIN profiles p ON p.did = a.author_did
    LEFT JOIN paper_metadata pm
        ON pm.repo_uri = a.repo_uri AND pm.source_path = a.source_path
    LEFT JOIN (SELECT target_uri, SUM(value) AS score FROM votes GROUP BY target_uri) v
        ON v.target_uri = article_uri(a.repo_uri, a.source_path)
    LEFT JOIN (SELECT repo_uri, source_path, COUNT(*) AS cnt FROM user_bookmarks GROUP BY repo_uri, source_path) bk
        ON bk.repo_uri = a.repo_uri AND bk.source_path = a.source_path
    LEFT JOIN (SELECT content_uri, COUNT(*) AS cnt FROM comments GROUP BY content_uri) cm
        ON cm.content_uri = article_uri(a.repo_uri, a.source_path)
    WHERE {vis} AND a.kind = 'question'
),
scored AS (
    SELECT q.*,
        CASE WHEN ct_total.cnt IS NULL OR ct_total.cnt = 0 THEN 0.5
             ELSE COALESCE(ct_mastered.cnt, 0)::float / ct_total.cnt::float
        END AS answerability,
        1.0 / (q.answer_count + 1.0) AS answer_need,
        1.0 / POWER(EXTRACT(EPOCH FROM (NOW() - q.created_at)) / 3600.0 + 2.0, 0.8) AS recency,
        LEAST(1.0, GREATEST(0.0, q.vote_score::float / 10.0)) AS quality
    FROM questions q
    LEFT JOIN (
        SELECT ct.content_uri, COUNT(*) AS cnt FROM content_teaches ct GROUP BY ct.content_uri
    ) ct_total ON ct_total.content_uri = q.synth_uri
    LEFT JOIN (
        SELECT ct.content_uri, COUNT(*) AS cnt FROM content_teaches ct
        JOIN user_mastered um ON um.tag_id = ct.tag_id
        GROUP BY ct.content_uri
    ) ct_mastered ON ct_mastered.content_uri = q.synth_uri
    WHERE q.author_did != $1
)
SELECT repo_uri, source_path, author_did,
       author_handle, author_display_name, author_avatar, author_reputation,
       kind, category, license, prereq_threshold, restricted, answer_count,
       cover_url, cover_file,
       question_repo_uri, question_source_path,
       book_id, edition_id, course_id, book_chapter_id, course_session_id,
       lang, at_uri, file_path, title, summary, summary_html,
       content_hash, content_format, translator_did,
       paper_venue, paper_year, paper_accepted,
       vote_score, bookmark_count, comment_count, created_at, updated_at
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
SELECT ft.tag_id,
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
