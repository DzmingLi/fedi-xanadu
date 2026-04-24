//! Shared CLI helpers: i18n parsing/display, douban page parsing, cover download.

use anyhow::{Context, Result, bail};
use std::path::PathBuf;

/// Parse an i18n input string. If it starts with `{`, parse as JSON object
/// `{"zh": "...", "en": "..."}`. Otherwise wrap as `{"en": "..."}`.
pub fn parse_i18n(s: &str) -> serde_json::Value {
    if s.starts_with('{') {
        serde_json::from_str(s).unwrap_or_else(|_| serde_json::json!({ "en": s }))
    } else {
        serde_json::json!({ "en": s })
    }
}

/// Render an i18n JSON value for CLI display. Accepts either a plain string
/// (returned as-is) or an object like `{"zh": "...", "en": "..."}` (picks
/// zh > en > first non-empty value).
pub fn i18n_display(value: &serde_json::Value) -> String {
    if let Some(s) = value.as_str() {
        return s.to_string();
    }
    if let Some(obj) = value.as_object() {
        for key in ["zh", "en"] {
            if let Some(s) = obj.get(key).and_then(|v| v.as_str()) {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
        }
        for (_, v) in obj {
            if let Some(s) = v.as_str() {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
        }
    }
    "?".to_string()
}

/// Metadata parsed from a douban book page.
#[derive(Debug, Default, Clone)]
pub struct DoubanBook {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub original_title: Option<String>,
    pub authors: Vec<String>,
    pub translators: Vec<String>,
    pub publisher: Option<String>,
    pub year: Option<String>,
    pub isbn: Option<String>,
    pub pages: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub douban_url: String,
}

/// Fetch + parse a douban book subject page.
pub async fn fetch_douban(url: &str) -> Result<DoubanBook> {
    let html = crate::client()
        .get(url)
        .header(reqwest::header::USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120 Safari/537.36")
        .send().await
        .with_context(|| format!("fetch {url}"))?
        .error_for_status()?
        .text().await?;
    parse_douban_html(&html, url)
}

fn parse_douban_html(html: &str, url: &str) -> Result<DoubanBook> {
    let mut book = DoubanBook { douban_url: url.to_string(), ..Default::default() };

    if let Some(title) = extract_meta(html, "og:title").or_else(|| extract_between(html, "<h1>", "</h1>")) {
        let cleaned = strip_tags(&title).trim().to_string();
        if !cleaned.is_empty() {
            book.title = Some(cleaned);
        }
    }
    if let Some(img) = extract_meta(html, "og:image") {
        book.cover_url = Some(img);
    }

    // The #info div holds metadata as a series of
    //   <span class="pl">label</span>: value<br/>
    // runs. Normalize the whole block to plain text by collapsing the
    // label span into its text, splitting on <br/>, then parsing each
    // resulting line by its trailing colon.
    let info_block = extract_between(html, r#"<div id="info""#, "</div>").unwrap_or_default();
    let normalized = info_block
        .replace("\r", "")
        .replace("\n", " ")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n");
    let lines_text = html_to_text(&normalized);

    for raw_line in lines_text.lines() {
        let line = raw_line.trim();
        if line.is_empty() { continue; }
        // Each line looks like "作者: Achim Klenke" or "ISBN: 9781447153603".
        // Split on the FIRST ':' or '：' (not subsequent ones, in case the
        // value itself contains a colon, e.g. a URL).
        let Some((label_raw, value_raw)) = split_first_colon(line) else { continue; };
        let label = label_raw.trim();
        let value = value_raw.trim();
        if value.is_empty() { continue; }
        match label {
            "副标题" => book.subtitle = Some(value.to_string()),
            "原作名" => book.original_title = Some(value.to_string()),
            "作者"   => book.authors = split_names(value),
            "译者"   => book.translators = split_names(value),
            "出版社" => book.publisher = Some(value.to_string()),
            "出版年" => book.year = Some(normalize_year(value)),
            "ISBN"   => book.isbn = Some(value.replace('-', "").trim().to_string()),
            "页数"   => book.pages = Some(value.to_string()),
            _ => {}
        }
    }

    if book.title.is_none() {
        bail!("could not parse title from douban page");
    }
    Ok(book)
}

fn split_first_colon(line: &str) -> Option<(&str, &str)> {
    // Accept ASCII ':' and fullwidth '：'. Find whichever comes first.
    let ascii = line.find(':');
    let full  = line.find('：');
    let idx = match (ascii, full) {
        (Some(a), Some(f)) => a.min(f),
        (Some(a), None) => a,
        (None, Some(f)) => f,
        (None, None) => return None,
    };
    let (left, right) = line.split_at(idx);
    // Advance past the colon character (ASCII=1 byte, fullwidth=3 bytes).
    let delim_len = if right.starts_with(':') { 1 } else { '：'.len_utf8() };
    Some((left, &right[delim_len..]))
}

fn split_names(s: &str) -> Vec<String> {
    s.split(['/', '、', ','])
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn normalize_year(s: &str) -> String {
    // douban formats: "2020", "2020-5", "2020-5-1", "2020 年 5 月"
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 4 { digits } else { s.trim().to_string() }
}

fn extract_meta(html: &str, property: &str) -> Option<String> {
    // Walk the HTML looking for <meta ...> tags in order. For each one,
    // if it carries the requested property= and a content= attr, return
    // its content. This avoids the "nearest content= to property=" trap
    // where a sibling meta's value leaks in (site_name bleeding into
    // og:image on douban, for example).
    let mut cursor = 0;
    while let Some(rel) = html[cursor..].find("<meta") {
        let start = cursor + rel;
        let end = html[start..].find('>').map(|i| start + i)?;
        let tag = &html[start..=end];
        cursor = end + 1;

        if !tag.contains(&format!(r#"property="{property}""#)) {
            continue;
        }
        let content_idx = tag.find(r#"content=""#)?;
        let after = &tag[content_idx + 9..];
        let close = after.find('"')?;
        return Some(after[..close].to_string());
    }
    None
}

fn extract_between(html: &str, start_needle: &str, end_needle: &str) -> Option<String> {
    let s = html.find(start_needle)?;
    let after_start = &html[s + start_needle.len()..];
    let gt = after_start.find('>')?;
    let body = &after_start[gt + 1..];
    let e = body.find(end_needle)?;
    Some(body[..e].to_string())
}

fn html_to_text(html: &str) -> String {
    let no_tags = strip_tags(html);
    no_tags.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

/// Download a URL to a tempfile and return the path. Sends a Referer
/// header since doubanio hotlink-blocks requests without one.
pub async fn download_to_tempfile(url: &str, referer: &str) -> Result<PathBuf> {
    let bytes = crate::client()
        .get(url)
        .header(reqwest::header::REFERER, referer)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
        .send().await
        .with_context(|| format!("fetch {url}"))?
        .error_for_status()?
        .bytes().await?;

    let suffix = url.rsplit('.').next().unwrap_or("jpg");
    let suffix = if suffix.len() > 5 { "jpg" } else { suffix };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut tmp = std::env::temp_dir();
    tmp.push(format!("nbt-cover-{}.{}", nanos, suffix));
    std::fs::write(&tmp, &bytes).with_context(|| format!("write {}", tmp.display()))?;
    Ok(tmp)
}
