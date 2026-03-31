use pulldown_cmark::{Options, Parser, Event, html};

/// Render Markdown with LaTeX math to HTML using MathML.
///
/// Math delimiters: `$...$` for inline, `$$...$$` for display.
/// LaTeX math is converted to MathML server-side for native browser rendering.
pub fn render_markdown_to_html(source: &str) -> anyhow::Result<String> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_MATH);

    let parser = Parser::new_ext(source, options);

    // Transform math events into MathML
    let events: Vec<Event<'_>> = parser.map(|event| match event {
        Event::InlineMath(math) => {
            match latex2mathml::latex_to_mathml(&math, latex2mathml::DisplayStyle::Inline) {
                Ok(mathml) => Event::Html(mathml.into()),
                Err(_) => {
                    let escaped = html_escape(&math);
                    Event::Html(format!(r#"<code class="math-error">{escaped}</code>"#).into())
                }
            }
        }
        Event::DisplayMath(math) => {
            match latex2mathml::latex_to_mathml(&math, latex2mathml::DisplayStyle::Block) {
                Ok(mathml) => Event::Html(mathml.into()),
                Err(_) => {
                    let escaped = html_escape(&math);
                    Event::Html(format!(r#"<div class="math-error"><code>{escaped}</code></div>"#).into())
                }
            }
        }
        other => other,
    }).collect();

    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());

    Ok(html_output)
}

/// Render a markdown series by concatenating all chapters.
/// Each chapter is rendered individually and the results are joined.
/// Returns the full HTML.
pub fn render_markdown_series(chapters: &[(String, String)]) -> anyhow::Result<String> {
    let mut full_html = String::new();
    for (_uri, source) in chapters {
        let chapter_html = render_markdown_to_html(source)?;
        full_html.push_str(&chapter_html);
        full_html.push('\n');
    }
    Ok(full_html)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_markdown() {
        let html = render_markdown_to_html("# Hello\n\nSome **bold** text").unwrap();
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_inline_math_mathml() {
        let html = render_markdown_to_html("The formula $x^2 + y^2 = r^2$ is a circle.").unwrap();
        assert!(html.contains("<math"));
        assert!(html.contains("</math>"));
    }

    #[test]
    fn test_display_math_mathml() {
        let html = render_markdown_to_html("Display:\n\n$$\nE = mc^2\n$$").unwrap();
        assert!(html.contains("<math"));
        assert!(html.contains(r#"display="block""#));
    }

    #[test]
    fn test_code_block() {
        let html = render_markdown_to_html("```rust\nfn main() {}\n```").unwrap();
        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn test_table() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |";
        let html = render_markdown_to_html(md).unwrap();
        assert!(html.contains("<table>"));
    }
}
