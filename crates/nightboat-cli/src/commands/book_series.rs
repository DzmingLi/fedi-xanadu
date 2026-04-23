use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Subcommand;

use crate::{Config, client};

#[derive(Subcommand)]
pub enum BookSeriesCommand {
    /// List all book series
    #[command(alias = "ls")]
    List,
    /// Create a new book series
    Create {
        /// Series ID (e.g. bs-wangdao-cs-408)
        #[arg(long)]
        id: String,
        /// Title as JSON (e.g. '{"zh":"王道考研408全套","en":"Wangdao CS 408 Series"}')
        #[arg(short, long)]
        title: String,
        /// Description
        #[arg(short, long)]
        desc: Option<String>,
        /// Cover image URL
        #[arg(long)]
        cover_url: Option<String>,
    },
    /// Show a series' detail
    Show {
        /// Series ID
        id: String,
    },
    /// Update a series' metadata
    Update {
        /// Series ID
        id: String,
        /// New title JSON
        #[arg(short, long)]
        title: Option<String>,
        /// New description JSON
        #[arg(short, long)]
        desc: Option<String>,
    },
    /// Add a book to a series
    #[command(name = "add-member")]
    AddMember {
        /// Series ID
        #[arg(long)]
        series_id: String,
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Position (0-based)
        #[arg(long, default_value = "0")]
        position: i16,
    },
    /// Remove a book from a series
    #[command(name = "remove-member")]
    RemoveMember {
        /// Series ID
        #[arg(long)]
        series_id: String,
        /// Book ID
        #[arg(long)]
        book_id: String,
    },
    /// Upload a cover image for a series
    #[command(name = "upload-cover")]
    UploadCover {
        /// Series ID
        #[arg(long)]
        id: String,
        /// Path to image file
        file: PathBuf,
    },
}

pub async fn handle_book_series(base: &str, config: &Config, action: BookSeriesCommand) -> Result<()> {
    let token = config.token().unwrap_or_default().to_string();

    match action {
        BookSeriesCommand::List => {
            let resp: Vec<serde_json::Value> = client()
                .get(format!("{base}/book-series"))
                .send().await?
                .error_for_status().context("List series failed")?
                .json().await?;
            if resp.is_empty() {
                println!("No book series yet.");
            } else {
                for s in &resp {
                    let id = s["id"].as_str().unwrap_or("?");
                    let title = s["title"].as_object()
                        .and_then(|t| t.get("zh").or_else(|| t.get("en")))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let count = s["member_count"].as_i64().unwrap_or(0);
                    println!("  {id}  {title}  ({count} books)");
                }
                println!("{} series", resp.len());
            }
        }

        BookSeriesCommand::Create { id, title, desc, cover_url } => {
            let parse_i18n = |s: &str| -> serde_json::Value {
                if s.starts_with('{') {
                    serde_json::from_str(s).unwrap_or(serde_json::json!({"zh": s}))
                } else {
                    serde_json::json!({"zh": s})
                }
            };
            let title_val = parse_i18n(&title);
            let desc_val = desc.as_deref().map(parse_i18n).unwrap_or(serde_json::json!({}));
            let body = serde_json::json!({
                "id": id,
                "title": title_val,
                "description": desc_val,
                "cover_url": cover_url,
            });
            let resp: serde_json::Value = client()
                .post(format!("{base}/book-series"))
                .bearer_auth(&token)
                .json(&body)
                .send().await?
                .error_for_status().context("Create series failed")?
                .json().await?;
            println!("Created series: {}", resp["id"].as_str().unwrap_or(&id));
        }

        BookSeriesCommand::Show { id } => {
            let resp: serde_json::Value = client()
                .get(format!("{base}/book-series/{id}"))
                .send().await?
                .error_for_status().context("Series not found")?
                .json().await?;
            let s = &resp["series"];
            let title = s["title"].as_object()
                .and_then(|t| t.get("zh").or_else(|| t.get("en")))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            println!("ID: {id}");
            println!("Title: {title}");
            let avg = resp["member_avg_rating"].as_f64().unwrap_or(0.0);
            let count = resp["member_rating_count"].as_i64().unwrap_or(0);
            println!("Member avg rating: {avg:.1} ({count} ratings)");
            if let Some(members) = resp["members"].as_array() {
                println!("\nMembers ({}):", members.len());
                for m in members {
                    let bid = m["id"].as_str().unwrap_or("?");
                    let btitle = m["title"].as_object()
                        .and_then(|t| t.get("zh").or_else(|| t.get("en")))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    println!("  {bid}  {btitle}");
                }
            }
        }

        BookSeriesCommand::Update { id, title, desc } => {
            let parse_i18n = |s: &str| -> serde_json::Value {
                if s.starts_with('{') {
                    serde_json::from_str(s).unwrap_or(serde_json::json!({"zh": s}))
                } else {
                    serde_json::json!({"zh": s})
                }
            };
            let mut body = serde_json::json!({});
            if let Some(t) = title { body["title"] = parse_i18n(&t); }
            if let Some(d) = desc { body["description"] = parse_i18n(&d); }
            client()
                .put(format!("{base}/book-series/{id}"))
                .bearer_auth(&token)
                .json(&body)
                .send().await?
                .error_for_status().context("Update series failed")?;
            println!("Updated series {id}");
        }

        BookSeriesCommand::AddMember { series_id, book_id, position } => {
            let body = serde_json::json!({ "book_id": book_id, "position": position });
            client()
                .post(format!("{base}/book-series/{series_id}/members"))
                .bearer_auth(&token)
                .json(&body)
                .send().await?
                .error_for_status().context("Add member failed")?;
            println!("Added {book_id} to series {series_id} at position {position}");
        }

        BookSeriesCommand::RemoveMember { series_id, book_id } => {
            client()
                .delete(format!("{base}/book-series/{series_id}/members/{book_id}"))
                .bearer_auth(&token)
                .send().await?
                .error_for_status().context("Remove member failed")?;
            println!("Removed {book_id} from series {series_id}");
        }

        BookSeriesCommand::UploadCover { id, file } => {
            let file_bytes = std::fs::read(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;
            let file_name = file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("cover.jpg");
            let part = reqwest::multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string());
            let form = reqwest::multipart::Form::new().part("file", part);
            client()
                .post(format!("{base}/book-series/{id}/cover"))
                .bearer_auth(&token)
                .multipart(form)
                .send().await?
                .error_for_status().context("Upload cover failed")?;
            println!("Uploaded cover for series {id}");
        }
    }

    Ok(())
}
