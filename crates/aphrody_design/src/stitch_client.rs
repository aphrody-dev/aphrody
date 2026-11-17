//! Native Rust Google Stitch (`https://stitch.withgoogle.com/`) & `google-labs-code/stitch-sdk`
//! client, project/screen model, signed artifact downloader, and `DESIGN.md`-constrained
//! NSH TSX / GPUI code generator.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::design_md::DesignMdDocument;
use crate::http_mirror::{
    DesignSitemapSnapshot, StitchRouteEntry, fetch_text, post_json,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchProject {
    pub id: String,
    pub title: String,
    pub design_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchScreen {
    pub id: String,
    pub stitch_id: String,
    pub title: String,
    pub device_type: String,
    pub html_code: String,
    pub html_download_url: Option<String>,
    pub screenshot_download_url: Option<String>,
}

/// Client for Google Stitch REST / MCP backends (`STITCH_API_KEY` env override).
#[derive(Debug, Clone)]
pub struct StitchClient {
    pub base_url: String,
    pub api_key: Option<String>,
}

impl Default for StitchClient {
    fn default() -> Self {
        Self::new(
            std::env::var("STITCH_API_URL")
                .unwrap_or_else(|_| "https://stitch.withgoogle.com".to_owned()),
            std::env::var("STITCH_API_KEY").ok().filter(|s| !s.is_empty()),
        )
    }
}

impl StitchClient {
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key,
        }
    }

    pub fn has_live_credentials(&self) -> bool {
        self.api_key.is_some()
    }

    pub fn stitch_sitemap_routes() -> Vec<StitchRouteEntry> {
        DesignSitemapSnapshot::bundled()
            .map(|s| s.stitch_routes)
            .unwrap_or_default()
    }

    /// Generates a Stitch screen HTML/TSX layout anchored to `DESIGN.md` tokens and `data-stitch-id`.
    /// When `STITCH_API_KEY` is configured and `allow_remote` is true, submits the request to the
    /// configured Stitch API endpoint; otherwise synthesizes a deterministic Material 3 Expressive screen.
    pub fn synthesize_screen(
        &self,
        prompt: &str,
        device_type: &str,
        design_md: &DesignMdDocument,
        allow_remote: bool,
    ) -> Result<StitchScreen> {
        if allow_remote && let Some(ref api_key) = self.api_key {
            let endpoint = format!("{}/api/v1/screens:generate", self.base_url.trim_end_matches('/'));
            let payload = serde_json::json!({
                "prompt": prompt,
                "deviceType": device_type,
                "designMd": design_md.body_markdown,
            });
            match post_json(&endpoint, Some(("X-Goog-Api-Key", api_key)), &payload) {
                Ok(val) => {
                    if let Some(html) = val.get("html").and_then(serde_json::Value::as_str) {
                        return Ok(StitchScreen {
                            id: val
                                .get("id")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("stitch-remote")
                                .to_owned(),
                            stitch_id: "remote-screen".to_owned(),
                            title: prompt.to_owned(),
                            device_type: device_type.to_owned(),
                            html_code: html.to_owned(),
                            html_download_url: None,
                            screenshot_download_url: None,
                        });
                    }
                }
                Err(err) => log::warn!("Remote Stitch generation failed, using local synthesizer: {err}"),
            }
        }

        let slug: String = prompt
            .to_ascii_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .take(36)
            .collect();
        let stitch_id = format!("stitch-{}", slug.trim_matches('-'));
        let surface = design_md
            .colors
            .get("surface")
            .map(String::as_str)
            .unwrap_or("#0E141B");
        let on_surface = design_md
            .colors
            .get("on-surface")
            .map(String::as_str)
            .unwrap_or("#E0E6F0");
        let primary = design_md
            .colors
            .get("primary")
            .map(String::as_str)
            .unwrap_or("#8FD0FF");
        let container = design_md
            .colors
            .get("surface-container")
            .map(String::as_str)
            .unwrap_or("#1B222B");
        let font = if design_md.sans_flex_family.is_empty() {
            "Google Sans Flex"
        } else {
            design_md.sans_flex_family.as_str()
        };

        let html = format!(
            r#"<!doctype html>
<html lang="en" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{prompt} — Aphrody Stitch Studio</title>
  <style>
    :root {{
      --md-sys-color-surface: {surface};
      --md-sys-color-on-surface: {on_surface};
      --md-sys-color-primary: {primary};
      --md-sys-color-surface-container: {container};
      --md-ref-typeface-brand: "{font}", "Inter", system-ui, sans-serif;
    }}
    body {{ margin: 0; font-family: var(--md-ref-typeface-brand); background: var(--md-sys-color-surface); color: var(--md-sys-color-on-surface); }}
  </style>
</head>
<body>
  <main data-stitch-id="{stitch_id}" data-design-source="https://design.google/">
    <header>
      <h1 data-stitch-id="{stitch_id}-title">{prompt}</h1>
    </header>
    <section data-stitch-id="{stitch_id}-canvas">
      <p>Synthesized from DESIGN.md ({font}, {device_type})</p>
      <button type="button" data-stitch-id="{stitch_id}-action">Launch in GPUI / WebGPU</button>
    </section>
  </main>
</body>
</html>
"#
        );

        Ok(StitchScreen {
            id: stitch_id.clone(),
            stitch_id,
            title: prompt.to_owned(),
            device_type: device_type.to_owned(),
            html_code: html,
            html_download_url: None,
            screenshot_download_url: None,
        })
    }

    pub fn download_signed_html(&self, signed_url: &str) -> Result<String> {
        fetch_text(signed_url, None).context("downloading signed Stitch HTML asset")
    }
}

/// Converts a [`StitchScreen`] into NSH TSX preserving `data-stitch-id` attributes per `stitch-skills`.
pub fn stitch_screen_to_nsh_tsx(screen: &StitchScreen) -> String {
    let escaped_title = screen.title.replace('"', "'");
    format!(
        r#"import {{ mount, signal, ScrollView, Text, View }} from "nsh";

// Source of truth: https://design.google/ | Stitch ID: {stitch_id}
export function StitchGeneratedView() {{
  const activations = signal(0);

  return (
    <ScrollView class="flex-1 bg-background">
      <View class="p-6 gap-4" data-stitch-id="{stitch_id}">
        <View class="gap-1">
          <Text class="text-2xl font-bold text-foreground">
            {escaped_title}
          </Text>
          <Text class="text-sm text-muted-foreground">
            Google Design Material 3 Expressive • Google Sans Flex • {device}
          </Text>
        </View>
        <View
          role="button"
          data-stitch-id="{stitch_id}-action"
          onPress={{() => activations.update((n) => n + 1)}}
          class="rounded-full bg-primary px-5 py-2.5"
        >
          <Text class="text-sm font-semibold text-primary-foreground">
            Run Wasm/WebGPU Screen ({{activations()}})
          </Text>
        </View>
      </View>
    </ScrollView>
  );
}}

mount(<StitchGeneratedView />);
"#,
        stitch_id = screen.stitch_id,
        device = screen.device_type,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design_md::load_bundled_root_design_md;

    #[test]
    fn synthesizes_screen_and_emits_nsh_tsx_with_stitch_ids() -> Result<()> {
        let design_md = load_bundled_root_design_md()?;
        let client = StitchClient::new("https://stitch.withgoogle.com", None);
        let screen = client.synthesize_screen(
            "WebGPU Shader Studio",
            "DESKTOP",
            &design_md,
            false,
        )?;
        assert!(screen.html_code.contains("data-stitch-id="));
        assert!(screen.html_code.contains("https://design.google/"));
        let tsx = stitch_screen_to_nsh_tsx(&screen);
        assert!(tsx.contains(&screen.stitch_id));
        assert!(!StitchClient::stitch_sitemap_routes().is_empty());
        Ok(())
    }
}
