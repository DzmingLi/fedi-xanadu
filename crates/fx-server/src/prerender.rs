//! SEO & pre-rendering middleware.
//!
//! Intercepts SPA route requests and injects Open Graph meta tags,
//! structured data (JSON-LD), and canonical URLs into the HTML shell.
//! This enables rich previews when links are shared on social media,
//! messaging apps, and search engines — for ALL visitors, not just bots.
//!
//! For bot/crawler requests, additionally injects readable article
//! content into the page body.

use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode, header},
    middleware::Next,
};
use sqlx::PgPool;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SeoState {
    pub pool: PgPool,
    /// Cached contents of `frontend/dist/index.html`.
    pub template: Arc<String>,
    /// e.g. "https://nightboat.dzming.li"
    pub public_url: String,
}

// ---------------------------------------------------------------------------
// Page metadata
// ---------------------------------------------------------------------------

struct PageMeta {
    title: String,
    description: String,
    og_type: &'static str,
    author: Option<String>,
    published_time: Option<String>,
    tags: Vec<String>,
    canonical_path: String,
}

impl PageMeta {
    fn for_path(path: &str) -> Self {
        let (title, desc) = match path {
            "/" => ("NightBoat", "Engineering knowledge platform with skill tree matching"),
            "/skills" => ("Skill Tree", "Explore and build your knowledge skill tree"),
            "/questions" => ("Questions", "Ask and answer engineering questions"),
            "/library" => ("Library", "Your bookmarked articles and collections"),
            "/books" => ("Books", "Browse recommended textbooks and references"),
            "/guide" => ("Guide", "Learn how to use NightBoat"),
            "/about" => ("About", "About NightBoat — an engineering knowledge platform"),
            "/roadmap" => ("Roadmap", "NightBoat development roadmap"),
            "/thoughts" => ("Thoughts", "Share ideas, observations, and discussions"),
            "/listings" => ("Academic Listings", "Browse academic positions — PhD, postdoc, RA, and more"),
            "/new-listing" => ("Post Listing", "Post an academic position on NightBoat"),
            "/admin" => ("Admin", "NightBoat admin dashboard"),
            _ => ("NightBoat", "Engineering knowledge platform with skill tree matching"),
        };
        Self {
            title: title.into(),
            description: desc.into(),
            og_type: "website",
            author: None,
            published_time: None,
            tags: vec![],
            canonical_path: path.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Bot detection
// ---------------------------------------------------------------------------

const BOT_PATTERNS: &[&str] = &[
    "googlebot", "bingbot", "yandexbot", "baiduspider", "duckduckbot",
    "slurp", "ia_archiver", "facebookexternalhit", "twitterbot", "linkedinbot",
    "whatsapp", "telegrambot", "discordbot", "applebot",
    "claude-", "chatgpt", "gptbot", "anthropic", "perplexity",
    "cohere", "bytespider",
];

fn is_bot(ua: &str) -> bool {
    let lower = ua.to_lowercase();
    BOT_PATTERNS.iter().any(|p| lower.contains(p))
}

/// Returns true for client-side SPA routes (not static assets, API, etc.)
fn is_spa_route(path: &str) -> bool {
    !path.starts_with("/api/")
        && !path.starts_with("/oauth/")
        && !path.starts_with("/assets/")
        && !path.starts_with("/feed/")
        && path != "/sitemap.xml"
        // Files with extensions are static assets (favicon.ico, robots.txt …)
        && !path
            .rsplit('/')
            .next()
            .map_or(false, |seg| seg.contains('.'))
}

// ---------------------------------------------------------------------------
// Middleware entry point
// ---------------------------------------------------------------------------

pub async fn seo_middleware(
    State(seo): State<SeoState>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let ua = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let path = request.uri().path().to_string();
    let query = request.uri().query().unwrap_or("").to_string();

    if !is_spa_route(&path) {
        return next.run(request).await;
    }

    let bot = is_bot(ua);
    let meta = resolve_meta(&seo.pool, &path, &query).await;

    let bot_body = if bot {
        render_bot_body(&seo.pool, &path, &query).await
    } else {
        None
    };

    let html = inject_into_template(&seo.template, &meta, &seo.public_url, bot_body.as_deref());

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(html))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Meta resolution — query DB for page-specific metadata
// ---------------------------------------------------------------------------

async fn resolve_meta(pool: &PgPool, path: &str, query: &str) -> PageMeta {
    let resolved = match path {
        "/article" | "/question" => {
            let uri = extract_param(query, "uri");
            if let Some(uri) = uri {
                resolve_article(pool, &uri, path, query).await
            } else {
                None
            }
        }
        "/series" => {
            let id = extract_param(query, "id");
            if let Some(id) = id {
                resolve_series(pool, &id, query).await
            } else {
                None
            }
        }
        "/profile" => {
            let did = extract_param(query, "did");
            if let Some(did) = did {
                resolve_profile(pool, &did, query).await
            } else {
                None
            }
        }
        "/tag" => {
            let id = extract_param(query, "id");
            if let Some(id) = id {
                resolve_tag(pool, &id, query).await
            } else {
                None
            }
        }
        "/book" => {
            let id = extract_param(query, "id");
            if let Some(id) = id {
                resolve_book(pool, &id, query).await
            } else {
                None
            }
        }
        _ => None,
    };
    resolved.unwrap_or_else(|| PageMeta::for_path(path))
}

async fn resolve_article(pool: &PgPool, uri: &str, path: &str, raw_query: &str) -> Option<PageMeta> {
    let row: (String, String, String, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        "SELECT a.title, a.description, \
         COALESCE(p.display_name, p.handle, a.did), a.created_at \
         FROM articles a LEFT JOIN profiles p ON a.did = p.did \
         WHERE a.at_uri = $1 AND a.removed_at IS NULL",
    )
    .bind(uri)
    .fetch_optional(pool)
    .await
    .ok()??;

    let (title, description, author, created_at) = row;

    let tags: Vec<String> = sqlx::query_scalar("SELECT tag_id FROM content_teaches WHERE content_uri = $1")
        .bind(uri)
        .fetch_all(pool)
        .await
        .ok()
        .unwrap_or_default();

    Some(PageMeta {
        title,
        description,
        og_type: "article",
        author: Some(author),
        published_time: Some(created_at.to_rfc3339()),
        tags,
        canonical_path: format!("{}?{}", path, raw_query),
    })
}

async fn resolve_series(pool: &PgPool, id: &str, raw_query: &str) -> Option<PageMeta> {
    let row: (String, Option<String>, String) = sqlx::query_as(
        "SELECT s.title, s.description, \
         COALESCE(p.display_name, p.handle, s.created_by) \
         FROM series s LEFT JOIN profiles p ON s.created_by = p.did \
         WHERE s.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()??;

    let (title, description, author) = row;

    Some(PageMeta {
        title,
        description: description.unwrap_or_default(),
        og_type: "article",
        author: Some(author),
        published_time: None,
        tags: vec![],
        canonical_path: format!("/series?{}", raw_query),
    })
}

async fn resolve_profile(pool: &PgPool, did: &str, raw_query: &str) -> Option<PageMeta> {
    let row: (String, Option<String>) = sqlx::query_as(
        "SELECT handle, display_name FROM profiles WHERE did = $1",
    )
    .bind(did)
    .fetch_optional(pool)
    .await
    .ok()??;

    let (handle, display_name) = row;
    let title = display_name
        .as_ref()
        .map(|dn| format!("{} (@{})", dn, handle))
        .unwrap_or_else(|| format!("@{}", handle));

    Some(PageMeta {
        title,
        description: format!("Articles and activity by @{} on NightBoat", handle),
        og_type: "profile",
        author: None,
        published_time: None,
        tags: vec![],
        canonical_path: format!("/profile?{}", raw_query),
    })
}

async fn resolve_tag(pool: &PgPool, id: &str, raw_query: &str) -> Option<PageMeta> {
    let row: (String, Option<String>) = sqlx::query_as(
        "SELECT name, description FROM tags WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()??;

    let (name, description) = row;

    Some(PageMeta {
        title: name,
        description: description.unwrap_or_else(|| format!("Articles tagged with {}", id)),
        og_type: "website",
        author: None,
        published_time: None,
        tags: vec![id.to_string()],
        canonical_path: format!("/tag?{}", raw_query),
    })
}

async fn resolve_book(pool: &PgPool, id: &str, raw_query: &str) -> Option<PageMeta> {
    let row: (String, String, Vec<String>) = sqlx::query_as(
        "SELECT title, description, authors FROM books WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()??;

    let (title, description, authors) = row;

    Some(PageMeta {
        title,
        description,
        og_type: "book",
        author: if authors.is_empty() {
            None
        } else {
            Some(authors.join(", "))
        },
        published_time: None,
        tags: vec![],
        canonical_path: format!("/book?{}", raw_query),
    })
}

// ---------------------------------------------------------------------------
// Bot body rendering — readable content for crawlers
// ---------------------------------------------------------------------------

async fn render_bot_body(pool: &PgPool, path: &str, query: &str) -> Option<String> {
    match path {
        "/article" | "/question" => {
            let uri = extract_param(query, "uri")?;
            render_bot_article(pool, &uri).await
        }
        "/series" => {
            let id = extract_param(query, "id")?;
            render_bot_series(pool, &id).await
        }
        _ => None,
    }
}

async fn render_bot_article(pool: &PgPool, uri: &str) -> Option<String> {
    let row: (String, String, String) = sqlx::query_as(
        "SELECT a.title, a.description, \
         COALESCE(p.display_name, p.handle, a.did) \
         FROM articles a LEFT JOIN profiles p ON a.did = p.did \
         WHERE a.at_uri = $1 AND a.removed_at IS NULL",
    )
    .bind(uri)
    .fetch_optional(pool)
    .await
    .ok()??;

    let (title, description, author) = row;

    // Try latest version source text as fallback content
    let source: Option<String> =
        sqlx::query_scalar("SELECT source_text FROM article_versions WHERE article_uri = $1 ORDER BY created_at DESC LIMIT 1")
            .bind(uri)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    let body = if let Some(src) = &source {
        format!(
            "<article><h1>{}</h1><p><em>By {}</em></p><p>{}</p><pre>{}</pre></article>",
            esc(&title),
            esc(&author),
            esc(&description),
            esc(src)
        )
    } else {
        format!(
            "<article><h1>{}</h1><p><em>By {}</em></p><p>{}</p></article>",
            esc(&title),
            esc(&author),
            esc(&description)
        )
    };

    Some(body)
}

async fn render_bot_series(pool: &PgPool, id: &str) -> Option<String> {
    let row: (String, Option<String>, String) = sqlx::query_as(
        "SELECT s.title, s.description, \
         COALESCE(p.display_name, p.handle, s.created_by) \
         FROM series s LEFT JOIN profiles p ON s.created_by = p.did \
         WHERE s.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()??;

    let (title, description, author) = row;

    let chapters: Vec<(String, String)> = sqlx::query_as(
        "SELECT sa.article_uri, a.title FROM series_articles sa \
         JOIN articles a ON a.at_uri = sa.article_uri \
         WHERE sa.series_id = $1 ORDER BY sa.order_index",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .ok()?;

    let toc: String = chapters
        .iter()
        .map(|(_, t)| format!("<li>{}</li>", esc(t)))
        .collect::<Vec<_>>()
        .join("\n    ");

    Some(format!(
        "<article>\
         <h1>{}</h1>\
         <p><em>By {}</em></p>\
         <p>{}</p>\
         <h2>Contents</h2>\
         <ol>\n    {}\n  </ol>\
         </article>",
        esc(&title),
        esc(&author),
        esc(description.as_deref().unwrap_or("")),
        toc
    ))
}

// ---------------------------------------------------------------------------
// Template injection
// ---------------------------------------------------------------------------

fn inject_into_template(
    template: &str,
    meta: &PageMeta,
    base_url: &str,
    bot_body: Option<&str>,
) -> String {
    let mut html = template.to_string();

    // 1. Replace <title>
    let full_title = if meta.title == "NightBoat" {
        "NightBoat".to_string()
    } else {
        format!("{} — NightBoat", meta.title)
    };
    html = html.replace("<title>NightBoat</title>", &format!("<title>{}</title>", esc(&full_title)));

    // 2. Build meta tags + JSON-LD
    let canonical = format!("{}{}", base_url, meta.canonical_path);
    let tags_html = build_meta_tags(meta, base_url, &canonical);

    // 3. Inject before </head>
    html = html.replace("</head>", &format!("{tags_html}\n</head>"));

    // 4. For bots, inject content into <div id="app">
    if let Some(body) = bot_body {
        html = html.replace(
            "<div id=\"app\"></div>",
            &format!("<div id=\"app\">{body}</div>"),
        );
    }

    html
}

fn build_meta_tags(meta: &PageMeta, _base_url: &str, canonical: &str) -> String {
    let t = esc(&meta.title);
    let d = esc(&meta.description);

    let mut s = String::with_capacity(1024);

    // Standard
    s.push_str(&format!("  <meta name=\"description\" content=\"{d}\">\n"));
    s.push_str(&format!("  <link rel=\"canonical\" href=\"{canonical}\">\n"));

    // Open Graph
    s.push_str(&format!("  <meta property=\"og:title\" content=\"{t}\">\n"));
    s.push_str(&format!("  <meta property=\"og:description\" content=\"{d}\">\n"));
    s.push_str(&format!("  <meta property=\"og:type\" content=\"{}\">\n", meta.og_type));
    s.push_str(&format!("  <meta property=\"og:url\" content=\"{canonical}\">\n"));
    s.push_str(&format!("  <meta property=\"og:site_name\" content=\"NightBoat\">\n"));

    if let Some(author) = &meta.author {
        s.push_str(&format!("  <meta property=\"article:author\" content=\"{}\">\n", esc(author)));
    }
    if let Some(time) = &meta.published_time {
        s.push_str(&format!("  <meta property=\"article:published_time\" content=\"{time}\">\n"));
    }
    for tag in &meta.tags {
        s.push_str(&format!("  <meta property=\"article:tag\" content=\"{}\">\n", esc(tag)));
    }

    // Twitter Card
    s.push_str("  <meta name=\"twitter:card\" content=\"summary\">\n");
    s.push_str(&format!("  <meta name=\"twitter:title\" content=\"{t}\">\n"));
    s.push_str(&format!("  <meta name=\"twitter:description\" content=\"{d}\">\n"));

    // JSON-LD structured data (articles only)
    if meta.og_type == "article" {
        s.push_str("  <script type=\"application/ld+json\">\n");
        s.push_str(&json_ld(meta, canonical));
        s.push_str("\n  </script>\n");
    }

    s
}

fn json_ld(meta: &PageMeta, url: &str) -> String {
    let mut ld = format!(
        r#"  {{
    "@context": "https://schema.org",
    "@type": "Article",
    "headline": "{}",
    "description": "{}",
    "url": "{url}",
    "publisher": {{ "@type": "Organization", "name": "NightBoat" }}"#,
        json_escape(&meta.title),
        json_escape(&meta.description),
    );

    if let Some(author) = &meta.author {
        ld.push_str(&format!(
            ",\n    \"author\": {{ \"@type\": \"Person\", \"name\": \"{}\" }}",
            json_escape(author)
        ));
    }
    if let Some(time) = &meta.published_time {
        ld.push_str(&format!(",\n    \"datePublished\": \"{time}\""));
    }

    ld.push_str("\n  }");
    ld
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn extract_param(query: &str, key: &str) -> Option<String> {
    for part in query.split('&') {
        let mut kv = part.splitn(2, '=');
        if kv.next() == Some(key) {
            return kv
                .next()
                .map(|v| urlencoding::decode(v).unwrap_or_default().into_owned());
        }
    }
    None
}

/// HTML-attribute-safe escaping.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// JSON string value escaping.
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
