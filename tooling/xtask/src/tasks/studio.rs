#![allow(clippy::disallowed_methods, reason = "tooling is exempt")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail, ensure};
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::tasks::{brand, licenses, package_conformity};
use crate::workspace::load_workspace;

#[derive(Subcommand, Debug)]
pub enum StudioCommand {
    /// Inspects consolidated Zed + Google Design + Storytold + egui/Bevy studio layout & manifests.
    Status(StudioStatusArgs),
    /// Runs reproducible conformity, brand, license, design, Storytold, i18n, and hygiene gates.
    Check,
    /// Executes the headless GPUI vs egui 0.37 vs Unreal Bevy 5.8 parity & Lighthouse report.
    Parity,
    /// Launches, exports, or visually verifies the NSH AGY/Gemini Session & Storytold Studio.
    Session(StudioSessionArgs),
}

#[derive(Args, Debug)]
pub struct StudioStatusArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct StudioSessionArgs {
    #[arg(long, default_value_t = 4789)]
    pub port: u16,
    #[arg(long)]
    pub export: Option<PathBuf>,
    #[arg(long)]
    pub verify: bool,
    #[arg(long)]
    pub snapshot: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StudioCrateEntry {
    pub name: &'static str,
    pub manifest_rel: &'static str,
    pub present: bool,
    pub example_rel: Option<&'static str>,
    pub example_present: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StudioStatusReport {
    pub ok: bool,
    pub workspace_root: String,
    pub product_toml_present: bool,
    pub storytold_manifest_present: bool,
    pub craft_catalog_present: bool,
    pub design_manifest_present: bool,
    pub design_md_uppercase_present: bool,
    pub unified_bun_cli_present: bool,
    pub zed_studio_feature_wired: bool,
    pub crates: Vec<StudioCrateEntry>,
}

const CONSOLIDATED_CRATES: &[(&str, &str, Option<&str>)] = &[
    ("aphrody_ui", "crates/aphrody_ui/Cargo.toml", None),
    ("aphrody_craft", "crates/aphrody_craft/Cargo.toml", None),
    (
        "aphrody_design",
        "crates/aphrody_design/Cargo.toml",
        None,
    ),
    (
        "egui_gpui",
        "crates/egui_gpui/Cargo.toml",
        Some("crates/egui_gpui/examples/google_design_egui.rs"),
    ),
    (
        "bevy_gpui",
        "crates/bevy_gpui/Cargo.toml",
        Some("crates/bevy_gpui/examples/unreal_solari_showcase.rs"),
    ),
    (
        "aphrody_craft",
        "crates/aphrody_craft/Cargo.toml",
        Some("crates/aphrody_craft/examples/studio_parity_report.rs"),
    ),
    ("aphrody_nsh", "crates/aphrody_nsh/Cargo.toml", None),
    (
        "aphrody_cli",
        "crates/aphrody_cli/Cargo.toml",
        Some("crates/aphrody_cli/examples/webgpu_naga_pipeline.rs"),
    ),
];

pub fn collect_studio_status(root: &Path) -> Result<StudioStatusReport> {
    let product_toml_present = root.join("product.toml").is_file();
    let storytold_manifest_present = root.join("assets/os/storytold_ecosystem.toml").is_file();
    let craft_catalog_present = root.join("assets/os/craft_catalog.toml").is_file();
    let design_manifest_present = root.join("aphrody/docs/design/manifest.json").is_file();
    let design_md_uppercase_present = root.join("aphrody/docs/design/DESIGN.md").is_file();
    let unified_bun_cli_present = root.join("tooling/bun/cli.ts").is_file();

    let zed_cargo = fs::read_to_string(root.join("crates/zed/Cargo.toml"))
        .context("reading crates/zed/Cargo.toml")?;
    let zed_studio_feature_wired =
        zed_cargo.contains("studio = [") && zed_cargo.contains("\"studio\"");

    let mut crates = Vec::with_capacity(CONSOLIDATED_CRATES.len());
    for &(name, manifest_rel, example_rel) in CONSOLIDATED_CRATES {
        let present = root.join(manifest_rel).is_file();
        let example_present = example_rel.map_or(true, |rel| root.join(rel).is_file());
        crates.push(StudioCrateEntry {
            name,
            manifest_rel,
            present,
            example_rel,
            example_present,
        });
    }

    let ok = product_toml_present
        && storytold_manifest_present
        && craft_catalog_present
        && design_manifest_present
        && design_md_uppercase_present
        && unified_bun_cli_present
        && zed_studio_feature_wired
        && crates.iter().all(|c| c.present && c.example_present);

    Ok(StudioStatusReport {
        ok,
        workspace_root: root.display().to_string(),
        product_toml_present,
        storytold_manifest_present,
        craft_catalog_present,
        design_manifest_present,
        design_md_uppercase_present,
        unified_bun_cli_present,
        zed_studio_feature_wired,
        crates,
    })
}

pub fn run_studio(command: StudioCommand) -> Result<()> {
    let workspace = load_workspace()?;
    let root = workspace.workspace_root.as_std_path();

    match command {
        StudioCommand::Status(args) => {
            let report = collect_studio_status(root)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Studio consolidation status: {} ({})",
                    if report.ok { "OK" } else { "INCOMPLETE" },
                    report.workspace_root
                );
                for entry in &report.crates {
                    println!(
                        "  - {:<22} manifest={} example={}",
                        entry.name, entry.present, entry.example_present
                    );
                }
            }
            ensure!(report.ok, "Studio consolidation status reported missing items");
            Ok(())
        }
        StudioCommand::Check => {
            brand::run_brand(brand::BrandCommand::Check(brand::BrandCheckArgs::default()))?;
            package_conformity::run_package_conformity(
                package_conformity::PackageConformityArgs {},
            )?;
            licenses::run_licenses(licenses::LicensesArgs {})?;

            let report = collect_studio_status(root)?;
            ensure!(report.ok, "Studio structural status check failed");

            let status = Command::new("bun")
                .current_dir(root)
                .args(["tooling/bun/cli.ts", "check"])
                .status()
                .context("failed to execute `bun tooling/bun/cli.ts check`")?;
            if !status.success() {
                bail!("`bun tooling/bun/cli.ts check` exited with {status}");
            }
            Ok(())
        }
        StudioCommand::Parity => {
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
            let status = Command::new(cargo)
                .current_dir(root)
                .args(["run", "-p", "aphrody_craft", "--example", "studio_parity_report"])
                .status()
                .context("failed to run studio_parity_report example")?;
            if !status.success() {
                bail!("studio_parity_report exited with {status}");
            }
            Ok(())
        }
        StudioCommand::Session(args) => {
            let mut cmd = Command::new("bun");
            cmd.current_dir(root).arg("tooling/bun/agy-session-studio.ts");
            if let Some(out) = args.export {
                cmd.arg("export").arg("--out").arg(out);
            } else if args.verify {
                cmd.arg("verify")
                    .arg("--url")
                    .arg(format!("http://127.0.0.1:{}/", args.port));
            } else if args.snapshot {
                cmd.arg("snapshot");
            } else {
                cmd.arg("serve").arg("--port").arg(args.port.to_string());
            }
            let status = cmd
                .status()
                .context("failed to execute agy-session-studio.ts")?;
            if !status.success() {
                bail!("agy-session-studio.ts exited with {status}");
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_status_verifies_consolidated_zed_layout() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .context("locating workspace root from tooling/xtask")?;
        let report = collect_studio_status(root)?;
        assert!(report.product_toml_present);
        assert!(report.storytold_manifest_present);
        assert!(report.craft_catalog_present);
        assert!(report.design_manifest_present);
        assert!(report.zed_studio_feature_wired);
        assert!(report.crates.iter().all(|c| c.present && c.example_present));
        Ok(())
    }
}
