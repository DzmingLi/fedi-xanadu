use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{Config, client};
use super::util;

#[derive(Subcommand)]
pub enum BookCommand {
    /// List all books
    #[command(alias = "ls")]
    List {
        /// Filter by exam-prep tags. Pass "any" / "none" to filter on
        /// any/no tag, or a specific tag (e.g. "kaoyan-408") to require
        /// that tag.
        #[arg(long)]
        exam: Option<String>,
    },
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
        /// Exam-prep tags (comma-separated, e.g. "kaoyan-math-1,kaoyan-408")
        #[arg(long, value_delimiter = ',')]
        exam_tags: Vec<String>,
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
        /// Replace exam-prep tags (comma-separated, e.g. "kaoyan-math-1,kaoyan-408").
        /// Pass `--clear-exam` to mark the book as non-exam.
        #[arg(long, value_delimiter = ',')]
        exam_tags: Option<Vec<String>>,
        /// Clear exam-prep tags (mark non-exam).
        #[arg(long)]
        clear_exam: bool,
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
    /// Delete a book. Cascades to every edition, chapter, resource, and
    /// rating. The final `book_edit_log` entry survives via FK SET NULL,
    /// so the audit trail persists after the row is gone.
    Delete {
        /// Book ID
        id: String,
    },
    /// Delete a single edition from a book
    #[command(name = "delete-edition")]
    DeleteEdition {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Edition ID
        #[arg(long)]
        edition_id: String,
    },
    /// Ingest a book from one or more douban URLs.
    ///
    /// The first URL becomes the book shell + first edition. Remaining URLs
    /// are added as extra editions. Covers are downloaded and uploaded.
    /// Each douban URL is also added as a resource (kind=other, label=豆瓣).
    #[command(name = "ingest-douban")]
    IngestDouban {
        /// Douban subject URL(s), space separated. First URL sets the book's
        /// canonical title/authors; remaining ones become additional editions.
        #[arg(required = true, num_args = 1..)]
        urls: Vec<String>,
        /// Language for each URL in order (comma-separated, e.g. en,en,zh).
        /// Defaults to en for all if not provided.
        #[arg(long, value_delimiter = ',')]
        langs: Vec<String>,
        /// Edition name for each URL in order (comma-separated, e.g. "First Edition,Second Edition").
        /// Defaults to the parsed edition info from douban.
        #[arg(long, value_delimiter = ',')]
        edition_names: Vec<String>,
        /// Tags to apply to the book (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Prereq tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        prereqs: Vec<String>,
        /// Override book title (by default uses the canonical title from the first URL)
        #[arg(long)]
        title: Option<String>,
        /// Dry run — print what would be created without doing anything
        #[arg(long)]
        dry_run: bool,
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
        BookCommand::List { exam } => {
            let mut req = client().get(format!("{base}/books"));
            if let Some(ref e) = exam {
                req = req.query(&[("exam", e.as_str())]);
            }
            let resp: Vec<serde_json::Value> = req
                .send().await?
                .error_for_status().context("List books failed")?
                .json().await?;

            if resp.is_empty() {
                println!("No books yet.");
            } else {
                for b in &resp {
                    let id = b["id"].as_str().unwrap_or("?");
                    let title = util::i18n_display(&b["title"]);
                    let authors = b["authors"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                        .unwrap_or_default();
                    println!("  {id}  {title}  ({authors})");
                }
                println!("{} book(s)", resp.len());
            }
        }

        BookCommand::Create { title, subtitle, authors, desc, cover_url, tags, prereqs,
                             exam_tags,
                             edition, lang, isbn, publisher, year, translators, purchase_links, edition_cover_url, edition_subtitle } => {
            let title_val = util::parse_i18n(&title);
            let subtitle_val = subtitle.as_deref().map(util::parse_i18n).unwrap_or(serde_json::json!({}));
            let desc_val = desc.as_deref().map(util::parse_i18n).unwrap_or(serde_json::json!({}));
            let exam_tags_val: Option<&Vec<String>> = if exam_tags.is_empty() { None } else { Some(&exam_tags) };
            let links: Vec<serde_json::Value> = if let Some(ref pl) = purchase_links {
                serde_json::from_str(pl).context("Invalid JSON for --purchase-links")?
            } else {
                vec![]
            };
            // Backend requires first_edition; bundling it into the same POST
            // means a book never lives without at least one edition.
            let body = serde_json::json!({
                "title": title_val,
                "subtitle": subtitle_val,
                "authors": authors,
                "description": desc_val,
                "cover_url": cover_url,
                "tags": tags,
                "prereqs": prereqs,
                "exam_tags": exam_tags_val,
                "first_edition": {
                    "title": title,
                    "subtitle": edition_subtitle,
                    "edition_name": edition,
                    "lang": lang,
                    "isbn": isbn,
                    "publisher": publisher,
                    "year": year,
                    "translators": translators,
                    "purchase_links": links,
                    "cover_url": edition_cover_url.or(cover_url.clone()),
                },
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
            println!("First edition: {edition} ({lang})");
        }

        BookCommand::Update { id, title, desc, cover_url, summary, exam_tags, clear_exam } => {
            let mut body = serde_json::json!({
                "id": id,
                "title": title,
                "description": desc,
                "cover_url": cover_url,
                "edit_summary": summary,
            });
            // JSON null deserializes as `Some(None)` via deserialize_double_option,
            // which the server treats as "clear the field".
            if clear_exam {
                body["exam_tags"] = serde_json::Value::Null;
            } else if let Some(tags) = exam_tags {
                body["exam_tags"] = serde_json::to_value(&tags)?;
            }

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
            println!("Title: {}", util::i18n_display(&book["title"]));
            let subtitle = util::i18n_display(&book["subtitle"]);
            if subtitle != "?" && !subtitle.is_empty() {
                println!("Subtitle: {subtitle}");
            }
            println!("Authors: {}", book["authors"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                .unwrap_or_default());
            let desc = util::i18n_display(&book["description"]);
            if desc != "?" && !desc.is_empty() {
                println!("Description: {desc}");
            }

            if let Some(editions) = resp["editions"].as_array() {
                if !editions.is_empty() {
                    println!("\nEditions:");
                    for ed in editions {
                        let ed_id = ed["id"].as_str().unwrap_or("?");
                        let ed_title = ed["title"].as_str().unwrap_or("?");
                        let ed_lang = ed["lang"].as_str().unwrap_or("?");
                        let ed_isbn = ed["isbn"].as_str().unwrap_or("-");
                        let ed_name = ed["edition_name"].as_str().unwrap_or("");
                        let ed_year = ed["year"].as_str().unwrap_or("-");
                        let ed_publisher = ed["publisher"].as_str().unwrap_or("-");
                        let name_part = if ed_name.is_empty() { String::new() } else { format!(" — {ed_name}") };
                        println!("  [{ed_lang}] {ed_title}{name_part}  ({ed_publisher}, {ed_year})  ISBN: {ed_isbn}  {ed_id}");
                    }
                }
            }

            let review_count = resp["review_count"].as_u64().unwrap_or(0);
            println!("\n{review_count} review(s)");

            if let Some(chapters) = resp["chapters"].as_array() {
                if !chapters.is_empty() {
                    println!("\nTable of Contents:");
                    for ch in chapters {
                        let ch_title = ch["title"].as_str().unwrap_or("?");
                        let ch_id = ch["id"].as_str().unwrap_or("?");
                        let indent = if ch["parent_id"].is_null() { "" } else { "  " };
                        println!("  {indent}{ch_title}  ({ch_id})");
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

        BookCommand::Delete { id } => {
            client()
                .delete(format!("{base}/books/{id}"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("Delete book failed")?;
            println!("Deleted book {id} — audit entry recorded in book_edit_log.");
        }

        BookCommand::DeleteEdition { book_id, edition_id } => {
            client()
                .delete(format!("{base}/books/{book_id}/editions/{edition_id}"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("Delete edition failed")?;
            println!("Deleted edition {edition_id} from book {book_id}.");
        }

        BookCommand::IngestDouban { urls, langs, edition_names, tags, prereqs, title: title_override, dry_run } => {
            handle_ingest_douban(base, &token, urls, langs, edition_names, tags, prereqs, title_override, dry_run).await?;
        }
    }
    Ok(())
}

async fn handle_ingest_douban(
    base: &str,
    token: &str,
    urls: Vec<String>,
    langs: Vec<String>,
    edition_names: Vec<String>,
    tags: Vec<String>,
    prereqs: Vec<String>,
    title_override: Option<String>,
    dry_run: bool,
) -> Result<()> {
    // Fetch + parse each douban page up front so we fail fast on bad URLs.
    let mut fetched = Vec::with_capacity(urls.len());
    for (i, url) in urls.iter().enumerate() {
        eprintln!("[{}/{}] fetching {url}", i + 1, urls.len());
        let d = util::fetch_douban(url).await
            .with_context(|| format!("parsing douban page {url}"))?;
        fetched.push(d);
    }

    let first = fetched.first().context("no douban URLs provided")?;
    let canonical_title = title_override.clone().unwrap_or_else(|| {
        // Prefer the original (English) title if douban exposes it — that
        // stays stable across translation/reprint pages.
        first.original_title.clone().unwrap_or_else(|| first.title.clone().unwrap_or_default())
    });
    let canonical_authors = first.authors.clone();
    let canonical_desc = first.description.clone();

    if dry_run {
        println!("DRY RUN — no changes would be made.");
        println!("Book title: {canonical_title}");
        println!("Authors:    {}", canonical_authors.join(", "));
        for (i, d) in fetched.iter().enumerate() {
            let lang = langs.get(i).cloned().unwrap_or_else(|| "en".to_string());
            let en = edition_names.get(i).cloned().unwrap_or_else(|| format!("Edition {}", i + 1));
            println!("  Edition #{}: [{lang}] {en}  ISBN={}  {} ({})",
                i + 1,
                d.isbn.clone().unwrap_or_else(|| "-".to_string()),
                d.publisher.clone().unwrap_or_else(|| "-".to_string()),
                d.year.clone().unwrap_or_else(|| "-".to_string()),
            );
            if let Some(ref c) = d.cover_url { println!("    cover: {c}"); }
            println!("    douban: {}", d.douban_url);
        }
        return Ok(());
    }

    // 1. Create the book shell. Use --no-edition semantics: send
    //    POST /books with just the book-level fields, skip first edition.
    let title_json = serde_json::json!({ "en": canonical_title });
    let desc_json = canonical_desc
        .as_deref()
        .map(|d| serde_json::json!({ "en": d }))
        .unwrap_or(serde_json::json!({}));
    let body = serde_json::json!({
        "title": title_json,
        "subtitle": serde_json::json!({}),
        "authors": canonical_authors,
        "description": desc_json,
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
    let book_id = resp["id"].as_str().context("no book id in response")?.to_string();
    println!("Created book: {canonical_title} ({book_id})");

    // 2. For each douban URL: add edition, upload cover, add douban resource.
    for (i, d) in fetched.iter().enumerate() {
        let lang = langs.get(i).cloned().unwrap_or_else(|| "en".to_string());
        let edition_name = edition_names.get(i).cloned().unwrap_or_else(|| {
            if i == 0 { "First Edition".to_string() } else { format!("Edition {}", i + 1) }
        });
        let ed_title = d.original_title.clone()
            .or_else(|| d.title.clone())
            .unwrap_or_else(|| canonical_title.clone());
        let ed_body = serde_json::json!({
            "title": ed_title,
            "subtitle": d.subtitle,
            "edition_name": edition_name,
            "lang": lang,
            "isbn": d.isbn,
            "publisher": d.publisher,
            "year": d.year,
            "translators": d.translators,
            "purchase_links": serde_json::json!([]),
            "cover_url": null,
        });
        let ed_resp: serde_json::Value = client()
            .post(format!("{base}/books/{book_id}/editions"))
            .bearer_auth(token)
            .json(&ed_body)
            .send().await?
            .error_for_status()
            .with_context(|| format!("Add edition #{} failed", i + 1))?
            .json().await?;
        let edition_id = ed_resp["id"].as_str().context("no edition id")?.to_string();
        println!("  Added edition: {edition_name} ({edition_id})");

        // Cover — download with referer then upload via multipart.
        if let Some(ref cover_url) = d.cover_url {
            let local = util::download_to_tempfile(cover_url, &d.douban_url).await
                .with_context(|| format!("download cover from {cover_url}"))?;
            let bytes = std::fs::read(&local)?;
            let filename = local.file_name().and_then(|n| n.to_str()).unwrap_or("cover.jpg").to_string();
            let form = reqwest::multipart::Form::new()
                .part("file", reqwest::multipart::Part::bytes(bytes).file_name(filename));
            client()
                .post(format!("{base}/books/{book_id}/editions/{edition_id}/cover"))
                .bearer_auth(token)
                .multipart(form)
                .send().await?
                .error_for_status().context("Upload cover failed")?;
            let _ = std::fs::remove_file(&local);
            println!("    uploaded cover ({})", local.display());
        }

        // Douban resource — via book_resources, per the project rule that
        // douban is not a purchase link.
        let res_body = serde_json::json!({
            "edition_id": edition_id,
            "kind": "other",
            "label": "豆瓣",
            "url": d.douban_url,
            "position": 0,
        });
        client()
            .post(format!("{base}/books/{book_id}/resources"))
            .bearer_auth(token)
            .json(&res_body)
            .send().await?
            .error_for_status().context("Add douban resource failed")?;
        println!("    linked douban");
    }

    println!("\nIngested {} edition(s) into {book_id}.", fetched.len());
    Ok(())
}
