use std::path::PathBuf;
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use fx_core::content::ContentFormat;
use fx_core::models::CreateArticle;
use serde::{Deserialize, Serialize};

const CONFIG_DIR: &str = "fedi-xanadu";
const CONFIG_FILE: &str = "cli.json";

#[derive(Parser)]
#[command(name = "fx", about = "Fedi-Xanadu CLI — upload and manage articles")]
struct Cli {
    /// Server URL (default: http://localhost:3847)
    #[arg(long, env = "FX_SERVER", default_value = "http://localhost:3847")]
    server: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Login with your AT Protocol handle and password
    Login {
        /// Handle (e.g. user.bsky.social)
        handle: String,
        /// App password
        password: String,
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
    /// Admin operations (manage platform users, publish as any user)
    Admin {
        #[command(subcommand)]
        action: AdminCommand,
    },
    /// Logout (remove saved token)
    Logout,
}

#[derive(Subcommand)]
enum AdminCommand {
    /// Create a platform user
    #[command(name = "create-user")]
    CreateUser {
        /// User handle
        handle: String,
        /// Password
        password: String,
        /// Display name
        #[arg(long)]
        display_name: Option<String>,
    },
    /// List all platform users
    #[command(name = "list-users", alias = "users")]
    ListUsers,
    /// Set a localized name for a tag
    #[command(name = "set-tag-name")]
    SetTagName {
        /// Tag ID
        id: String,
        /// Locale code (e.g. zh, en, fr)
        locale: String,
        /// Localized name
        name: String,
    },
    /// Merge one tag into another (migrate all references)
    #[command(name = "merge-tag")]
    MergeTag {
        /// Source tag ID (will be deleted)
        #[arg(long)]
        from: String,
        /// Target tag ID (will absorb references)
        #[arg(long)]
        into: String,
    },
    /// Create a series as a platform user
    #[command(name = "create-series")]
    CreateSeries {
        /// Platform user handle
        #[arg(long)]
        r#as: String,
        /// Series title
        #[arg(short, long)]
        title: String,
        /// Short description
        #[arg(short, long)]
        desc: Option<String>,
        /// Topic tags (comma-separated, e.g. cs,math)
        #[arg(long)]
        topics: Option<String>,
        /// Parent series ID (for sub-series)
        #[arg(long)]
        parent: Option<String>,
        /// Language code (default: zh)
        #[arg(short, long)]
        lang: Option<String>,
        /// Source series ID this is a translation of
        #[arg(long)]
        translation_of: Option<String>,
    },
    /// Add an article to a series
    #[command(name = "add-to-series")]
    AddToSeries {
        /// Series ID
        #[arg(long)]
        series: String,
        /// Article AT URI
        #[arg(long)]
        article: String,
    },
    /// Update an article's content (admin override, no auth needed)
    Update {
        /// Article AT URI
        #[arg(long)]
        uri: String,
        /// Path to .md, .typ, or .html file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// New title
        #[arg(short, long)]
        title: Option<String>,
        /// New description
        #[arg(short, long)]
        desc: Option<String>,
    },
    /// Ban a user (by DID or handle)
    #[command(name = "ban-user")]
    BanUser {
        /// DID or handle of the user to ban
        did_or_handle: String,
        /// Reason for the ban
        #[arg(long)]
        reason: Option<String>,
    },
    /// Unban a user (by DID or handle)
    #[command(name = "unban-user")]
    UnbanUser {
        /// DID or handle of the user to unban
        did_or_handle: String,
    },
    /// List all banned users
    #[command(name = "banned-users")]
    BannedUsers,
    /// Delete an article (admin override, soft-delete with 30-day appeal window)
    #[command(name = "delete-article")]
    DeleteArticle {
        /// Article AT URI
        uri: String,
        /// Reason for deletion
        #[arg(long)]
        reason: Option<String>,
    },
    /// Set article visibility (public, cn_hidden, unlisted, pending_review, removed)
    #[command(name = "set-visibility")]
    SetVisibility {
        /// Article AT URI
        uri: String,
        /// Visibility: public, cn_hidden, unlisted, pending_review, removed
        visibility: String,
        /// Reason (shown to author for cn_hidden/removed)
        #[arg(long)]
        reason: Option<String>,
    },
    /// List pending appeals
    #[command(name = "appeals")]
    Appeals,
    /// Resolve an appeal (approve or reject)
    #[command(name = "resolve-appeal")]
    ResolveAppeal {
        /// Appeal ID
        id: String,
        /// "approved" or "rejected"
        #[arg(long)]
        status: String,
        /// Admin response message
        #[arg(long)]
        response: Option<String>,
    },
    /// Merge two questions (move answers from one to another)
    #[command(name = "merge-questions")]
    MergeQuestions {
        /// Source question URI (will be deleted)
        #[arg(long)]
        from: String,
        /// Target question URI (will absorb answers)
        #[arg(long)]
        into: String,
    },
    /// Publish a question as a platform user
    #[command(name = "publish-question")]
    PublishQuestion {
        /// Platform user handle to publish as
        #[arg(long)]
        r#as: String,
        /// Path to .md, .typ, or .html file
        #[arg(short, long)]
        file: PathBuf,
        /// Question title
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
    },
    /// Post an answer to a question as a platform user
    #[command(name = "publish-answer")]
    PublishAnswer {
        /// Platform user handle to publish as
        #[arg(long)]
        r#as: String,
        /// Question AT URI to answer
        #[arg(long)]
        question: String,
        /// Path to .md, .typ, or .html file
        #[arg(short, long)]
        file: PathBuf,
        /// Answer title
        #[arg(short, long)]
        title: Option<String>,
        /// Short description
        #[arg(short, long)]
        desc: Option<String>,
        /// Language code (default: zh)
        #[arg(short, long, default_value = "zh")]
        lang: String,
    },
    /// Verify a user's credentials (education + affiliation)
    #[command(name = "verify-credentials")]
    VerifyCredentials {
        /// DID or handle
        did_or_handle: String,
        /// Education entries as JSON: [{"degree":"PhD","school":"MIT","year":"2024","current":false}]
        #[arg(long)]
        education: Option<String>,
        /// Current affiliation
        #[arg(long)]
        affiliation: Option<String>,
    },
    /// Revoke a user's credentials verification
    #[command(name = "revoke-credentials")]
    RevokeCredentials {
        /// DID or handle
        did_or_handle: String,
    },
    /// Revert a book edit by edit log ID
    #[command(name = "revert-book-edit")]
    RevertBookEdit {
        /// Edit log ID to revert
        edit_id: String,
    },
    /// Show edit history for a book
    #[command(name = "book-history")]
    BookHistory {
        /// Book ID
        book_id: String,
    },
    /// Publish an article as a platform user
    Publish {
        /// Platform user handle to publish as
        #[arg(long)]
        r#as: String,
        /// Path to .md, .typ, or .html file
        #[arg(short, long)]
        file: PathBuf,
        /// Article title (defaults to filename)
        #[arg(short, long)]
        title: Option<String>,
        /// Short description
        #[arg(short, long)]
        desc: Option<String>,
        /// Category (e.g. general, lecture, paper, review, or custom)
        #[arg(long, default_value = "general")]
        category: String,
        /// Book ID (for reviews)
        #[arg(long)]
        book_id: Option<String>,
        /// Language code (default: zh)
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// Tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// License (default: CC-BY-SA-3.0)
        #[arg(long, default_value = "CC-BY-SA-3.0")]
        license: String,
        /// AT URI of the article this is a translation of
        #[arg(long)]
        translation_of: Option<String>,
        /// Series ID — add this article to a series
        #[arg(long)]
        series: Option<String>,
        /// Resource files to upload to the series repo (e.g. references.bib)
        #[arg(long, value_delimiter = ',')]
        resource: Vec<PathBuf>,
    },
}

#[derive(Subcommand)]
enum BookCommand {
    /// List all books
    #[command(alias = "ls")]
    List,
    /// Create a new book
    Create {
        /// Book title
        #[arg(short, long)]
        title: String,
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
        /// Edition title
        #[arg(short, long)]
        title: String,
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
    },
    /// Show a book's detail
    Show {
        /// Book ID
        id: String,
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
}

#[derive(Subcommand)]
enum TreeCommand {
    /// List all community skill trees
    #[command(alias = "ls")]
    List,
    /// Show a skill tree's edges
    Show {
        /// Skill tree AT URI
        uri: String,
    },
    /// Create a skill tree from a TOML file
    Create {
        /// Path to a TOML file defining the tree
        file: PathBuf,
    },
    /// Export a skill tree to a TOML file for editing
    Export {
        /// Skill tree AT URI
        uri: String,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import/sync edges from a TOML file into an existing tree
    Import {
        /// Skill tree AT URI
        uri: String,
        /// Path to TOML file with edges
        file: PathBuf,
    },
    /// Add a single edge
    #[command(name = "add-edge")]
    AddEdge {
        /// Skill tree AT URI
        uri: String,
        /// Parent tag ID
        parent: String,
        /// Child tag ID
        child: String,
    },
    /// Remove a single edge
    #[command(name = "rm-edge")]
    RmEdge {
        /// Skill tree AT URI
        uri: String,
        /// Parent tag ID
        parent: String,
        /// Child tag ID
        child: String,
    },
    /// Fork a skill tree
    Fork {
        /// Source skill tree AT URI
        uri: String,
    },
    /// Adopt a skill tree as your active tree
    Adopt {
        /// Skill tree AT URI
        uri: String,
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
    /// Locally unique key for referencing as a parent
    #[serde(default)]
    key: String,
    /// Title shown in the table of contents
    title: String,
    /// Path to a file to upload as an article (relative to the manifest file)
    #[serde(default)]
    file: Option<PathBuf>,
    /// Key of the parent chapter (omit for top-level)
    #[serde(default)]
    parent: Option<String>,
    /// Display order among siblings (0-based)
    #[serde(default)]
    order: i32,
    /// Tag IDs this chapter teaches
    #[serde(default)]
    teaches: Vec<String>,
    /// Prereq tags
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

/// TOML format for skill tree files
#[derive(Serialize, Deserialize)]
struct TreeFile {
    title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    uri: Option<String>,
    edges: Vec<TreeEdge>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
struct TreeEdge {
    parent: String,
    child: String,
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
        self.token.as_deref().context("Not logged in. Run: fx login <handle> <password>")
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
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
                description: None,
                content: String::new(),
                content_format: fx_core::content::ContentFormat::Markdown,
                lang: Some(lang),
                license: Some("CC-BY-SA-4.0".to_string()),
                translation_of: None,
                restricted: None,
                category: None,
                book_id: None,
                edition_id: None,
                tags,
                prereqs: vec![],
                series_id: None,
                invites: invite,
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

        Command::Upload { file, title, desc, lang, tags, license, category, book_id, series, resource } => {
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

            let body = CreateArticle {
                title: title.clone(),
                description: Some(desc.unwrap_or_default()),
                content,
                content_format,
                lang: Some(lang),
                license: Some(license),
                translation_of: None,
                restricted: None,
                category: Some(category),
                book_id,
                edition_id: None,
                tags,
                prereqs: vec![],
                series_id: series.clone(),
                invites: vec![],
            };

            let resp: serde_json::Value = client()
                .post(format!("{base}/articles"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Upload failed")?
                .json().await?;

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

            // Upload resource files (images, bib, etc.)
            for res_path in &resource {
                let file_name = res_path.file_name()
                    .and_then(|n| n.to_str())
                    .context("Invalid resource filename")?;
                let file_bytes = std::fs::read(res_path)
                    .with_context(|| format!("Cannot read {}", res_path.display()))?;

                if let Some(ref series_id) = series {
                    // Upload to series repo
                    let part = reqwest::multipart::Part::bytes(file_bytes)
                        .file_name(file_name.to_string());
                    let form = reqwest::multipart::Form::new().part("file", part);
                    client()
                        .post(format!("{base}/series/{series_id}/resource"))
                        .bearer_auth(token)
                        .multipart(form)
                        .send().await?
                        .error_for_status()
                        .with_context(|| format!("Failed to upload resource: {file_name}"))?;
                    println!("Uploaded to series: {file_name}");
                } else {
                    // Upload to article repo (as image/resource)
                    let part = reqwest::multipart::Part::bytes(file_bytes)
                        .file_name(file_name.to_string());
                    let form = reqwest::multipart::Form::new()
                        .text("article_uri", uri.to_string())
                        .part("file", part);
                    client()
                        .post(format!("{base}/articles/upload-image"))
                        .bearer_auth(token)
                        .multipart(form)
                        .send().await?
                        .error_for_status()
                        .with_context(|| format!("Failed to upload resource: {file_name}"))?;
                    println!("Uploaded to article: {file_name}");
                }
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

        Command::Book { action } => {
            handle_book(&base, &config, action).await?;
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

async fn handle_book(base: &str, config: &Config, action: BookCommand) -> Result<()> {
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

        BookCommand::Create { title, authors, desc, cover_url, tags, prereqs } => {
            let body = serde_json::json!({
                "title": title,
                "authors": authors,
                "description": desc.unwrap_or_default(),
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

            let id = resp["id"].as_str().unwrap_or("?");
            println!("Created book: {title}");
            println!("ID: {id}");
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

        BookCommand::AddEdition { book_id, title, lang, isbn, publisher, year, translators, purchase_links, cover_url } => {
            let links: Vec<serde_json::Value> = if let Some(ref pl) = purchase_links {
                serde_json::from_str(pl).context("Invalid JSON for --purchase-links")?
            } else {
                vec![]
            };

            let body = serde_json::json!({
                "book_id": book_id,
                "title": title,
                "lang": lang,
                "isbn": isbn,
                "publisher": publisher,
                "year": year,
                "translators": translators,
                "purchase_links": links,
                "cover_url": cover_url,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/books/editions"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Add edition failed")?
                .json().await?;

            let eid = resp["id"].as_str().unwrap_or("?");
            println!("Added edition to book {book_id}: {title} ({lang})");
            println!("Edition ID: {eid}");
        }

        BookCommand::Show { id } => {
            let resp: serde_json::Value = client()
                .get(format!("{base}/books/by-id?id={id}"))
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

            // Show chapters
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
            let token = config.token()?;
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
            let token = config.token()?;
            let manifest_dir = manifest.parent().unwrap_or(std::path::Path::new("."));
            let manifest_text = std::fs::read_to_string(&manifest)
                .with_context(|| format!("Cannot read {}", manifest.display()))?;
            let cm: ChapterManifest = toml::from_str(&manifest_text)
                .context("Invalid chapters TOML")?;

            // key → created chapter ID
            let mut key_to_id: std::collections::HashMap<String, String> = std::collections::HashMap::new();

            for (i, ch) in cm.chapters.iter().enumerate() {
                let parent_id = ch.parent.as_ref().and_then(|k| key_to_id.get(k)).cloned();

                // Upload article if file is specified
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
    }
    Ok(())
}

async fn handle_tree(base: &str, config: &Config, action: TreeCommand) -> Result<()> {
    match action {
        TreeCommand::List => {
            let trees: Vec<serde_json::Value> = client()
                .get(format!("{base}/skill-trees"))
                .send().await?
                .error_for_status()?
                .json().await?;

            if trees.is_empty() {
                println!("No skill trees found.");
                return Ok(());
            }

            for t in &trees {
                let uri = t["at_uri"].as_str().unwrap_or("");
                let title = t["title"].as_str().unwrap_or("(untitled)");
                let field = t["field"].as_str().unwrap_or("-");
                let edges = t["edge_count"].as_i64().unwrap_or(0);
                let adopts = t["adopt_count"].as_i64().unwrap_or(0);
                let author = t["author_handle"].as_str()
                    .or_else(|| t["did"].as_str())
                    .unwrap_or("?");
                println!("{title}  [{field}]  {edges} edges  {adopts} adopts  by {author}");
                println!("  {uri}");
            }
        }

        TreeCommand::Show { uri } => {
            let detail: serde_json::Value = client()
                .get(format!("{base}/skill-trees/by-uri"))
                .query(&[("uri", &uri)])
                .send().await?
                .error_for_status().context("Skill tree not found")?
                .json().await?;

            let title = detail["tree"]["title"].as_str().unwrap_or("?");
            let field = detail["tree"]["field"].as_str().unwrap_or("-");
            let desc = detail["tree"]["description"].as_str().unwrap_or("");
            println!("{title}  [{field}]");
            if !desc.is_empty() {
                println!("{desc}");
            }
            println!();

            if let Some(edges) = detail["edges"].as_array() {
                for e in edges {
                    let p = e["parent_tag"].as_str().unwrap_or("?");
                    let c = e["child_tag"].as_str().unwrap_or("?");
                    println!("  {p} -> {c}");
                }
                println!("\n{} edges total", edges.len());
            }
        }

        TreeCommand::Create { file } => {
            let token = config.token()?;
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;
            let tree_file: TreeFile = toml::from_str(&content)
                .with_context(|| format!("Invalid TOML in {}", file.display()))?;

            let edges: Vec<serde_json::Value> = tree_file.edges.iter().map(|e| {
                serde_json::json!({ "parent_tag": e.parent, "child_tag": e.child })
            }).collect();

            let body = serde_json::json!({
                "title": tree_file.title,
                "description": tree_file.description,
                "field": tree_file.field,
                "edges": edges,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/skill-trees"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Create skill tree failed")?
                .json().await?;

            let uri = resp["at_uri"].as_str().unwrap_or("?");
            println!("Created: {}", tree_file.title);
            println!("URI: {uri}");
        }

        TreeCommand::Export { uri, output } => {
            let detail: serde_json::Value = client()
                .get(format!("{base}/skill-trees/by-uri"))
                .query(&[("uri", &uri)])
                .send().await?
                .error_for_status().context("Skill tree not found")?
                .json().await?;

            let tree_file = TreeFile {
                title: detail["tree"]["title"].as_str().unwrap_or("").to_string(),
                description: detail["tree"]["description"].as_str().map(String::from),
                field: detail["tree"]["field"].as_str().map(String::from),
                uri: Some(uri),
                edges: detail["edges"].as_array()
                    .map(|arr| arr.iter().map(|e| TreeEdge {
                        parent: e["parent_tag"].as_str().unwrap_or("").to_string(),
                        child: e["child_tag"].as_str().unwrap_or("").to_string(),
                    }).collect())
                    .unwrap_or_default(),
            };

            let toml_str = toml::to_string_pretty(&tree_file)?;

            if let Some(path) = output {
                std::fs::write(&path, &toml_str)
                    .with_context(|| format!("Cannot write {}", path.display()))?;
                println!("Exported to {}", path.display());
            } else {
                print!("{toml_str}");
            }
        }

        TreeCommand::Import { uri, file } => {
            let token = config.token()?;

            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;
            let tree_file: TreeFile = toml::from_str(&content)
                .with_context(|| format!("Invalid TOML in {}", file.display()))?;

            // Get current edges
            let detail: serde_json::Value = client()
                .get(format!("{base}/skill-trees/by-uri"))
                .query(&[("uri", &uri)])
                .send().await?
                .error_for_status().context("Skill tree not found")?
                .json().await?;

            let current: std::collections::HashSet<TreeEdge> = detail["edges"].as_array()
                .map(|arr| arr.iter().map(|e| TreeEdge {
                    parent: e["parent_tag"].as_str().unwrap_or("").to_string(),
                    child: e["child_tag"].as_str().unwrap_or("").to_string(),
                }).collect())
                .unwrap_or_default();

            let desired: std::collections::HashSet<TreeEdge> = tree_file.edges.into_iter().collect();

            // Compute diff
            let to_add: Vec<&TreeEdge> = desired.difference(&current).collect();
            let to_remove: Vec<&TreeEdge> = current.difference(&desired).collect();

            if to_add.is_empty() && to_remove.is_empty() {
                println!("Already up to date.");
                return Ok(());
            }

            // Apply removals
            for e in &to_remove {
                client()
                    .post(format!("{base}/skill-trees/edges/remove"))
                    .bearer_auth(token)
                    .json(&serde_json::json!({
                        "tree_uri": uri,
                        "parent_tag": e.parent,
                        "child_tag": e.child,
                    }))
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to remove edge: {} -> {}", e.parent, e.child))?;
            }

            // Apply additions
            for e in &to_add {
                client()
                    .post(format!("{base}/skill-trees/edges"))
                    .bearer_auth(token)
                    .json(&serde_json::json!({
                        "tree_uri": uri,
                        "parent_tag": e.parent,
                        "child_tag": e.child,
                    }))
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to add edge: {} -> {}", e.parent, e.child))?;
            }

            println!("Synced: +{} added, -{} removed", to_add.len(), to_remove.len());
        }

        TreeCommand::AddEdge { uri, parent, child } => {
            let token = config.token()?;
            client()
                .post(format!("{base}/skill-trees/edges"))
                .bearer_auth(token)
                .json(&serde_json::json!({
                    "tree_uri": uri,
                    "parent_tag": parent,
                    "child_tag": child,
                }))
                .send().await?
                .error_for_status().context("Add edge failed")?;

            println!("Added: {parent} -> {child}");
        }

        TreeCommand::RmEdge { uri, parent, child } => {
            let token = config.token()?;
            client()
                .post(format!("{base}/skill-trees/edges/remove"))
                .bearer_auth(token)
                .json(&serde_json::json!({
                    "tree_uri": uri,
                    "parent_tag": parent,
                    "child_tag": child,
                }))
                .send().await?
                .error_for_status().context("Remove edge failed")?;

            println!("Removed: {parent} -> {child}");
        }

        TreeCommand::Fork { uri } => {
            let token = config.token()?;
            let resp: serde_json::Value = client()
                .post(format!("{base}/skill-trees/fork"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "uri": uri }))
                .send().await?
                .error_for_status().context("Fork failed")?
                .json().await?;

            let new_uri = resp["at_uri"].as_str().unwrap_or("?");
            let title = resp["title"].as_str().unwrap_or("?");
            println!("Forked: {title}");
            println!("URI: {new_uri}");
        }

        TreeCommand::Adopt { uri } => {
            let token = config.token()?;
            client()
                .post(format!("{base}/skill-trees/adopt"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "tree_uri": uri }))
                .send().await?
                .error_for_status().context("Adopt failed")?;

            println!("Adopted skill tree as active.");
        }
    }

    Ok(())
}

/// Validate that an HTML file is a content fragment, not a full page.
/// Rejects files containing <html>, <head>, <body>, or <script> tags.
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
                 See: https://fedi-xanadu.dzming.li/#/guide for details."
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

async fn handle_admin(base: &str, config: &mut Config, action: AdminCommand) -> Result<()> {
    let secret = std::env::var("FX_ADMIN_SECRET")
        .ok()
        .or_else(|| config.admin_secret.clone())
        .context("Admin secret not set. Use FX_ADMIN_SECRET env var")?;

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
            // First get existing names
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

        AdminCommand::CreateSeries { r#as: as_handle, title, desc, topics, parent, lang, translation_of } => {
            let topics_vec: Vec<&str> = topics.as_deref().map(|t| t.split(',').collect()).unwrap_or_default();
            let body = serde_json::json!({
                "as_handle": as_handle,
                "title": title,
                "description": desc,
                "topics": topics_vec,
                "parent_id": parent,
                "lang": lang,
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
            println!("Created series: {title}");
            println!("ID: {id}");
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

            // Add to series if specified
            if let Some(ref series_id) = series {
                client()
                    .post(format!("{base}/series/{series_id}/articles"))
                    .header("x-admin-secret", &secret)
                    .json(&serde_json::json!({ "article_uri": uri }))
                    .send().await?
                    .error_for_status().context("Failed to add article to series")?;
                println!("Added to series: {series_id}");
            }

            // Upload resource files
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
    }

    Ok(())
}
