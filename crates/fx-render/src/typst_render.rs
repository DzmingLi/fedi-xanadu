use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt, World};
use typst_html::HtmlDocument;

/// Global packages cache directory. Set via `set_packages_dir()`.
static PACKAGES_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Set the global directory for caching Typst packages.
/// Call once at startup. Default: `{data_dir}/typst-packages`.
pub fn set_packages_dir(dir: PathBuf) {
    let _ = PACKAGES_DIR.set(dir);
}

fn packages_dir() -> Option<&'static Path> {
    PACKAGES_DIR.get().map(|p| p.as_path())
}

/// Mathyml library files, embedded at compile time.
const MATHYML_FILES: &[(&str, &str)] = &[
    ("mathyml/lib.typ", include_str!("../typst-libs/mathyml/lib.typ")),
    ("mathyml/convert.typ", include_str!("../typst-libs/mathyml/convert.typ")),
    ("mathyml/prelude.typ", include_str!("../typst-libs/mathyml/prelude.typ")),
    ("mathyml/unicode.typ", include_str!("../typst-libs/mathyml/unicode.typ")),
    ("mathyml/utils.typ", include_str!("../typst-libs/mathyml/utils.typ")),
];

/// Standard library for Fedi-Xanadu articles (theorem envs etc.)
const FX_LIB: &str = include_str!("../typst-libs/fx/lib.typ");

/// Preamble injected before user content.
const MATH_PREAMBLE: &str = r#"#import "mathyml/lib.typ": try-to-mathml, include-mathfont
#show math.equation: try-to-mathml
#import "fx/lib.typ": *
"#;

/// Extended preamble for series documents (enables heading numbering for cross-references).
const SERIES_PREAMBLE: &str = r#"#import "mathyml/lib.typ": try-to-mathml, include-mathfont
#show math.equation: try-to-mathml
#import "fx/lib.typ": *
#set heading(numbering: "1.1")
"#;

struct RenderWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: Source,
    sources: HashMap<FileId, Source>,
    /// Optional repo directory for resolving images and other binary files.
    repo_path: Option<PathBuf>,
}

impl RenderWorld {
    fn new(text: &str) -> Self {
        Self::with_preamble(text, MATH_PREAMBLE, None)
    }

    fn with_repo(text: &str, repo_path: Option<&Path>) -> Self {
        Self::with_preamble(text, MATH_PREAMBLE, repo_path)
    }

    fn with_preamble(text: &str, preamble: &str, repo_path: Option<&Path>) -> Self {
        let features: Features = [Feature::Html].into_iter().collect();
        let library = Library::builder().with_features(features).build();

        // Load bundled fonts
        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        for data in typst_assets::fonts() {
            let buffer = Bytes::new(data.to_vec());
            for font in Font::iter(buffer) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }

        // Build virtual filesystem
        let mut sources = HashMap::new();

        // Add mathyml library files
        for (path, content) in MATHYML_FILES {
            let id = FileId::new(None, VirtualPath::new(path));
            sources.insert(id, Source::new(id, (*content).into()));
        }

        // Add fx standard library
        {
            let id = FileId::new(None, VirtualPath::new("fx/lib.typ"));
            sources.insert(id, Source::new(id, FX_LIB.into()));
        }

        // Main source = preamble + user content
        let full_source = format!("{preamble}{text}");
        let main_id = FileId::new(None, VirtualPath::new("main.typ"));
        let main = Source::new(main_id, full_source);
        sources.insert(main_id, main.clone());

        Self {
            library: LazyHash::new(library),
            book: LazyHash::new(book),
            fonts,
            main,
            sources,
            repo_path: repo_path.map(|p| p.to_path_buf()),
        }
    }
}

impl World for RenderWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        // Check in-memory sources first
        if let Some(s) = self.sources.get(&id) {
            return Ok(s.clone());
        }
        // Try loading from package or repo
        let bytes = self.file(id)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text.into()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        // Return raw bytes for any known source file
        if let Some(s) = self.sources.get(&id) {
            return Ok(Bytes::new(s.text().as_bytes().to_vec()));
        }

        // Try loading from package cache
        if let Some(pkg) = id.package() {
            if let Some(pkg_dir) = resolve_package(pkg) {
                let rel = id.vpath().as_rootless_path();
                let path = pkg_dir.join(rel);
                if path.exists() {
                    let data = std::fs::read(&path)
                        .map_err(|_| FileError::NotFound(rel.into()))?;
                    return Ok(Bytes::new(data));
                }
            }
            return Err(FileError::NotFound(id.vpath().as_rootless_path().into()));
        }

        // Try loading from repo directory (for images etc.)
        if let Some(ref repo) = self.repo_path {
            let rel = id.vpath().as_rootless_path();
            let path = repo.join(rel);
            if path.exists() {
                let data = std::fs::read(&path)
                    .map_err(|_| FileError::NotFound(rel.into()))?;
                return Ok(Bytes::new(data));
            }
        }
        Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

/// Resolve a Typst package to a local directory, downloading if needed.
/// Packages are cached at `{PACKAGES_DIR}/{namespace}/{name}/{version}/`.
fn resolve_package(pkg: &typst::syntax::package::PackageSpec) -> Option<PathBuf> {
    let cache_dir = packages_dir()?;
    let pkg_dir = cache_dir
        .join(pkg.namespace.as_str())
        .join(pkg.name.as_str())
        .join(pkg.version.to_string());

    // Already cached
    if pkg_dir.join("typst.toml").exists() {
        return Some(pkg_dir);
    }

    // Download from registry
    let url = format!(
        "https://packages.typst.org/{}/{}-{}.tar.gz",
        pkg.namespace, pkg.name, pkg.version
    );
    tracing::info!("downloading typst package: {url}");

    match download_and_extract_package(&url, &pkg_dir) {
        Ok(()) => Some(pkg_dir),
        Err(e) => {
            tracing::warn!("failed to download package {pkg}: {e}");
            None
        }
    }
}

fn download_and_extract_package(url: &str, dest: &Path) -> anyhow::Result<()> {
    let response = ureq::get(url).call()
        .map_err(|e| anyhow::anyhow!("HTTP request failed: {e}"))?;

    let reader = response.into_body().into_reader();
    let gz = flate2::read::GzDecoder::new(reader);
    let mut archive = tar::Archive::new(gz);

    std::fs::create_dir_all(dest)?;
    archive.unpack(dest)?;

    Ok(())
}

/// Render Typst source to HTML, resolving images from a repo directory.
pub fn render_typst_to_html_with_images(source: &str, repo_path: &Path) -> anyhow::Result<String> {
    let world = RenderWorld::with_repo(source, Some(repo_path));
    render_world(&world)
}

/// Render Typst source to HTML using Typst's native HTML export.
///
/// Math equations are automatically converted to MathML via the mathyml library.
pub fn render_typst_to_html(source: &str) -> anyhow::Result<String> {
    let world = RenderWorld::new(source);
    render_world(&world)
}

fn render_world(world: &RenderWorld) -> anyhow::Result<String> {

    let warned = typst::compile::<HtmlDocument>(&world);
    let document = warned.output.map_err(|diags| {
        let msgs: Vec<String> = diags.iter().map(|d| d.message.to_string()).collect();
        anyhow::anyhow!("Typst compilation errors: {}", msgs.join("; "))
    })?;

    let html = typst_html::html(&document).map_err(|diags| {
        let msgs: Vec<String> = diags.iter().map(|d| d.message.to_string()).collect();
        anyhow::anyhow!("Typst HTML export errors: {}", msgs.join("; "))
    })?;

    Ok(extract_body(&html))
}

/// Extract content between <body> and </body> tags.
/// Falls back to returning the full string if tags aren't found.
fn extract_body(html: &str) -> String {
    let start = html.find("<body>").map(|i| i + "<body>".len());
    let end = html.rfind("</body>");
    match (start, end) {
        (Some(s), Some(e)) => html[s..e].trim().to_string(),
        _ => html.to_string(),
    }
}

/// Render an entire series as a single typst document, then split the output
/// back into per-chapter HTML fragments. Cross-chapter references resolve
/// naturally because typst compiles the whole document.
///
/// `chapters` is a list of (article_uri, typst_source) in series order.
/// Render a Typst series to a per-chapter HTML map.
///
/// If `main.typ` exists in repo, compile it (user-maintained).
/// Otherwise, auto-concatenate chapter files in order, with bib discovery.
///
/// `chapter_ids` maps (article_uri, chapter_index) for splitting output.
pub fn render_series_to_html(
    chapter_ids: &[(String, usize)], // (article_uri, chapter_index)
    repo_path: &Path,
) -> anyhow::Result<HashMap<String, String>> {
    if chapter_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let main_path = repo_path.join("main.typ");
    let source = if main_path.exists() {
        // User-maintained main.typ
        std::fs::read_to_string(&main_path)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", main_path.display()))?
    } else {
        // Auto-concat: build virtual main.typ from chapter files
        build_auto_concat_source(chapter_ids, repo_path)?
    };

    let world = RenderWorld::with_preamble(&source, SERIES_PREAMBLE, Some(repo_path));
    let html = render_world(&world)?;

    split_series_html(&html, chapter_ids)
}

/// Render a Typst series and return the complete HTML (unsplit).
/// Used by the compile service for heading extraction.
pub fn render_series_full_html(repo_path: &Path) -> anyhow::Result<String> {
    let main_path = repo_path.join("main.typ");
    let source = if main_path.exists() {
        std::fs::read_to_string(&main_path)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", main_path.display()))?
    } else {
        // Auto-concat all .typ files in chapters/ sorted by name
        let mut chapter_files = Vec::new();
        let chapters_dir = repo_path.join("chapters");
        if chapters_dir.exists() {
            for entry in std::fs::read_dir(&chapters_dir)?.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".typ") {
                    chapter_files.push(name);
                }
            }
        }
        chapter_files.sort();
        build_auto_concat_source_from_files(&chapter_files, repo_path)?
    };

    let world = RenderWorld::with_preamble(&source, SERIES_PREAMBLE, Some(repo_path));
    render_world(&world)
}

/// Build a virtual main.typ by concatenating chapter files.
fn build_auto_concat_source(
    chapter_ids: &[(String, usize)],
    repo_path: &Path,
) -> anyhow::Result<String> {
    let chapters_dir = repo_path.join("chapters");

    // Find chapter files matching the IDs
    let mut files = Vec::new();
    for (uri, idx) in chapter_ids {
        let tid = uri.rsplit('/').next().unwrap_or("unknown");
        // Find file with this TID
        if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(tid) && name.ends_with(".typ") {
                    files.push((name, *idx));
                    break;
                }
            }
        }
    }
    files.sort_by_key(|(_, idx)| *idx);

    let file_names: Vec<String> = files.iter().map(|(name, _)| name.clone()).collect();
    build_auto_concat_source_from_files(&file_names, repo_path)
}

fn build_auto_concat_source_from_files(
    files: &[String],
    repo_path: &Path,
) -> anyhow::Result<String> {
    let mut source = String::new();

    for (i, name) in files.iter().enumerate() {
        source.push_str(&format!(
            "\n#html.elem(\"section\", attrs: (\"data-chapter\": \"{i}\"))[\n#include \"chapters/{name}\"\n]\n"
        ));
    }

    // Auto-discover .bib files in repo root
    let mut bib_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(repo_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".bib") {
                bib_files.push(name);
            }
        }
    }
    if !bib_files.is_empty() {
        if bib_files.len() == 1 {
            source.push_str(&format!("\n#bibliography(\"{}\")\n", bib_files[0]));
        } else {
            let args: Vec<String> = bib_files.iter().map(|f| format!("\"{f}\"")).collect();
            source.push_str(&format!("\n#bibliography(({}))\n", args.join(", ")));
        }
    }

    Ok(source)
}

/// Split compiled series HTML into per-chapter fragments by
/// finding `<section data-chapter="N">` markers.
fn split_series_html(
    html: &str,
    chapter_ids: &[(String, usize)],
) -> anyhow::Result<HashMap<String, String>> {
    let mut result = HashMap::new();

    for (uri, idx) in chapter_ids {
        let marker = format!("data-chapter=\"{idx}\"");

        if let Some(marker_pos) = html.find(&marker) {
            let content_start = match html[marker_pos..].find('>') {
                Some(offset) => marker_pos + offset + 1,
                None => continue,
            };

            let mut depth = 1i32;
            let mut pos = 0;
            let slice = &html[content_start..];
            while pos < slice.len() && depth > 0 {
                if slice[pos..].starts_with("<section") {
                    depth += 1;
                } else if slice[pos..].starts_with("</section>") {
                    depth -= 1;
                    if depth == 0 {
                        result.insert(uri.clone(), slice[..pos].trim().to_string());
                        break;
                    }
                }
                pos += 1;
            }
        }
    }

    for (uri, _) in chapter_ids {
        result.entry(uri.clone()).or_default();
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_heading() {
        let html = render_typst_to_html("= Hello\nSome *bold* text").unwrap();
        eprintln!("=== heading HTML ===\n{html}");
        assert!(html.contains("Hello"));
        assert!(html.contains("bold"));
        assert!(!html.contains("<!DOCTYPE"));
    }

    #[test]
    fn test_heading_structure() {
        let html = render_typst_to_html(
            "#set heading(numbering: \"1.1\")\n= Chapter One\nSome text.\n== Section 1.1\nMore text."
        ).unwrap();
        eprintln!("=== heading structure ===\n{html}");
        // Check what heading tags look like (id attributes, etc.)
    }

    #[test]
    fn test_render_math() {
        let html = render_typst_to_html("The formula $x^2 + y^2 = r^2$ is a circle.").unwrap();
        assert!(html.contains("<math"));
        assert!(html.contains("circle"));
    }

    #[test]
    fn test_render_block_math() {
        let html = render_typst_to_html("Display:\n$\nE = m c^2\n$").unwrap();
        assert!(html.contains("<math"));
    }

    #[test]
    fn test_theorem_classes() {
        let html = render_typst_to_html(r#"
#definition(name: "连续")[函数在某点连续。]
#theorem[如果$f$连续则$f$可积。]
#proof[显然。]
"#).unwrap();
        eprintln!("=== theorem output ===\n{html}");
        assert!(html.contains(r#"class="thm-block thm-defn""#), "missing defn class");
        assert!(html.contains(r#"class="thm-block thm-thm""#), "missing thm class");
        assert!(html.contains(r#"class="thm-block thm-proof""#), "missing proof class");
    }

    // Series render tests require a temp repo with main.typ — covered by integration tests.

    #[test]
    fn test_render_error() {
        let result = render_typst_to_html("#invalid-func()");
        assert!(result.is_err());
    }

    #[test]
    fn test_block_integral_mathml() {
        let cases = vec![
            ("full integral", "$ integral_a^b f(x) dif x $"),
            ("dif x", "$ dif x $"),
            ("bold x", "$ bold(x) $"),
            ("upright d", "$ upright(d) $"),
            ("bb R", "$ bb(R) $"),
            ("cal F", "$ cal(F) $"),
            ("display frac", "$ display(a/b) $"),
            ("lim", "$ lim_(n -> infinity) a_n $"),
        ];
        for (name, src) in cases {
            let html = render_typst_to_html(src).unwrap();
            let has_mathml = html.contains("<math display=\"block\"");
            let has_svg = html.contains("typst-frame");
            eprintln!("[{name}] MathML={has_mathml} SVG={has_svg}");
            if has_svg {
                // Extract a small snippet showing what was generated
                if let Some(i) = html.find("mathyml-block-center") {
                    eprintln!("  snippet: {}...", &html[i..html.len().min(i+120)]);
                }
            }
        }
    }

    #[test]
    fn test_block_sum_mathml() {
        let html = render_typst_to_html("$ sum_(i=1)^n a_i $").unwrap();
        eprintln!("=== sum output ===\n{html}");
        let has_mathml = html.contains("<math display=\"block\"");
        let has_svg = html.contains("typst-frame");
        eprintln!("MathML: {has_mathml}, SVG fallback: {has_svg}");
    }

    #[test]
    fn test_block_frac_mathml() {
        let html = render_typst_to_html("$ (a+b) / (c+d) $").unwrap();
        eprintln!("=== frac output ===\n{html}");
        let has_mathml = html.contains("<math display=\"block\"");
        let has_svg = html.contains("typst-frame");
        eprintln!("MathML: {has_mathml}, SVG fallback: {has_svg}");
    }

    #[test]
    fn test_footnote_html() {
        let src = r#"
This is some text with a footnote.#footnote[This is the footnote content.]

Another paragraph with another note.#footnote[Second footnote here.]
"#;
        let html = render_typst_to_html(src).unwrap();
        eprintln!("=== footnote output ===\n{html}");
    }

    #[test]
    fn test_block_aligned_mathml() {
        let html = render_typst_to_html("$ a &= b + c \\\\ &= d $").unwrap();
        eprintln!("=== aligned output ===\n{html}");
        let has_mathml = html.contains("<math display=\"block\"");
        let has_svg = html.contains("typst-frame");
        eprintln!("MathML: {has_mathml}, SVG fallback: {has_svg}");
    }
}
