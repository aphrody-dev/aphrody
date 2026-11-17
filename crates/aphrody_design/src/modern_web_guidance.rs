//! Pure-Rust Chrome `GoogleChrome/modern-web-guidance-src` Baseline & Coherence Auditor.

use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebGuidanceFinding {
    pub rule: String,
    pub severity: String,
    pub file: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebGuidanceAuditReport {
    pub ok: bool,
    pub files_checked: usize,
    pub findings: Vec<WebGuidanceFinding>,
}

/// Audits an HTML string against Chrome Modern Web Guidance Baseline rules.
pub fn audit_html_source(file_label: &str, html: &str) -> Vec<WebGuidanceFinding> {
    let lower = html.to_ascii_lowercase();
    let mut findings = Vec::new();

    if !lower.contains("<!doctype html>") {
        findings.push(WebGuidanceFinding {
            rule: "baseline/doctype".to_owned(),
            severity: "error".to_owned(),
            file: file_label.to_owned(),
            message: "Missing HTML5 `<!doctype html>` declaration".to_owned(),
        });
    }
    if !lower.contains("<meta name=\"viewport\"") && !lower.contains("<meta name='viewport'") {
        findings.push(WebGuidanceFinding {
            rule: "baseline/viewport".to_owned(),
            severity: "error".to_owned(),
            file: file_label.to_owned(),
            message: "Missing responsive `<meta name=\"viewport\">` tag".to_owned(),
        });
    }
    if !lower.contains("--md-sys-") && !lower.contains("--md-ref-") {
        findings.push(WebGuidanceFinding {
            rule: "design-google/css-custom-properties".to_owned(),
            severity: "warning".to_owned(),
            file: file_label.to_owned(),
            message: "Prefer M3 Expressive `--md-sys-*` CSS custom properties".to_owned(),
        });
    }
    if lower.contains("document.write(") {
        findings.push(WebGuidanceFinding {
            rule: "baseline/no-document-write".to_owned(),
            severity: "error".to_owned(),
            file: file_label.to_owned(),
            message: "Avoid synchronous `document.write()` which blocks HTML parser".to_owned(),
        });
    }

    findings
}

/// Recursively audits `.html` files under a template/project directory.
pub fn audit_directory(dir: &Path) -> Result<WebGuidanceAuditReport> {
    let mut files_checked = 0usize;
    let mut findings = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let entries = fs::read_dir(&current)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if name_str == "target" || name_str == "node_modules" || name_str.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("html") {
                files_checked += 1;
                let html = fs::read_to_string(&path)?;
                let rel = path
                    .strip_prefix(dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                findings.extend(audit_html_source(&rel, &html));
            }
        }
    }

    let ok = !findings.iter().any(|f| f.severity == "error");
    Ok(WebGuidanceAuditReport {
        ok,
        files_checked,
        findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design_md::load_bundled_root_design_md;
    use crate::stitch_client::StitchClient;

    #[test]
    fn synthesized_stitch_html_passes_modern_web_guidance_audit() -> Result<()> {
        let doc = load_bundled_root_design_md()?;
        let client = StitchClient::new("https://stitch.withgoogle.com", None);
        let screen = client.synthesize_screen("Baseline Test", "DESKTOP", &doc, false)?;
        let findings = audit_html_source("index.html", &screen.html_code);
        assert!(
            !findings.iter().any(|f| f.severity == "error"),
            "unexpected errors: {findings:?}"
        );
        Ok(())
    }
}
