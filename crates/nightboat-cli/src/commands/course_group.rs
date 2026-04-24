use anyhow::{Context, Result};
use clap::Subcommand;

use crate::{Config, client};

#[derive(Subcommand)]
pub enum CourseGroupCommand {
    /// Create a new course group (an umbrella over several iterations)
    Create {
        /// Canonical title (e.g. "Machine Learning")
        #[arg(short, long)]
        title: String,
        /// Course code (e.g. "CS229", "ML4201")
        #[arg(long)]
        code: Option<String>,
        /// Institution (e.g. "Stanford", "University of Tübingen")
        #[arg(long)]
        institution: Option<String>,
        /// Description
        #[arg(short, long)]
        desc: Option<String>,
    },
    /// List all course groups
    #[command(alias = "ls")]
    List,
    /// Show a course group with every iteration inside it
    Show {
        /// Group ID (cg-xxx)
        id: String,
    },
    /// Delete a course group (iterations become ungrouped, not deleted)
    Delete {
        /// Group ID
        id: String,
    },
}

pub async fn handle_course_group(base: &str, config: &Config, action: CourseGroupCommand) -> Result<()> {
    let token = config.token()?;
    match action {
        CourseGroupCommand::Create { title, code, institution, desc } => {
            let body = serde_json::json!({
                "title": title,
                "code": code,
                "institution": institution,
                "description": desc,
            });
            let resp: serde_json::Value = client()
                .post(format!("{base}/course-groups"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Create course group failed")?
                .json().await?;
            let id = resp["id"].as_str().unwrap_or("?");
            println!("Created course group: {title}");
            println!("ID: {id}");
        }

        CourseGroupCommand::List => {
            let rows: Vec<serde_json::Value> = client()
                .get(format!("{base}/course-groups"))
                .send().await?
                .error_for_status().context("List course groups failed")?
                .json().await?;
            if rows.is_empty() {
                println!("No course groups yet.");
            } else {
                for g in &rows {
                    let id = g["id"].as_str().unwrap_or("?");
                    let title = g["title"].as_str().unwrap_or("?");
                    let code = g["code"].as_str().unwrap_or("");
                    let inst = g["institution"].as_str().unwrap_or("");
                    let code_part = if code.is_empty() { String::new() } else { format!(" [{code}]") };
                    let inst_part = if inst.is_empty() { String::new() } else { format!(" ({inst})") };
                    println!("  {id}  {title}{code_part}{inst_part}");
                }
                println!("{} group(s)", rows.len());
            }
        }

        CourseGroupCommand::Show { id } => {
            let resp: serde_json::Value = client()
                .get(format!("{base}/course-groups/{id}"))
                .send().await?
                .error_for_status().context("Get course group failed")?
                .json().await?;
            let group = &resp["group"];
            let title = group["title"].as_str().unwrap_or("?");
            let code = group["code"].as_str().unwrap_or("-");
            let inst = group["institution"].as_str().unwrap_or("-");
            let desc = group["description"].as_str().unwrap_or("");
            println!("Title:       {title}");
            println!("Code:        {code}");
            println!("Institution: {inst}");
            if !desc.is_empty() {
                println!("Description: {desc}");
            }
            if let Some(courses) = resp["courses"].as_array() {
                println!("\nIterations:");
                for c in courses {
                    let cid = c["id"].as_str().unwrap_or("?");
                    let ctitle = c["title"].as_str().unwrap_or("?");
                    let sem = c["semester"].as_str().unwrap_or("-");
                    println!("  {cid}  {sem:<16}  {ctitle}");
                }
            }
        }

        CourseGroupCommand::Delete { id } => {
            client()
                .delete(format!("{base}/course-groups/{id}"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("Delete course group failed")?;
            println!("Deleted course group {id} (member courses are now ungrouped).");
        }
    }
    Ok(())
}
