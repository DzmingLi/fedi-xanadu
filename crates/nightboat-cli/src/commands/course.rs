use anyhow::{Context, Result};
use clap::Subcommand;

use crate::{Config, client};

#[derive(Subcommand)]
pub enum CourseCommand {
    /// Create a new course (an umbrella over several iterations)
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
    /// List all courses
    #[command(alias = "ls")]
    List,
    /// Show a course with every iteration inside it
    Show {
        /// Course ID (crs-xxx)
        id: String,
    },
    /// Delete a course (iterations become unlinked, not deleted)
    Delete {
        /// Course ID
        id: String,
    },
}

pub async fn handle_course(base: &str, config: &Config, action: CourseCommand) -> Result<()> {
    let token = config.token()?;
    match action {
        CourseCommand::Create { title, code, institution, desc } => {
            let body = serde_json::json!({
                "title": title,
                "code": code,
                "institution": institution,
                "description": desc,
            });
            let resp: serde_json::Value = client()
                .post(format!("{base}/courses"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Create course failed")?
                .json().await?;
            let id = resp["id"].as_str().unwrap_or("?");
            println!("Created course: {title}");
            println!("ID: {id}");
        }

        CourseCommand::List => {
            let rows: Vec<serde_json::Value> = client()
                .get(format!("{base}/courses"))
                .send().await?
                .error_for_status().context("List courses failed")?
                .json().await?;
            if rows.is_empty() {
                println!("No courses yet.");
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
                println!("{} course(s)", rows.len());
            }
        }

        CourseCommand::Show { id } => {
            let resp: serde_json::Value = client()
                .get(format!("{base}/courses/{id}"))
                .send().await?
                .error_for_status().context("Get course failed")?
                .json().await?;
            let course = &resp["course"];
            let title = course["title"].as_str().unwrap_or("?");
            let code = course["code"].as_str().unwrap_or("-");
            let inst = course["institution"].as_str().unwrap_or("-");
            let desc = course["description"].as_str().unwrap_or("");
            println!("Title:       {title}");
            println!("Code:        {code}");
            println!("Institution: {inst}");
            if !desc.is_empty() {
                println!("Description: {desc}");
            }
            if let Some(terms) = resp["terms"].as_array() {
                println!("\nIterations:");
                for c in terms {
                    let cid = c["id"].as_str().unwrap_or("?");
                    let ctitle = c["title"].as_str().unwrap_or("?");
                    let sem = c["semester"].as_str().unwrap_or("-");
                    println!("  {cid}  {sem:<16}  {ctitle}");
                }
            }
        }

        CourseCommand::Delete { id } => {
            client()
                .delete(format!("{base}/courses/{id}"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("Delete course failed")?;
            println!("Deleted course {id} (member terms are now unlinked).");
        }
    }
    Ok(())
}
