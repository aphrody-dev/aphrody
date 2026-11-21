//! `google-labs-code/design.md` specification parser, linter, diff tool, and
//! multi-target token exporter (W3C DTCG JSON, CSS `--md-sys-*`, Tailwind, WGSL).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::BUNDLED_ROOT_DESIGN_MD;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignMdDocument {
    pub name: String,
    pub source_of_truth: String,
    pub system: String,
    pub stitch_url: String,
    pub sans_flex_family: String,
    pub colors: BTreeMap<String, String>,
    pub typography_raw: BTreeMap<String, serde_yaml::Value>,
    pub rounded: BTreeMap<String, String>,
    pub spacing: BTreeMap<String, String>,
    pub body_markdown: String,
    pub headings: Vec<String>,
    pub yaml_blocks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignMdLintReport {
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub color_count: usize,
    pub contrast_ratio_primary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignMdDiff {
    pub added_colors: Vec<String>,
    pub removed_colors: Vec<String>,
    pub changed_colors: Vec<(String, String, String)>,
}

/// Parses a `DESIGN.md` document following `google-labs-code/design.md` YAML frontmatter rules.
pub fn parse_design_md(raw: &str) -> Result<DesignMdDocument> {
    let trimmed = raw.trim_start_matches('\u{feff}');
    let Some(after_open) = trimmed.strip_prefix("---") else {
        bail!("DESIGN.md must start with YAML frontmatter `---`");
    };
    let after_open = after_open
        .strip_prefix("\r\n")
        .or_else(|| after_open.strip_prefix('\n'))
        .unwrap_or(after_open);
    let end_idx = after_open
        .find("\n---")
        .or_else(|| after_open.find("\r\n---"))
        .context("DESIGN.md frontmatter missing closing `---` delimiter")?;

    let yaml_src = &after_open[..end_idx];
    let rest = &after_open[end_idx..];
    let body_start = rest.find('\n').map_or(rest.len(), |idx| idx + 1);
    let body = rest[body_start..].trim_start_matches(['\r', '\n']);

    let front: serde_yaml::Mapping =
        serde_yaml::from_str(yaml_src).context("parsing DESIGN.md YAML frontmatter")?;

    let get_str = |key: &str| -> String {
        front
            .get(serde_yaml::Value::String(key.to_owned()))
            .and_then(serde_yaml::Value::as_str)
            .unwrap_or("")
            .to_owned()
    };

    let mut colors = BTreeMap::new();
    if let Some(serde_yaml::Value::Mapping(map)) =
        front.get(serde_yaml::Value::String("colors".to_owned()))
    {
        for (k, v) in map {
            if let (Some(ks), Some(vs)) = (k.as_str(), v.as_str()) {
                colors.insert(ks.to_owned(), vs.to_owned());
            }
        }
    }

    let mut typography_raw = BTreeMap::new();
    if let Some(serde_yaml::Value::Mapping(map)) =
        front.get(serde_yaml::Value::String("typography".to_owned()))
    {
        for (k, v) in map {
            if let Some(ks) = k.as_str() {
                typography_raw.insert(ks.to_owned(), v.clone());
            }
        }
    }

    let read_str_map = |section: &str| -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        if let Some(serde_yaml::Value::Mapping(map)) =
            front.get(serde_yaml::Value::String(section.to_owned()))
        {
            for (k, v) in map {
                if let Some(ks) = k.as_str() {
                    let rendered = match v {
                        serde_yaml::Value::String(s) => s.clone(),
                        serde_yaml::Value::Number(n) => n.to_string(),
                        _ => String::new(),
                    };
                    if !rendered.is_empty() {
                        out.insert(ks.to_owned(), rendered);
                    }
                }
            }
        }
        out
    };

    let mut headings = Vec::new();
    let mut yaml_blocks = 0usize;
    for line in body.lines() {
        let t = line.trim();
        if let Some(h) = t.strip_prefix("# ") {
            headings.push(h.trim().to_owned());
        } else if let Some(h) = t.strip_prefix("## ") {
            headings.push(h.trim().to_owned());
        } else if t.starts_with("```yaml") {
            yaml_blocks += 1;
        }
    }

    Ok(DesignMdDocument {
        name: get_str("name"),
        source_of_truth: get_str("source_of_truth"),
        system: get_str("system"),
        stitch_url: get_str("stitch_url"),
        sans_flex_family: get_str("sans_flex_family"),
        colors,
        typography_raw,
        rounded: read_str_map("rounded"),
        spacing: read_str_map("spacing"),
        body_markdown: body.to_owned(),
        headings,
        yaml_blocks,
    })
}

pub fn load_bundled_root_design_md() -> Result<DesignMdDocument> {
    parse_design_md(BUNDLED_ROOT_DESIGN_MD)
}

pub fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    brand::design::parse_hex_rgb(hex)
}

pub fn contrast_ratio_hex(fg: &str, bg: &str) -> Option<f32> {
    brand::design::contrast_ratio_hex(fg, bg)
}

pub fn lint_design_md(doc: &DesignMdDocument) -> DesignMdLintReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if doc.name.is_empty() {
        errors.push("missing frontmatter `name`".to_owned());
    }
    if doc.source_of_truth != "https://design.google/" {
        errors.push("`source_of_truth` must be `https://design.google/`".to_owned());
    }
    for required_role in ["surface", "on-surface", "primary", "on-primary", "background"] {
        if !doc.colors.contains_key(required_role) {
            errors.push(format!("missing required color token `{required_role}`"));
        }
    }

    let primary_ratio = match (doc.colors.get("on-primary"), doc.colors.get("primary")) {
        (Some(fg), Some(bg)) => match contrast_ratio_hex(fg, bg) {
            Some(ratio) => {
                if ratio < 4.5 {
                    errors.push(format!(
                        "primary button WCAG contrast {ratio:.2} < 4.5 ({fg} on {bg})"
                    ));
                }
                format!("{ratio:.2}:1")
            }
            None => {
                errors.push("invalid hex in `primary`/`on-primary`".to_owned());
                "invalid".to_owned()
            }
        },
        _ => "n/a".to_owned(),
    };

    if let (Some(fg), Some(bg)) = (doc.colors.get("on-surface"), doc.colors.get("surface"))
        && let Some(ratio) = contrast_ratio_hex(fg, bg)
        && ratio < 4.5
    {
        errors.push(format!(
            "surface text WCAG contrast {ratio:.2} < 4.5 ({fg} on {bg})"
        ));
    }

    if !doc.sans_flex_family.contains("Google Sans") {
        warnings.push("expected Google Sans Flex typeface pairing".to_owned());
    }

    DesignMdLintReport {
        ok: errors.is_empty(),
        errors,
        warnings,
        color_count: doc.colors.len(),
        contrast_ratio_primary: primary_ratio,
    }
}

pub fn diff_design_md(before: &DesignMdDocument, after: &DesignMdDocument) -> DesignMdDiff {
    let added_colors = after
        .colors
        .keys()
        .filter(|k| !before.colors.contains_key(*k))
        .cloned()
        .collect();
    let removed_colors = before
        .colors
        .keys()
        .filter(|k| !after.colors.contains_key(*k))
        .cloned()
        .collect();
    let mut changed_colors = Vec::new();
    for (k, v1) in &before.colors {
        if let Some(v2) = after.colors.get(k)
            && v1 != v2
        {
            changed_colors.push((k.clone(), v1.clone(), v2.clone()));
        }
    }
    DesignMdDiff {
        added_colors,
        removed_colors,
        changed_colors,
    }
}

/// Exports CSS custom properties (`--md-sys-color-*`, `--md-sys-shape-*`) & Google Sans Flex font variables.
pub fn export_css_variables(doc: &DesignMdDocument) -> String {
    let mut out = String::from(":root {\n");
    if !doc.sans_flex_family.is_empty() {
        if let Err(err) = writeln!(
            out,
            "  --md-ref-typeface-brand: \"{}\", \"Inter\", system-ui, sans-serif;",
            doc.sans_flex_family
        ) {
            log::warn!("format err: {err}");
        }
    }
    for (name, value) in &doc.colors {
        if let Err(err) = writeln!(out, "  --md-sys-color-{name}: {value};") {
            log::warn!("format err: {err}");
        }
    }
    for (name, value) in &doc.rounded {
        if let Err(err) = writeln!(out, "  --md-sys-shape-{name}: {value};") {
            log::warn!("format err: {err}");
        }
    }
    out.push_str("}\n");
    out
}

/// Exports Tailwind v4 `@theme inline` + shadcn/Base UI semantic tokens mapped from `DESIGN.md`.
pub fn export_shadcn_tailwind_css(doc: &DesignMdDocument) -> String {
    let pick = |key: &str, fallback: &str| -> String {
        doc.colors
            .get(key)
            .cloned()
            .unwrap_or_else(|| fallback.to_owned())
    };

    let bg_light = pick("light-surface", "#FAF9F9");
    let fg_light = pick("light-on-surface", "#161C24");
    let primary_light = pick("light-primary", "#006399");
    let on_primary_light = pick("light-on-primary", "#FFFFFF");

    let bg_dark = pick("surface", "#131314");
    let fg_dark = pick("on-surface", "#E0E6F0");
    let card_dark = pick("surface-container-low", "#161D25");
    let popover_dark = pick("surface-container-high", "#252C36");
    let primary_dark = pick("primary", "#A8C7FA");
    let on_primary_dark = pick("on-primary", "#003352");
    let secondary_dark = pick("secondary-container", "#34485F");
    let on_secondary_dark = pick("on-secondary-container", "#D2E4FF");
    let muted_dark = pick("surface-container", "#1B222B");
    let muted_fg_dark = pick("on-surface-variant", "#C0CAD7");
    let accent_dark = pick("tertiary-container", "#434077");
    let on_accent_dark = pick("on-tertiary-container", "#E2DFFF");
    let destructive_dark = pick("error", "#FFB4AB");
    let on_destructive_dark = pick("on-error", "#690005");
    let border_dark = pick("outline-variant", "#404956");
    let ring_dark = pick("primary", "#A8C7FA");
    let radius_md = doc
        .rounded
        .get("md")
        .map(String::as_str)
        .unwrap_or("12px");
    let font_sans = if doc.sans_flex_family.is_empty() {
        "Google Sans Flex"
    } else {
        doc.sans_flex_family.as_str()
    };

    format!(
        r#"/* Derived from {source} via aphrody_design (Tailwind v4 + shadcn/Base UI) */
@theme inline {{
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --color-card: var(--card);
  --color-card-foreground: var(--card-foreground);
  --color-popover: var(--popover);
  --color-popover-foreground: var(--popover-foreground);
  --color-primary: var(--primary);
  --color-primary-foreground: var(--primary-foreground);
  --color-secondary: var(--secondary);
  --color-secondary-foreground: var(--secondary-foreground);
  --color-muted: var(--muted);
  --color-muted-foreground: var(--muted-foreground);
  --color-accent: var(--accent);
  --color-accent-foreground: var(--accent-foreground);
  --color-destructive: var(--destructive);
  --color-destructive-foreground: var(--destructive-foreground);
  --color-border: var(--border);
  --color-input: var(--input);
  --color-ring: var(--ring);
  --radius-md: var(--radius);
  --font-sans: "{font_sans}", "Inter", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", "Roboto Mono", monospace;
}}

:root {{
  --radius: {radius_md};
  --background: {bg_light};
  --foreground: {fg_light};
  --card: #FFFFFF;
  --card-foreground: {fg_light};
  --popover: #FFFFFF;
  --popover-foreground: {fg_light};
  --primary: {primary_light};
  --primary-foreground: {on_primary_light};
  --secondary: #D2E4FF;
  --secondary-foreground: #34485F;
  --muted: #E7EDF6;
  --muted-foreground: #3D4754;
  --accent: #E2DFFF;
  --accent-foreground: #434077;
  --destructive: #BA1A1A;
  --destructive-foreground: #FFFFFF;
  --border: #BEC7D4;
  --input: #BEC7D4;
  --ring: {primary_light};
}}

.dark {{
  --background: {bg_dark};
  --foreground: {fg_dark};
  --card: {card_dark};
  --card-foreground: {fg_dark};
  --popover: {popover_dark};
  --popover-foreground: {fg_dark};
  --primary: {primary_dark};
  --primary-foreground: {on_primary_dark};
  --secondary: {secondary_dark};
  --secondary-foreground: {on_secondary_dark};
  --muted: {muted_dark};
  --muted-foreground: {muted_fg_dark};
  --accent: {accent_dark};
  --accent-foreground: {on_accent_dark};
  --destructive: {destructive_dark};
  --destructive-foreground: {on_destructive_dark};
  --border: {border_dark};
  --input: {border_dark};
  --ring: {ring_dark};
}}
"#,
        source = doc.source_of_truth,
    )
}

/// Emits a shadcn/Base UI compatible `components.json` schema bound to Material 3 Expressive tokens.
pub fn export_shadcn_components_json(doc: &DesignMdDocument) -> serde_json::Value {
    json!({
        "$schema": "https://ui.shadcn.com/schema.json",
        "style": "google-expressive",
        "baseLibrary": "base-ui",
        "rsc": false,
        "tsx": true,
        "tailwind": {
            "css": "styles/shadcn-tailwind.css",
            "baseColor": "slate",
            "cssVariables": true
        },
        "iconLibrary": "lucide-gpui",
        "designSource": doc.source_of_truth,
        "aliases": {
            "components": "@/components",
            "utils": "@/lib/utils",
            "ui": "@/components/ui",
            "gpui": "aphrody_ui::starter_pack"
        }
    })
}

/// Exports W3C Design Token JSON (`design_tokens.json` compatible with `google-labs-code/design.md`).
pub fn export_w3c_tokens_json(doc: &DesignMdDocument) -> serde_json::Value {
    let mut color_group = serde_json::Map::new();
    for (name, value) in &doc.colors {
        color_group.insert(
            name.clone(),
            json!({ "$type": "color", "$value": value }),
        );
    }
    json!({
        "$schema": "https://www.designtokens.org/tr/2025.10/format/",
        "$source": doc.source_of_truth,
        "name": doc.name,
        "system": doc.system,
        "color": color_group,
        "radius": doc.rounded,
        "spacing": doc.spacing,
    })
}

/// Returns a 16-byte uniform-aligned WGSL theme struct parameterized by M3 Expressive tokens.
pub fn export_wgsl_theme_struct(doc: &DesignMdDocument) -> String {
    let hex_to_vec4 = |key: &str, fallback: [f32; 4]| -> [f32; 4] {
        doc.colors
            .get(key)
            .and_then(|hex| parse_hex_rgb(hex))
            .map_or(fallback, |(r, g, b)| {
                [
                    f32::from(r) / 255.0,
                    f32::from(g) / 255.0,
                    f32::from(b) / 255.0,
                    1.0,
                ]
            })
    };
    let surface = hex_to_vec4("surface", [0.055, 0.078, 0.106, 1.0]);
    let primary = hex_to_vec4("primary", [0.561, 0.816, 1.0, 1.0]);
    let tertiary = hex_to_vec4("tertiary", [0.776, 0.761, 1.0, 1.0]);
    let on_surface = hex_to_vec4("on-surface", [0.878, 0.902, 0.941, 1.0]);

    format!(
        r#"// Auto-derived from https://design.google/ (DESIGN.md -> WGSL Uniform)
struct GoogleDesignTheme {{
    surface: vec4<f32>,
    primary: vec4<f32>,
    tertiary: vec4<f32>,
    on_surface: vec4<f32>,
}}

fn default_google_design_theme() -> GoogleDesignTheme {{
    return GoogleDesignTheme(
        vec4<f32>({:.4}, {:.4}, {:.4}, {:.4}),
        vec4<f32>({:.4}, {:.4}, {:.4}, {:.4}),
        vec4<f32>({:.4}, {:.4}, {:.4}, {:.4}),
        vec4<f32>({:.4}, {:.4}, {:.4}, {:.4}),
    );
}}
"#,
        surface[0],
        surface[1],
        surface[2],
        surface[3],
        primary[0],
        primary[1],
        primary[2],
        primary[3],
        tertiary[0],
        tertiary[1],
        tertiary[2],
        tertiary[3],
        on_surface[0],
        on_surface[1],
        on_surface[2],
        on_surface[3],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_root_design_md_parses_lints_and_exports() -> Result<()> {
        let doc = load_bundled_root_design_md()?;
        let lint = lint_design_md(&doc);
        assert!(lint.ok, "lint errors: {:?}", lint.errors);
        assert!(lint.color_count >= 20);
        let css = export_css_variables(&doc);
        assert!(css.contains("--md-sys-color-primary:"));
        assert!(css.contains("Google Sans Flex"));
        let w3c = export_w3c_tokens_json(&doc);
        assert_eq!(w3c["$source"], "https://design.google/");
        let wgsl = export_wgsl_theme_struct(&doc);
        assert!(wgsl.contains("struct GoogleDesignTheme"));
        let shadcn_css = export_shadcn_tailwind_css(&doc);
        assert!(shadcn_css.contains("@theme inline"));
        assert!(shadcn_css.contains("--color-background: var(--background);"));
        let components = export_shadcn_components_json(&doc);
        assert_eq!(components["style"], "google-expressive");
        assert_eq!(components["tailwind"]["css"], "styles/shadcn-tailwind.css");
        Ok(())
    }

    #[test]
    fn diffs_changed_design_md_colors() -> Result<()> {
        let doc_a = load_bundled_root_design_md()?;
        let mut doc_b = doc_a.clone();
        doc_b
            .colors
            .insert("primary".to_owned(), "#1A73E8".to_owned());
        let diff = diff_design_md(&doc_a, &doc_b);
        assert_eq!(diff.changed_colors.len(), 1);
        assert_eq!(diff.changed_colors[0].0, "primary");
        Ok(())
    }
}
