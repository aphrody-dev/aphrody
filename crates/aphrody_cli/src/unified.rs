//! Dioxus-style (`dx`) unified full-stack CLI commands for Aphrody:
//! `build`, `deploy`, `dev`, `craft`, `cosmic`, `zed`, and `install` / `switch-de`.

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use aphrody_craft::{CraftCatalog, CraftExposureManifest, StorytoldEcosystemLedger};
use aphrody_installer::{
    InstallerConfig, InstallerProfileCatalog, plan_desktop_switch, plan_installation,
};
use aphrody_pilot::{HeadlessZedController, SyntaxLanguage, analyze_with_tree_sitter};
use aphrody_session::CosmicSettingsDaemon;
use aphrody_store::{AwesomeGpuiCatalog, StoreTarget};
use clap::{Args, Subcommand, ValueEnum};
use cosmic_gpui::{COSMIC_EPOCH_COMPONENTS, inspect_cosmic_stack};
use serde::Serialize;
use serde_json::json;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub(crate) enum UnifiedBuildTarget {
    #[default]
    Desktop,
    Web,
    Headless,
    Craft,
    Cosmic,
    BunRelease,
    All,
}

#[derive(Debug, Args)]
pub(crate) struct SuiteArgs {
    /// Emit structured JSON capability profile
    #[arg(long)]
    pub json: bool,
}

pub(crate) fn run_suite(args: SuiteArgs) -> Result<()> {
    let profile = aphrody::AphrodySuiteProfile::current();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&profile)?);
    } else {
        println!(
            "{} Consolidated Suite — {}/{} pillars compiled in CLI | Design: {} | Modes: {}",
            profile.product,
            profile.compiled_pillars,
            profile.total_pillars,
            profile.design_source_of_truth,
            profile.theme_modes.join(", "),
        );
        for pillar in &profile.pillars {
            let state = if pillar.compiled { "active" } else { "available" };
            println!(
                "  [{state:9}] {:16} ({}) [{}]",
                pillar.id,
                pillar.primary_crate,
                pillar.surfaces.join(", "),
            );
        }
    }
    Ok(())
}

#[derive(Debug, Args)]
pub(crate) struct BuildArgs {
    /// Target surface (`desktop`, `web`, `headless`, `craft`, `cosmic`, `bun-release`, `all`)
    #[arg(long, value_enum, default_value_t = UnifiedBuildTarget::Desktop)]
    pub target: UnifiedBuildTarget,
    /// Emit structured JSON build plan without running local cargo/bun commands
    #[arg(long)]
    pub json: bool,
    /// Optional Craft/COSMIC app slug
    #[arg(long)]
    pub app: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct DeployArgs {
    /// Target suite (`craft`, `cosmic`, `all`)
    #[arg(long, default_value = "all")]
    pub suite: String,
    /// Store or OS package manager target (22 targets: `winget`, `flatpak`, `arch`, `nix`, `oci`, ...)
    #[arg(long)]
    pub target: Option<String>,
    /// Optional app id to scope deploy manifest
    #[arg(long)]
    pub id: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum DevSubcommand {
    /// Parse a Rust/TS/TSX/Python/JSON file via Zed's bundled Tree-sitter grammars
    Ast {
        path: PathBuf,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Print the Dioxus-style full-stack Rust module & 82-repo Storytold matrix
    Matrix {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum CraftSubcommand {
    /// Expose all Storytold / ArtCraft apps across CLI, MCP, Web, Desktop, build, and deploy
    Expose {
        #[arg(long)]
        json: bool,
    },
    /// List Craft Catalog domains and 82 Storytold repositories
    Catalog {
        #[arg(long)]
        json: bool,
    },
    /// Sniff any media/shader/CAD/binary asset via pure-Rust craft-sniff + codecs
    Sniff {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum CosmicSubcommand {
    /// Show live COSMIC Epoch + Pop!_OS compositor, settings daemon, and 22-target summary
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Expose every COSMIC Epoch component across CLI, MCP, Web, and Desktop
    Expose {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ZedSubcommand {
    /// Inspect a workspace directory headlessly (Zed headless controller + Tree-sitter readiness)
    Headless {
        #[arg(default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Run unified headless Zed + Tree-sitter + Agent-Browser + AI Pilot stdio MCP server
    Mcp {
        #[arg(default_value = ".")]
        root: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum InstallSubcommand {
    /// List CachyOS Net-Installer & Aphrody Rust Userland DE profiles
    Profiles {
        #[arg(long)]
        json: bool,
    },
    /// Generate a CachyOS/Aphrody dry-run partition, Btrfs subvolume, and package plan
    Plan {
        #[arg(long, default_value = "aphrody-gpui")]
        desktop: String,
        #[arg(long, default_value = "btrfs")]
        filesystem: String,
        #[arg(long)]
        json: bool,
    },
    /// Plan switching an existing machine cleanly between Rust Userlands & Arch DEs
    SwitchDe {
        #[arg(long, default_value = "aphrody-gpui")]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
pub(crate) struct UnifiedBuildPlan {
    pub target: &'static str,
    pub profile: &'static str,
    pub commands: Vec<String>,
    pub remote_build_hint: String,
}

#[must_use]
pub(crate) fn compute_build_plan(args: &BuildArgs) -> UnifiedBuildPlan {
    let app_flag = args
        .app
        .as_deref()
        .map(|a| format!(" -- {a}"))
        .unwrap_or_default();
    match args.target {
        UnifiedBuildTarget::Desktop => UnifiedBuildPlan {
            target: "desktop",
            profile: "release",
            commands: vec![format!("cargo build -p aphrody_desktop -p zed --bin aphrody --bin aphrody-os{app_flag}")],
            remote_build_hint: "bun tooling/bun/fleet-sync.ts verify --host fr1".to_owned(),
        },
        UnifiedBuildTarget::Web => UnifiedBuildPlan {
            target: "web",
            profile: "web-lto",
            commands: vec![
                "bun tooling/bun/web-build.ts check".to_owned(),
                "cargo check --target wasm32-unknown-unknown -p aphrody_desktop --features web --example web_os".to_owned(),
            ],
            remote_build_hint: "bun tooling/bun/fleet-sync.ts sync --host fr1".to_owned(),
        },
        UnifiedBuildTarget::Headless => UnifiedBuildPlan {
            target: "headless",
            profile: "bun-release",
            commands: vec![
                "cargo build --profile bun-release -p aphrody_pilot -p zed --bin aphrody".to_owned(),
                "aphrody --headless --mcp".to_owned(),
            ],
            remote_build_hint: "ssh fr1 'cd /srv/xbuild/scratch/aphrody && cargo build --profile bun-release -p zed --bin aphrody'".to_owned(),
        },
        UnifiedBuildTarget::Craft => UnifiedBuildPlan {
            target: "craft",
            profile: "release",
            commands: vec![
                "cargo build -p aphrody_craft --all-features".to_owned(),
                "cargo xtask studio check".to_owned(),
            ],
            remote_build_hint: "bun tooling/bun/fleet-sync.ts verify --host fr1".to_owned(),
        },
        UnifiedBuildTarget::Cosmic => UnifiedBuildPlan {
            target: "cosmic",
            profile: "release",
            commands: vec![
                "cargo build -p cosmic_gpui -p aphrody_session -p aphrody_desktop".to_owned(),
            ],
            remote_build_hint: "bun tooling/bun/fleet-sync.ts verify --host fr1".to_owned(),
        },
        UnifiedBuildTarget::BunRelease => UnifiedBuildPlan {
            target: "bun-release",
            profile: "bun-release",
            commands: vec![
                "cargo build --profile bun-release -p aphrody -p aphrody_cli -p aphrody_desktop -p zed".to_owned(),
            ],
            remote_build_hint: "ssh fr1 'cd /srv/xbuild/scratch/aphrody && cargo build --profile bun-release -p aphrody_cli'".to_owned(),
        },
        UnifiedBuildTarget::All => UnifiedBuildPlan {
            target: "all",
            profile: "bun-release + web-lto",
            commands: vec![
                "bun tooling/bun/fleet-sync.ts verify --host fr1 --pull-shots".to_owned(),
                "bun tooling/bun/web-build.ts check".to_owned(),
            ],
            remote_build_hint: "ssh fr1 'cd /srv/xbuild/scratch/aphrody && cargo build --profile bun-release -p aphrody -p aphrody_cli'".to_owned(),
        },
    }
}

pub(crate) fn run_build(args: BuildArgs) -> Result<()> {
    let plan = compute_build_plan(&args);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!("[aphrody-cli build] target={} profile={}", plan.target, plan.profile);
        for cmd in &plan.commands {
            println!("  {cmd}");
        }
        println!("  remote: {}", plan.remote_build_hint);
    }
    Ok(())
}

pub(crate) fn run_deploy(args: DeployArgs) -> Result<()> {
    let craft = CraftCatalog::load_with_user_override()?;
    let craft_manifest = CraftExposureManifest::from_catalog(&craft);
    let store = AwesomeGpuiCatalog::bundled()?;
    let target_filter = args.target.as_deref().and_then(StoreTarget::from_slug);
    let targets: Vec<&str> = match target_filter {
        Some(t) => vec![t.slug()],
        None => StoreTarget::ALL.iter().map(|t| t.slug()).collect(),
    };
    let craft_entries: Vec<_> = craft_manifest
        .entries
        .into_iter()
        .filter(|e| args.id.as_deref().is_none_or(|id| e.app_id == id))
        .collect();
    let payload = json!({
        "suite": args.suite,
        "targets": targets,
        "store_entries": store.entries.len(),
        "craft_apps": craft_entries,
        "cosmic_components": COSMIC_EPOCH_COMPONENTS,
    });
    if args.json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "Deploy suite '{}' across {} packaging targets ({} Craft apps, {} COSMIC subsystems)",
            args.suite,
            targets.len(),
            craft_entries.len(),
            COSMIC_EPOCH_COMPONENTS.len()
        );
    }
    Ok(())
}

pub(crate) fn run_dev(cmd: DevSubcommand) -> Result<()> {
    match cmd {
        DevSubcommand::Ast {
            path,
            language,
            json: as_json,
        } => {
            let source = fs::read_to_string(&path)?;
            let lang = language
                .as_deref()
                .and_then(SyntaxLanguage::from_name_or_path)
                .or_else(|| SyntaxLanguage::from_name_or_path(&path.to_string_lossy()))
                .unwrap_or(SyntaxLanguage::Rust);
            let report = analyze_with_tree_sitter(lang, &source)?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "{}: {:?} · {} lines · {} symbols · {} errors",
                    path.display(),
                    report.language,
                    report.line_count,
                    report.symbols.len(),
                    report.syntax_errors
                );
                for sym in &report.symbols {
                    println!("  L{}-L{}: {} {}", sym.start_line, sym.end_line, sym.kind, sym.name);
                }
            }
        }
        DevSubcommand::Matrix { json: as_json } => {
            let ledger = StorytoldEcosystemLedger::bundled()?;
            let out = json!({
                "facade_crate": "aphrody",
                "ui_backends": ["gpui", "iced_gpui", "egui_gpui", "bevy_gpui", "tauri_gpui", "cosmic_gpui", "nsh"],
                "storytold_repos": ledger.total_repositories,
                "craft_libs_crates": ledger.craft_libs_crates_on_disk,
                "cosmic_components": COSMIC_EPOCH_COMPONENTS.len(),
                "store_targets": StoreTarget::ALL.len(),
            });
            if as_json {
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
        }
    }
    Ok(())
}

pub(crate) fn run_craft(cmd: CraftSubcommand) -> Result<()> {
    let catalog = CraftCatalog::load_with_user_override()?;
    match cmd {
        CraftSubcommand::Expose { json: as_json } => {
            let manifest = CraftExposureManifest::from_catalog(&catalog);
            if as_json {
                println!("{}", serde_json::to_string_pretty(&manifest)?);
            } else {
                println!("Craft Unified Exposure ({} apps across CLI/MCP/Web/Desktop):", manifest.total_apps);
                for e in &manifest.entries {
                    println!("  {} -> {} | mcp={} | web={}", e.app_id, e.cli_command, e.mcp_tool, e.web_route);
                }
            }
        }
        CraftSubcommand::Catalog { json: as_json } => {
            let ledger = StorytoldEcosystemLedger::bundled()?;
            let out = json!({
                "apps": catalog.apps,
                "storytold": ledger,
            });
            if as_json {
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("{} Craft apps · {} Storytold repos", catalog.apps.len(), ledger.total_repositories);
            }
        }
        CraftSubcommand::Sniff { path, json: as_json } => {
            let report = aphrody_craft::pipeline::inspect_path(&path, Some(&catalog))?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{:?}", report);
            }
        }
    }
    Ok(())
}

pub(crate) fn run_cosmic(cmd: CosmicSubcommand) -> Result<()> {
    match cmd {
        CosmicSubcommand::Status { json: as_json } => {
            let summary = inspect_cosmic_stack();
            let daemon = CosmicSettingsDaemon::default();
            let out = json!({
                "summary": summary,
                "daemon": daemon,
            });
            if as_json {
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!(
                    "COSMIC Stack: {} components, {} workspaces, power={}",
                    summary.component_count,
                    summary.default_workspace_count,
                    summary.daemon_power_profile
                );
            }
        }
        CosmicSubcommand::Expose { json: as_json } => {
            if as_json {
                println!("{}", serde_json::to_string_pretty(&COSMIC_EPOCH_COMPONENTS)?);
            } else {
                for comp in COSMIC_EPOCH_COMPONENTS {
                    println!(
                        "{} ({}) -> {} | web={} | desktop={}",
                        comp.id, comp.upstream_repo, comp.cli_command, comp.web_route, comp.desktop_surface
                    );
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn run_zed(cmd: ZedSubcommand) -> Result<()> {
    match cmd {
        ZedSubcommand::Headless { root, json: as_json } => {
            let mut ctrl = HeadlessZedController::new(root);
            let val = ctrl.execute_tool("zed_headless_status", &json!({}))?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(&val)?);
            } else {
                println!("{val}");
            }
        }
        ZedSubcommand::Mcp { root } => {
            aphrody_pilot::run_headless_mcp_stdio(Some(root))?;
        }
    }
    Ok(())
}

pub(crate) fn run_install(cmd: InstallSubcommand) -> Result<()> {
    let catalog = InstallerProfileCatalog::bundled()?;
    match cmd {
        InstallSubcommand::Profiles { json: as_json } => {
            if as_json {
                println!("{}", serde_json::to_string_pretty(&catalog.environments)?);
            } else {
                for env in &catalog.environments {
                    println!(
                        "{} ({}) [{}] -> {}",
                        env.id,
                        env.display_name,
                        env.toolkit.badge(),
                        env.session_exec
                    );
                }
            }
        }
        InstallSubcommand::Plan {
            desktop,
            filesystem,
            json: as_json,
        } => {
            let mut cfg = InstallerConfig::default();
            cfg.desktop = desktop;
            if let Some(fs_kind) = aphrody_installer::FilesystemKind::parse(&filesystem) {
                cfg.fs_name = fs_kind.slug().to_owned();
            }
            let plan = plan_installation(&cfg, &catalog);
            if as_json {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            } else {
                println!(
                    "Install plan: {} packages, {} steps (valid={})",
                    plan.packages.len(),
                    plan.steps.len(),
                    plan.preflight.valid
                );
            }
        }
        InstallSubcommand::SwitchDe {
            from,
            to,
            json: as_json,
        } => {
            let sw = plan_desktop_switch(&from, &to, &catalog)?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(&sw)?);
            } else {
                println!(
                    "Switch DE: {} -> {} (dm={}, exec={})",
                    sw.from_env, sw.to_env, sw.display_manager, sw.session_exec
                );
            }
        }
    }
    Ok(())
}
