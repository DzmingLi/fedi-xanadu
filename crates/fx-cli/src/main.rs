use std::path::PathBuf;
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
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
        /// Path to .md or .typ file
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
        /// License (default: CC-BY-NC-SA-4.0)
        #[arg(long, default_value = "CC-BY-NC-SA-4.0")]
        license: String,
    },
    /// Update an existing article's content from a local file
    Update {
        /// Article AT URI
        uri: String,
        /// Path to .md or .typ file (updates content if provided)
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
    /// Logout (remove saved token)
    Logout,
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    server: Option<String>,
    token: Option<String>,
    did: Option<String>,
    handle: Option<String>,
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

        Command::Upload { file, title, desc, lang, tags, license } => {
            let token = config.token()?;

            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;

            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            let content_format = match ext {
                "md" | "markdown" => "markdown",
                "typ" | "typst" => "typst",
                _ => bail!("Unsupported file extension: .{ext} (use .md or .typ)"),
            };

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
                content_format: content_format.to_string(),
                lang: Some(lang),
                license: Some(license),
                translation_of: None,
                tags,
                prereqs: vec![],
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
