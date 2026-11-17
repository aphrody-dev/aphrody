//! Pure-Rust Core Web Vitals, Lighthouse 4-category scoring, W3C (HTML5/WCAG/WAI-ARIA/BCP47),
//! Google Style Guide, Vercel Web Interface, and shadcn/Base UI/Tailwind v4 audit engine.

use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::design_md::{DesignMdDocument, contrast_ratio_hex};

/// Authoritative Core Web Vitals (2026) & GPUI / WebGPU / Wasm budgets.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CoreWebVitalsBudgets {
    pub lcp_good_ms: u32,
    pub inp_good_ms: u32,
    pub cls_good_max: f32,
    pub fcp_good_ms: u32,
    pub ttfb_good_ms: u32,
    pub gpu_frame_60hz_ms: f32,
    pub gpu_frame_120hz_ms: f32,
    pub gpui_input_latency_ms: f32,
    pub wasm_critical_brotli_bytes: usize,
}

impl Default for CoreWebVitalsBudgets {
    fn default() -> Self {
        Self {
            lcp_good_ms: 2500,
            inp_good_ms: 200,
            cls_good_max: 0.10,
            fcp_good_ms: 1800,
            ttfb_good_ms: 800,
            gpu_frame_60hz_ms: 16.67,
            gpu_frame_120hz_ms: 8.33,
            gpui_input_latency_ms: 8.0,
            wasm_critical_brotli_bytes: 358_400,
        }
    }
}

/// Core Web Vitals static safety summary for an HTML/Wasm surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoreWebVitalsReport {
    pub lcp_non_blocking_ok: bool,
    pub inp_async_dispatch_ok: bool,
    pub cls_explicit_geometry_ok: bool,
    pub wasm_brotli_within_budget: bool,
    pub budgets: CoreWebVitalsBudgets,
}

/// Lighthouse 0..=100 scores mapped across Performance, Accessibility, Best Practices, and SEO/Agentic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LighthouseCategoryScores {
    pub performance: u8,
    pub accessibility: u8,
    pub best_practices: u8,
    pub seo_agentic: u8,
    pub overall: u8,
}

/// Individual Web / Style / A11y finding emitted by [`audit_html_standards`] or style scanners.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandardsFinding {
    pub category: String,
    pub rule: String,
    pub severity: String,
    pub file: String,
    pub line: usize,
    pub message: String,
}

/// Combined CWV, W3C, Google Style, Vercel UI, and Lighthouse report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LighthouseAuditReport {
    pub ok: bool,
    pub files_checked: usize,
    pub cwv: CoreWebVitalsReport,
    pub scores: LighthouseCategoryScores,
    pub findings: Vec<StandardsFinding>,
}

/// Canonical Base UI + shadcn + Tailwind v4 + GPUI primitive entry for AI agents and MCP queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarterPrimitiveSummary {
    pub slot: &'static str,
    pub headless_primitive: &'static str,
    pub gpui_component: &'static str,
    pub nsh_tsx_component: &'static str,
    pub tailwind_v4_recipe: &'static str,
    pub wai_aria_role: &'static str,
}

/// Returns the 16 canonical GPUI Starter Pack primitives shared by MCP tools and `aphrody_ui`.
#[must_use]
pub const fn starter_primitive_summaries() -> &'static [StarterPrimitiveSummary] {
    &[
        StarterPrimitiveSummary {
            slot: "button",
            headless_primitive: "nsh_base::button / gpui::InteractiveElement",
            gpui_component: "aphrody_ui::starter_pack::StarterButton",
            nsh_tsx_component: "Button",
            tailwind_v4_recipe: "inline-flex items-center justify-center gap-2 rounded-md bg-primary text-primary-foreground",
            wai_aria_role: "button",
        },
        StarterPrimitiveSummary {
            slot: "badge",
            headless_primitive: "gpui::div inline pill",
            gpui_component: "aphrody_ui::starter_pack::StarterBadge",
            nsh_tsx_component: "Badge",
            tailwind_v4_recipe: "inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium",
            wai_aria_role: "status",
        },
        StarterPrimitiveSummary {
            slot: "card",
            headless_primitive: "gpui::v_flex semantic container",
            gpui_component: "aphrody_ui::starter_pack::StarterCard",
            nsh_tsx_component: "Card",
            tailwind_v4_recipe: "rounded-xl border border-border bg-card text-card-foreground p-4",
            wai_aria_role: "region",
        },
        StarterPrimitiveSummary {
            slot: "kpi-card",
            headless_primitive: "gpui::v_flex tabular-nums metric card",
            gpui_component: "aphrody_ui::starter_pack::StarterKpiCard",
            nsh_tsx_component: "KpiCard",
            tailwind_v4_recipe: "rounded-xl border bg-card p-4 tabular-nums",
            wai_aria_role: "group",
        },
        StarterPrimitiveSummary {
            slot: "alert",
            headless_primitive: "ui::Banner / semantic surface",
            gpui_component: "aphrody_ui::starter_pack::StarterAlert",
            nsh_tsx_component: "Alert",
            tailwind_v4_recipe: "rounded-lg border border-border bg-muted px-4 py-3 text-sm",
            wai_aria_role: "alert",
        },
        StarterPrimitiveSummary {
            slot: "skeleton",
            headless_primitive: "CLS-safe reserved box",
            gpui_component: "aphrody_ui::starter_pack::StarterSkeleton",
            nsh_tsx_component: "Skeleton",
            tailwind_v4_recipe: "animate-pulse rounded-md bg-muted",
            wai_aria_role: "presentation",
        },
        StarterPrimitiveSummary {
            slot: "switch",
            headless_primitive: "nsh_base::switch / aphrody_ui::ToggleControl",
            gpui_component: "aphrody_ui::reactive::ToggleControl",
            nsh_tsx_component: "Switch",
            tailwind_v4_recipe: "inline-flex h-6 w-11 items-center rounded-full bg-input",
            wai_aria_role: "switch",
        },
        StarterPrimitiveSummary {
            slot: "slider",
            headless_primitive: "nsh_base::slider / aphrody_ui::SliderControl",
            gpui_component: "aphrody_ui::reactive::SliderControl",
            nsh_tsx_component: "Slider",
            tailwind_v4_recipe: "relative flex w-full touch-none items-center select-none",
            wai_aria_role: "slider",
        },
        StarterPrimitiveSummary {
            slot: "progress",
            headless_primitive: "nsh_base::progress / aphrody_ui::ProgressMeter",
            gpui_component: "aphrody_ui::reactive::ProgressMeter",
            nsh_tsx_component: "Progress",
            tailwind_v4_recipe: "h-2 w-full overflow-hidden rounded-full bg-secondary",
            wai_aria_role: "progressbar",
        },
        StarterPrimitiveSummary {
            slot: "dialog",
            headless_primitive: "nsh_base::alert_dialog / ui::Modal",
            gpui_component: "ui::Modal",
            nsh_tsx_component: "AlertDialog",
            tailwind_v4_recipe: "fixed inset-0 z-50 rounded-xl border bg-popover p-6",
            wai_aria_role: "dialog",
        },
        StarterPrimitiveSummary {
            slot: "popover",
            headless_primitive: "nsh_base::popover / ui::PopoverMenu",
            gpui_component: "ui::Popover",
            nsh_tsx_component: "Popover",
            tailwind_v4_recipe: "z-50 w-72 rounded-md border bg-popover p-4 text-popover-foreground",
            wai_aria_role: "dialog",
        },
        StarterPrimitiveSummary {
            slot: "select",
            headless_primitive: "nsh_base::select / nsh_base::listbox",
            gpui_component: "ui::DropdownMenu",
            nsh_tsx_component: "Select",
            tailwind_v4_recipe: "flex h-9 w-full items-center justify-between rounded-md border bg-background px-3",
            wai_aria_role: "combobox",
        },
        StarterPrimitiveSummary {
            slot: "tabs",
            headless_primitive: "nsh_base::tabs / ui::TabBar",
            gpui_component: "ui::TabBar",
            nsh_tsx_component: "Tabs",
            tailwind_v4_recipe: "inline-flex h-9 items-center rounded-lg bg-muted p-1",
            wai_aria_role: "tablist",
        },
        StarterPrimitiveSummary {
            slot: "data-table",
            headless_primitive: "nsh_base::data_table / gpui::UniformList",
            gpui_component: "ui::Table",
            nsh_tsx_component: "DataTable",
            tailwind_v4_recipe: "w-full caption-bottom text-sm tabular-nums",
            wai_aria_role: "table",
        },
        StarterPrimitiveSummary {
            slot: "navigation-rail",
            headless_primitive: "aphrody_ui::cosmic_shell::NavRail",
            gpui_component: "aphrody_ui::cosmic_shell::NavRail",
            nsh_tsx_component: "Sidebar",
            tailwind_v4_recipe: "flex flex-col gap-2 border-r border-border bg-card p-2",
            wai_aria_role: "navigation",
        },
        StarterPrimitiveSummary {
            slot: "tooltip",
            headless_primitive: "nsh_base::tooltip / ui::Tooltip",
            gpui_component: "ui::Tooltip",
            nsh_tsx_component: "Tooltip",
            tailwind_v4_recipe: "z-50 rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground",
            wai_aria_role: "tooltip",
        },
    ]
}

fn push_finding(
    findings: &mut Vec<StandardsFinding>,
    category: &str,
    rule: &str,
    severity: &str,
    file: &str,
    line: usize,
    message: impl Into<String>,
) {
    findings.push(StandardsFinding {
        category: category.to_owned(),
        rule: rule.to_owned(),
        severity: severity.to_owned(),
        file: file.to_owned(),
        line,
        message: message.into(),
    });
}

fn subtract_score(score: u8, penalty: u8) -> u8 {
    score.saturating_sub(penalty)
}

/// Audits an HTML document across Core Web Vitals, W3C, Google Style Guide, Vercel UI, and shadcn/M3 design tokens.
pub fn audit_html_standards(
    label: &str,
    html: &str,
    doc: &DesignMdDocument,
    wasm_brotli_bytes: Option<usize>,
) -> LighthouseAuditReport {
    let lower = html.to_ascii_lowercase();
    let budgets = CoreWebVitalsBudgets::default();
    let mut findings = Vec::new();
    let mut perf = 100u8;
    let mut a11y = 100u8;
    let mut best = 100u8;
    let mut seo = 100u8;

    let mut lcp_ok = true;
    let mut inp_ok = true;
    let mut cls_ok = true;

    let has_doctype = lower.contains("<!doctype html>");
    if !has_doctype {
        perf = subtract_score(perf, 10);
        best = subtract_score(best, 15);
        push_finding(
            &mut findings,
            "w3c/html5",
            "w3c/doctype-html5",
            "error",
            label,
            1,
            "Missing HTML5 `<!doctype html>` prevents standards mode layout",
        );
    }

    if !lower.contains("<meta charset=\"utf-8\"") && !lower.contains("<meta charset='utf-8'") {
        best = subtract_score(best, 15);
        a11y = subtract_score(a11y, 10);
        push_finding(
            &mut findings,
            "w3c/charmod",
            "w3c/meta-charset-utf8",
            "error",
            label,
            1,
            "Missing `<meta charset=\"utf-8\">` required by W3C CharMod-Norm",
        );
    }

    if !lower.contains("name=\"viewport\"") && !lower.contains("name='viewport'") {
        a11y = subtract_score(a11y, 15);
        perf = subtract_score(perf, 10);
        push_finding(
            &mut findings,
            "cwv/inp",
            "w3c/meta-viewport",
            "error",
            label,
            1,
            "Missing `<meta name=\"viewport\">` degrades mobile responsiveness and INP",
        );
    }

    let has_lang = lower.contains("<html") && (lower.contains(" lang=\"") || lower.contains(" lang='"));
    let has_dir = lower.contains("<html") && (lower.contains(" dir=\"") || lower.contains(" dir='"));
    if !has_lang {
        a11y = subtract_score(a11y, 10);
        seo = subtract_score(seo, 10);
        push_finding(
            &mut findings,
            "w3c/ltli",
            "w3c/html-lang",
            "error",
            label,
            1,
            "Missing BCP 47 `<html lang=\"...\">` attribute",
        );
    }
    if !has_dir {
        a11y = subtract_score(a11y, 5);
        push_finding(
            &mut findings,
            "w3c/string-meta",
            "w3c/html-dir",
            "warn",
            label,
            1,
            "Prefer explicit `<html dir=\"ltr|rtl|auto\">` for W3C bidi metadata",
        );
    }

    if !lower.contains("<title>") {
        seo = subtract_score(seo, 15);
        a11y = subtract_score(a11y, 5);
        push_finding(
            &mut findings,
            "lighthouse/seo",
            "w3c/document-title",
            "error",
            label,
            1,
            "Missing `<title>` element",
        );
    }

    if !lower.contains("name=\"description\"") && !lower.contains("name='description'") {
        seo = subtract_score(seo, 5);
        push_finding(
            &mut findings,
            "lighthouse/seo",
            "lighthouse/meta-description",
            "warn",
            label,
            1,
            "Missing `<meta name=\"description\">` for SEO and agent discovery",
        );
    }

    if !lower.contains("<main") {
        a11y = subtract_score(a11y, 15);
        push_finding(
            &mut findings,
            "w3c/wai-aria",
            "w3c/landmark-main",
            "error",
            label,
            1,
            "Missing semantic `<main>` landmark for screen readers and AccessKit",
        );
    }

    if lower.contains("document.write(") {
        lcp_ok = false;
        inp_ok = false;
        perf = subtract_score(perf, 25);
        best = subtract_score(best, 15);
        push_finding(
            &mut findings,
            "cwv/lcp",
            "cwv/no-document-write",
            "error",
            label,
            1,
            "Avoid synchronous `document.write()` blocking parser and LCP",
        );
    }

    if lower.contains("@import") {
        lcp_ok = false;
        perf = subtract_score(perf, 15);
        push_finding(
            &mut findings,
            "cwv/lcp",
            "cwv/no-render-blocking-css-import",
            "error",
            label,
            1,
            "Avoid render-blocking `@import` in `<style>`",
        );
    }

    // Synchronous external script tags without async/defer/type="module" stall FCP/LCP/INP.
    let has_blocking_script = lower
        .split("<script ")
        .skip(1)
        .any(|tag| {
            let head = tag.split('>').next().unwrap_or("");
            head.contains("src=")
                && !head.contains("defer")
                && !head.contains("async")
                && !head.contains("type=\"module\"")
                && !head.contains("type='module'")
        });
    if has_blocking_script {
        lcp_ok = false;
        inp_ok = false;
        perf = subtract_score(perf, 15);
        push_finding(
            &mut findings,
            "cwv/lcp",
            "cwv/non-blocking-module-script",
            "error",
            label,
            1,
            "External `<script src=...>` must use `type=\"module\"`, `defer`, or `async`",
        );
    }

    // CLS prevention: `<canvas>` or `<img>` must have explicit dimensions or fixed viewport CSS.
    if lower.contains("<canvas")
        && !((lower.contains("width=") && lower.contains("height="))
            || lower.contains("aspect-ratio")
            || lower.contains("inset: 0")
            || lower.contains("inset:0"))
    {
        cls_ok = false;
        perf = subtract_score(perf, 20);
        push_finding(
            &mut findings,
            "cwv/cls",
            "cwv/canvas-explicit-geometry",
            "error",
            label,
            1,
            "`<canvas>` should declare explicit `width`/`height` attributes or `aspect-ratio` to keep CLS <= 0.10",
        );
    }

    // Check WCAG 2.2 AA contrast on DESIGN.md foreground pairs.
    if let (Some(fg), Some(bg)) = (doc.colors.get("on-surface"), doc.colors.get("surface"))
        && let Some(ratio) = contrast_ratio_hex(fg, bg)
        && ratio < 4.5
    {
        a11y = subtract_score(a11y, 20);
        push_finding(
            &mut findings,
            "w3c/wcag",
            "w3c/wcag-aa-contrast",
            "error",
            label,
            1,
            format!("Surface contrast ratio {ratio:.2}:1 is below WCAG 2.2 AA 4.5:1"),
        );
    }

    if !lower.contains(":focus-visible") {
        a11y = subtract_score(a11y, 5);
        push_finding(
            &mut findings,
            "vercel/ui",
            "vercel/focus-visible-ring",
            "warn",
            label,
            1,
            "Include explicit `:focus-visible` keyboard focus styles per Vercel Web Interface guidelines",
        );
    }

    if (lower.contains("outline: none") || lower.contains("outline:none")) && !lower.contains(":focus-visible") {
        a11y = subtract_score(a11y, 15);
        push_finding(
            &mut findings,
            "vercel/ui",
            "vercel/never-remove-focus-outline",
            "error",
            label,
            1,
            "Never remove focus outline without a `:focus-visible` replacement",
        );
    }

    if !lower.contains("prefers-reduced-motion") {
        a11y = subtract_score(a11y, 3);
        push_finding(
            &mut findings,
            "vercel/ui",
            "vercel/prefers-reduced-motion",
            "warn",
            label,
            1,
            "Respect `@media (prefers-reduced-motion: reduce)` for WebGPU/CSS motion",
        );
    }

    if !lower.contains("--md-sys-") && !lower.contains("--background") {
        best = subtract_score(best, 5);
        push_finding(
            &mut findings,
            "google-style/css",
            "design/css-tokens",
            "warn",
            label,
            1,
            "Use `--md-sys-*` / shadcn `--background` CSS tokens anchored to https://design.google/",
        );
    }

    if !lower.contains("data-slot=") && !lower.contains("data-stitch-id=") {
        seo = subtract_score(seo, 5);
        push_finding(
            &mut findings,
            "agentic/ui",
            "agent/data-slot-or-stitch-id",
            "warn",
            label,
            1,
            "Annotate primary interactive regions with `data-slot` or `data-stitch-id` for deterministic agent locators",
        );
    }

    let wasm_brotli_within_budget = match wasm_brotli_bytes {
        Some(bytes) if bytes > budgets.wasm_critical_brotli_bytes => {
            perf = subtract_score(perf, 15);
            push_finding(
                &mut findings,
                "cwv/wasm-budget",
                "cwv/wasm-brotli-budget",
                "warn",
                label,
                1,
                format!(
                    "Compressed Wasm ({bytes} B) exceeds critical initial budget ({} B)",
                    budgets.wasm_critical_brotli_bytes
                ),
            );
            false
        }
        _ => true,
    };

    let overall = ((u16::from(perf) + u16::from(a11y) + u16::from(best) + u16::from(seo)) / 4) as u8;
    let has_errors = findings.iter().any(|f| f.severity == "error");

    LighthouseAuditReport {
        ok: !has_errors && overall >= 90,
        files_checked: 1,
        cwv: CoreWebVitalsReport {
            lcp_non_blocking_ok: lcp_ok,
            inp_async_dispatch_ok: inp_ok,
            cls_explicit_geometry_ok: cls_ok,
            wasm_brotli_within_budget,
            budgets,
        },
        scores: LighthouseCategoryScores {
            performance: perf,
            accessibility: a11y,
            best_practices: best,
            seo_agentic: seo,
            overall,
        },
        findings,
    }
}

/// Recursively audits all `.html` files under `dir` (skipping build artifacts) and aggregates minimum category scores.
pub fn audit_project_standards(dir: &Path, doc: &DesignMdDocument) -> Result<LighthouseAuditReport> {
    let mut files_checked = 0usize;
    let mut min_scores = LighthouseCategoryScores {
        performance: 100,
        accessibility: 100,
        best_practices: 100,
        seo_agentic: 100,
        overall: 100,
    };
    let mut findings = Vec::new();
    let mut lcp_ok = true;
    let mut inp_ok = true;
    let mut cls_ok = true;
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
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
                let rep = audit_html_standards(&rel, &html, doc, None);
                min_scores.performance = min_scores.performance.min(rep.scores.performance);
                min_scores.accessibility = min_scores.accessibility.min(rep.scores.accessibility);
                min_scores.best_practices = min_scores.best_practices.min(rep.scores.best_practices);
                min_scores.seo_agentic = min_scores.seo_agentic.min(rep.scores.seo_agentic);
                lcp_ok &= rep.cwv.lcp_non_blocking_ok;
                inp_ok &= rep.cwv.inp_async_dispatch_ok;
                cls_ok &= rep.cwv.cls_explicit_geometry_ok;
                findings.extend(rep.findings);
            }
        }
    }

    min_scores.overall = ((u16::from(min_scores.performance)
        + u16::from(min_scores.accessibility)
        + u16::from(min_scores.best_practices)
        + u16::from(min_scores.seo_agentic))
        / 4) as u8;
    let has_errors = findings.iter().any(|f| f.severity == "error");

    Ok(LighthouseAuditReport {
        ok: !has_errors && min_scores.overall >= 90,
        files_checked,
        cwv: CoreWebVitalsReport {
            lcp_non_blocking_ok: lcp_ok,
            inp_async_dispatch_ok: inp_ok,
            cls_explicit_geometry_ok: cls_ok,
            wasm_brotli_within_budget: true,
            budgets: CoreWebVitalsBudgets::default(),
        },
        scores: min_scores,
        findings,
    })
}

/// Checks a Rust source snippet against Aphrody / Google Rust style invariants (outside `#[cfg(test)]`).
#[must_use]
pub fn audit_rust_source_style(label: &str, src: &str) -> Vec<StandardsFinding> {
    let mut findings = Vec::new();
    let prod = src.split("#[cfg(test)]").next().unwrap_or(src);
    if label.ends_with("mod.rs") {
        push_finding(
            &mut findings,
            "google-style/rust",
            "aphrody-rust/no-mod-rs",
            "error",
            label,
            1,
            "Avoid `mod.rs`; use `<module>.rs`",
        );
    }
    for (idx, line) in prod.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed.contains(".unwrap()") {
            push_finding(
                &mut findings,
                "google-style/rust",
                "aphrody-rust/no-prod-unwrap",
                "error",
                label,
                idx + 1,
                "Avoid `.unwrap()` in production Rust; return `Result` with `?`",
            );
        }
        if trimmed.starts_with("let _ =") || trimmed.contains(" let _ =") || trimmed.contains("{let _ =") {
            push_finding(
                &mut findings,
                "google-style/rust",
                "aphrody-rust/no-let-underscore",
                "warn",
                label,
                idx + 1,
                "Handle or log errors explicitly instead of `let _ =`",
            );
        }
    }
    findings
}

/// Checks TypeScript/TSX source against Google TS Style Guide + Bun-native baseline.
#[must_use]
pub fn audit_ts_source_style(label: &str, src: &str) -> Vec<StandardsFinding> {
    let mut findings = Vec::new();
    for (idx, line) in src.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if line.contains("child_process") || line.contains("node-fetch") {
            push_finding(
                &mut findings,
                "google-style/ts",
                "bun-native/no-legacy-node",
                "error",
                label,
                idx + 1,
                "Prefer `Bun.spawn`, `Bun.spawnSync`, and `fetch` over Node/legacy wrappers",
            );
        }
        if line.contains(": any") || line.contains("as any") {
            push_finding(
                &mut findings,
                "google-style/ts",
                "google-ts/no-explicit-any",
                "warn",
                label,
                idx + 1,
                "Replace explicit `any` with narrow types or `unknown`",
            );
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design_md::{export_shadcn_tailwind_css, load_bundled_root_design_md};

    #[test]
    fn compliant_html5_with_shadcn_tokens_scores_95_plus_across_all_lighthouse_categories() -> Result<()> {
        let doc = load_bundled_root_design_md()?;
        let css = export_shadcn_tailwind_css(&doc);
        let html = format!(
            r#"<!doctype html>
<html lang="en" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="description" content="Aphrody GPUI + WebGPU Starter Pack">
  <title>Aphrody Starter Pack</title>
  <style>
    {css}
    :focus-visible {{ outline: 2px solid var(--ring); outline-offset: 2px; }}
    @media (prefers-reduced-motion: reduce) {{ * {{ animation: none !important; }} }}
  </style>
</head>
<body>
  <main data-slot="app-shell">
    <canvas width="1280" height="720" aria-label="WebGPU viewport"></canvas>
    <button type="button" data-stitch-id="run-btn">Run</button>
  </main>
</body>
</html>
"#
        );

        let report = audit_html_standards("starter.html", &html, &doc, Some(96_000));
        assert!(report.ok, "findings: {:?}", report.findings);
        assert!(report.scores.performance >= 95);
        assert!(report.scores.accessibility >= 95);
        assert!(report.scores.best_practices >= 95);
        assert!(report.scores.seo_agentic >= 95);
        assert!(report.scores.overall >= 95);
        assert_eq!(starter_primitive_summaries().len(), 16);
        Ok(())
    }

    #[test]
    fn malformed_html_ts_and_rust_trigger_expected_findings() -> Result<()> {
        let doc = load_bundled_root_design_md()?;
        let bad_html = "<html><body><script src=\"app.js\"></script><canvas></canvas></body></html>";
        let report = audit_html_standards("bad.html", bad_html, &doc, Some(500_000));
        assert!(!report.ok);
        assert!(!report.cwv.lcp_non_blocking_ok);
        assert!(!report.cwv.cls_explicit_geometry_ok);
        assert!(!report.cwv.wasm_brotli_within_budget);

        let rust_findings = audit_rust_source_style("src/mod.rs", "fn run() { let _ = x(); y.unwrap(); }\n");
        assert!(rust_findings.iter().any(|f| f.rule == "aphrody-rust/no-mod-rs"));
        assert!(rust_findings.iter().any(|f| f.rule == "aphrody-rust/no-prod-unwrap"));
        assert!(rust_findings.iter().any(|f| f.rule == "aphrody-rust/no-let-underscore"));

        let ts_findings = audit_ts_source_style("bad.ts", "import { spawn } from 'child_process'; const x: any = 1;\n");
        assert!(ts_findings.iter().any(|f| f.rule == "bun-native/no-legacy-node"));
        assert!(ts_findings.iter().any(|f| f.rule == "google-ts/no-explicit-any"));
        Ok(())
    }
}
