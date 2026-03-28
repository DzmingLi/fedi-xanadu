use std::io::Write;
use std::process::Command;

/// Render LaTeX source to HTML via pandoc.
///
/// Uses pandoc with `--mathml` for math rendering (consistent with Typst's MathML output).
/// Expects pandoc to be available in PATH.
pub fn render_tex_to_html(source: &str) -> anyhow::Result<String> {
    let mut child = Command::new("pandoc")
        .args([
            "--from", "latex",
            "--to", "html5",
            "--mathml",
            "--no-highlight",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to launch pandoc (is it installed?): {e}"))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_latex() {
        let html = render_tex_to_html(r"\section{Hello}\textbf{bold} text").unwrap();
        assert!(html.contains("Hello"));
        assert!(html.contains("bold"));
    }

    #[test]
    fn test_inline_math() {
        let html = render_tex_to_html(r"The formula $x^2 + y^2 = r^2$ is a circle.").unwrap();
        assert!(html.contains("<math"));
    }

    #[test]
    fn test_display_math() {
        let html = render_tex_to_html(r"\[ E = mc^2 \]").unwrap();
        assert!(html.contains("<math"));
    }

    #[test]
    fn test_environments() {
        let src = r"\begin{enumerate}
\item First
\item Second
\end{enumerate}";
        let html = render_tex_to_html(src).unwrap();
        assert!(html.contains("<li>"));
    }
}
