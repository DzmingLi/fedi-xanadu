use std::collections::HashMap;
use std::path::{Path, PathBuf};

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt, World};
use typst_html::HtmlDocument;

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
        Self::with_repo(text, None)
    }

    fn with_repo(text: &str, repo_path: Option<&Path>) -> Self {
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
        let full_source = format!("{MATH_PREAMBLE}{text}");
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
        self.sources
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        // Return raw bytes for any known source file
        if let Some(s) = self.sources.get(&id) {
            return Ok(Bytes::new(s.text().as_bytes().to_vec()));
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
/// Returns a map from article_uri to rendered HTML.
pub fn render_series_to_html(chapters: &[(String, String)]) -> anyhow::Result<HashMap<String, String>> {
    if chapters.is_empty() {
        return Ok(HashMap::new());
    }

    // Build the virtual document: wrap each chapter in an html.elem div with a unique marker
    let mut full_source = String::new();
    for (i, (uri, source)) in chapters.iter().enumerate() {
        // Unique marker div wrapping each chapter
        full_source.push_str(&format!(
            "\n#html.elem(\"div\", attrs: (\"class\": \"fx-chapter\", \"data-uri\": \"{uri}\", \"data-idx\": \"{i}\"))[\n"
        ));
        full_source.push_str(source);
        full_source.push_str("\n]\n");
    }

    let world = RenderWorld::new(&full_source);
    let html = render_world(&world)?;

    split_chapters(&html, chapters)
}

/// Split compiled HTML into per-chapter fragments by finding the fx-chapter marker divs.
fn split_chapters(html: &str, chapters: &[(String, String)]) -> anyhow::Result<HashMap<String, String>> {
    let mut result = HashMap::new();

    for (i, (uri, _)) in chapters.iter().enumerate() {
        let marker = format!(r#"data-uri="{uri}" data-idx="{i}""#);
        // Also try the alternate attribute order
        let marker_alt = format!(r#"data-idx="{i}" data-uri="{uri}""#);

        let start_pos = html.find(&marker).or_else(|| html.find(&marker_alt));

        if let Some(marker_pos) = start_pos {
            // Find the opening > of this div
            let div_content_start = match html[marker_pos..].find('>') {
                Some(offset) => marker_pos + offset + 1,
                None => continue,
            };

            // Track div nesting to find the matching </div>
            let mut depth = 1i32;
            let content_bytes = html[div_content_start..].as_bytes();
            let mut pos = 0;
            while pos < content_bytes.len() && depth > 0 {
                if content_bytes[pos] == b'<' {
                    let rest = &html[div_content_start + pos..];
                    if rest.starts_with("<div") {
                        depth += 1;
                    } else if rest.starts_with("</div>") {
                        depth -= 1;
                        if depth == 0 {
                            let chapter_html = html[div_content_start..div_content_start + pos].trim();
                            result.insert(uri.clone(), chapter_html.to_string());
                            break;
                        }
                    }
                }
                pos += 1;
            }
        }
    }

    // For any chapters not found in the split, return empty string
    for (uri, _) in chapters {
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
        assert!(html.contains("Hello"));
        assert!(html.contains("bold"));
        assert!(!html.contains("<!DOCTYPE"));
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

    #[test]
    fn test_series_split() {
        let ch1 = (
            "at://did:local:test/article/ch1".to_string(),
            "= Chapter One\nFirst chapter content with $x^2$ math.\n".to_string(),
        );
        let ch2 = (
            "at://did:local:test/article/ch2".to_string(),
            "= Chapter Two\nSecond chapter about $y = f(x)$.\n".to_string(),
        );
        let result = render_series_to_html(&[ch1.clone(), ch2.clone()]).unwrap();
        eprintln!("=== ch1 ===\n{}", result.get(&ch1.0).unwrap());
        eprintln!("=== ch2 ===\n{}", result.get(&ch2.0).unwrap());

        let ch1_html = result.get(&ch1.0).unwrap();
        let ch2_html = result.get(&ch2.0).unwrap();
        assert!(ch1_html.contains("First chapter"), "ch1 should have its content");
        assert!(ch2_html.contains("Second chapter"), "ch2 should have its content");
        assert!(!ch1_html.contains("Second chapter"), "ch1 should not have ch2 content");
        assert!(!ch2_html.contains("First chapter"), "ch2 should not have ch1 content");
        // Both should have math
        assert!(ch1_html.contains("<math"), "ch1 should have MathML");
        assert!(ch2_html.contains("<math"), "ch2 should have MathML");
    }

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
