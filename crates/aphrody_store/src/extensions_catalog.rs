//! Bundled Zed extension registry (`zed-industries/extensions`) and
//! upstream WinGet baseline (`zed-industries/winget-pkgs`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::StoreError;

/// Embedded live-synced Zed Extensions registry (`assets/os/zed_extensions_catalog.toml`).
pub const BUNDLED_EXTENSIONS_TOML: &str =
    include_str!("../../../assets/os/zed_extensions_catalog.toml");

/// Embedded upstream WinGet manifest baseline (`assets/os/winget_upstream_baseline.toml`).
pub const BUNDLED_WINGET_BASELINE_TOML: &str =
    include_str!("../../../assets/os/winget_upstream_baseline.toml");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZedExtensionEntry {
    pub id: String,
    pub submodule: String,
    pub repo_url: String,
    pub version: String,
    pub category: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZedExtensionsCatalog {
    pub schema_version: u32,
    pub source_repo: String,
    pub source_url: String,
    pub synced_at: String,
    pub total_extensions: usize,
    pub total_submodules: usize,
    pub category_counts: BTreeMap<String, usize>,
    pub entries: Vec<ZedExtensionEntry>,
}

impl ZedExtensionsCatalog {
    pub fn bundled() -> Result<Self, StoreError> {
        Ok(toml::from_str(BUNDLED_EXTENSIONS_TOML)?)
    }

    pub fn by_id(&self, id: &str) -> Option<&ZedExtensionEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn search(&self, query: &str, category: Option<&str>) -> Vec<&ZedExtensionEntry> {
        let query_lower = query.to_ascii_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                category.is_none_or(|cat| entry.category == cat)
                    && (query_lower.is_empty()
                        || entry.id.to_ascii_lowercase().contains(&query_lower)
                        || entry.submodule.to_ascii_lowercase().contains(&query_lower))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WingetUpstreamBaseline {
    pub schema_version: u32,
    pub source_repo: String,
    pub source_url: String,
    pub synced_at: String,
    pub package_identifier: String,
    pub latest_version: String,
    pub manifest_version: String,
    pub scope: String,
    pub install_modes: Vec<String>,
    pub upgrade_behavior: String,
    pub moniker: String,
}

impl WingetUpstreamBaseline {
    pub fn bundled() -> Result<Self, StoreError> {
        Ok(toml::from_str(BUNDLED_WINGET_BASELINE_TOML)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_zed_extensions_and_winget_baseline_parse() {
        let catalog = match ZedExtensionsCatalog::bundled() {
            Ok(catalog) => catalog,
            Err(err) => panic!("failed to parse bundled zed extensions catalog: {err}"),
        };
        assert_eq!(catalog.source_repo, "zed-industries/extensions");
        assert!(catalog.total_extensions >= 100);
        assert_eq!(catalog.total_extensions, catalog.entries.len());
        assert!(!catalog.search("rust", None).is_empty());

        let baseline = match WingetUpstreamBaseline::bundled() {
            Ok(baseline) => baseline,
            Err(err) => panic!("failed to parse bundled winget baseline: {err}"),
        };
        assert_eq!(baseline.package_identifier, "ZedIndustries.Zed");
        assert_eq!(baseline.manifest_version, "1.28.0");
    }
}
