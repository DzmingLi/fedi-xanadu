use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{Config, client};

#[derive(Subcommand)]
pub enum BookCommand {
    /// List all books
    #[command(alias = "ls")]
    List,
    /// Create a new book (with its first edition)
    Create {
        /// Book title
        #[arg(short, long)]
        title: String,
        /// Subtitle
        #[arg(short = 'S', long)]
        subtitle: Option<String>,
        /// Authors (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        authors: Vec<String>,
        /// Description
        #[arg(short, long)]
        desc: Option<String>,
        /// Cover image URL
        #[arg(long)]
        cover_url: Option<String>,
        /// Tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Prereq tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        prereqs: Vec<String>,
        // -- First edition fields --
        /// Edition title (e.g. "Fourth Edition")
        #[arg(long, default_value = "First Edition")]
        edition: String,
        /// Language code (e.g. zh, en, ja)
        #[arg(short, long, default_value = "en")]
        lang: String,
        /// ISBN
        #[arg(long)]
        isbn: Option<String>,
        /// Publisher
        #[arg(long)]
        publisher: Option<String>,
        /// Year
        #[arg(long)]
        year: Option<String>,
        /// Translators (comma-separated)
        #[arg(long, value_delimiter = ',')]
        translators: Vec<String>,
        /// Purchase links as JSON: [{"label":"Amazon","url":"https://..."}]
        #[arg(long)]
        purchase_links: Option<String>,
        /// Cover image URL for this edition
        #[arg(long)]
        edition_cover_url: Option<String>,
        /// Subtitle of the first edition
        #[arg(long)]
        edition_subtitle: Option<String>,
    },
    /// Update a book's info
    Update {
        /// Book ID (e.g. bk-xxx)
        id: String,
        /// New title
        #[arg(short, long)]
        title: Option<String>,
        /// New description
        #[arg(short, long)]
        desc: Option<String>,
        /// New cover URL
        #[arg(long)]
        cover_url: Option<String>,
        /// Edit summary
        #[arg(long)]
        summary: Option<String>,
    },
    /// Add an edition to a book
    #[command(name = "add-edition")]
    AddEdition {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Edition title (localized title of this edition, e.g. Chinese translation title)
        #[arg(short, long)]
        title: String,
        /// Edition subtitle
        #[arg(short = 'S', long)]
        subtitle: Option<String>,
        /// Edition name (e.g. "Fourth Edition", "Revised Edition"). Defaults to title.
        #[arg(short = 'n', long)]
        edition_name: Option<String>,
        /// Language code (e.g. zh, en, ja)
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// ISBN
        #[arg(long)]
        isbn: Option<String>,
        /// Publisher
        #[arg(long)]
        publisher: Option<String>,
        /// Year
        #[arg(long)]
        year: Option<String>,
        /// Translators (comma-separated)
        #[arg(long, value_delimiter = ',')]
        translators: Vec<String>,
        /// Purchase links as JSON: [{"label":"Amazon","url":"https://..."}]
        #[arg(long)]
        purchase_links: Option<String>,
        /// Cover image URL for this edition
        #[arg(long)]
        cover_url: Option<String>,
        /// Publication status: 'draft' or 'published' (default)
        #[arg(long)]
        status: Option<String>,
    },
    /// Update an existing edition's info (only supplied fields change)
    #[command(name = "update-edition")]
    UpdateEdition {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Edition ID
        #[arg(long)]
        edition_id: String,
        /// Edition title (localized title of this edition)
        #[arg(short, long)]
        title: Option<String>,
        /// Edition subtitle (pass empty string to clear)
        #[arg(short = 'S', long)]
        subtitle: Option<String>,
        /// Edition name (e.g. "Fourth Edition")
        #[arg(short = 'n', long)]
        edition_name: Option<String>,
        /// Language code (e.g. zh, en, ja)
        #[arg(short, long)]
        lang: Option<String>,
        /// ISBN
        #[arg(long)]
        isbn: Option<String>,
        /// Publisher
        #[arg(long)]
        publisher: Option<String>,
        /// Year
        #[arg(long)]
        year: Option<String>,
        /// Translators (comma-separated, replaces existing)
        #[arg(long, value_delimiter = ',')]
        translators: Option<Vec<String>>,
        /// Purchase links as JSON (replaces existing)
        #[arg(long)]
        purchase_links: Option<String>,
        /// Cover image URL for this edition
        #[arg(long)]
        cover_url: Option<String>,
        /// Publication status: 'draft' or 'published'
        #[arg(long)]
        status: Option<String>,
    },
    /// Show a book's detail
    Show {
        /// Book ID
        id: String,
    },
    /// Upload a cover image for a book edition
    #[command(name = "upload-cover")]
    UploadCover {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Edition ID
        #[arg(long)]
        edition_id: String,
        /// Path to image file
        file: PathBuf,
    },
    /// Add a chapter to a book's table of contents
    #[command(name = "add-chapter")]
    AddChapter {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Chapter title
        #[arg(short, long)]
        title: String,
        /// Parent chapter ID (for sub-chapters)
        #[arg(long)]
        parent_id: Option<String>,
        /// Order index (0-based)
        #[arg(long, default_value = "0")]
        order: i32,
        /// Linked article URI
        #[arg(long)]
        article_uri: Option<String>,
        /// Tags this chapter teaches (comma-separated)
        #[arg(long, value_delimiter = ',')]
        teaches: Vec<String>,
        /// Prereq tags as "tag_id:required" or "tag_id:recommended" (comma-separated)
        #[arg(long, value_delimiter = ',')]
        prereqs: Vec<String>,
    },
    /// Upload a directory of chapters from a TOML manifest
    ///
    /// The TOML file describes chapters (with optional file paths to upload as articles).
    /// Example manifest: see `fx book upload-chapters --help`.
    #[command(name = "upload-chapters")]
    UploadChapters {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Path to chapters TOML manifest
        manifest: PathBuf,
        /// Language for uploaded articles (default: zh)
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// License for uploaded articles (default: CC-BY-SA-4.0)
        #[arg(long, default_value = "CC-BY-SA-4.0")]
        license: String,
        /// Dry run — print what would happen without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Add a supplementary resource to a book (solutions, videos, slides, etc.)
    #[command(name = "add-resource")]
    AddResource {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Resource kind (solutions, exercises, video, slides, errata, code, other)
        #[arg(short, long)]
        kind: String,
        /// Display label
        #[arg(short, long)]
        label: String,
        /// URL
        #[arg(short, long)]
        url: String,
        /// Edition ID (omit for all editions)
        #[arg(long)]
        edition_id: Option<String>,
        /// Display order
        #[arg(long, default_value = "0")]
        position: i16,
    },
    /// Write or update a short review for a book (rating + 500-char text)
    #[command(name = "short-review")]
    ShortReview {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Review body (max 500 chars)
        #[arg(short, long)]
        body: String,
        /// Rating 1-10 (half-stars)
        #[arg(long)]
        rating: i16,
        /// Edition ID (optional)
        #[arg(long)]
        edition_id: Option<String>,
        /// Visibility: public, followers, private (default: public)
        #[arg(long)]
        visibility: Option<String>,
    },
    /// List short reviews for a book
    #[command(name = "short-review-list")]
    ShortReviewList {
        /// Book ID
        #[arg(long)]
        book_id: String,
    },
    /// Delete your short review for a book
    #[command(name = "short-review-delete")]
    ShortReviewDelete {
        /// Book ID
        #[arg(long)]
        book_id: String,
    },
}

/// TOML manifest for uploading book chapters.
///
/// Example:
/// ```toml
/// [[chapter]]
/// key = "ch1"
/// title = "第一章：绪论"
/// file = "ch01.typ"
/// order = 0
/// teaches = ["cat-intro"]
///
/// [[chapter]]
/// key = "ch1-1"
/// parent = "ch1"
/// title = "1.1 基本定义"
/// file = "ch01-01.typ"
/// order = 0
/// teaches = ["cat-def"]
/// prereqs = [{ tag = "cat-intro", type = "required" }]
/// ```
#[derive(Serialize, Deserialize)]
struct ChapterManifest {
    #[serde(default, rename = "chapter")]
    chapters: Vec<ChapterEntry>,
}

#[derive(Serialize, Deserialize)]
struct ChapterEntry {
    #[serde(default)]
    key: String,
    title: String,
    #[serde(default)]
    file: Option<PathBuf>,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    order: i32,
    #[serde(default)]
    teaches: Vec<String>,
    #[serde(default)]
    prereqs: Vec<ChapterPrereqEntry>,
}

#[derive(Serialize, Deserialize)]
struct ChapterPrereqEntry {
    tag: String,
    #[serde(default = "default_prereq_type")]
    r#type: String,
}
fn default_prereq_type() -> String { "required".to_string() }

pub async fn handle_book(base: &str, config: &Config, action: BookCommand) -> Result<()> {
    let token = config.token()?;
    match action {
        BookCommand::List => {
            let resp: Vec<serde_json::Value> = client()
                .get(format!("{base}/books"))
                .send().await?
                .error_for_status().context("List books failed")?
                .json().await?;

            if resp.is_empty() {
                println!("No books yet.");
            } else {
                for b in &resp {
                    let id = b["id"].as_str().unwrap_or("?");
                    let title = b["title"].as_str().unwrap_or("?");
                    let authors = b["authors"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                        .unwrap_or_default();
                    println!("  {id}  {title}  ({authors})");
                }
                println!("{} book(s)", resp.len());
            }
        }

        BookCommand::Create { title, subtitle, authors, desc, cover_url, tags, prereqs,
                             edition, lang, isbn, publisher, year, translators, purchase_links, edition_cover_url, edition_subtitle } => {
            let parse_i18n = |s: &str| -> serde_json::Value {
                if s.starts_with('{') {
                    serde_json::from_str(s).unwrap_or(serde_json::json!({"en": s}))
                } else {
                    serde_json::json!({"en": s})
                }
            };
            let title_val = parse_i18n(&title);
            let subtitle_val = subtitle.as_deref().map(parse_i18n).unwrap_or(serde_json::json!({}));
            let desc_val = desc.as_deref().map(parse_i18n).unwrap_or(serde_json::json!({}));
            let body = serde_json::json!({
                "title": title_val,
                "subtitle": subtitle_val,
                "authors": authors,
                "description": desc_val,
                "cover_url": cover_url,
                "tags": tags,
                "prereqs": prereqs,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/books"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Create book failed")?
                .json().await?;

            let book_id = resp["id"].as_str().unwrap_or("?");
            if let Some(warning) = resp["warning"].as_str() {
                eprintln!("Warning: {warning}");
            }
            println!("Created book: {title}");
            println!("ID: {book_id}");

            let links: Vec<serde_json::Value> = if let Some(ref pl) = purchase_links {
                serde_json::from_str(pl).context("Invalid JSON for --purchase-links")?
            } else {
                vec![]
            };
            let ed_body = serde_json::json!({
                "book_id": book_id,
                "title": title,
                "subtitle": edition_subtitle,
                "edition_name": edition,
                "lang": lang,
                "isbn": isbn,
                "publisher": publisher,
                "year": year,
                "translators": translators,
                "purchase_links": links,
                "cover_url": edition_cover_url,
            });

            let ed_response = client()
                .post(format!("{base}/books/{book_id}/editions"))
                .bearer_auth(token)
                .json(&ed_body)
                .send().await?;
            if !ed_response.status().is_success() {
                let status = ed_response.status();
                let body = ed_response.text().await.unwrap_or_default();
                bail!("Create first edition failed ({status}): {body}");
            }
            let ed_resp: serde_json::Value = ed_response.json().await?;

            let eid = ed_resp["id"].as_str().unwrap_or("?");
            println!("Created edition: {edition} ({lang})");
            println!("Edition ID: {eid}");
        }

        BookCommand::Update { id, title, desc, cover_url, summary } => {
            let body = serde_json::json!({
                "id": id,
                "title": title,
                "description": desc,
                "cover_url": cover_url,
                "edit_summary": summary,
            });

            client()
                .post(format!("{base}/books/update"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Update book failed")?;

            println!("Updated book {id}");
        }

        BookCommand::AddEdition { book_id, title, subtitle, edition_name, lang, isbn, publisher, year, translators, purchase_links, cover_url, status } => {
            let links: Vec<serde_json::Value> = if let Some(ref pl) = purchase_links {
                serde_json::from_str(pl).context("Invalid JSON for --purchase-links")?
            } else {
                vec![]
            };

            let edition_name = edition_name.unwrap_or_else(|| title.clone());
            let body = serde_json::json!({
                "book_id": book_id,
                "title": title,
                "subtitle": subtitle,
                "edition_name": edition_name,
                "lang": lang,
                "isbn": isbn,
                "publisher": publisher,
                "year": year,
                "translators": translators,
                "purchase_links": links,
                "cover_url": cover_url,
                "status": status,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/books/{book_id}/editions"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Add edition failed")?
                .json().await?;

            let eid = resp["id"].as_str().unwrap_or("?");
            println!("Added edition to book {book_id}: {title} ({lang})");
            println!("Edition ID: {eid}");
        }

        BookCommand::UpdateEdition {
            book_id, edition_id, title, subtitle, edition_name, lang,
            isbn, publisher, year, translators, purchase_links, cover_url, status,
        } => {
            let current: serde_json::Value = client()
                .get(format!("{base}/books/{book_id}"))
                .send().await?
                .error_for_status().context("Get book failed")?
                .json().await?;
            let ed = current["editions"].as_array()
                .and_then(|eds| eds.iter().find(|e| e["id"].as_str() == Some(&edition_id)))
                .with_context(|| format!("Edition {edition_id} not found on book {book_id}"))?;

            let merge_str = |new: Option<String>, key: &str| -> Option<String> {
                new.or_else(|| ed[key].as_str().map(String::from))
            };
            let title = merge_str(title, "title").context("title missing")?;
            let edition_name = merge_str(edition_name, "edition_name").unwrap_or_else(|| title.clone());
            let lang = merge_str(lang, "lang").context("lang missing")?;
            let subtitle = subtitle
                .map(|s| if s.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(s) })
                .unwrap_or_else(|| ed["subtitle"].clone());
            let isbn = merge_str(isbn, "isbn").map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
            let publisher = merge_str(publisher, "publisher").map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
            let year = merge_str(year, "year").map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
            let cover_url = merge_str(cover_url, "cover_url").map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
            let translators = match translators {
                Some(v) => serde_json::Value::Array(v.into_iter().map(serde_json::Value::String).collect()),
                None => ed["translators"].clone(),
            };
            let purchase_links = match purchase_links {
                Some(pl) => serde_json::from_str(&pl).context("Invalid JSON for --purchase-links")?,
                None => ed["purchase_links"].clone(),
            };

            let status = status.or_else(|| ed["status"].as_str().map(String::from));
            let body = serde_json::json!({
                "title": title,
                "subtitle": subtitle,
                "edition_name": edition_name,
                "lang": lang,
                "isbn": isbn,
                "publisher": publisher,
                "year": year,
                "translators": translators,
                "purchase_links": purchase_links,
                "cover_url": cover_url,
                "status": status,
            });

            client()
                .put(format!("{base}/books/{book_id}/editions/{edition_id}"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Update edition failed")?;

            println!("Updated edition {edition_id}");
        }

        BookCommand::UploadCover { book_id, edition_id, file } => {
            let file_bytes = std::fs::read(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;
            let file_name = file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("cover.jpg");
            let part = reqwest::multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string());
            let form = reqwest::multipart::Form::new().part("file", part);

            client()
                .post(format!("{base}/books/{book_id}/editions/{edition_id}/cover"))
                .bearer_auth(token)
                .multipart(form)
                .send().await?
                .error_for_status().context("Upload cover failed")?;

            println!("Uploaded cover for edition {edition_id}");
        }

        BookCommand::Show { id } => {
            let resp: serde_json::Value = client()
                .get(format!("{base}/books/{id}"))
                .send().await?
                .error_for_status().context("Get book failed")?
                .json().await?;

            let book = &resp["book"];
            println!("Title: {}", book["title"].as_str().unwrap_or("?"));
            println!("Authors: {}", book["authors"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                .unwrap_or_default());
            if let Some(d) = book["description"].as_str() {
                if !d.is_empty() { println!("Description: {d}"); }
            }

            if let Some(editions) = resp["editions"].as_array() {
                if !editions.is_empty() {
                    println!("\nEditions:");
                    for ed in editions {
                        let etitle = ed["title"].as_str().unwrap_or("?");
                        let elang = ed["lang"].as_str().unwrap_or("?");
                        let eisbn = ed["isbn"].as_str().unwrap_or("-");
                        println!("  [{elang}] {etitle}  ISBN: {eisbn}");
                    }
                }
            }

            let review_count = resp["review_count"].as_u64().unwrap_or(0);
            println!("\n{review_count} review(s)");

            if let Some(chapters) = resp["chapters"].as_array() {
                if !chapters.is_empty() {
                    println!("\nTable of Contents:");
                    for ch in chapters {
                        let ctitle = ch["title"].as_str().unwrap_or("?");
                        let cid = ch["id"].as_str().unwrap_or("?");
                        let indent = if ch["parent_id"].is_null() { "" } else { "  " };
                        println!("  {indent}{ctitle}  ({cid})");
                    }
                }
            }
        }

        BookCommand::AddChapter { book_id, title, parent_id, order, article_uri, teaches, prereqs } => {
            let prereqs_json: Vec<serde_json::Value> = prereqs.iter().map(|p| {
                let (tag_id, prereq_type) = p.split_once(':').unwrap_or((p, "required"));
                serde_json::json!({ "tag_id": tag_id, "prereq_type": prereq_type })
            }).collect();
            let body = serde_json::json!({
                "title": title,
                "parent_id": parent_id,
                "order_index": order,
                "article_uri": article_uri,
                "teaches": teaches,
                "prereqs": prereqs_json,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/books/{book_id}/chapters"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Add chapter failed")?
                .json().await?;

            let cid = resp["id"].as_str().unwrap_or("?");
            println!("Added chapter: {title} ({cid})");
        }

        BookCommand::UploadChapters { book_id, manifest, lang, license, dry_run } => {
            let manifest_dir = manifest.parent().unwrap_or(std::path::Path::new("."));
            let manifest_text = std::fs::read_to_string(&manifest)
                .with_context(|| format!("Cannot read {}", manifest.display()))?;
            let cm: ChapterManifest = toml::from_str(&manifest_text)
                .context("Invalid chapters TOML")?;

            let mut key_to_id: std::collections::HashMap<String, String> = std::collections::HashMap::new();

            for (i, ch) in cm.chapters.iter().enumerate() {
                let parent_id = ch.parent.as_ref().and_then(|k| key_to_id.get(k)).cloned();

                let article_uri: Option<String> = if let Some(ref rel_path) = ch.file {
                    let abs_path = manifest_dir.join(rel_path);
                    let content = std::fs::read_to_string(&abs_path)
                        .with_context(|| format!("Cannot read {}", abs_path.display()))?;
                    let ext = abs_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    let fmt = match ext { "typ" => "typst", "html" => "html", _ => "markdown" };

                    if dry_run {
                        println!("[dry-run] Would upload {} as article ({fmt})", abs_path.display());
                        None
                    } else {
                        let article_body = serde_json::json!({
                            "title": ch.title,
                            "content": content,
                            "content_format": fmt,
                            "lang": lang,
                            "license": license,
                            "tags": [],
                            "prereqs": [],
                            "book_id": book_id,
                        });
                        let article_resp: serde_json::Value = client()
                            .post(format!("{base}/articles"))
                            .bearer_auth(token)
                            .json(&article_body)
                            .send().await
                            .with_context(|| format!("Upload failed for chapter {}", i + 1))?
                            .error_for_status()
                            .with_context(|| format!("Server rejected chapter {} article", i + 1))?
                            .json().await?;
                        let uri = article_resp["at_uri"].as_str()
                            .map(String::from)
                            .context("No at_uri in article response")?;
                        println!("  Uploaded article: {uri}");
                        Some(uri)
                    }
                } else {
                    None
                };

                let prereqs_json: Vec<serde_json::Value> = ch.prereqs.iter().map(|p| {
                    serde_json::json!({ "tag_id": p.tag, "prereq_type": p.r#type })
                }).collect();

                let chapter_body = serde_json::json!({
                    "title": ch.title,
                    "parent_id": parent_id,
                    "order_index": ch.order,
                    "article_uri": article_uri,
                    "teaches": ch.teaches,
                    "prereqs": prereqs_json,
                });

                if dry_run {
                    println!("[dry-run] Would create chapter: {} (order={}, parent={:?}, teaches={:?})",
                        ch.title, ch.order, ch.parent, ch.teaches);
                    if !ch.key.is_empty() {
                        key_to_id.insert(ch.key.clone(), format!("dry-{}", i));
                    }
                    continue;
                }

                let chapter_resp: serde_json::Value = client()
                    .post(format!("{base}/books/{book_id}/chapters"))
                    .bearer_auth(token)
                    .json(&chapter_body)
                    .send().await
                    .with_context(|| format!("Create chapter failed: {}", ch.title))?
                    .error_for_status()
                    .with_context(|| format!("Server rejected chapter: {}", ch.title))?
                    .json().await?;

                let cid = chapter_resp["id"].as_str().unwrap_or("?").to_string();
                println!("Chapter: {} ({cid})", ch.title);
                if !ch.key.is_empty() {
                    key_to_id.insert(ch.key.clone(), cid);
                }
            }

            if !dry_run {
                println!("\nDone. {} chapter(s) created.", cm.chapters.len());
            }
        }

        BookCommand::AddResource { book_id, kind, label, url, edition_id, position } => {
            let body = serde_json::json!({
                "kind": kind,
                "label": label,
                "url": url,
                "edition_id": edition_id,
                "position": position,
            });
            let resp: serde_json::Value = client()
                .post(format!("{base}/books/{book_id}/resources"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Add resource failed")?
                .json().await?;
            let id = resp["id"].as_str().unwrap_or("?");
            println!("Added resource: {label} ({id})");
        }

        BookCommand::ShortReview { book_id, body, rating, edition_id, visibility } => {
            if !(1..=10).contains(&rating) {
                bail!("Rating must be 1-10 (half-stars)");
            }
            let req_body = serde_json::json!({
                "body": body,
                "rating": rating,
                "edition_id": edition_id,
                "visibility": visibility.as_deref().unwrap_or("public"),
            });
            let resp: serde_json::Value = client()
                .post(format!("{base}/books/{book_id}/short-reviews"))
                .bearer_auth(token)
                .json(&req_body)
                .send().await?
                .error_for_status().context("Short review failed")?
                .json().await?;
            let id = resp["id"].as_str().unwrap_or("?");
            println!("Short review saved: {id}");
        }

        BookCommand::ShortReviewList { book_id } => {
            let reviews: Vec<serde_json::Value> = client()
                .get(format!("{base}/books/{book_id}/short-reviews"))
                .send().await?
                .error_for_status().context("List short reviews failed")?
                .json().await?;
            if reviews.is_empty() {
                println!("No short reviews for {book_id}.");
            } else {
                for r in &reviews {
                    let did = r["did"].as_str().unwrap_or("?");
                    let rating = r["rating"].as_i64().unwrap_or(0);
                    let body = r["body"].as_str().unwrap_or("");
                    println!("  {did}  [{rating}/10]  {body}");
                }
                println!("{} short review(s)", reviews.len());
            }
        }

        BookCommand::ShortReviewDelete { book_id } => {
            client()
                .delete(format!("{base}/books/{book_id}/short-reviews/my"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("Delete short review failed")?;
            println!("Short review deleted.");
        }
    }
    Ok(())
}
