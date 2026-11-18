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

/// Metadata descriptor for one of the 12 canonical cross-platform Stitch maquettes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchMaquetteSpec {
    pub id: String,
    pub stitch_id: String,
    pub surface: String,
    pub device_type: String,
    pub title: String,
    pub relative_html_path: String,
    pub gpui_surface: String,
}

/// Returns all 12 canonical Aphrody Labs, Website, Desktop, and Mobile maquette specifications.
#[must_use]
pub fn bundled_maquette_catalog() -> Vec<StitchMaquetteSpec> {
    let entries = [
        ("01-labs-ecosystem-architecture", "stitch-labs-architecture", "labs", "DESKTOP", "Aphrody Labs 10-Domain Ecosystem Architecture", "aphrody/docs/design/maquettes/01-labs-ecosystem-architecture.html", "aphrody_nsh::StudioMode::GoogleDesignMirror"),
        ("02-labs-roadmap-2026-2027", "stitch-labs-roadmap", "labs", "DESKTOP", "Aphrody Labs Roadmap (2026-Q4 to 2027-Q4)", "aphrody/docs/design/maquettes/02-labs-roadmap-2026-2027.html", "aphrody_ui::RoadmapHorizonCard"),
        ("03-web-landing-hero", "stitch-web-landing", "website", "DESKTOP", "aphrody.com — Website Landing & Live WebGPU Playground", "aphrody/docs/design/maquettes/03-web-landing-hero.html", "aphrody_desktop::examples::web_os"),
        ("04-web-docs-llms-portal", "stitch-web-docs", "website", "DESKTOP", "docs.aphrody.com — Fumadocs v16, Twoslash & llms.txt Portal", "aphrody/docs/design/maquettes/04-web-docs-llms-portal.html", "aphrody_nsh::StudioMode::GpuiStarterPack"),
        ("05-web-store-and-downloads", "stitch-web-store", "website", "DESKTOP", "releases.aphrody.com & Store — 22-Target Distribution Portal", "aphrody/docs/design/maquettes/05-web-store-and-downloads.html", "aphrody_store::AwesomeGpuiCatalog"),
        ("06-desktop-aphrody-ide", "stitch-desktop-ide", "desktop", "DESKTOP", "Desktop: Aphrody Studio IDE", "aphrody/docs/design/maquettes/06-desktop-aphrody-ide.html", "crates/zed + aphrody_nsh::DesignStitchStudioItem"),
        ("07-desktop-aphrody-os-cosmic", "stitch-desktop-cosmic-os", "desktop", "DESKTOP", "Desktop: aphrody-os COSMIC Compositor & Dual Craft Studio", "aphrody/docs/design/maquettes/07-desktop-aphrody-os-cosmic.html", "aphrody_desktop::OsDesktopShell"),
        ("08-desktop-agy-neural-session", "stitch-desktop-neural-agy", "desktop", "DESKTOP", "Desktop: Neural Expressive Home & Grounding Stack", "aphrody/docs/design/maquettes/08-desktop-agy-neural-session.html", "aphrody_ui::google_design_components"),
        ("09-desktop-browser-vpn-shield", "stitch-desktop-browser-vpn", "desktop", "DESKTOP", "Desktop: Aphrody Browser + Per-Tab Mullvad WireGuard VPN", "aphrody/docs/design/maquettes/09-desktop-browser-vpn-shield.html", "aphrody_browser::ChromiumBrowserView"),
        ("10-mobile-companion-home", "stitch-mobile-home", "mobile", "MOBILE", "Mobile: Companion Home & Agent", "aphrody/docs/design/maquettes/10-mobile-companion-home.html", "gpui_ios + nsh-platform-android"),
        ("11-mobile-code-diff-review", "stitch-mobile-diff", "mobile", "MOBILE", "Mobile: Code Diff Review & Bottom Sheet", "aphrody/docs/design/maquettes/11-mobile-code-diff-review.html", "nsh_base::DiffView"),
        ("12-mobile-pocket-craft-vpn", "stitch-mobile-craft-oled", "mobile", "TABLET", "Mobile / Tablet OLED Pitch: Pocket Craft & VPN", "aphrody/docs/design/maquettes/12-mobile-pocket-craft-vpn.html", "nsh-platform-ios + nsh-platform-android"),
    ];
    entries
        .into_iter()
        .map(|(id, stitch_id, surface, device_type, title, relative_html_path, gpui_surface)| {
            StitchMaquetteSpec {
                id: id.to_owned(),
                stitch_id: stitch_id.to_owned(),
                surface: surface.to_owned(),
                device_type: device_type.to_owned(),
                title: title.to_owned(),
                relative_html_path: relative_html_path.to_owned(),
                gpui_surface: gpui_surface.to_owned(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design_md::load_bundled_root_design_md;

    #[test]
    fn synthesizes_screen_and_emits_nsh_tsx_with_stitch_ids() -> Result<()> {
        let design_md = load_bundled_root_design_md()?;
        let client = StitchClient::new("https://stitch.withgoogle.com", None);
        for device in ["DESKTOP", "MOBILE", "TABLET"] {
            let screen = client.synthesize_screen(
                &format!("WebGPU Shader Studio {device}"),
                device,
                &design_md,
                false,
            )?;
            assert!(screen.html_code.contains("data-stitch-id="));
            assert!(screen.html_code.contains("https://design.google/"));
            let tsx = stitch_screen_to_nsh_tsx(&screen);
            assert!(tsx.contains(&screen.stitch_id));
            assert!(tsx.contains(device));
        }
        assert!(!StitchClient::stitch_sitemap_routes().is_empty());
        let maquettes = bundled_maquette_catalog();
        assert_eq!(maquettes.len(), 12);
        assert!(maquettes.iter().any(|m| m.surface == "labs"));
        assert!(maquettes.iter().any(|m| m.surface == "website"));
        assert!(maquettes.iter().any(|m| m.surface == "desktop"));
        assert!(maquettes.iter().any(|m| m.surface == "mobile"));
        Ok(())
    }
}
