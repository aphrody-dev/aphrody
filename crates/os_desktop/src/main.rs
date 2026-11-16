use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use clap::{Parser, Subcommand};
use os_craft::{CraftCatalog, McpServerState, inspect_path, serve_http, serve_stdio};
use os_session::{
    DistBundleSpec, DistFormat, HardwareCapabilities, SessionEnvironment, SessionManifest,
    discover_system_apps,
};
use os_ui::{AppearanceMode, OsThemeCatalog};

#[derive(Debug, Parser)]
#[command(
    name = "aphrody-os",
    version,
    about = "Aphrody GPUI OS desktop shell, ArtCraft 17-app suite, and Craft Anything MCP host"
)]
struct Cli {
    /// Initial theme ID (e.g. aphrody, cosmic, m3, oled-studio, hyprland, omarchy).
    #[arg(long)]
    theme: Option<String>,

    /// Appearance polarity override (`dark` or `light`).
    #[arg(long)]
    mode: Option<String>,

    /// Initial ArtCraft studio app ID (`photocraft`, `vectorcraft`, `craft-anything`, ...).
    #[arg(long)]
    app: Option<String>,

    /// Optional file path to pre-load in the Craft inspector when launching the GUI.
    #[arg(long)]
    inspect: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Launch the COSMIC-style GPUI desktop shell (`aphrody-os`).
    Desktop,
    /// Inspect an arbitrary image/vector/binary/Wasm/Plist artifact via `craft-libs` & `object`/`wasmparser`.
    Inspect {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Print all bundled 17 ArtCraft + Craft Anything studio entries.
    Catalog {
        #[arg(long)]
        json: bool,
    },
    /// Print OS session environment, compositor/theme catalog diagnostics, and discovered system apps.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Emit Pop!_OS-inspired package descriptors (`deb` control or `.desktop` entry).
    Dist {
        #[arg(long, default_value = "deb")]
        format: String,
    },
    /// Serve the Craft Anything + Compositor + Session MCP control plane over stdin/stdout JSON-RPC 2.0.
    Mcp,
    /// Serve the Craft Anything MCP + health endpoint over local HTTP (`APHRODY_OS_MCP_ADDR` or `127.0.0.1:4193`).
    Serve {
        #[arg(long)]
        addr: Option<String>,
    },
}

fn parse_mode(raw: Option<&str>) -> Option<AppearanceMode> {
    raw.and_then(|s| match s.trim().to_ascii_lowercase().as_str() {
        "light" => Some(AppearanceMode::Light),
        "dark" => Some(AppearanceMode::Dark),
        _ => None,
    })
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("os_desktop", log::LevelFilter::Info)
        .try_init()
        .ok();

    let cli = Cli::parse();
    match cli.command {
        None | Some(Command::Desktop) => {
            let opts = os_desktop::DesktopOptions {
                initial_theme: cli.theme,
                initial_mode: parse_mode(cli.mode.as_deref()),
                initial_app: cli.app,
                inspect_path: cli.inspect,
                ..Default::default()
            };
            os_desktop::run_desktop_app(opts)?;
        }
        Some(Command::Inspect { path, json }) => {
            let catalog = CraftCatalog::load_with_user_override()?;
            let report = inspect_path(&path, Some(&catalog))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "{}: {} ({}, {}, {} B, entropy {:.3}, app {:?})",
                    report.source_name,
                    report.format_name,
                    report.family,
                    report.mime_type,
                    report.byte_len,
                    report.shannon_entropy,
                    report.recommended_app
                );
            }
        }
        Some(Command::Catalog { json }) => {
            let catalog = CraftCatalog::load_with_user_override()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&catalog)?);
            } else {
                println!("{} Craft Studio Applications:", catalog.apps.len());
                for app in &catalog.apps {
                    println!(
                        "- {:<16} [{}] ({})",
                        app.id,
                        app.category,
                        app.extensions.join(", ")
                    );
                }
            }
        }
        Some(Command::Doctor { json }) => {
            let manifest = SessionManifest::load_with_user_override()?;
            let env_info = SessionEnvironment::detect(&manifest);
            let themes = OsThemeCatalog::bundled()?;
            let catalog = CraftCatalog::load_with_user_override()?;
            let system_apps = discover_system_apps(16);
            if json {
                let hardware = HardwareCapabilities::detect();
                let payload = serde_json::json!({
                    "brand": brand::APP_NAME,
                    "version": env!("CARGO_PKG_VERSION"),
                    "hardware": hardware,
                    "environment": env_info,
                    "theme_ids": themes.ids(),
                    "craft_app_count": catalog.apps.len(),
                    "system_app_count": system_apps.len(),
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!(
                    "{} OS v{} ({}) — {} themes, {} Craft apps, {} system entries",
                    brand::APP_NAME,
                    env!("CARGO_PKG_VERSION"),
                    std::env::consts::OS,
                    themes.ids().len(),
                    catalog.apps.len(),
                    system_apps.len(),
                );
            }
        }
        Some(Command::Dist { format }) => {
            let spec = DistBundleSpec::from_brand(env!("CARGO_PKG_VERSION"));
            match DistFormat::parse(&format) {
                Some(DistFormat::PopDeb) => print!("{}", spec.render_debian_control()),
                _ => print!(
                    "{}",
                    spec.render_desktop_entry(&format!("{}-os", brand::APP_SLUG))
                ),
            }
        }
        Some(Command::Mcp) => {
            let state = Arc::new(Mutex::new(McpServerState::bootstrap()?));
            serve_stdio(state)?;
        }
        Some(Command::Serve { addr }) => {
            let state = Arc::new(Mutex::new(McpServerState::bootstrap()?));
            serve_http(state, addr.as_deref())?;
        }
    }
    Ok(())
}
