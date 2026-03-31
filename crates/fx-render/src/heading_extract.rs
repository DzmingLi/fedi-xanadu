//! Extract headings from rendered HTML and split into article slices.

/// A heading found in rendered HTML.
#[derive(Debug, Clone)]
pub struct Heading {
    pub level: u32,
    pub title: String,
    /// Auto-generated anchor: `section-{order}` (0-indexed).
    pub anchor: String,
    /// Byte offset of the `<hN` tag in the HTML.
    pub byte_offset: usize,
}

/// A slice of HTML corresponding to one article (from one heading to the next at the same or higher level).
#[derive(Debug, Clone)]
pub struct HtmlSlice {
    pub heading_title: String,
    pub heading_anchor: String,
    pub html: String,
    pub sub_headings: Vec<Heading>,
}

/// A node in the heading tree (for TOC rendering).
#[derive(Debug, Clone)]
pub struct HeadingNode {
    pub level: u32,
    pub title: String,
    pub anchor: String,
    pub children: Vec<HeadingNode>,
}

/// Extract all headings from HTML. Handles both:
/// - Typst output: `<h2>1 Chapter Title</h2>` (no id attribute)
/// - Markdown output: `<h1 id="chapter-title">Chapter Title</h1>` (has id)
///
/// All headings get a stable positional anchor `section-{N}`.
pub fn extract_headings(html: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    let mut search_from = 0;

    while search_from < html.len() {
        let remaining = &html[search_from..];

        // Find next <h1> through <h6>
        let Some(h_pos) = find_heading_tag(remaining) else {
            break;
        };
        let abs_offset = search_from + h_pos;
        let tag_str = &html[abs_offset..];

        // Parse level from <hN
        let level = match tag_str.as_bytes().get(2) {
            Some(b) if b.is_ascii_digit() => (*b - b'0') as u32,
            _ => { search_from = abs_offset + 1; continue; }
        };

        // Find the closing >
        let Some(close_bracket) = tag_str.find('>') else {
            search_from = abs_offset + 1;
            continue;
        };

        // Find the closing </hN>
        let close_tag = format!("</h{level}>");
        let content_start = close_bracket + 1;
        let Some(close_pos) = tag_str[content_start..].find(&close_tag) else {
            search_from = abs_offset + content_start;
            continue;
        };

        let title_html = &tag_str[content_start..content_start + close_pos];
        let title = strip_html_tags(title_html).trim().to_string();

        let anchor = format!("section-{}", headings.len());

        headings.push(Heading {
            level,
            title,
            anchor,
            byte_offset: abs_offset,
        });

        search_from = abs_offset + content_start + close_pos + close_tag.len();
    }

    headings
}

/// Split HTML at headings of the given level. Each slice spans from one
/// split-level heading to the next (or end of document).
pub fn split_at_level(html: &str, headings: &[Heading], split_level: u32) -> Vec<HtmlSlice> {
    let split_points: Vec<usize> = headings
        .iter()
        .enumerate()
        .filter(|(_, h)| h.level == split_level)
        .map(|(i, _)| i)
        .collect();

    if split_points.is_empty() {
        return vec![];
    }

    let mut slices = Vec::new();

    for (idx, &heading_idx) in split_points.iter().enumerate() {
        let start = headings[heading_idx].byte_offset;
        let end = if idx + 1 < split_points.len() {
            headings[split_points[idx + 1]].byte_offset
        } else {
            html.len()
        };

        let slice_html = html[start..end].trim().to_string();
        let heading = &headings[heading_idx];

        // Collect sub-headings within this slice
        let next_split = if idx + 1 < split_points.len() {
            split_points[idx + 1]
        } else {
            headings.len()
        };
        let sub_headings: Vec<Heading> = headings[heading_idx + 1..next_split]
            .iter()
            .filter(|h| h.level > split_level)
            .cloned()
            .collect();

        slices.push(HtmlSlice {
            heading_title: heading.title.clone(),
            heading_anchor: heading.anchor.clone(),
            html: slice_html,
            sub_headings,
        });
    }

    slices
}

/// Build a nested heading tree from a flat list (for TOC).
pub fn build_heading_tree(headings: &[Heading]) -> Vec<HeadingNode> {
    let mut root: Vec<HeadingNode> = Vec::new();
    let mut stack: Vec<(u32, usize)> = Vec::new(); // (level, index into parent's children)

    for h in headings {
        let node = HeadingNode {
            level: h.level,
            title: h.title.clone(),
            anchor: h.anchor.clone(),
            children: Vec::new(),
        };

        // Pop stack until we find a parent with a lower level
        while let Some(&(lvl, _)) = stack.last() {
            if lvl >= h.level {
                stack.pop();
            } else {
                break;
            }
        }

        if stack.is_empty() {
            let idx = root.len();
            root.push(node);
            stack.push((h.level, idx));
        } else {
            // Navigate to the correct parent
            let parent = get_node_mut(&mut root, &stack);
            let idx = parent.children.len();
            parent.children.push(node);
            stack.push((h.level, idx));
        }
    }

    root
}

fn get_node_mut<'a>(root: &'a mut Vec<HeadingNode>, stack: &[(u32, usize)]) -> &'a mut HeadingNode {
    let mut current = &mut root[stack[0].1];
    for &(_, idx) in &stack[1..] {
        current = &mut current.children[idx];
    }
    current
}

/// Find the byte offset of the next `<h[1-6]` tag.
fn find_heading_tag(html: &str) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if bytes[i] == b'<'
            && bytes[i + 1] == b'h'
            && bytes[i + 2].is_ascii_digit()
            && (1..=6).contains(&(bytes[i + 2] - b'0'))
            && (bytes[i + 3] == b'>' || bytes[i + 3] == b' ')
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Strip HTML tags from a string, returning plain text.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_headings_typst() {
        let html = r#"<h2>1 Introduction</h2>
<p>Some text.</p>
<h3>1.1 Background</h3>
<p>More text.</p>
<h2>2 Methods</h2>
<p>Method text.</p>"#;

        let headings = extract_headings(html);
        assert_eq!(headings.len(), 3);
        assert_eq!(headings[0].level, 2);
        assert_eq!(headings[0].title, "1 Introduction");
        assert_eq!(headings[0].anchor, "section-0");
        assert_eq!(headings[1].level, 3);
        assert_eq!(headings[1].title, "1.1 Background");
        assert_eq!(headings[2].level, 2);
        assert_eq!(headings[2].title, "2 Methods");
    }

    #[test]
    fn test_extract_headings_markdown() {
        let html = r#"<h1 id="intro">Introduction</h1>
<p>Text.</p>
<h2 id="bg">Background</h2>
<p>More.</p>"#;

        let headings = extract_headings(html);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].title, "Introduction");
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].title, "Background");
    }

    #[test]
    fn test_split_at_level() {
        let html = r#"<h2>1 Chapter One</h2>
<p>Content 1.</p>
<h3>1.1 Section A</h3>
<p>Section A text.</p>
<h2>2 Chapter Two</h2>
<p>Content 2.</p>"#;

        let headings = extract_headings(html);
        let slices = split_at_level(html, &headings, 2);

        assert_eq!(slices.len(), 2);
        assert_eq!(slices[0].heading_title, "1 Chapter One");
        assert!(slices[0].html.contains("Content 1"));
        assert!(slices[0].html.contains("Section A"));
        assert_eq!(slices[0].sub_headings.len(), 1);

        assert_eq!(slices[1].heading_title, "2 Chapter Two");
        assert!(slices[1].html.contains("Content 2"));
        assert!(!slices[1].html.contains("Content 1"));
    }

    #[test]
    fn test_build_heading_tree() {
        let headings = vec![
            Heading { level: 1, title: "Ch1".into(), anchor: "s-0".into(), byte_offset: 0 },
            Heading { level: 2, title: "Sec1.1".into(), anchor: "s-1".into(), byte_offset: 10 },
            Heading { level: 2, title: "Sec1.2".into(), anchor: "s-2".into(), byte_offset: 20 },
            Heading { level: 1, title: "Ch2".into(), anchor: "s-3".into(), byte_offset: 30 },
            Heading { level: 2, title: "Sec2.1".into(), anchor: "s-4".into(), byte_offset: 40 },
        ];

        let tree = build_heading_tree(&headings);
        assert_eq!(tree.len(), 2); // Ch1, Ch2
        assert_eq!(tree[0].children.len(), 2); // Sec1.1, Sec1.2
        assert_eq!(tree[1].children.len(), 1); // Sec2.1
        assert_eq!(tree[0].children[0].title, "Sec1.1");
    }
}
