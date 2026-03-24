use fx_core::models::{Article, ArticlePrereqRow, ForkWithTitle, Tag};

fn layout(title: &str, instance_name: &str, content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="zh">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} - {instance_name}</title>
    <script src="https://unpkg.com/htmx.org@2.0.4"></script>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: system-ui, -apple-system, sans-serif; max-width: 52rem; margin: 0 auto; padding: 2rem 1rem; color: #1a1a1a; line-height: 1.6; }}
        nav {{ display: flex; gap: 1.5rem; padding-bottom: 1rem; border-bottom: 1px solid #e5e5e5; margin-bottom: 2rem; align-items: center; }}
        nav a {{ color: #2563eb; text-decoration: none; }}
        nav a:hover {{ text-decoration: underline; }}
        h1 {{ font-size: 1.5rem; margin-bottom: 1rem; }}
        h2 {{ font-size: 1.2rem; margin-bottom: 0.5rem; }}
        h3 {{ font-size: 1rem; margin: 1.5rem 0 0.5rem; color: #374151; }}
        .card {{ border: 1px solid #e5e5e5; border-radius: 0.5rem; padding: 1rem; margin-bottom: 1rem; }}
        .card:hover {{ border-color: #2563eb; }}
        .card a {{ text-decoration: none; color: inherit; display: block; }}
        .tag {{ display: inline-block; background: #eff6ff; color: #2563eb; padding: 0.125rem 0.5rem; border-radius: 0.25rem; font-size: 0.875rem; margin: 0.125rem; }}
        .tag.required {{ background: #fef2f2; color: #dc2626; }}
        .tag.recommended {{ background: #fffbeb; color: #d97706; }}
        .tag.suggested {{ background: #f0fdf4; color: #16a34a; }}
        .tag.lit {{ background: #22c55e; color: white; }}
        .meta {{ color: #6b7280; font-size: 0.875rem; }}
        .empty {{ color: #9ca3af; text-align: center; padding: 3rem; }}
        .content {{ margin: 1.5rem 0; padding: 1.5rem; background: #fafafa; border-radius: 0.5rem; border: 1px solid #e5e5e5; }}
        .btn {{ display: inline-block; padding: 0.5rem 1rem; border-radius: 0.375rem; text-decoration: none; font-size: 0.875rem; cursor: pointer; border: 1px solid #e5e5e5; background: white; color: #374151; }}
        .btn:hover {{ background: #f9fafb; border-color: #2563eb; color: #2563eb; }}
        .btn-primary {{ background: #2563eb; color: white; border-color: #2563eb; }}
        .btn-primary:hover {{ background: #1d4ed8; }}
        .fork-card {{ padding: 0.75rem; border: 1px solid #e5e5e5; border-radius: 0.375rem; margin-bottom: 0.5rem; }}
        textarea, input, select {{ font-family: inherit; font-size: 0.875rem; padding: 0.5rem; border: 1px solid #d1d5db; border-radius: 0.375rem; width: 100%; }}
        textarea {{ min-height: 12rem; font-family: monospace; }}
        label {{ display: block; margin-bottom: 0.25rem; font-size: 0.875rem; font-weight: 500; }}
        .form-group {{ margin-bottom: 1rem; }}
        .actions {{ display: flex; gap: 0.5rem; margin: 1rem 0; }}
    </style>
</head>
<body>
    <nav>
        <a href="/"><strong>{instance_name}</strong></a>
        <a href="/tags">Tags</a>
        <a href="/graph">Graph</a>
        <a href="/skills">Skills</a>
        <a href="/new" class="btn btn-primary" style="margin-left:auto">New Article</a>
    </nav>
    {content}
</body>
</html>"#
    )
}

pub fn index_page(instance_name: &str, articles: &[Article]) -> String {
    let content = if articles.is_empty() {
        String::from(r#"<div class="empty"><p>No articles yet.</p><p class="meta"><a href="/new">Create the first one</a></p></div>"#)
    } else {
        let cards: String = articles
            .iter()
            .map(|a| {
                let encoded_uri = url_encode(&a.at_uri);
                format!(
                    r#"<div class="card"><a href="/article?uri={encoded_uri}">
                        <h2>{}</h2>
                        <p class="meta">{} &middot; {} &middot; {}</p>
                    </a></div>"#,
                    html_escape(&a.title),
                    html_escape(&a.did),
                    &a.content_format,
                    &a.created_at,
                )
            })
            .collect();
        format!("<h1>Recent Articles</h1>{cards}")
    };
    layout("Home", instance_name, &content)
}

pub fn tags_page(instance_name: &str, tags: &[Tag]) -> String {
    let content = if tags.is_empty() {
        String::from(r#"<div class="empty"><p>No tags yet.</p></div>"#)
    } else {
        let items: String = tags
            .iter()
            .map(|t| {
                let desc = t.description.as_deref().unwrap_or("");
                format!(
                    r#"<div class="card">
                        <h2><span class="tag">{}</span> {}</h2>
                        <p class="meta">{}</p>
                    </div>"#,
                    html_escape(&t.id),
                    html_escape(&t.name),
                    html_escape(desc),
                )
            })
            .collect();
        format!("<h1>Tags</h1>{items}")
    };
    layout("Tags", instance_name, &content)
}

pub fn article_page(
    instance_name: &str,
    article: &Article,
    rendered_html: &str,
    prereqs: &[ArticlePrereqRow],
    forks: &[ForkWithTitle],
    tags: &[String],
) -> String {
    let tags_html: String = tags
        .iter()
        .map(|t| format!(r#"<span class="tag">{}</span>"#, html_escape(t)))
        .collect();

    let prereqs_html = if prereqs.is_empty() {
        String::new()
    } else {
        let items: String = prereqs
            .iter()
            .map(|p| {
                format!(
                    r#"<span class="tag {ptype}">{name} ({ptype})</span>"#,
                    ptype = html_escape(&p.prereq_type),
                    name = html_escape(&p.tag_name),
                )
            })
            .collect();
        format!(r#"<div style="margin:1rem 0"><h3>Prerequisites</h3>{items}</div>"#)
    };

    let forks_html = if forks.is_empty() {
        String::new()
    } else {
        let items: String = forks
            .iter()
            .map(|f| {
                let encoded = url_encode(&f.forked_uri);
                format!(
                    r#"<div class="fork-card">
                        <a href="/article/{encoded}">{}</a>
                        <span class="meta"> by {} &middot; score: {}</span>
                    </div>"#,
                    html_escape(&f.title),
                    html_escape(&f.did),
                    f.vote_score,
                )
            })
            .collect();
        format!(r#"<div style="margin:1.5rem 0"><h3>Forks</h3>{items}</div>"#)
    };

    let encoded_uri = url_encode(&article.at_uri);
    let content = format!(
        r#"<article>
            <h1>{title}</h1>
            <p class="meta">{did} &middot; {format} &middot; {date} &middot; {license}</p>
            <div>{tags_html}</div>
            {prereqs_html}
            <div class="content">{rendered_html}</div>
            <div class="actions">
                <a href="/api/articles/fork?uri={encoded_uri}" class="btn">Fork</a>
            </div>
            {forks_html}
        </article>"#,
        title = html_escape(&article.title),
        did = html_escape(&article.did),
        format = html_escape(&article.content_format),
        date = html_escape(&article.created_at),
        license = html_escape(&article.license),
    );
    layout(&article.title, instance_name, &content)
}

pub fn new_article_page(instance_name: &str, tags: &[Tag]) -> String {
    let tag_options: String = tags
        .iter()
        .map(|t| {
            format!(
                r#"<option value="{}">{}</option>"#,
                html_escape(&t.id),
                html_escape(&t.name),
            )
        })
        .collect();

    let content = format!(
        r#"<h1>New Article</h1>
        <form id="new-article-form" action="/api/articles" method="POST">
            <div class="form-group">
                <label for="title">Title</label>
                <input type="text" id="title" name="title" required placeholder="Article title">
            </div>
            <div class="form-group">
                <label for="content">Content (Typst)</label>
                <textarea id="content" name="content" required placeholder="= My Article"></textarea>
            </div>
            <div class="form-group">
                <label for="tags">Tags</label>
                <select id="tags" name="tags" multiple>{tag_options}</select>
            </div>
            <button type="submit" class="btn btn-primary">Publish</button>
        </form>
        <script src="/static/new-article.js"></script>"#
    );
    layout("New Article", instance_name, &content)
}

pub fn skills_page(instance_name: &str, tags: &[Tag], lit_tags: &[String]) -> String {
    let items: String = tags
        .iter()
        .map(|t| {
            let is_lit = lit_tags.contains(&t.id);
            let (class, data) = if is_lit {
                ("tag lit", format!(r#"hx-get="/api/skills/unlight?tag_id={}" hx-swap="none""#, url_encode(&t.id)))
            } else {
                ("tag", format!(
                    r#"hx-post="/api/skills" hx-vals='{{"tag_id":"{}"}}' hx-swap="none""#,
                    html_escape(&t.id)
                ))
            };
            format!(
                r#"<span class="{class}" style="cursor:pointer" {data} hx-on::after-request="location.reload()">{}</span>"#,
                html_escape(&t.name),
            )
        })
        .collect();

    let content = format!(
        r#"<h1>My Skills</h1>
        <p class="meta">Click to toggle. Green = mastered.</p>
        <div style="margin:1rem 0">{items}</div>"#
    );
    layout("Skills", instance_name, &content)
}

pub fn graph_page(instance_name: &str) -> String {
    let content = r#"<h1>Knowledge Graph</h1>
        <div id="graph" style="width:100%;height:70vh;border:1px solid #e5e5e5;border-radius:0.5rem;overflow:hidden;position:relative"></div>
        <p class="meta" style="margin-top:0.5rem">Drag to pan, scroll to zoom. Node color: <span class="tag lit">mastered</span> <span class="tag">unlearned</span>. Edge: <span style="color:#dc2626">required</span> / <span style="color:#d97706">recommended</span> / <span style="color:#16a34a">suggested</span>.</p>
        <script src="/static/graph.js"></script>"#;
    layout("Knowledge Graph", instance_name, content)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn url_encode(s: &str) -> String {
    s.replace('%', "%25")
        .replace('/', "%2F")
        .replace(':', "%3A")
        .replace('#', "%23")
        .replace('?', "%3F")
}
