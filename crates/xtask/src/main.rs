//! Workspace xtask runner.
//!
//! Subcommands:
//!   gen-lexicon    — Parse lexicons/*.json and emit Rust record types into
//!                    crates/fx-atproto/src/records/. Run this whenever a
//!                    lexicon JSON changes so the Rust types stay in sync.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use heck::{ToPascalCase, ToSnakeCase};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Parser)]
#[command(name = "xtask")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Regenerate Rust record types from lexicons/*.json
    GenLexicon {
        /// Lexicon source directory
        #[arg(long, default_value = "lexicons")]
        lexicons: PathBuf,
        /// Output directory for generated records
        #[arg(long, default_value = "crates/fx-atproto/src/records")]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::GenLexicon { lexicons, out } => gen_lexicon(&lexicons, &out),
    }
}

// ---- Lexicon schema (minimal subset) ----

#[derive(Deserialize)]
struct Lexicon {
    id: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    defs: BTreeMap<String, Def>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Def {
    #[serde(rename = "record")]
    Record {
        // AT Proto record key strategy ("tid", "any", etc.). Parsed so the
        // lexicon JSON round-trips cleanly, but the generator doesn't need
        // it today — the key just becomes the record's rkey at runtime.
        #[serde(default)]
        #[allow(dead_code)]
        key: Option<String>,
        record: ObjectDef,
    },
    #[serde(rename = "object")]
    Object(ObjectDef),
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Default)]
struct ObjectDef {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    properties: BTreeMap<String, FieldType>,
}

/// Flattened lexicon field: lexicon JSON mixes `{type, ...}` with shorthand
/// objects that omit `type` (e.g. `{ref: "#foo"}` for inline refs), so we
/// keep every property optional and classify at render time.
#[derive(Deserialize, Default)]
struct FieldType {
    #[serde(rename = "type", default)]
    ty: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    items: Option<Box<FieldType>>,
    #[serde(rename = "ref", default)]
    ref_target: Option<String>,
}

#[derive(PartialEq)]
enum Kind {
    Str,
    Int,
    Bool,
    Array,
    Blob,
    RefOrUnion,
    Object,
    Unknown,
}

impl FieldType {
    fn kind(&self) -> Kind {
        if self.ref_target.is_some() {
            return Kind::RefOrUnion;
        }
        match self.ty.as_deref() {
            Some("string") => Kind::Str,
            Some("integer") => Kind::Int,
            Some("boolean") => Kind::Bool,
            Some("array") => Kind::Array,
            Some("blob") => Kind::Blob,
            Some("ref") | Some("union") => Kind::RefOrUnion,
            Some("object") => Kind::Object,
            _ => Kind::Unknown,
        }
    }
}

// ---- Generator ----

fn gen_lexicon(src: &Path, out: &Path) -> Result<()> {
    std::fs::create_dir_all(out).with_context(|| format!("create {}", out.display()))?;

    let mut modules: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(src).with_context(|| format!("read {}", src.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))?;
        let lex: Lexicon = serde_json::from_str(&text)
            .with_context(|| format!("parse {}", path.display()))?;

        let module_name = module_name_for(&lex.id);
        let out_file = out.join(format!("{module_name}.rs"));
        let generated = render_module(&lex)?;
        std::fs::write(&out_file, &generated)
            .with_context(|| format!("write {}", out_file.display()))?;
        modules.push(module_name);
        println!("  generated {}", out_file.display());
    }

    modules.sort();
    let mut mod_rs = String::from(
        "//! Generated AT Protocol record types. DO NOT EDIT.\n\
         //! Regenerate with `cargo xtask gen-lexicon`.\n\n",
    );
    for m in &modules {
        mod_rs.push_str(&format!("pub mod {m};\n"));
    }
    std::fs::write(out.join("mod.rs"), mod_rs)?;
    println!("{} modules written to {}", modules.len(), out.display());
    Ok(())
}

/// Turn `at.nightbo.book.rating` into `book_rating` so the generated module
/// name is a valid Rust identifier and reads naturally.
fn module_name_for(nsid: &str) -> String {
    let tail = nsid.strip_prefix("at.nightbo.").unwrap_or(nsid);
    tail.replace('.', "_").to_snake_case()
}

fn render_module(lex: &Lexicon) -> Result<String> {
    let mut out = String::new();
    out.push_str("//! Generated from lexicons/");
    out.push_str(&lex.id);
    out.push_str(".json — DO NOT EDIT. Regenerate with `cargo xtask gen-lexicon`.\n");
    if let Some(desc) = &lex.description {
        out.push_str("//!\n");
        for line in desc.lines() {
            out.push_str("//! ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push('\n');
    out.push_str("use serde::{Deserialize, Serialize};\n\n");

    out.push_str("/// NSID for this record type.\n");
    out.push_str(&format!("pub const NSID: &str = {:?};\n\n", lex.id));

    // Emit auxiliary defs (objects) first, then main as `Record`.
    for (name, def) in &lex.defs {
        if name == "main" { continue; }
        if let Def::Object(obj) = def {
            out.push_str(&render_struct(&name.to_pascal_case(), obj));
        }
    }
    if let Some(Def::Record { key: _, record }) = lex.defs.get("main") {
        out.push_str(&render_struct("Record", record));
    } else {
        bail!("{}: lexicon has no main record", lex.id);
    }
    Ok(out)
}

fn render_struct(name: &str, obj: &ObjectDef) -> String {
    let mut out = String::new();
    if let Some(desc) = &obj.description {
        for line in desc.lines() {
            out.push_str("/// ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    out.push_str(&format!("pub struct {name} {{\n"));
    for (field_name, field) in &obj.properties {
        let required = obj.required.iter().any(|r| r == field_name);
        out.push_str(&render_field(field_name, field, required));
    }
    out.push_str("}\n\n");
    out
}

fn render_field(name: &str, field: &FieldType, required: bool) -> String {
    let snake = to_rust_field_name(name);
    let rename_needed = snake != *name;
    let inner_ty = rust_type_for(field);
    let (ty, skip_if) = if required {
        (inner_ty, None)
    } else {
        let opt = format!("Option<{inner_ty}>");
        (opt, Some("Option::is_none"))
    };

    let mut out = String::new();
    if let Some(desc) = field_description(field) {
        for line in desc.lines() {
            out.push_str("    /// ");
            out.push_str(line);
            out.push('\n');
        }
    }
    match (rename_needed, skip_if) {
        (true, Some(skip)) => out.push_str(&format!(
            "    #[serde(rename = {name:?}, default, skip_serializing_if = \"{skip}\")]\n"
        )),
        (true, None) => out.push_str(&format!("    #[serde(rename = {name:?})]\n")),
        (false, Some(skip)) => out.push_str(&format!(
            "    #[serde(default, skip_serializing_if = \"{skip}\")]\n"
        )),
        (false, None) => {}
    }
    out.push_str(&format!("    pub {snake}: {ty},\n"));
    out
}

fn field_description(field: &FieldType) -> Option<&str> {
    field.description.as_deref()
}

fn rust_type_for(field: &FieldType) -> String {
    match field.kind() {
        Kind::Str => "String".into(),
        Kind::Int => "i64".into(),
        Kind::Bool => "bool".into(),
        Kind::Array => {
            let inner = field.items.as_deref()
                .map(rust_type_for)
                .unwrap_or_else(|| "serde_json::Value".into());
            format!("Vec<{inner}>")
        }
        // Blobs / refs / unions / nested objects / unknowns all round-trip
        // as `Value` for now. Tighter typing (BlobRef from atproto-identity,
        // generated aux structs for objects, enums for unions) is a follow-up
        // once consumers need it.
        Kind::Blob | Kind::RefOrUnion | Kind::Object | Kind::Unknown => {
            "serde_json::Value".into()
        }
    }
}

fn to_rust_field_name(name: &str) -> String {
    let snake = name.to_snake_case();
    // Avoid collisions with Rust keywords/reserved words by prepending `r#`.
    match snake.as_str() {
        "type" | "ref" | "move" | "match" | "self" | "fn" | "use" | "trait" | "mod"
        | "let" | "const" | "static" | "loop" | "for" | "if" | "else" | "while" => {
            format!("r#{snake}")
        }
        _ => snake,
    }
}
