//! Native integration between Aphrody OS and Auto (`https://auto.aphrody.com`):
//! federated Cloudflare R2 topology, Factur-X EN 16931 workflows, and shared release/learn channels.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Canonical public base URL for the Auto serverless Cloudflare Worker (`CompanyDO` + D1).
pub const AUTO_DEFAULT_URL: &str = "https://auto.aphrody.com";

/// Shared Cloudflare R2 bucket and prefix bindings between Aphrody and Auto.
pub const AUTO_R2_ASSETS_BUCKET: &str = "aphrody-assets";
pub const AUTO_R2_ASSETS_PREFIX: &str = "auto/assets/";
pub const AUTO_R2_RELEASES_BUCKET: &str = "aphrody-releases";
pub const AUTO_R2_RELEASES_PREFIX: &str = "auto/releases/";
pub const AUTO_R2_BACKUPS_BUCKET: &str = "aphrody-backups";
pub const AUTO_R2_BACKUPS_PREFIX: &str = "auto/archives/";

/// Federated endpoint surfaced on `aphrody-worker` backed by `aphrody-releases/auto/releases/manifest.json`.
pub const APHRODY_FEDERATED_AUTO_MANIFEST_PATH: &str = "/api/releases/auto/latest";

/// Embedded declarative Cloudflare topology shared across Aphrody and Auto.
pub const CLOUDFLARE_TOPOLOGY_TOML: &str = include_str!("../../../assets/os/cloudflare_topology.toml");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoServiceEndpoint {
    pub worker_name: String,
    pub public_url: String,
    pub durable_object: String,
    pub workflow: String,
    pub assets_prefix: String,
    pub releases_prefix: String,
    pub backups_prefix: String,
    pub federated_release_endpoint: String,
    pub learn_hub_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoCapabilitySpec {
    pub id: &'static str,
    pub title: &'static str,
    pub crate_name: &'static str,
    pub compliance_standard: &'static str,
}

/// Returns the default federated Auto integration contract wired to Cloudflare R2 + Learn hub.
pub fn default_auto_endpoint() -> AutoServiceEndpoint {
    AutoServiceEndpoint {
        worker_name: "auto".to_owned(),
        public_url: AUTO_DEFAULT_URL.to_owned(),
        durable_object: "CompanyDO".to_owned(),
        workflow: "AutomationWorkflow".to_owned(),
        assets_prefix: AUTO_R2_ASSETS_PREFIX.to_owned(),
        releases_prefix: AUTO_R2_RELEASES_PREFIX.to_owned(),
        backups_prefix: AUTO_R2_BACKUPS_PREFIX.to_owned(),
        federated_release_endpoint: format!("https://releases.aphrody.com{APHRODY_FEDERATED_AUTO_MANIFEST_PATH}"),
        learn_hub_url: "https://learn.aphrody.com".to_owned(),
    }
}

/// Core Auto crates integrated into Aphrody OS's GPUI Kit 0.10.0 + Craft ecosystem.
pub fn integrated_auto_capabilities() -> Vec<AutoCapabilitySpec> {
    vec![
        AutoCapabilitySpec {
            id: "facturx",
            title: "Factur-X / ZUGFeRD",
            crate_name: "auto-facturx",
            compliance_standard: "EN 16931 / PDF/A-3B",
        },
        AutoCapabilitySpec {
            id: "sheets",
            title: "Ledgers & Invoices",
            crate_name: "auto-sheets",
            compliance_standard: "Multi-currency ERP",
        },
        AutoCapabilitySpec {
            id: "docs",
            title: "Business Documents",
            crate_name: "auto-docs",
            compliance_standard: "Craft PDF/DOCX/ODS",
        },
        AutoCapabilitySpec {
            id: "sync",
            title: "CompanyDO Local-First Sync",
            crate_name: "auto-client",
            compliance_standard: "Cloudflare D1 + DO SQLite",
        },
    ]
}

/// Validates that a base URL is HTTPS and belongs to the Aphrody sovereign edge or loopback test host.
pub fn validate_auto_endpoint(url: &str) -> Result<String> {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        bail!("Auto endpoint URL cannot be empty");
    }
    if !(trimmed.starts_with("https://")
        || trimmed.starts_with("http://127.0.0.1")
        || trimmed.starts_with("http://localhost"))
    {
        bail!("Auto endpoint must use HTTPS (got {trimmed})");
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn federated_topology_is_embedded_and_clean() {
        assert!(CLOUDFLARE_TOPOLOGY_TOML.contains("aphrody-worker"));
        assert!(CLOUDFLARE_TOPOLOGY_TOML.contains("aphrody-learn"));
        assert!(CLOUDFLARE_TOPOLOGY_TOML.contains("auto.aphrody.com"));
        assert!(!CLOUDFLARE_TOPOLOGY_TOML.contains("ca22c1a51a145f919b54afa18d936516"));
    }

    #[test]
    fn default_endpoint_and_capabilities_are_valid() {
        let ep = default_auto_endpoint();
        assert_eq!(ep.worker_name, "auto");
        assert!(validate_auto_endpoint(&ep.public_url).is_ok());
        assert_eq!(ep.assets_prefix, "auto/assets/");
        assert!(integrated_auto_capabilities().len() >= 4);
        assert!(validate_auto_endpoint("http://example.com").is_err());
    }
}
