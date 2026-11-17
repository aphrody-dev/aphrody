//! Unified Craft Exposure Matrix across CLI, MCP, Web (Wasm/WebGPU), Desktop (GPUI/egui/Bevy),
//! unified builds, and unified 22-target deployment.

use serde::{Deserialize, Serialize};

use crate::catalog::CraftCatalog;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CraftSurfaceChannel {
    Cli,
    Mcp,
    Web,
    Desktop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftExposureEntry {
    pub app_id: String,
    pub display_name: String,
    pub category: String,
    pub cli_command: String,
    pub mcp_tool: String,
    pub web_route: String,
    pub desktop_window: String,
    pub unified_build_cmd: String,
    pub unified_deploy_cmd: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftExposureManifest {
    pub total_apps: usize,
    pub channels: Vec<CraftSurfaceChannel>,
    pub entries: Vec<CraftExposureEntry>,
}

impl CraftExposureManifest {
    #[must_use]
    pub fn from_catalog(catalog: &CraftCatalog) -> Self {
        let entries = catalog
            .apps
            .iter()
            .map(|app| CraftExposureEntry {
                app_id: app.id.clone(),
                display_name: app.name.clone(),
                category: app.category.clone(),
                cli_command: format!("aphrody-cli craft open --app {}", app.id),
                mcp_tool: "craft_sniff_file".to_owned(),
                web_route: format!("/craft/{}", app.id),
                desktop_window: format!("aphrody-os desktop --app {}", app.id),
                unified_build_cmd: format!("aphrody-cli build --target all --app {}", app.id),
                unified_deploy_cmd: format!("aphrody-cli deploy --suite craft --id {}", app.id),
            })
            .collect::<Vec<_>>();

        Self {
            total_apps: entries.len(),
            channels: vec![
                CraftSurfaceChannel::Cli,
                CraftSurfaceChannel::Mcp,
                CraftSurfaceChannel::Web,
                CraftSurfaceChannel::Desktop,
            ],
            entries,
        }
    }
}
