use pulldown_cmark::{Options, Parser, Event, html};

/// Render Markdown with LaTeX math to HTML using MathML.
///
/// Math delimiters: `$...$` for inline, `$$...$$` for display.
/// LaTeX math is converted to MathML server-side for native browser rendering.
///
/// Pre-processes:
/// - Strips pandoc-style `{reference-type="..." reference="..."}` attributes from links
/// - Expands HoTT-book custom LaTeX macros in math blocks
pub fn render_markdown_to_html(source: &str) -> anyhow::Result<String> {
    // Pre-process: strip pandoc reference attributes
    let source = strip_pandoc_attributes(source);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_MATH);

    let parser = Parser::new_ext(&source, options);

    // Transform math events into MathML
    let events: Vec<Event<'_>> = parser.map(|event| match event {
        Event::InlineMath(math) => {
            let expanded = expand_hott_macros(&math);
            match latex2mathml::latex_to_mathml(&expanded, latex2mathml::DisplayStyle::Inline) {
                Ok(mathml) => Event::Html(mathml.into()),
                Err(_) => {
                    let escaped = html_escape(&math);
                    Event::Html(format!(r#"<code class="math-error">{escaped}</code>"#).into())
                }
            }
        }
        Event::DisplayMath(math) => {
            let expanded = expand_hott_macros(&math);
            match latex2mathml::latex_to_mathml(&expanded, latex2mathml::DisplayStyle::Block) {
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

/// Strip pandoc-style attribute spans from markdown text.
///
/// Converts patterns like:
///   `[text](#anchor){reference-type="ref" reference="anchor"}`
/// to:
///   `[text](#anchor)`
///
/// Also handles standalone attribute blocks like `{#id}` and `{.class}`.
fn strip_pandoc_attributes(source: &str) -> String {
    // First: strip `[]{#id label="..."}` empty anchor spans
    let source = strip_empty_label_spans(source);

    // Then: strip `]{...}` and `){...}` attribute blocks
    let mut result = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && (chars[i] == ']' || chars[i] == ')') && chars[i + 1] == '{' {
            result.push(chars[i]);
            i += 1;
            if i < len && chars[i] == '{' {
                let start = i;
                let mut depth = 1;
                i += 1;
                while i < len && depth > 0 {
                    if chars[i] == '{' { depth += 1; }
                    if chars[i] == '}' { depth -= 1; }
                    i += 1;
                }
                let block: String = chars[start..i].iter().collect();
                if block.contains("reference-type") || block.contains("label=") || block.starts_with("{#") {
                    // Skip pandoc attribute
                } else {
                    result.push_str(&block);
                }
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Remove `[]{#id label="..."}` patterns that pandoc generates for anchors.
fn strip_empty_label_spans(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Match `[]{#...}` — empty link text with attribute
        if i + 2 < len && chars[i] == '[' && chars[i + 1] == ']' && chars[i + 2] == '{' {
            // Skip []{...}
            i += 3; // skip `[]{`
            let mut depth = 1;
            while i < len && depth > 0 {
                if chars[i] == '{' { depth += 1; }
                if chars[i] == '}' { depth -= 1; }
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Expand HoTT-book custom LaTeX macros into standard LaTeX.
///
/// The HoTT book defines custom macros like \id, \opp, \idtype etc.
/// latex2mathml doesn't know these, so we expand them first.
fn expand_hott_macros(math: &str) -> String {
    let mut s = math.to_string();

    // \id[A]{x}{y} → \mathrm{id}_{A}(x, y)
    // \id[A]xy → \mathrm{id}_{A}(x, y) (single char args)
    s = expand_macro_with_opt_and_args(&s, "\\id", "\\mathrm{id}");

    // \idtype[A]{x}{y} → \mathrm{Id}_{A}(x, y)
    s = expand_macro_with_opt_and_args(&s, "\\idtype", "\\mathrm{Id}");

    // \opp{p} or \opp p → p^{-1}
    s = expand_opp_macro(&s);

    // Simple replacements
    s = s.replace("\\vcentcolon\\equiv", "\\coloneqq");
    s = s.replace("\\vcentcolon", ":");

    s
}

/// Expand a macro like `\id[A]{x}{y}` or `\id[A]xy` into `\mathrm{id}_{A}(x, y)`.
fn expand_macro_with_opt_and_args(s: &str, macro_name: &str, replacement: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let macro_chars: Vec<char> = macro_name.chars().collect();
    let len = chars.len();
    let mlen = macro_chars.len();
    let mut i = 0;

    while i < len {
        if i + mlen <= len && chars[i..i + mlen] == macro_chars[..] {
            // Check that the next char after macro name is not alphanumeric
            // (to avoid matching \identity when looking for \id)
            let after = if i + mlen < len { chars[i + mlen] } else { ' ' };
            if after.is_alphanumeric() && macro_name != "\\idtype" {
                result.push(chars[i]);
                i += 1;
                continue;
            }

            i += mlen;
            result.push_str(replacement);

            // Optional argument [A]
            if i < len && chars[i] == '[' {
                i += 1; // skip [
                let mut opt = String::new();
                while i < len && chars[i] != ']' {
                    opt.push(chars[i]);
                    i += 1;
                }
                if i < len { i += 1; } // skip ]
                result.push_str(&format!("_{{{opt}}}"));
            }

            // Collect up to 2 arguments (braced or single char)
            let mut args = Vec::new();
            for _ in 0..2 {
                // skip whitespace
                while i < len && chars[i] == ' ' { i += 1; }
                if i >= len { break; }

                if chars[i] == '{' {
                    i += 1;
                    let mut arg = String::new();
                    let mut depth = 1;
                    while i < len && depth > 0 {
                        if chars[i] == '{' { depth += 1; }
                        if chars[i] == '}' { depth -= 1; }
                        if depth > 0 { arg.push(chars[i]); }
                        i += 1;
                    }
                    args.push(arg);
                } else if chars[i].is_alphanumeric() || chars[i] == '\\' {
                    // Single character argument
                    args.push(chars[i].to_string());
                    i += 1;
                } else {
                    break;
                }
            }

            if !args.is_empty() {
                result.push('(');
                result.push_str(&args.join(", "));
                result.push(')');
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Expand `\opp{p}` or `\opp p` into `{p}^{-1}`.
fn expand_opp_macro(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let opp = "\\opp";
    let opp_chars: Vec<char> = opp.chars().collect();
    let olen = opp_chars.len();
    let mut i = 0;

    while i < len {
        if i + olen <= len && chars[i..i + olen] == opp_chars[..] {
            let after = if i + olen < len { chars[i + olen] } else { ' ' };
            if after.is_alphanumeric() {
                result.push(chars[i]);
                i += 1;
                continue;
            }

            i += olen;
            // skip whitespace
            while i < len && chars[i] == ' ' { i += 1; }

            let arg;
            if i < len && chars[i] == '{' {
                i += 1;
                let mut a = String::new();
                let mut depth = 1;
                while i < len && depth > 0 {
                    if chars[i] == '{' { depth += 1; }
                    if chars[i] == '}' { depth -= 1; }
                    if depth > 0 { a.push(chars[i]); }
                    i += 1;
                }
                arg = a;
            } else if i < len {
                arg = chars[i].to_string();
                i += 1;
            } else {
                arg = String::new();
            }

            result.push_str(&format!("{{{arg}}}^{{-1}}"));
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
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

    #[test]
    fn test_strip_pandoc_ref_attributes() {
        let input = r#"see [sec:foo](#sec:foo){reference-type="ref" reference="sec:foo"}"#;
        let stripped = strip_pandoc_attributes(input);
        assert_eq!(stripped, "see [sec:foo](#sec:foo)");
    }

    #[test]
    fn test_strip_pandoc_eqref() {
        let input = r#"[eq:Jconv](#eq:Jconv){reference-type="eqref" reference="eq:Jconv"}"#;
        let stripped = strip_pandoc_attributes(input);
        assert_eq!(stripped, "[eq:Jconv](#eq:Jconv)");
    }

    #[test]
    fn test_strip_empty_label_span() {
        let input = r#"**Lemma.**[]{#lem:opp label="lem:opp"} For every"#;
        let stripped = strip_pandoc_attributes(input);
        assert!(stripped.contains("**Lemma.** For every"));
    }

    #[test]
    fn test_expand_id_macro() {
        assert_eq!(expand_hott_macros(r"\id[A]xy"), r"\mathrm{id}_{A}(x, y)");
        assert_eq!(expand_hott_macros(r"\id[A]{x}{y}"), r"\mathrm{id}_{A}(x, y)");
    }

    #[test]
    fn test_expand_opp_macro() {
        assert_eq!(expand_hott_macros(r"\opp p"), r"{p}^{-1}");
        assert_eq!(expand_hott_macros(r"\opp{foo}"), r"{foo}^{-1}");
    }

    #[test]
    fn test_expand_idtype_macro() {
        assert_eq!(
            expand_hott_macros(r"\idtype[A]{x}{y}"),
            r"\mathrm{Id}_{A}(x, y)"
        );
    }
}
