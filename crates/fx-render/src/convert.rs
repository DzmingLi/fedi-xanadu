use std::io::Write;
use std::process::Command;

/// Convert content between formats using pandoc.
///
/// Supported formats: "markdown", "typst", "tex", "html".
/// Returns the converted source text (not rendered HTML).
pub fn convert_format(source: &str, from: &str, to: &str) -> anyhow::Result<String> {
    if from == to {
        return Ok(source.to_string());
    }

    let pandoc_from = to_pandoc_format(from)?;
    let pandoc_to = to_pandoc_format(to)?;

    let mut child = Command::new("pandoc")
        .args(["--from", pandoc_from, "--to", pandoc_to, "--wrap=none"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to launch pandoc: {e}"))?;

    child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("failed to open pandoc stdin"))?
        .write_all(source.as_bytes())?;

    let output = child
        .wait_with_output()
        .map_err(|e| anyhow::anyhow!("pandoc process error: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("pandoc conversion failed: {stderr}");
    }

    String::from_utf8(output.stdout)
        .map_err(|e| anyhow::anyhow!("pandoc output is not valid UTF-8: {e}"))
}

fn to_pandoc_format(fmt: &str) -> anyhow::Result<&'static str> {
    match fmt {
        "markdown" => Ok("markdown"),
        "typst" => Ok("typst"),
        "tex" => Ok("latex"),
        "html" => Ok("html"),
        _ => anyhow::bail!("unsupported format for conversion: {fmt}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_format_noop() {
        let src = "# Hello";
        let out = convert_format(src, "markdown", "markdown").unwrap();
        assert_eq!(out, src);
    }

    #[test]
    fn test_markdown_to_typst() {
        let out = convert_format("# Hello\n\n**bold** text", "markdown", "typst").unwrap();
        assert!(out.contains("Hello"));
        assert!(out.contains("bold"));
    }

    #[test]
    fn test_markdown_to_tex() {
        let out = convert_format("# Hello\n\n**bold** text", "markdown", "tex").unwrap();
        assert!(out.contains("Hello"));
        assert!(out.contains("bold") || out.contains("textbf"));
    }

    #[test]
    fn test_tex_to_typst() {
        let out = convert_format(r"\section{Hello}\textbf{bold}", "tex", "typst").unwrap();
        assert!(out.contains("Hello"));
    }
}
