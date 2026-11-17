use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default embedded ArtCraft + Craft Anything studio manifest (`assets/os/craft_catalog.toml`).
pub const BUNDLED_CATALOG_TOML: &str = include_str!("../../../assets/os/craft_catalog.toml");

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("failed to parse craft studio catalog TOML: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDefaults {
    pub default_names: Vec<String>,
    pub default_layout: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftAppSpec {
    pub id: String,
    pub name: String,
    pub tagline: String,
    pub category: String,
    pub icon: String,
    #[serde(default)]
    pub upstream: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub sniff_families: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub inspector_groups: Vec<String>,
}

impl CraftAppSpec {
    #[must_use]
    pub fn supports_extension(&self, ext: &str) -> bool {
        let lower = ext.trim_start_matches('.').to_ascii_lowercase();
        self.extensions.iter().any(|e| e == &lower)
    }
}

/// Declarative studio catalog replacing per-app hardcoded match trees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftCatalog {
    pub schema_version: u32,
    pub workspaces: WorkspaceDefaults,
    pub apps: Vec<CraftAppSpec>,
}

impl CraftCatalog {
    pub fn bundled() -> Result<Self, CatalogError> {
        Self::from_toml_str(BUNDLED_CATALOG_TOML)
    }

    pub fn from_toml_str(raw: &str) -> Result<Self, CatalogError> {
        Ok(toml::from_str(raw)?)
    }

    /// Loads bundled catalog and merges `$XDG_CONFIG_HOME/aphrody/craft_catalog.toml` overrides when present.
    pub fn load_with_user_override() -> Result<Self, CatalogError> {
        let mut base = Self::bundled()?;
        if let Some(cfg_dir) = dirs::config_dir() {
            let user_path = cfg_dir.join(brand::APP_SLUG).join("craft_catalog.toml");
            if user_path.is_file()
                && let Ok(text) = std::fs::read_to_string(&user_path)
                && let Ok(extra) = Self::from_toml_str(&text)
            {
                for app in extra.apps {
                    if let Some(existing) = base.apps.iter_mut().find(|a| a.id == app.id) {
                        *existing = app;
                    } else {
                        base.apps.push(app);
                    }
                }
            }
        }
        Ok(base)
    }

    #[must_use]
    pub fn by_id(&self, id: &str) -> Option<&CraftAppSpec> {
        self.apps
            .iter()
            .find(|app| app.id.eq_ignore_ascii_case(id.trim()))
    }

    /// Resolves the best studio application for an inspection family and file path.
    #[must_use]
    pub fn resolve_for_file(
        &self,
        path: &Path,
        sniff_family: Option<&str>,
    ) -> Option<&CraftAppSpec> {
        if let Some(ext) = path.extension().and_then(|s| s.to_str())
            && let Some(matched) = self.apps.iter().find(|app| app.supports_extension(ext))
        {
            return Some(matched);
        }
        if let Some(fam) = sniff_family {
            return self.apps.iter().find(|app| {
                app.sniff_families
                    .iter()
                    .any(|f| f.eq_ignore_ascii_case(fam))
            });
        }
        None
    }

    #[must_use]
    pub fn categories(&self) -> Vec<String> {
        let mut out = Vec::new();
        for app in &self.apps {
            if !out.contains(&app.category) {
                out.push(app.category.clone());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_resolves_all_artcraft_domains() {
        let catalog = CraftCatalog::bundled().expect("craft_catalog.toml parses cleanly");
        assert!(catalog.apps.len() >= 17);
        assert_eq!(
            catalog
                .resolve_for_file(Path::new("mockup.psd"), None)
                .map(|a| a.id.as_str()),
            Some("photocraft")
        );
        assert_eq!(
            catalog
                .resolve_for_file(Path::new("schema.svg"), None)
                .map(|a| a.id.as_str()),
            Some("vectorcraft")
        );
        assert_eq!(
            catalog
                .resolve_for_file(Path::new("firmware.wasm"), Some("executable"))
                .map(|a| a.id.as_str()),
            Some("craft-anything")
        );
    }
}
