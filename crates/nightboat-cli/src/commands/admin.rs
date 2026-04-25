use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Subcommand;

use crate::{Config, client, resolve_did_or_handle, validate_html_fragment};

#[derive(Subcommand)]
pub enum AdminCommand {
    /// Create a platform user
    #[command(name = "create-user")]
    CreateUser {
        handle: String,
        password: String,
        #[arg(long)]
        display_name: Option<String>,
    },
    /// List all platform users
    #[command(name = "list-users", alias = "users")]
    ListUsers,
    /// Set a localized name for a tag
    #[command(name = "set-tag-name")]
    SetTagName {
        id: String,
        locale: String,
        name: String,
    },
    /// Add an alias for a tag (e.g. "CV" -> "computer-vision")
    #[command(name = "add-tag-alias")]
    AddTagAlias {
        id: String,
        alias: String,
    },
    /// Remove a tag alias
    #[command(name = "rm-tag-alias")]
    RmTagAlias {
        alias: String,
    },
    /// Merge one tag into another (migrate all references)
    #[command(name = "merge-tag")]
    MergeTag {
        #[arg(long)]
        from: String,
        #[arg(long)]
        into: String,
    },
    /// Create a series as a platform user.
    #[command(name = "create-series")]
    CreateSeries {
        #[arg(long)]
        r#as: String,
        #[arg(long, value_name = "META_YAML")]
        from: Option<PathBuf>,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        desc: Option<String>,
        #[arg(long)]
        topics: Option<String>,
        #[arg(long)]
        parent: Option<String>,
        #[arg(short, long)]
        lang: Option<String>,
        #[arg(long)]
        translation_of: Option<String>,
    },
    /// Rebuild a series' DB index from its source files. (Disabled under
    /// the blob storage model — pending redesign; see server-side stub.)
    #[command(name = "rebuild-series-index")]
    RebuildSeriesIndex {
        #[arg(long)]
        series: String,
    },
    /// Upload a cover image for a series (admin override)
    #[command(name = "upload-series-cover")]
    UploadSeriesCover {
        #[arg(long)]
        id: String,
        #[arg(short, long)]
        file: PathBuf,
    },
    /// Reference an existing file in the series' blob cache as its cover
    #[command(name = "set-series-cover-ref")]
    SetSeriesCoverRef {
        #[arg(long)]
        id: String,
        #[arg(short, long)]
        file: String,
    },
    /// Add an article to a series
    #[command(name = "add-to-series")]
    AddToSeries {
        #[arg(long)]
        series: String,
        #[arg(long)]
        article: String,
    },
    /// Update an article's content (admin override)
    Update {
        #[arg(long)]
        uri: String,
        #[arg(short, long)]
        file: Option<PathBuf>,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        desc: Option<String>,
    },
    /// Ban a user (by DID or handle)
    #[command(name = "ban-user")]
    BanUser {
        did_or_handle: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Unban a user
    #[command(name = "unban-user")]
    UnbanUser {
        did_or_handle: String,
    },
    /// List all banned users
    #[command(name = "banned-users")]
    BannedUsers,
    /// Delete an article (soft-delete with appeal window)
    #[command(name = "delete-article")]
    DeleteArticle {
        uri: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Set article visibility
    #[command(name = "set-visibility")]
    SetVisibility {
        uri: String,
        visibility: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// List pending appeals
    #[command(name = "appeals")]
    Appeals,
    /// Resolve an appeal
    #[command(name = "resolve-appeal")]
    ResolveAppeal {
        id: String,
        #[arg(long)]
        status: String,
        #[arg(long)]
        response: Option<String>,
    },
    /// Merge two questions
    #[command(name = "merge-questions")]
    MergeQuestions {
        #[arg(long)]
        from: String,
        #[arg(long)]
        into: String,
    },
    /// Publish a question as a platform user
    #[command(name = "publish-question")]
    PublishQuestion {
        #[arg(long)]
        r#as: String,
        #[arg(short, long)]
        file: PathBuf,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        desc: Option<String>,
        #[arg(short, long, default_value = "zh")]
        lang: String,
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
    },
    /// Post an answer to a question as a platform user
    #[command(name = "publish-answer")]
    PublishAnswer {
        #[arg(long)]
        r#as: String,
        #[arg(long)]
        question: String,
        #[arg(short, long)]
        file: PathBuf,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        desc: Option<String>,
        #[arg(short, long, default_value = "zh")]
        lang: String,
    },
    /// Verify a user's credentials
    #[command(name = "verify-credentials")]
    VerifyCredentials {
        did_or_handle: String,
        #[arg(long)]
        education: Option<String>,
        #[arg(long)]
        affiliation: Option<String>,
    },
    /// Revoke a user's credentials verification
    #[command(name = "revoke-credentials")]
    RevokeCredentials {
        did_or_handle: String,
    },
    /// Revert a book edit by edit log ID
    #[command(name = "revert-book-edit")]
    RevertBookEdit {
        edit_id: String,
    },
    /// Show edit history for a book
    #[command(name = "book-history")]
    BookHistory {
        book_id: String,
    },
    /// Publish an article as a platform user
    Publish {
        #[arg(long)]
        r#as: String,
        #[arg(short, long)]
        file: PathBuf,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        desc: Option<String>,
        #[arg(long, default_value = "general")]
        category: String,
        #[arg(long)]
        book_id: Option<String>,
        #[arg(short, long, default_value = "zh")]
        lang: String,
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        #[arg(long, default_value = "CC-BY-SA-3.0")]
        license: String,
        #[arg(long)]
        translation_of: Option<String>,
        #[arg(long)]
        series: Option<String>,
        #[arg(long, value_delimiter = ',')]
        resource: Vec<PathBuf>,
    },
    /// Import a directory tree as series chapters with resources
    #[command(name = "import-dir")]
    ImportDir {
        #[arg(long)]
        r#as: String,
        #[arg(long)]
        series: String,
        #[arg(short, long)]
        dir: PathBuf,
        #[arg(short, long, default_value = "zh")]
        lang: String,
        #[arg(long, default_value = "CC-BY-SA-4.0")]
        license: String,
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Import a repository directory as a series using batch-publish
    #[command(name = "import-repo")]
    ImportRepo {
        #[arg(long)]
        r#as: String,
        #[arg(long)]
        series: String,
        #[arg(short, long)]
        dir: PathBuf,
        #[arg(short, long)]
        manifest: PathBuf,
        #[arg(short, long, default_value = "en")]
        lang: String,
        #[arg(long, default_value = "CC-BY-SA-4.0")]
        license: String,
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        image_dirs: Vec<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
}

pub async fn handle_admin(base: &str, config: &mut Config, action: AdminCommand) -> Result<()> {
    let secret = std::env::var("NBT_ADMIN_SECRET")
        .ok()
        .or_else(|| std::env::var("FX_ADMIN_SECRET").ok())
        .or_else(|| config.admin_secret.clone())
        .context("Admin secret not set. Use NBT_ADMIN_SECRET env var")?;

    match action {
        AdminCommand::CreateUser { handle, password, display_name } => {
            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/platform-users"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({
                    "handle": handle,
                    "password": password,
                    "display_name": display_name,
                }))
                .send().await?
                .error_for_status().context("Create user failed")?
                .json().await?;

            let did = resp["did"].as_str().unwrap_or("?");
            println!("Created: {handle} ({did})");
        }

        AdminCommand::ListUsers => {
            let users: Vec<serde_json::Value> = client()
                .get(format!("{base}/admin/platform-users"))
                .header("x-admin-secret", &secret)
                .send().await?
                .error_for_status().context("List users failed")?
                .json().await?;

            if users.is_empty() {
                println!("No platform users.");
            }
            for u in &users {
                let handle = u["handle"].as_str().unwrap_or("?");
                let did = u["did"].as_str().unwrap_or("?");
                let name = u["display_name"].as_str().unwrap_or("");
                println!("{handle}\t{did}\t{name}");
            }
        }

        AdminCommand::SetTagName { id, locale, name } => {
            let tag: serde_json::Value = client()
                .get(format!("{base}/tags/by-id"))
                .query(&[("id", &id)])
                .send().await?
                .error_for_status().context("Tag not found")?
                .json().await?;

            let mut names: std::collections::HashMap<String, String> = tag["names"]
                .as_object()
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                .unwrap_or_default();

            names.insert(locale.clone(), name.clone());

            let resp: serde_json::Value = client()
                .post(format!("{base}/tags/names"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "id": id, "names": names }))
                .send().await?
                .error_for_status().context("Update tag names failed")?
                .json().await?;

            let updated_names = &resp["names"];
            println!("Updated tag '{id}': {updated_names}");
        }

        AdminCommand::AddTagAlias { id, alias } => {
            client()
                .post(format!("{base}/admin/tags/alias"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "tag_id": id, "alias": alias }))
                .send().await?
                .error_for_status().context("Add alias failed")?;
            println!("Added alias '{alias}' -> tag '{id}'");
        }

        AdminCommand::RmTagAlias { alias } => {
            client()
                .delete(format!("{base}/admin/tags/alias"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "alias": alias }))
                .send().await?
                .error_for_status().context("Remove alias failed")?;
            println!("Removed alias '{alias}'");
        }

        AdminCommand::MergeTag { from, into } => {
            client()
                .post(format!("{base}/admin/tags/merge"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "from": from, "into": into }))
                .send().await?
                .error_for_status().context("Merge tag failed")?;

            println!("Merged tag '{from}' into '{into}'");
        }

        AdminCommand::MergeQuestions { from, into } => {
            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/questions/merge"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "from_uri": from, "into_uri": into }))
                .send().await?
                .error_for_status().context("Merge questions failed")?
                .json().await?;

            let moved = resp.get("answers_moved").and_then(|v| v.as_u64()).unwrap_or(0);
            println!("Merged question into '{into}' ({moved} answers moved)");
        }

        AdminCommand::CreateSeries { r#as: as_handle, from, title, desc, topics, parent, lang, translation_of } => {
            let from_meta: Option<fx_core::meta::SeriesMeta> = if let Some(path) = &from {
                let data = std::fs::read_to_string(path)
                    .with_context(|| format!("Cannot read {}", path.display()))?;
                Some(serde_yml::from_str(&data).context("Invalid meta.yaml")?)
            } else {
                None
            };

            let resolved_title = title.clone()
                .or_else(|| from_meta.as_ref().map(|m| m.title.clone()))
                .context("title required (pass --title or --from with a `title:` field)")?;
            let resolved_desc = desc.clone()
                .or_else(|| from_meta.as_ref().and_then(|m| m.description.clone()));
            let resolved_lang = lang.clone()
                .or_else(|| from_meta.as_ref().and_then(|m| m.lang.clone()));
            let resolved_topics: Vec<String> = topics.as_deref()
                .map(|t| t.split(',').map(str::to_string).collect())
                .or_else(|| from_meta.as_ref().map(|m| m.topics.clone()))
                .unwrap_or_default();

            let body = serde_json::json!({
                "as_handle": as_handle,
                "title": resolved_title,
                "description": resolved_desc,
                "topics": resolved_topics,
                "parent_id": parent,
                "lang": resolved_lang,
                "translation_of": translation_of,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/series"))
                .header("x-admin-secret", &secret)
                .json(&body)
                .send().await?
                .error_for_status().context("Create series failed")?
                .json().await?;

            let id = resp["id"].as_str().unwrap_or("?");
            println!("Created series: {resolved_title}");
            println!("ID: {id}");
        }

        AdminCommand::RebuildSeriesIndex { series } => {
            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/series/{series}/rebuild-index"))
                .header("x-admin-secret", &secret)
                .send().await?
                .error_for_status().context("Rebuild index failed")?
                .json().await?;
            let updated = resp.get("chapters_updated").and_then(|v| v.as_u64()).unwrap_or(0);
            let added = resp.get("chapters_added").and_then(|v| v.as_u64()).unwrap_or(0);
            let removed = resp.get("chapters_removed").and_then(|v| v.as_u64()).unwrap_or(0);
            println!("Rebuilt series {series}: +{added} / ~{updated} / -{removed}");
        }

        AdminCommand::UploadSeriesCover { id, file } => {
            let file_bytes = std::fs::read(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;
            let file_name = file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("cover.jpg");
            let part = reqwest::multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string());
            let form = reqwest::multipart::Form::new().part("file", part);

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/series/cover"))
                .query(&[("id", &id)])
                .header("x-admin-secret", &secret)
                .multipart(form)
                .send().await?
                .error_for_status().context("Upload series cover failed")?
                .json().await?;

            let url = resp["cover_url"].as_str().unwrap_or("?");
            println!("Uploaded cover for series {id}");
            println!("URL: {url}");
        }

        AdminCommand::SetSeriesCoverRef { id, file } => {
            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/series/cover/reference"))
                .query(&[("id", &id)])
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "file": file }))
                .send().await?
                .error_for_status().context("Set series cover reference failed")?
                .json().await?;

            let url = resp["cover_url"].as_str().unwrap_or("?");
            let f = resp["cover_file"].as_str().unwrap_or("?");
            println!("Set cover for series {id} → {f}");
            println!("URL: {url}");
        }

        AdminCommand::AddToSeries { series, article } => {
            client()
                .post(format!("{base}/admin/series/articles"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({
                    "series_id": series,
                    "article_uri": article,
                }))
                .send().await?
                .error_for_status().context("Add to series failed")?;

            println!("Added {article} to series {series}");
        }

        AdminCommand::Update { uri, file, title, desc } => {
            let mut body = serde_json::json!({ "uri": uri });
            if let Some(ref t) = title {
                body["title"] = serde_json::Value::String(t.clone());
            }
            if let Some(ref d) = desc {
                body["description"] = serde_json::Value::String(d.clone());
            }
            if let Some(ref f) = file {
                let content = std::fs::read_to_string(f)
                    .with_context(|| format!("Cannot read {}", f.display()))?;
                if f.extension().and_then(|e| e.to_str()) == Some("html") {
                    validate_html_fragment(&content)?;
                }
                body["content"] = serde_json::Value::String(content);
            }

            client()
                .post(format!("{base}/admin/articles/update"))
                .header("x-admin-secret", &secret)
                .json(&body)
                .send().await?
                .error_for_status().context("Admin update failed")?;

            println!("Updated article: {uri}");
        }

        AdminCommand::BanUser { did_or_handle, reason } => {
            let did = resolve_did_or_handle(&did_or_handle);
            client()
                .post(format!("{base}/admin/ban-user"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "did": did, "reason": reason }))
                .send().await?
                .error_for_status().context("Ban user failed")?;

            println!("Banned: {did_or_handle} ({did})");
        }

        AdminCommand::UnbanUser { did_or_handle } => {
            let did = resolve_did_or_handle(&did_or_handle);
            client()
                .post(format!("{base}/admin/unban-user"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "did": did }))
                .send().await?
                .error_for_status().context("Unban user failed")?;

            println!("Unbanned: {did_or_handle} ({did})");
        }

        AdminCommand::BannedUsers => {
            let users: Vec<serde_json::Value> = client()
                .get(format!("{base}/admin/banned-users"))
                .header("x-admin-secret", &secret)
                .send().await?
                .error_for_status().context("List banned users failed")?
                .json().await?;

            if users.is_empty() {
                println!("No banned users.");
            }
            for u in &users {
                let handle = u["handle"].as_str().unwrap_or("?");
                let did = u["did"].as_str().unwrap_or("?");
                let reason = u["ban_reason"].as_str().unwrap_or("-");
                let at = u["banned_at"].as_str().unwrap_or("?");
                println!("{handle}\t{did}\t{at}\t{reason}");
            }
        }

        AdminCommand::DeleteArticle { uri, reason } => {
            client()
                .post(format!("{base}/admin/articles/delete"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "uri": uri, "reason": reason }))
                .send().await?
                .error_for_status().context("Delete article failed")?;

            println!("Soft-deleted (30-day appeal window): {uri}");
        }

        AdminCommand::SetVisibility { uri, visibility, reason } => {
            client()
                .post(format!("{base}/admin/articles/visibility"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "uri": uri, "visibility": visibility, "reason": reason }))
                .send().await?
                .error_for_status().context("Set visibility failed")?;

            println!("Set visibility to '{visibility}': {uri}");
        }

        AdminCommand::Appeals => {
            let appeals: Vec<serde_json::Value> = client()
                .get(format!("{base}/admin/appeals"))
                .header("x-admin-secret", &secret)
                .send().await?
                .error_for_status().context("List appeals failed")?
                .json().await?;

            if appeals.is_empty() {
                println!("No pending appeals.");
            }
            for a in &appeals {
                let id = a["id"].as_str().unwrap_or("?");
                let did = a["did"].as_str().unwrap_or("?");
                let kind = a["kind"].as_str().unwrap_or("?");
                let reason = a["reason"].as_str().unwrap_or("-");
                let target = a["target_uri"].as_str().unwrap_or("-");
                let at = a["created_at"].as_str().unwrap_or("?");
                println!("[{id}] {kind}\t{did}\t{at}");
                if target != "-" {
                    println!("  target: {target}");
                }
                println!("  reason: {reason}");
                println!();
            }
        }

        AdminCommand::ResolveAppeal { id, status, response } => {
            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/appeals/resolve"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({
                    "id": id,
                    "status": status,
                    "response": response,
                }))
                .send().await?
                .error_for_status().context("Resolve appeal failed")?
                .json().await?;

            let kind = resp["kind"].as_str().unwrap_or("?");
            let did = resp["did"].as_str().unwrap_or("?");
            println!("Appeal {id} ({kind} by {did}): {status}");
        }

        AdminCommand::PublishQuestion { r#as: as_handle, file, title, desc, lang, tags } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;

            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            let (content_format, content) = match ext {
                "md" | "markdown" => ("markdown", content),
                "typ" | "typst" => ("typst", content),
                "html" | "htm" => ("html", content),
                _ => bail!("Unsupported file extension: .{ext} (use .md, .typ, or .html)"),
            };

            let title = title.unwrap_or_else(|| {
                file.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

            let body = serde_json::json!({
                "as_handle": as_handle,
                "title": title,
                "description": desc.unwrap_or_default(),
                "content": content,
                "content_format": content_format,
                "lang": lang,
                "tags": tags,
                "prereqs": [],
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/questions"))
                .header("x-admin-secret", &secret)
                .json(&body)
                .send().await?
                .error_for_status().context("Publish question failed")?
                .json().await?;

            let uri = resp["at_uri"].as_str().unwrap_or("?");
            println!("Published question as {as_handle}: {title}");
            println!("URI: {uri}");
        }

        AdminCommand::PublishAnswer { r#as: as_handle, question, file, title, desc, lang } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;

            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            let (content_format, content) = match ext {
                "md" | "markdown" => ("markdown", content),
                "typ" | "typst" => ("typst", content),
                "html" | "htm" => ("html", content),
                _ => bail!("Unsupported file extension: .{ext} (use .md, .typ, or .html)"),
            };

            let title = title.unwrap_or_else(|| {
                file.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Answer")
                    .to_string()
            });

            let body = serde_json::json!({
                "as_handle": as_handle,
                "question_uri": question,
                "title": title,
                "description": desc.unwrap_or_default(),
                "content": content,
                "content_format": content_format,
                "lang": lang,
                "tags": [],
                "prereqs": [],
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/questions/answer"))
                .header("x-admin-secret", &secret)
                .json(&body)
                .send().await?
                .error_for_status().context("Publish answer failed")?
                .json().await?;

            let uri = resp["at_uri"].as_str().unwrap_or("?");
            println!("Published answer as {as_handle}: {title}");
            println!("URI: {uri}");
        }

        AdminCommand::VerifyCredentials { did_or_handle, education, affiliation } => {
            let did = resolve_did_or_handle(&did_or_handle);
            let education_val: serde_json::Value = if let Some(ref e) = education {
                serde_json::from_str(e).context("Invalid JSON for --education")?
            } else {
                serde_json::json!([])
            };

            client()
                .post(format!("{base}/admin/credentials/verify"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({
                    "did": did,
                    "education": education_val,
                    "affiliation": affiliation,
                }))
                .send().await?
                .error_for_status().context("Verify credentials failed")?;

            println!("Verified credentials for {did_or_handle} ({did})");
        }

        AdminCommand::RevokeCredentials { did_or_handle } => {
            let did = resolve_did_or_handle(&did_or_handle);
            client()
                .post(format!("{base}/admin/credentials/revoke"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "did": did }))
                .send().await?
                .error_for_status().context("Revoke credentials failed")?;

            println!("Revoked credentials for {did_or_handle} ({did})");
        }

        AdminCommand::Publish { r#as: as_handle, file, title, desc, lang, tags, license, translation_of, category, book_id, series, resource } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;

            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            let (content_format, content) = match ext {
                "md" | "markdown" => ("markdown", content),
                "typ" | "typst" => ("typst", content),
                "html" | "htm" => ("html", content),
                _ => bail!("Unsupported file extension: .{ext} (use .md, .typ, or .html)"),
            };

            if content_format == "html" {
                validate_html_fragment(&content)?;
            }

            let title = title.unwrap_or_else(|| {
                file.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

            let body = serde_json::json!({
                "as_handle": as_handle,
                "title": title,
                "description": desc.unwrap_or_default(),
                "content": content,
                "content_format": content_format,
                "lang": lang,
                "license": license,
                "translation_of": translation_of,
                "category": category,
                "book_id": book_id,
                "series_id": series,
                "tags": tags,
                "prereqs": [],
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/articles"))
                .header("x-admin-secret", &secret)
                .json(&body)
                .send().await?
                .error_for_status().context("Publish failed")?
                .json().await?;

            let uri = resp["at_uri"].as_str().unwrap_or("?");
            println!("Published as {as_handle}: {title}");
            println!("URI: {uri}");

            if let Some(ref series_id) = series {
                client()
                    .post(format!("{base}/series/{series_id}/articles"))
                    .header("x-admin-secret", &secret)
                    .json(&serde_json::json!({ "article_uri": uri }))
                    .send().await?
                    .error_for_status().context("Failed to add article to series")?;
                println!("Added to series: {series_id}");
            }

            for res_path in &resource {
                let file_name = res_path.file_name()
                    .and_then(|n| n.to_str())
                    .context("Invalid resource filename")?;
                let file_bytes = std::fs::read(res_path)
                    .with_context(|| format!("Cannot read {}", res_path.display()))?;

                if let Some(ref series_id) = series {
                    let part = reqwest::multipart::Part::bytes(file_bytes)
                        .file_name(file_name.to_string());
                    let form = reqwest::multipart::Form::new().part("file", part);
                    client()
                        .post(format!("{base}/series/{series_id}/resource"))
                        .header("x-admin-secret", &secret)
                        .multipart(form)
                        .send().await?
                        .error_for_status()
                        .with_context(|| format!("Failed to upload resource: {file_name}"))?;
                    println!("Uploaded to series: {file_name}");
                } else {
                    let part = reqwest::multipart::Part::bytes(file_bytes)
                        .file_name(file_name.to_string());
                    let form = reqwest::multipart::Form::new()
                        .text("article_uri", uri.to_string())
                        .part("file", part);
                    client()
                        .post(format!("{base}/admin/articles/upload-image"))
                        .header("x-admin-secret", &secret)
                        .multipart(form)
                        .send().await?
                        .error_for_status()
                        .with_context(|| format!("Failed to upload resource: {file_name}"))?;
                    println!("Uploaded to article: {file_name}");
                }
            }
        }

        AdminCommand::BookHistory { book_id } => {
            let resp: Vec<serde_json::Value> = client()
                .get(format!("{base}/books/history"))
                .query(&[("id", &book_id)])
                .header("x-admin-secret", &secret)
                .send().await?
                .error_for_status().context("Failed to get book history")?
                .json().await?;

            if resp.is_empty() {
                println!("No edit history for {book_id}");
            } else {
                for entry in &resp {
                    let id = entry["id"].as_str().unwrap_or("?");
                    let editor = entry["editor_handle"].as_str()
                        .unwrap_or(entry["editor_did"].as_str().unwrap_or("?"));
                    let summary = entry["summary"].as_str().unwrap_or("");
                    let time = entry["created_at"].as_str().unwrap_or("?");
                    println!("[{id}] {editor}: {summary} ({time})");
                }
            }
        }

        AdminCommand::RevertBookEdit { edit_id } => {
            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/books/revert-edit"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "edit_id": edit_id }))
                .send().await?
                .error_for_status().context("Revert failed")?
                .json().await?;

            let book_id = resp["book_id"].as_str().unwrap_or("?");
            println!("Reverted edit {edit_id} on book {book_id}");
        }

        AdminCommand::ImportDir { r#as: as_handle, series, dir, lang, license, tags, dry_run } => {
            let mut chapters: Vec<(String, PathBuf, String)> = Vec::new();
            let mut resources: Vec<(String, PathBuf)> = Vec::new();

            let root = dir.canonicalize().context("Cannot resolve directory")?;
            let mut entries: Vec<_> = std::fs::read_dir(&root)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();
            entries.sort_by_key(|e| e.file_name());

            for entry in &entries {
                let sub_dir = entry.path();
                let dir_name = entry.file_name().to_string_lossy().to_string();

                let content_file = ["index.md", "index.typ", "index.html"]
                    .iter()
                    .map(|f| sub_dir.join(f))
                    .find(|p| p.exists());

                let content_file = if let Some(f) = content_file {
                    f
                } else {
                    let mut files: Vec<_> = std::fs::read_dir(&sub_dir)?
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            let ext = e.path().extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
                            matches!(ext.as_str(), "md" | "typ" | "html")
                        })
                        .collect();
                    if files.len() == 1 {
                        files.remove(0).path()
                    } else {
                        println!("  Skipping {dir_name}: no content file found");
                        continue;
                    }
                };

                let ext = content_file.extension().and_then(|e| e.to_str()).unwrap_or("md");
                let format = match ext {
                    "typ" => "typst",
                    "html" => "html",
                    _ => "markdown",
                };

                chapters.push((dir_name.clone(), content_file, format.to_string()));

                for res in walkdir::WalkDir::new(&sub_dir).into_iter().filter_map(|e| e.ok()) {
                    if !res.file_type().is_file() { continue; }
                    let res_path = res.path();
                    let res_ext = res_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if matches!(res_ext, "md" | "typ" | "html") { continue; }
                    let rel = res_path.strip_prefix(&root).unwrap_or(res_path);
                    resources.push((rel.to_string_lossy().to_string(), res_path.to_path_buf()));
                }
            }

            println!("Found {} chapters, {} resource files in {}", chapters.len(), resources.len(), root.display());
            for (i, (name, _file, fmt)) in chapters.iter().enumerate() {
                println!("  Chapter {}: {} ({})", i + 1, name, fmt);
            }
            if !resources.is_empty() {
                println!("  Resources: {} files", resources.len());
            }

            if dry_run {
                println!("\n[dry run] No changes made.");
                return Ok(());
            }

            let mut published: Vec<(String, String)> = Vec::new();
            for (i, (dir_name, content_file, format)) in chapters.iter().enumerate() {
                let content = std::fs::read_to_string(content_file)
                    .with_context(|| format!("Cannot read {}", content_file.display()))?;

                let title = dir_name
                    .trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == '_')
                    .replace('-', " ")
                    .replace('_', " ");
                let title = if title.is_empty() { dir_name.clone() } else {
                    let mut chars = title.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                        None => title,
                    }
                };

                let body = serde_json::json!({
                    "as_handle": as_handle,
                    "title": title,
                    "description": "",
                    "content": content,
                    "content_format": format,
                    "lang": lang,
                    "license": license,
                    "category": "lecture",
                    "series_id": series,
                    "tags": tags,
                    "prereqs": [],
                });

                let resp: serde_json::Value = client()
                    .post(format!("{base}/admin/articles"))
                    .header("x-admin-secret", &secret)
                    .json(&body)
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to publish: {dir_name}"))?
                    .json().await?;

                let uri = resp["at_uri"].as_str().unwrap_or("?").to_string();
                println!("  [{}/{}] {} -> {}", i + 1, chapters.len(), dir_name, uri);

                client()
                    .post(format!("{base}/series/{series}/articles"))
                    .header("x-admin-secret", &secret)
                    .json(&serde_json::json!({ "article_uri": uri }))
                    .send().await?
                    .error_for_status()
                    .context("Failed to add to series")?;

                published.push((dir_name.clone(), uri));
            }

            let mut uploaded = 0;
            for (rel_path, abs_path) in &resources {
                let file_bytes = std::fs::read(abs_path)
                    .with_context(|| format!("Cannot read {}", abs_path.display()))?;
                let file_name = abs_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file");

                let part = reqwest::multipart::Part::bytes(file_bytes)
                    .file_name(file_name.to_string());
                let form = reqwest::multipart::Form::new()
                    .text("path", rel_path.clone())
                    .part("file", part);

                client()
                    .post(format!("{base}/series/{series}/resource"))
                    .header("x-admin-secret", &secret)
                    .multipart(form)
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to upload: {rel_path}"))?;
                uploaded += 1;
            }
            if uploaded > 0 {
                println!("Uploaded {} resource files", uploaded);
            }

            println!("\nImported {} chapters into series {}", published.len(), series);
        }

        AdminCommand::ImportRepo { r#as: as_handle, series, dir, manifest, lang, license, tags, image_dirs, dry_run } => {
            use base64::Engine;
            use std::collections::HashSet;

            let root = dir.canonicalize().context("Cannot resolve directory")?;
            let manifest_content = std::fs::read_to_string(&manifest)
                .with_context(|| format!("Cannot read manifest {}", manifest.display()))?;
            let manifest_data: toml::Value = manifest_content.parse().context("Invalid TOML manifest")?;

            let article_entries = manifest_data.get("article")
                .and_then(|v| v.as_array())
                .context("Expected [[article]] array in manifest")?;

            let mut articles = Vec::new();
            let mut referenced_images: HashSet<String> = HashSet::new();

            for entry in article_entries {
                let path = entry.get("path").and_then(|v| v.as_str()).context("article missing 'path'")?;
                let explicit_title = entry.get("title").and_then(|v| v.as_str());

                let full_path = root.join(path);
                let content = std::fs::read_to_string(&full_path)
                    .with_context(|| format!("Cannot read {}", full_path.display()))?;

                let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or("md");
                let format = match ext {
                    "typ" | "typst" => "typst",
                    "html" | "htm" => "html",
                    _ => "markdown",
                };

                let file_dir = std::path::Path::new(path).parent().unwrap_or(std::path::Path::new(""));
                for cap in regex_lite::Regex::new(r#"src="([^"]+\.(png|jpg|gif|svg))""#).unwrap().captures_iter(&content) {
                    let src = cap.get(1).unwrap().as_str();
                    if !src.starts_with("http") {
                        let src = src.strip_prefix("./").unwrap_or(src);
                        referenced_images.insert(file_dir.join(src).to_string_lossy().to_string());
                    }
                }
                for cap in regex_lite::Regex::new(r"!\[[^\]]*\]\(([^)]+\.(png|jpg|gif|svg))\)").unwrap().captures_iter(&content) {
                    let src = cap.get(1).unwrap().as_str();
                    if !src.starts_with("http") {
                        let src = src.strip_prefix("./").unwrap_or(src);
                        referenced_images.insert(file_dir.join(src).to_string_lossy().to_string());
                    }
                }

                let explicit_tags = entry.get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>());

                let mut body = serde_json::json!({
                    "content": content,
                    "content_format": format,
                    "path": path,
                    "license": entry.get("license").and_then(|v| v.as_str()).unwrap_or(&license),
                });
                if let Some(t) = explicit_title {
                    body["title"] = serde_json::Value::String(t.to_string());
                }
                match explicit_tags {
                    Some(v) => { body["tags"] = serde_json::json!(v); }
                    None if !tags.is_empty() => { body["tags"] = serde_json::json!(tags); }
                    None => {}
                }
                articles.push(body);
            }

            let mut all_image_paths: HashSet<String> = referenced_images;

            for img_dir in &image_dirs {
                let abs_dir = root.join(img_dir);
                if abs_dir.is_dir() {
                    for entry in walkdir::WalkDir::new(&abs_dir).into_iter().filter_map(|e| e.ok()) {
                        if !entry.file_type().is_file() { continue; }
                        let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
                        if matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp") {
                            let rel = entry.path().strip_prefix(&root).unwrap_or(entry.path());
                            all_image_paths.insert(rel.to_string_lossy().to_string());
                        }
                    }
                }
            }

            let mut files = Vec::new();
            for img_path in &all_image_paths {
                let abs = root.join(img_path);
                if !abs.exists() { continue; }
                let data = std::fs::read(&abs)
                    .with_context(|| format!("Cannot read image {}", abs.display()))?;
                files.push(serde_json::json!({
                    "path": img_path,
                    "data": base64::engine::general_purpose::STANDARD.encode(&data),
                }));
            }

            println!("Articles: {}", articles.len());
            println!("Images: {}", files.len());

            if dry_run {
                for a in &articles {
                    println!("  {} -> {}", a["path"].as_str().unwrap_or("?"), a["title"].as_str().unwrap_or("?"));
                }
                println!("\n[dry run] No changes made.");
                return Ok(());
            }

            let body = serde_json::json!({
                "as_handle": as_handle,
                "series_id": series,
                "articles": articles,
                "files": files,
                "lang": lang,
            });

            let payload = serde_json::to_string(&body)?;
            let payload_mb = payload.len() as f64 / 1024.0 / 1024.0;
            println!("Payload: {payload_mb:.1} MB");

            let tmp = std::env::temp_dir().join("fx-import-repo.json");
            std::fs::write(&tmp, &payload)?;

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/series/batch-publish"))
                .header("x-admin-secret", &secret)
                .header("content-type", "application/json")
                .body(std::fs::read(&tmp)?)
                .send().await?
                .error_for_status().context("Batch publish failed")?
                .json().await?;

            let count = resp.as_array().map(|a| a.len()).unwrap_or(0);
            println!("\nPublished {count} articles into series {series}");
        }
    }

    Ok(())
}
