use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Subcommand;

use crate::{Config, client};

#[derive(Subcommand)]
pub enum CourseCommand {
    /// List published courses
    #[command(alias = "ls")]
    List,
    /// Show course detail
    Show {
        /// Course ID (e.g. crs-xxx)
        id: String,
    },
    /// Create a new course
    Create {
        /// Course title
        #[arg(short, long)]
        title: String,
        /// Course code (e.g. 18.404)
        #[arg(long)]
        code: Option<String>,
        /// Description
        #[arg(short, long)]
        desc: Option<String>,
        /// Institution
        #[arg(long)]
        institution: Option<String>,
        /// Department
        #[arg(long)]
        department: Option<String>,
        /// Semester (e.g. Fall 2020)
        #[arg(long)]
        semester: Option<String>,
        /// Language (default: en)
        #[arg(short, long, default_value = "en")]
        lang: String,
        /// Source URL (e.g. OCW link)
        #[arg(long)]
        source_url: Option<String>,
        /// Source attribution
        #[arg(long)]
        source_attribution: Option<String>,
        /// Instructors / authors (comma-separated names). Creates author
        /// entities that can later be bound to a platform DID.
        #[arg(long, value_delimiter = ',')]
        authors: Vec<String>,
    },
    /// Update course metadata
    Update {
        /// Course ID
        id: String,
        /// New title
        #[arg(short, long)]
        title: Option<String>,
        /// New description
        #[arg(short, long)]
        desc: Option<String>,
        /// Replace the instructor list (comma-separated)
        #[arg(long, value_delimiter = ',')]
        authors: Option<Vec<String>>,
    },
    /// Add a session (lecture) to a course
    #[command(name = "add-session")]
    AddSession {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Topic
        #[arg(short, long)]
        topic: Option<String>,
        /// Date
        #[arg(long)]
        date: Option<String>,
        /// Materials as "kind:label:url" (e.g. "reading:§1 Intro:https://...", "handout:Handout:https://..."). kind ∈ {reading, slides, handout, summary, notes} or empty.
        #[arg(long, value_delimiter = ',')]
        material: Vec<String>,
        /// Resources as "type:label:url" — video, hw, or discussion only
        #[arg(long, value_delimiter = ',')]
        resource: Vec<String>,
        /// Sort order (auto-increments if omitted)
        #[arg(long)]
        order: Option<i32>,
        /// Tags (comma-separated tag IDs) — what this session covers
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Prereq tags (comma-separated tag IDs) — what you should know
        #[arg(long, value_delimiter = ',')]
        prereqs: Vec<String>,
    },
    /// Update a session
    #[command(name = "update-session")]
    UpdateSession {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Session ID
        #[arg(long)]
        session_id: String,
        /// Topic
        #[arg(short, long)]
        topic: Option<String>,
        /// Materials as "kind:label:url" (replaces all materials)
        #[arg(long, value_delimiter = ',')]
        material: Vec<String>,
        /// Resources as "type:label:url" (replaces all resources) — video, hw, discussion only
        #[arg(long, value_delimiter = ',')]
        resource: Vec<String>,
    },
    /// Delete a session
    #[command(name = "rm-session")]
    RmSession {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Session ID
        #[arg(long)]
        session_id: String,
    },
    /// Add a tag to the course
    #[command(name = "add-tag")]
    AddTag {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Tag ID
        tag_id: String,
    },
    /// Remove a tag from the course
    #[command(name = "rm-tag")]
    RmTag {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Tag ID
        tag_id: String,
    },
    /// Add a textbook to the course
    #[command(name = "add-textbook")]
    AddTextbook {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Role (required/recommended/supplementary)
        #[arg(long, default_value = "required")]
        role: String,
    },
    /// Import sessions from a TOML file
    Import {
        /// Course ID
        course_id: String,
        /// Path to TOML file
        file: PathBuf,
    },
    /// Link a course to a course group (so it appears alongside other iterations)
    #[command(name = "set-group")]
    SetGroup {
        /// Course ID
        course_id: String,
        /// Group ID (cg-xxx)
        #[arg(long)]
        group_id: String,
    },
    /// Remove a course from its current group (the group itself is kept)
    #[command(name = "unset-group")]
    UnsetGroup {
        /// Course ID
        course_id: String,
    },
    /// Add a homework / assignment entry to a course (optionally anchored
    /// to a specific session). First-class entity so questions can be
    /// filed against it.
    #[command(name = "add-homework")]
    AddHomework {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Session ID (omit for course-wide homeworks)
        #[arg(long)]
        session_id: Option<String>,
        /// Display label (e.g. "Homework 3 — GP Classification")
        #[arg(short, long)]
        label: String,
        /// URL to assignment PDF / github folder
        #[arg(short, long)]
        url: Option<String>,
        /// Description
        #[arg(short, long)]
        desc: Option<String>,
        /// Sort position
        #[arg(long, default_value = "0")]
        position: i32,
        /// Due date (YYYY-MM-DD)
        #[arg(long)]
        due_date: Option<String>,
    },
    /// List homeworks for a course (or just those anchored to one session)
    #[command(name = "list-homeworks")]
    ListHomeworks {
        /// Course ID (mutually exclusive with --session-id)
        #[arg(long)]
        course_id: Option<String>,
        /// Session ID
        #[arg(long)]
        session_id: Option<String>,
    },
    /// Delete a homework
    #[command(name = "rm-homework")]
    RmHomework {
        /// Homework ID (chw-xxx)
        id: String,
    },
}

pub async fn handle_course(base: &str, config: &Config, action: CourseCommand) -> Result<()> {
    let token = config.token()?;
    match action {
        CourseCommand::List => {
            let resp: Vec<serde_json::Value> = client()
                .get(format!("{base}/courses"))
                .send().await?
                .error_for_status().context("List courses failed")?
                .json().await?;

            if resp.is_empty() {
                println!("No courses.");
            }
            for c in &resp {
                let id = c["id"].as_str().unwrap_or("?");
                let title = c["title"].as_str().unwrap_or("?");
                let code = c["code"].as_str().unwrap_or("");
                let inst = c["institution"].as_str().unwrap_or("");
                if code.is_empty() {
                    println!("{id}\t{title}\t{inst}");
                } else {
                    println!("{id}\t{code} {title}\t{inst}");
                }
            }
        }

        CourseCommand::Show { id } => {
            let resp: serde_json::Value = client()
                .get(format!("{base}/courses/{id}"))
                .send().await?
                .error_for_status().context("Get course failed")?
                .json().await?;

            let c = &resp["course"];
            let code = c["code"].as_str().unwrap_or("");
            let title = c["title"].as_str().unwrap_or("?");
            let inst = c["institution"].as_str().unwrap_or("");
            println!("{code} {title}  ({inst})");
            println!("ID: {id}");

            if let Some(sessions) = resp["sessions"].as_array() {
                if !sessions.is_empty() {
                    println!("\nSessions:");
                    for s in sessions {
                        let sid = s["id"].as_str().unwrap_or("?");
                        let order = s["sort_order"].as_i64().unwrap_or(0);
                        let topic = s["topic"].as_str().unwrap_or("-");
                        let materials = s["materials"].as_array();
                        let res = s["resources"].as_array();
                        let mat_flag = if materials.map_or(false, |m| !m.is_empty()) { "📘" } else { "" };
                        let video = if res.map_or(false, |r| r.iter().any(|x| x["type"] == "video")) { "📹" } else { "" };
                        let hw = if res.map_or(false, |r| r.iter().any(|x| x["type"] == "hw")) { "📋" } else { "" };
                        print!("  {order}. {topic}");
                        if !mat_flag.is_empty() { print!(" {mat_flag}"); }
                        if !video.is_empty() { print!(" {video}"); }
                        if !hw.is_empty() { print!(" {hw}"); }

                        // Show tags
                        if let Some(tags) = s["tags"].as_array() {
                            if !tags.is_empty() {
                                let names: Vec<&str> = tags.iter()
                                    .filter_map(|t| t["tag_name"].as_str())
                                    .collect();
                                print!("  [{}]", names.join(", "));
                            }
                        }
                        println!("  ({sid})");
                    }
                }
            }

            if let Some(textbooks) = resp["textbooks"].as_array() {
                if !textbooks.is_empty() {
                    println!("\nTextbooks:");
                    for tb in textbooks {
                        let title = tb["title"].as_str().unwrap_or("?");
                        let role = tb["role"].as_str().unwrap_or("?");
                        println!("  - {title} ({role})");
                    }
                }
            }
        }

        CourseCommand::Create { title, code, desc, institution, department, semester, lang, source_url, source_attribution, authors } => {
            let body = serde_json::json!({
                "title": title,
                "code": code,
                "description": desc,
                "institution": institution,
                "department": department,
                "semester": semester,
                "lang": lang,
                "source_url": source_url,
                "source_attribution": source_attribution,
                "authors": authors,
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

        CourseCommand::Update { id, title, desc, authors } => {
            let mut body = serde_json::json!({});
            if let Some(t) = &title { body["title"] = serde_json::json!(t); }
            if let Some(d) = &desc { body["description"] = serde_json::json!(d); }
            if let Some(ref a) = authors { body["authors"] = serde_json::json!(a); }

            client()
                .put(format!("{base}/courses/{id}"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Update course failed")?;

            println!("Updated course {id}");
        }

        CourseCommand::AddSession { course_id, topic, date, material, resource, order, tags, prereqs } => {
            let materials: Vec<serde_json::Value> = material.iter().filter_map(|s| {
                let parts: Vec<&str> = s.splitn(3, ':').collect();
                match parts.len() {
                    3 => {
                        let kind = if parts[0].is_empty() { serde_json::Value::Null } else { serde_json::Value::String(parts[0].to_string()) };
                        Some(serde_json::json!({"kind": kind, "label": parts[1], "url": parts[2]}))
                    }
                    2 => {
                        let kind = if parts[0].is_empty() { serde_json::Value::Null } else { serde_json::Value::String(parts[0].to_string()) };
                        Some(serde_json::json!({"kind": kind, "label": parts[1]}))
                    }
                    _ => None,
                }
            }).collect();
            let resources: Vec<serde_json::Value> = resource.iter().filter_map(|s| {
                let parts: Vec<&str> = s.splitn(3, ':').collect();
                if parts.len() == 3 {
                    Some(serde_json::json!({"type": parts[0], "label": parts[1], "url": parts[2]}))
                } else { None }
            }).collect();

            let body = serde_json::json!({
                "topic": topic,
                "date": date,
                "materials": materials,
                "resources": resources,
                "sort_order": order,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/courses/{course_id}/sessions"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Create session failed")?
                .json().await?;

            let session_id = resp["id"].as_str().unwrap_or("?");
            let topic_str = topic.as_deref().unwrap_or("(untitled)");
            println!("Created session: {topic_str} ({session_id})");

            // Add tags
            for tag_id in &tags {
                client()
                    .post(format!("{base}/courses/{course_id}/sessions/{session_id}/tags"))
                    .bearer_auth(token)
                    .json(&serde_json::json!({ "tag_id": tag_id }))
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to add tag {tag_id}"))?;
            }
            if !tags.is_empty() {
                println!("  Added {} tags", tags.len());
            }

            // Add prereqs
            for tag_id in &prereqs {
                client()
                    .post(format!("{base}/courses/{course_id}/sessions/{session_id}/prereqs"))
                    .bearer_auth(token)
                    .json(&serde_json::json!({ "tag_id": tag_id }))
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to add prereq {tag_id}"))?;
            }
            if !prereqs.is_empty() {
                println!("  Added {} prereqs", prereqs.len());
            }
        }

        CourseCommand::UpdateSession { course_id, session_id, topic, material, resource } => {
            let mut body = serde_json::json!({
                "topic": topic,
            });
            if !material.is_empty() {
                let materials: Vec<serde_json::Value> = material.iter().filter_map(|s| {
                    let parts: Vec<&str> = s.splitn(3, ':').collect();
                    match parts.len() {
                        3 => {
                            let kind = if parts[0].is_empty() { serde_json::Value::Null } else { serde_json::Value::String(parts[0].to_string()) };
                            Some(serde_json::json!({"kind": kind, "label": parts[1], "url": parts[2]}))
                        }
                        2 => {
                            let kind = if parts[0].is_empty() { serde_json::Value::Null } else { serde_json::Value::String(parts[0].to_string()) };
                            Some(serde_json::json!({"kind": kind, "label": parts[1]}))
                        }
                        _ => None,
                    }
                }).collect();
                body["materials"] = serde_json::json!(materials);
            }
            if !resource.is_empty() {
                let resources: Vec<serde_json::Value> = resource.iter().filter_map(|s| {
                    let parts: Vec<&str> = s.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        Some(serde_json::json!({"type": parts[0], "label": parts[1], "url": parts[2]}))
                    } else { None }
                }).collect();
                body["resources"] = serde_json::json!(resources);
            }

            client()
                .put(format!("{base}/courses/{course_id}/sessions/{session_id}"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Update session failed")?;

            println!("Updated session {session_id}");
        }

        CourseCommand::RmSession { course_id, session_id } => {
            client()
                .delete(format!("{base}/courses/{course_id}/sessions/{session_id}"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("Delete session failed")?;

            println!("Deleted session {session_id}");
        }

        CourseCommand::AddTag { course_id, tag_id } => {
            client()
                .post(format!("{base}/courses/{course_id}/tags"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "tag_id": tag_id }))
                .send().await?
                .error_for_status().context("Add tag failed")?;

            println!("Added tag {tag_id} to course {course_id}");
        }

        CourseCommand::RmTag { course_id, tag_id } => {
            client()
                .delete(format!("{base}/courses/{course_id}/tags"))
                .bearer_auth(token)
                .query(&[("tag_id", &tag_id)])
                .send().await?
                .error_for_status().context("Remove tag failed")?;

            println!("Removed tag {tag_id} from course {course_id}");
        }

        CourseCommand::AddTextbook { course_id, book_id, role } => {
            client()
                .post(format!("{base}/courses/{course_id}/textbooks"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "book_id": book_id, "role": role }))
                .send().await?
                .error_for_status().context("Add textbook failed")?;

            println!("Added textbook {book_id} to course {course_id}");
        }

        CourseCommand::Import { course_id, file } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;
            let data: toml::Value = content.parse()
                .context("Invalid TOML")?;

            let sessions = data.get("session")
                .and_then(|v| v.as_array())
                .context("Expected [[session]] array in TOML")?;

            // Fetch existing sessions to enable incremental updates
            let detail: serde_json::Value = client()
                .get(format!("{base}/courses/{course_id}"))
                .send().await?
                .error_for_status().context("Failed to fetch course")?
                .json().await?;

            let existing: std::collections::HashMap<i64, serde_json::Value> = detail["sessions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|s| {
                    let order = s["sort_order"].as_i64()?;
                    Some((order, s.clone()))
                })
                .collect();

            let mut created = 0;
            let mut updated = 0;
            let mut skipped = 0;

            for (i, s) in sessions.iter().enumerate() {
                let sort_order = s.get("order").and_then(|v| v.as_integer()).unwrap_or((i + 1) as i64);

                // Build resources array from TOML
                let mut resources = Vec::new();
                if let Some(res_arr) = s.get("resources").and_then(|v| v.as_array()) {
                    // New format: [[session.resources]]
                    for r in res_arr {
                        resources.push(serde_json::json!({
                            "type": r.get("type").and_then(|v| v.as_str()).unwrap_or("notes"),
                            "url": r.get("url").and_then(|v| v.as_str()).unwrap_or(""),
                            "label": r.get("label").and_then(|v| v.as_str()).unwrap_or(""),
                        }));
                    }
                }

                let body = serde_json::json!({
                    "topic": s.get("topic").and_then(|v| v.as_str()),
                    "date": s.get("date").and_then(|v| v.as_str()),
                    "readings": s.get("readings").and_then(|v| v.as_str()),
                    "resources": resources,
                    "sort_order": sort_order,
                });

                let topic = s.get("topic").and_then(|v| v.as_str()).unwrap_or("-");

                if let Some(ex) = existing.get(&sort_order) {
                    // Check if anything changed
                    let changed = body["topic"] != ex["topic"]
                        || body["date"] != ex["date"]
                        || body["readings"] != ex["readings"]
                        || body["resources"] != ex["resources"];

                    if !changed {
                        skipped += 1;
                        continue;
                    }

                    // Update existing session
                    let session_id = ex["id"].as_str().unwrap_or("?");
                    client()
                        .put(format!("{base}/courses/{course_id}/sessions/{session_id}"))
                        .bearer_auth(token)
                        .json(&body)
                        .send().await?
                        .error_for_status()
                        .with_context(|| format!("Failed to update session {sort_order}"))?;
                    println!("  [{}/{}] ~ {topic} (updated)", i + 1, sessions.len());
                    updated += 1;
                } else {
                    // Create new session
                    let resp: serde_json::Value = client()
                        .post(format!("{base}/courses/{course_id}/sessions"))
                        .bearer_auth(token)
                        .json(&body)
                        .send().await?
                        .error_for_status()
                        .with_context(|| format!("Failed to create session {}", i + 1))?
                        .json().await?;

                    let session_id = resp["id"].as_str().unwrap_or("?");
                    println!("  [{}/{}] + {topic} ({session_id})", i + 1, sessions.len());
                    created += 1;

                    // Add tags
                    if let Some(tags) = s.get("tags").and_then(|v| v.as_array()) {
                        for tag in tags {
                            if let Some(tag_id) = tag.as_str() {
                                client()
                                    .post(format!("{base}/courses/{course_id}/sessions/{session_id}/tags"))
                                    .bearer_auth(token)
                                    .json(&serde_json::json!({ "tag_id": tag_id }))
                                    .send().await?
                                    .error_for_status()?;
                            }
                        }
                    }

                    // Add prereqs
                    if let Some(prereqs) = s.get("prereqs").and_then(|v| v.as_array()) {
                        for tag in prereqs {
                            if let Some(tag_id) = tag.as_str() {
                                client()
                                    .post(format!("{base}/courses/{course_id}/sessions/{session_id}/prereqs"))
                                    .bearer_auth(token)
                                    .json(&serde_json::json!({ "tag_id": tag_id }))
                                    .send().await?
                                    .error_for_status()?;
                            }
                        }
                    }
                }
            }

            println!("\n{created} created, {updated} updated, {skipped} unchanged");
        }

        CourseCommand::SetGroup { course_id, group_id } => {
            client()
                .put(format!("{base}/courses/{course_id}/group"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "group_id": group_id }))
                .send().await?
                .error_for_status().context("Set group failed")?;
            println!("Linked course {course_id} to group {group_id}.");
        }

        CourseCommand::UnsetGroup { course_id } => {
            client()
                .delete(format!("{base}/courses/{course_id}/group"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("Unset group failed")?;
            println!("Unlinked course {course_id} from its group.");
        }

        CourseCommand::AddHomework { course_id, session_id, label, url, desc, position, due_date } => {
            let body = serde_json::json!({
                "course_id": course_id,
                "session_id": session_id,
                "label": label,
                "url": url,
                "description": desc,
                "position": position,
                "due_date": due_date,
            });
            let resp: serde_json::Value = client()
                .post(format!("{base}/homeworks"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Add homework failed")?
                .json().await?;
            let id = resp["id"].as_str().unwrap_or("?");
            println!("Added homework: {label} ({id})");
        }

        CourseCommand::ListHomeworks { course_id, session_id } => {
            let mut url = format!("{base}/homeworks?");
            match (course_id.as_deref(), session_id.as_deref()) {
                (Some(c), None) => url.push_str(&format!("course_id={c}")),
                (None, Some(s)) => url.push_str(&format!("session_id={s}")),
                _ => bail!("pass exactly one of --course-id / --session-id"),
            }
            let rows: Vec<serde_json::Value> = client()
                .get(url)
                .send().await?
                .error_for_status().context("List homeworks failed")?
                .json().await?;
            if rows.is_empty() {
                println!("No homeworks yet.");
            } else {
                for h in &rows {
                    let hid = h["id"].as_str().unwrap_or("?");
                    let label = h["label"].as_str().unwrap_or("?");
                    let sid = h["session_id"].as_str().unwrap_or("-");
                    let due = h["due_date"].as_str().unwrap_or("-");
                    let u = h["url"].as_str().unwrap_or("-");
                    println!("  {hid}  [session: {sid}]  due {due}\n    {label}\n    {u}");
                }
                println!("{} homework(s)", rows.len());
            }
        }

        CourseCommand::RmHomework { id } => {
            client()
                .delete(format!("{base}/homeworks/{id}"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("Delete homework failed")?;
            println!("Deleted homework {id}.");
        }
    }

    Ok(())
}
