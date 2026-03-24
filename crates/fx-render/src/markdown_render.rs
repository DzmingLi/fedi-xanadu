use pulldown_cmark::{Options, Parser, Event, html};

/// Render Markdown with KaTeX math support to HTML.
///
/// Math delimiters: `$...$` for inline, `$$...$$` for display.
/// KaTeX is rendered server-side so the client only needs katex.min.css.
pub fn render_markdown_to_html(source: &str) -> anyhow::Result<String> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_MATH);

    let parser = Parser::new_ext(source, options);

    let inline_opts = katex::Opts::builder()
        .display_mode(false)
        .throw_on_error(false)
        .build()
        .unwrap();
    let display_opts = katex::Opts::builder()
        .display_mode(true)
        .throw_on_error(false)
        .build()
        .unwrap();

    // Transform math events into pre-rendered KaTeX HTML
    let events: Vec<Event<'_>> = parser.map(|event| match event {
        Event::InlineMath(math) => {
            match katex::render_with_opts(&math, &inline_opts) {
                Ok(rendered) => Event::Html(rendered.into()),
                Err(_) => {
                    let escaped = html_escape(&math);
                    Event::Html(format!(r#"<span class="katex-error">{escaped}</span>"#).into())
                }
            }
        }
        Event::DisplayMath(math) => {
            match katex::render_with_opts(&math, &display_opts) {
                Ok(rendered) => Event::Html(
                    format!(r#"<div class="katex-display-wrapper">{rendered}</div>"#).into()
                ),
                Err(_) => {
                    let escaped = html_escape(&math);
                    Event::Html(format!(r#"<div class="katex-error">{escaped}</div>"#).into())
                }
            }
        }
        other => other,
    }).collect();

    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());

    Ok(html_output)
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
    fn test_inline_math() {
        let html = render_markdown_to_html("The formula $x^2 + y^2 = r^2$ is a circle.").unwrap();
        assert!(html.contains("katex"));
        assert!(html.contains("x")); // rendered contains the math
    }

    #[test]
    fn test_display_math() {
        let html = render_markdown_to_html("Display:\n\n$$\nE = mc^2\n$$").unwrap();
        assert!(html.contains("katex-display-wrapper"));
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
