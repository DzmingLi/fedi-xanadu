use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{Config, client};

#[derive(Subcommand)]
pub enum TreeCommand {
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
    /// Add a prerequisite relationship between two tags
    #[command(name = "add-prereq")]
    AddPrereq {
        /// Source tag (must be mastered first)
        from: String,
        /// Target tag (requires the source)
        to: String,
        /// Prereq type: required or recommended
        #[arg(long, default_value = "required")]
        prereq_type: String,
    },
    /// Remove a prerequisite relationship
    #[command(name = "rm-prereq")]
    RmPrereq {
        /// Source tag
        from: String,
        /// Target tag
        to: String,
    },
    /// List all prerequisite relationships
    #[command(name = "list-prereqs", alias = "prereqs")]
    ListPrereqs,
}

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

pub async fn handle_tree(base: &str, config: &Config, action: TreeCommand) -> Result<()> {
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
        TreeCommand::AddPrereq { from, to, prereq_type } => {
            let token = config.token()?;
            client()
                .post(format!("{base}/tag-prereqs"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "from_tag": from, "to_tag": to, "prereq_type": prereq_type }))
                .send().await?
                .error_for_status().context("Add prereq failed")?;
            println!("Added prereq: {from} -> {to} ({prereq_type})");
        }
        TreeCommand::RmPrereq { from, to } => {
            let token = config.token()?;
            client()
                .delete(format!("{base}/tag-prereqs"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "from_tag": from, "to_tag": to }))
                .send().await?
                .error_for_status().context("Remove prereq failed")?;
            println!("Removed prereq: {from} -> {to}");
        }
        TreeCommand::ListPrereqs => {
            let token = config.token()?;
            let resp: Vec<serde_json::Value> = client()
                .get(format!("{base}/tag-prereqs"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("List prereqs failed")?
                .json().await?;
            if resp.is_empty() {
                println!("No prerequisite relationships defined.");
            } else {
                for e in &resp {
                    println!("{} -> {} ({})",
                        e["from_tag"].as_str().unwrap_or("?"),
                        e["to_tag"].as_str().unwrap_or("?"),
                        e["prereq_type"].as_str().unwrap_or("?"));
                }
                println!("\nTotal: {} prereqs", resp.len());
            }
        }
    }

    Ok(())
}
