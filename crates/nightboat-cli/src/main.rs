mod commands;

use std::path::PathBuf;
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use fx_core::content::ContentFormat;
use fx_core::models::CreateArticle;
use serde::{Deserialize, Serialize};

use commands::admin::{AdminCommand, handle_admin};
use commands::book::{BookCommand, handle_book};
use commands::book_series::{BookSeriesCommand, handle_book_series};
use commands::course::{CourseCommand, handle_course};
use commands::tree::{TreeCommand, handle_tree};

const CONFIG_DIR: &str = "nightboat";
const CONFIG_FILE: &str = "cli.json";

#[derive(Parser)]
#[command(name = "nbt", about = "NightBoat CLI — upload and manage articles")]
struct Cli {
    /// Server URL (default: http://localhost:3847)
    #[arg(long, env = "NBT_SERVER", default_value = "http://localhost:3847")]
    server: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Login via AT Protocol OAuth (opens browser), or with --password for platform-local users
    Login {
        /// Handle (e.g. user.bsky.social or dzming.li)
        handle: Option<String>,
        /// Platform-local password (skips OAuth, uses /auth/login)
        #[arg(long)]
        password: Option<String>,
    },
    /// Show current logged-in user
    Me,
    /// List recent articles
    #[command(alias = "ls")]
    List {
        /// Max number of articles to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// List available tags
    Tags,
    /// Upload a local file as a new article
    Upload {
        /// Path to .md, .typ, or .html file
        file: PathBuf,
        /// Article title (defaults to filename)
        #[arg(short, long)]
        title: Option<String>,
        /// Short description
        #[arg(short, long)]
        desc: Option<String>,
        /// Language code (default: zh)
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// Tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Prereqs (tag_id:type, e.g. calculus:required,linalg:recommended)
        #[arg(long, value_delimiter = ',')]
        prereqs: Vec<String>,
        /// License (default: CC-BY-SA-4.0)
        #[arg(long, default_value = "CC-BY-SA-4.0")]
        license: String,
        /// Category (e.g. general, lecture, paper, review, or custom)
        #[arg(long, default_value = "general")]
        category: String,
        /// Book ID (for reviews)
        #[arg(long)]
        book_id: Option<String>,
        /// Series ID — add this article to a series
        #[arg(long)]
        series: Option<String>,
        /// Resource files to upload to the series repo (e.g. references.bib)
        #[arg(long, value_delimiter = ',')]
        resource: Vec<PathBuf>,
        // -- Paper metadata (only for --category paper) --
        /// Venue (e.g. CVPR, NeurIPS, Nature)
        #[arg(long)]
        venue: Option<String>,
        /// Venue type (conference, journal, preprint, workshop, thesis)
        #[arg(long)]
        venue_type: Option<String>,
        /// Publication year
        #[arg(long)]
        year: Option<i16>,
        /// DOI
        #[arg(long)]
        doi: Option<String>,
        /// arXiv ID (e.g. 2406.12345)
        #[arg(long)]
        arxiv_id: Option<String>,
        /// Paper has been accepted
        #[arg(long)]
        accepted: bool,
        // -- Experience metadata (only for --category experience) --
        /// Experience kind (postgrad, interview, competition, application, other)
        #[arg(long)]
        exp_kind: Option<String>,
        /// Target school/company/competition
        #[arg(long)]
        target: Option<String>,
        /// Result (accepted, rejected, pending, passed, failed)
        #[arg(long)]
        result: Option<String>,
    },
    /// Update an existing article's content from a local file
    Update {
        /// Article AT URI
        uri: String,
        /// Path to .md, .typ, or .html file (updates content if provided)
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// New title
        #[arg(short, long)]
        title: Option<String>,
        /// New description
        #[arg(short, long)]
        desc: Option<String>,
    },
    /// Delete an article
    Delete {
        /// Article AT URI
        uri: String,
    },
    /// Get article content (source + rendered HTML)
    Get {
        /// Article AT URI
        uri: String,
        /// Output source to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Ask a question
    Question {
        /// Question title
        title: String,
        /// Language code (default: zh)
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// Tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Handles to invite to answer (comma-separated, e.g. alice,bob)
        #[arg(long, value_delimiter = ',')]
        invite: Vec<String>,
    },
    /// Manage skill trees
    Tree {
        #[command(subcommand)]
        action: TreeCommand,
    },
    /// Manage books (create, update, add editions)
    Book {
        #[command(subcommand)]
        action: BookCommand,
    },
    /// Manage book series (collections of related books)
    #[command(name = "book-series")]
    BookSeries {
        #[command(subcommand)]
        action: BookSeriesCommand,
    },
    /// Manage courses and sessions
    Course {
        #[command(subcommand)]
        action: CourseCommand,
    },
    /// Admin operations (manage platform users, publish as any user)
    Admin {
        #[command(subcommand)]
        action: AdminCommand,
    },
    /// Logout (remove saved token)
    Logout,
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    server: Option<String>,
    token: Option<String>,
    did: Option<String>,
    handle: Option<String>,
    admin_secret: Option<String>,
}

impl Config {
    fn path() -> Result<PathBuf> {
        let dir = dirs_next::config_dir()
            .context("Cannot determine config directory")?
            .join(CONFIG_DIR);
        std::fs::create_dir_all(&dir)?;
        Ok(dir.join(CONFIG_FILE))
    }

    fn load() -> Result<Self> {
        let path = Self::path()?;
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            Ok(Self::default())
        }
    }

    fn save(&self) -> Result<()> {
        let path = Self::path()?;
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn token(&self) -> Result<&str> {
        self.token.as_deref().context("Not logged in. Run: nbt login <handle>")
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

struct OAuthCallbackResult {
    token: String,
    did: String,
    handle: String,
}

/// Accept a single HTTP request on the local listener, extract token from query params,
/// respond with a success page, then return the result.
async fn accept_oauth_callback(listener: tokio::net::TcpListener) -> Result<OAuthCallbackResult> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut stream, _) = listener.accept().await?;
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse the GET request line to extract query params
    let path = request.lines().next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("");

    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");

    let params: std::collections::HashMap<&str, String> = query.split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            Some((k, urlencoding::decode(v).unwrap_or_default().into_owned()))
        })
        .collect();

    let token = params.get("token").cloned()
        .context("No token in callback")?;
    let did = params.get("did").cloned().unwrap_or_default();
    let handle = params.get("handle").cloned().unwrap_or_default();

    // Send a simple success response
    let body = "<!DOCTYPE html><html><body><h2>Login successful!</h2><p>You can close this tab and return to the terminal.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;

    Ok(OAuthCallbackResult { token, did, handle })
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;

    // Use server from CLI flag, falling back to saved config
    let server = if cli.server != "http://localhost:3847" {
        cli.server.clone()
    } else {
        config.server.clone().unwrap_or(cli.server.clone())
    };
    let base = format!("{}/api", server.trim_end_matches('/'));

    match cli.command {
        Command::Login { handle, password } => {
            if let Some(password) = password {
                // Password-based login (platform-local users)
                let handle = handle.context("Handle is required with --password")?;

                let resp: serde_json::Value = client()
                    .post(format!("{base}/auth/login"))
                    .json(&serde_json::json!({ "identifier": handle, "password": password }))
                    .send().await?
                    .error_for_status().context("Login failed")?
                    .json().await?;

                config.server = Some(server);
                config.token = resp["token"].as_str().map(String::from);
                config.did = resp["did"].as_str().map(String::from);
                config.handle = resp["handle"].as_str().map(String::from);
                config.save()?;

                let display = resp["handle"].as_str().unwrap_or("?");
                let did = resp["did"].as_str().unwrap_or("?");
                println!("Logged in as {display} ({did})");
            } else {
                // OAuth login (default): open browser, listen for callback
                let handle = handle.context("Handle is required. Usage: nbt login <handle>")?;
                let server_base = server.trim_end_matches('/');

                // Bind to a random port
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await
                    .context("Failed to bind local server")?;
                let port = listener.local_addr()?.port();
                let callback_url = format!("http://localhost:{port}/callback");

                let login_url = format!(
                    "{server_base}/oauth/login?handle={}&cli_redirect={}",
                    urlencoding::encode(&handle),
                    urlencoding::encode(&callback_url),
                );

                println!("Opening browser for AT Protocol authorization...");
                if open::that(&login_url).is_err() {
                    println!("Open this URL in your browser:\n  {login_url}");
                }

                // Wait for the callback (with timeout)
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(120),
                    accept_oauth_callback(listener),
                ).await
                    .context("Login timed out (2 minutes). Try again.")?
                    .context("OAuth callback failed")?;

                config.server = Some(server);
                config.token = Some(result.token);
                config.did = Some(result.did.clone());
                config.handle = Some(result.handle.clone());
                config.save()?;

                println!("Logged in as {} ({})", result.handle, result.did);
            }
        }

        Command::Me => {
            let token = config.token()?;
            let resp: serde_json::Value = client()
                .get(format!("{base}/auth/me"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("Not authenticated")?
                .json().await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }

        Command::List { limit } => {
            let articles: Vec<serde_json::Value> = client()
                .get(format!("{base}/articles"))
                .send().await?
                .error_for_status()?
                .json().await?;

            for a in articles.iter().take(limit) {
                let uri = a["at_uri"].as_str().unwrap_or("");
                let title = a["title"].as_str().unwrap_or("(untitled)");
                let author = a["author_handle"].as_str().unwrap_or("?");
                let format = a["content_format"].as_str().unwrap_or("?");
                let votes = a["vote_score"].as_i64().unwrap_or(0);
                println!("{title}  [{format}] by {author}  votes:{votes}");
                println!("  {uri}");
            }
            if articles.is_empty() {
                println!("No articles found.");
            }
        }

        Command::Tags => {
            let tags: Vec<serde_json::Value> = client()
                .get(format!("{base}/tags"))
                .send().await?
                .error_for_status()?
                .json().await?;

            for tag in &tags {
                let id = tag["id"].as_str().unwrap_or("");
                let name = tag["name"].as_str().unwrap_or("");
                println!("{id}\t{name}");
            }
            if tags.is_empty() {
                println!("No tags found.");
            }
        }

        Command::Question { title, lang, tags, invite } => {
            let token = config.token()?;
            let body = CreateArticle {
                title: title.clone(),
                summary: None,
                content: String::new(),
                content_format: fx_core::content::ContentFormat::Markdown,
                lang: Some(lang),
                license: Some("CC-BY-SA-4.0".to_string()),
                translation_of: None,
                restricted: None,
                category: None,
                tags,
                prereqs: vec![],
                related: vec![],
                topics: vec![],
                series_id: None,
                metadata: None,
                authors: vec![],
                invites: invite,
                book_chapter_id: None,
                course_session_id: None,
            };
            let article: serde_json::Value = client()
                .post(format!("{base}/questions"))
                .bearer_auth(&token)
                .json(&body)
                .send().await?
                .error_for_status().context("Failed to post question")?
                .json().await?;
            let uri = article["at_uri"].as_str().unwrap_or("?");
            println!("Asked: {title}");
            println!("URI:   {uri}");
            if !body.invites.is_empty() {
                println!("Invited: {}", body.invites.join(", "));
            }
        }

        Command::Upload { file, title, desc, lang, tags, prereqs, license, category, book_id, series, resource,
                          venue, venue_type, year, doi, arxiv_id, accepted,
                          exp_kind, target, result } => {
            let token = config.token()?;

            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;

            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            let (content_format, content) = match ext {
                "md" | "markdown" => (ContentFormat::Markdown, content),
                "typ" | "typst" => (ContentFormat::Typst, content),
                "html" | "htm" => (ContentFormat::Html, content),
                _ => bail!("Unsupported file extension: .{ext} (use .md, .typ, or .html)"),
            };

            if content_format == ContentFormat::Html {
                validate_html_fragment(&content)?;
            }

            let title = title.unwrap_or_else(|| {
                file.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

            let parsed_prereqs = parse_prereqs(&prereqs)?;

            let cat_metadata = if venue.is_some() || doi.is_some() || arxiv_id.is_some() || (year.is_some() && category == "paper") || accepted {
                Some(fx_core::models::CategoryMetadata::Paper(fx_core::models::CreatePaperMetadata {
                    venue, venue_type, year, doi, arxiv_id, accepted,
                }))
            } else if exp_kind.is_some() || target.is_some() || result.is_some() {
                Some(fx_core::models::CategoryMetadata::Experience(fx_core::models::CreateExperienceMetadata {
                    kind: exp_kind, target, year, result,
                }))
            } else if book_id.is_some() {
                Some(fx_core::models::CategoryMetadata::Review {
                    book_id, edition_id: None, course_id: None,
                })
            } else {
                None
            };

            let body = CreateArticle {
                title: title.clone(),
                summary: desc,
                content,
                content_format,
                lang: Some(lang),
                license: Some(license),
                translation_of: None,
                restricted: None,
                category: Some(category),
                tags,
                prereqs: parsed_prereqs,
                related: vec![],
                topics: vec![],
                series_id: series.clone(),
                metadata: cat_metadata,
                authors: vec![],
                invites: vec![],
                book_chapter_id: None,
                course_session_id: None,
            };

            // Collect resource files (expanding directories recursively)
            let mut resource_files: Vec<(PathBuf, String)> = Vec::new();
            for res_path in &resource {
                if res_path.is_dir() {
                    for entry in walkdir::WalkDir::new(res_path).into_iter().filter_map(|e| e.ok()) {
                        if entry.file_type().is_file() {
                            let path = entry.path().to_path_buf();
                            let rel = path.strip_prefix(res_path.parent().unwrap_or(res_path))
                                .unwrap_or(&path)
                                .to_string_lossy().into_owned();
                            resource_files.push((path, rel));
                        }
                    }
                } else {
                    let name = res_path.file_name()
                        .and_then(|n| n.to_str())
                        .context("Invalid resource filename")?
                        .to_string();
                    resource_files.push((res_path.clone(), name));
                }
            }

            let resp: serde_json::Value = if resource_files.is_empty() {
                // Simple JSON upload (no resources)
                client()
                    .post(format!("{base}/articles"))
                    .bearer_auth(token)
                    .json(&body)
                    .send().await?
                    .error_for_status().context("Upload failed")?
                    .json().await?
            } else {
                // Multipart upload: metadata + resources in one request
                let metadata = serde_json::to_string(&body)?;
                let mut form = reqwest::multipart::Form::new()
                    .text("metadata", metadata);

                for (abs_path, rel_name) in &resource_files {
                    let file_bytes = std::fs::read(abs_path)
                        .with_context(|| format!("Cannot read {}", abs_path.display()))?;
                    let part = reqwest::multipart::Part::bytes(file_bytes)
                        .file_name(rel_name.clone());
                    form = form.part("resources", part);
                    println!("  + {rel_name}");
                }

                let resp = client()
                    .post(format!("{base}/articles/upload"))
                    .bearer_auth(token)
                    .multipart(form)
                    .send().await?;
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    bail!("Upload failed ({status}): {body}");
                }
                resp
                    .json().await?
            };

            let uri = resp["at_uri"].as_str().unwrap_or("?");
            println!("Published: {title}");
            println!("URI: {uri}");

            // Add to series if specified
            if let Some(ref series_id) = series {
                client()
                    .post(format!("{base}/series/{series_id}/articles"))
                    .bearer_auth(token)
                    .json(&serde_json::json!({ "article_uri": uri }))
                    .send().await?
                    .error_for_status().context("Failed to add article to series")?;
                println!("Added to series: {series_id}");
            }
        }

        Command::Update { uri, file, title, desc } => {
            let token = config.token()?;

            let content = if let Some(ref path) = file {
                Some(std::fs::read_to_string(path)
                    .with_context(|| format!("Cannot read {}", path.display()))?)
            } else {
                None
            };

            let body = serde_json::json!({
                "uri": uri,
                "title": title,
                "description": desc,
                "content": content,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/articles/update"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Update failed")?
                .json().await?;

            let title = resp["title"].as_str().unwrap_or("?");
            println!("Updated: {title}");
        }

        Command::Delete { uri } => {
            let token = config.token()?;

            client()
                .post(format!("{base}/articles/delete"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "uri": uri }))
                .send().await?
                .error_for_status().context("Delete failed")?;

            println!("Deleted: {uri}");
        }

        Command::Get { uri, output } => {
            let article: serde_json::Value = client()
                .get(format!("{base}/articles/by-uri"))
                .query(&[("uri", &uri)])
                .send().await?
                .error_for_status().context("Article not found")?
                .json().await?;

            let content: serde_json::Value = client()
                .get(format!("{base}/articles/by-uri/content"))
                .query(&[("uri", &uri)])
                .send().await?
                .error_for_status()?
                .json().await?;

            let title = article["title"].as_str().unwrap_or("?");
            let source = content["source"].as_str().unwrap_or("");

            if let Some(path) = output {
                std::fs::write(&path, source)
                    .with_context(|| format!("Cannot write {}", path.display()))?;
                println!("{title} -> {}", path.display());
            } else {
                println!("# {title}\n");
                println!("{source}");
            }
        }

        Command::Tree { action } => {
            handle_tree(&base, &config, action).await?;
        }

        Command::Course { action } => {
            handle_course(&base, &config, action).await?;
        }

        Command::Book { action } => {
            handle_book(&base, &config, action).await?;
        }

        Command::BookSeries { action } => {
            handle_book_series(&base, &config, action).await?;
        }

        Command::Admin { action } => {
            handle_admin(&base, &mut config, action).await?;
        }

        Command::Logout => {
            if let Ok(token) = config.token() {
                let _ = client()
                    .post(format!("{base}/auth/logout"))
                    .bearer_auth(token)
                    .send().await;
            }
            config.token = None;
            config.did = None;
            config.handle = None;
            config.save()?;
            println!("Logged out.");
        }
    }

    Ok(())
}

/// Validate that an HTML file is a content fragment, not a full page.
/// Rejects files containing <html>, <head>, <body>, or <script> tags.
fn parse_prereqs(raw: &[String]) -> Result<Vec<fx_core::models::ArticlePrereq>> {
    use fx_core::content::PrereqType;
    raw.iter().map(|s| {
        let (tag_id, prereq_type) = if let Some((t, p)) = s.split_once(':') {
            let pt = match p {
                "required" | "r" => PrereqType::Required,
                "recommended" | "rec" => PrereqType::Recommended,
                _ => bail!("Invalid prereq type '{p}' (use required/recommended)"),
            };
            (t.to_string(), pt)
        } else {
            (s.clone(), PrereqType::Required)
        };
        Ok(fx_core::models::ArticlePrereq { tag_id, prereq_type })
    }).collect()
}

fn validate_html_fragment(content: &str) -> Result<()> {
    let lower = content.to_ascii_lowercase();
    let forbidden = [
        ("<!doctype", "<!DOCTYPE> declaration"),
        ("<html", "<html> tag"),
        ("<head", "<head> tag"),
        ("<body", "<body> tag"),
        ("<script", "<script> tag"),
    ];
    for (tag, label) in &forbidden {
        if lower.contains(tag) {
            bail!(
                "HTML file contains {label}.\n\
                 HTML articles should be content fragments (e.g. <h2>, <p>, <div>),\n\
                 not full HTML pages. Remove the page wrapper and try again.\n\
                 See: https://nightboat.dzming.li/#/guide for details."
            );
        }
    }
    Ok(())
}

/// Resolve a DID or handle to a DID. If already a DID (starts with "did:"), pass through.
/// Otherwise treat as a platform user handle and generate did:local:<handle>.
fn resolve_did_or_handle(input: &str) -> String {
    if input.starts_with("did:") {
        input.to_string()
    } else {
        format!("did:local:{input}")
    }
}

