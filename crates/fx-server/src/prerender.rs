//! Bot/AI pre-rendering middleware.
//!
//! Detects crawler and AI user agents, and returns pre-rendered HTML with
//! OG meta tags instead of the SPA shell. This makes the site readable by
//! search engines and AI assistants.

use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode, header},
    middleware::Next,
};
use sqlx::PgPool;

const BOT_PATTERNS: &[&str] = &[
    "googlebot", "bingbot", "yandexbot", "baiduspider", "duckduckbot",
    "slurp", "ia_archiver", "facebookexternalhit", "twitterbot", "linkedinbot",
    "whatsapp", "telegrambot", "discordbot", "applebot",
    "claude", "chatgpt", "gptbot", "anthropic", "perplexity",
    "cohere", "bytespider",
    // Generic fetchers used by AI tools
    "python-requests", "httpx", "node-fetch", "undici",
];

fn is_bot(ua: &str) -> bool {
    let lower = ua.to_lowercase();
    BOT_PATTERNS.iter().any(|p| lower.contains(p))
}

pub async fn prerender_middleware(
    State(pool): State<PgPool>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let ua = request.headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !is_bot(ua) {
        return next.run(request).await;
    }

    let path = request.uri().path();
    let query = request.uri().query().unwrap_or("");

    // Only prerender content pages
    let html = match path {
        "/article" => {
            let uri = extract_param(query, "uri");
            if let Some(uri) = uri {
                prerender_article(&pool, &uri).await
            } else {
                None
            }
        }
        "/series" => {
            let id = extract_param(query, "id");
            if let Some(id) = id {
                prerender_series(&pool, &id).await
            } else {
                None
            }
        }
        "/question" => {
            let uri = extract_param(query, "uri");
            if let Some(uri) = uri {
                prerender_article(&pool, &uri).await
            } else {
                None
            }
        }
        _ => None,
    };

    if let Some(html) = html {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(html))
            .unwrap()
    } else {
        next.run(request).await
    }
}

fn extract_param<'a>(query: &'a str, key: &str) -> Option<String> {
    for part in query.split('&') {
        let mut kv = part.splitn(2, '=');
        if kv.next() == Some(key) {
            return kv.next().map(|v| urlencoding::decode(v).unwrap_or_default().into_owned());
        }
    }
    None
}

async fn prerender_article(pool: &PgPool, uri: &str) -> Option<String> {
    let row: (String, String, String, String, String) = sqlx::query_as(
        "SELECT a.title, a.description, a.did, a.content_format::TEXT, \
         COALESCE(pu.handle, a.did) \
         FROM articles a LEFT JOIN platform_users pu ON a.did = pu.did \
         WHERE a.at_uri = $1 AND a.removed_at IS NULL"
    ).bind(uri).fetch_optional(pool).await.ok()??;

    let (title, description, _did, _fmt, author) = row;

    // Try to get rendered HTML from pijul cache
    let content_html = sqlx::query_scalar::<_, String>(
        "SELECT source_text FROM article_versions \
         WHERE article_uri = $1 ORDER BY created_at DESC LIMIT 1"
    ).bind(uri).fetch_optional(pool).await.ok().flatten();

    let body_html = if let Some(src) = &content_html {
        format!("<article class=\"content\">{}</article>", src)
    } else {
        format!("<p>{}</p>", html_escape(&description))
    };

    Some(render_page(&title, &description, &author, &body_html))
}

async fn prerender_series(pool: &PgPool, id: &str) -> Option<String> {
    let row: (String, Option<String>, String) = sqlx::query_as(
        "SELECT s.title, s.description, COALESCE(pu.handle, s.created_by) \
         FROM series s LEFT JOIN platform_users pu ON s.created_by = pu.did \
         WHERE s.id = $1"
    ).bind(id).fetch_optional(pool).await.ok()??;

    let (title, description, author) = row;
    let desc = description.as_deref().unwrap_or("");

    // List articles in the series
    let articles: Vec<(String, String)> = sqlx::query_as(
        "SELECT sa.article_uri, a.title FROM series_articles sa \
         JOIN articles a ON a.at_uri = sa.article_uri \
         WHERE sa.series_id = $1 ORDER BY sa.order_index"
    ).bind(id).fetch_all(pool).await.ok()?;

    let toc = articles.iter()
        .map(|(_, t)| format!("<li>{}</li>", html_escape(t)))
        .collect::<Vec<_>>()
        .join("\n");

    let body = format!(
        "<h1>{}</h1>\n<p>{}</p>\n<h2>Contents</h2>\n<ol>\n{}\n</ol>",
        html_escape(&title), html_escape(desc), toc
    );

    Some(render_page(&title, desc, &author, &body))
}

fn render_page(title: &str, description: &str, author: &str, body: &str) -> String {
    let t = html_escape(title);
    let d = html_escape(description);
    let a = html_escape(author);
    format!(
        r#"<!DOCTYPE html>
<html lang="zh">
<head>
  <meta charset="utf-8">
  <title>{t} - Fedi-Xanadu</title>
  <meta name="description" content="{d}">
  <meta name="author" content="{a}">
  <meta property="og:title" content="{t}">
  <meta property="og:description" content="{d}">
  <meta property="og:type" content="article">
  <meta property="og:site_name" content="Fedi-Xanadu">
  <meta name="twitter:card" content="summary">
  <meta name="twitter:title" content="{t}">
  <meta name="twitter:description" content="{d}">
  <style>body {{ max-width: 800px; margin: 2rem auto; padding: 0 1rem; font-family: serif; line-height: 1.6; }}</style>
</head>
<body>
  <header><nav><a href="/">Fedi-Xanadu</a></nav></header>
  <main>{body}</main>
  <footer><p>By {a}</p></footer>
</body>
</html>"#
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
