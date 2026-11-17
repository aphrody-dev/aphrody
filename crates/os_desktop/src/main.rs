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
    /// Launch the Zed app shell hosting the ArtCraft module and, with the `vpn` feature, the VPN client.
    #[cfg(feature = "shell-module")]
    Shell,
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
    /// Execute an AI OS Pilot computer-use step (`status`, `snapshot`, `click`, `type`, `ai_act_verify`, etc.) returning JSON envelope 2.6.
    Pilot {
        /// Pilot operation name (`status`, `permissions`, `snapshot`, `find`, `click`, `type`, `windows`, `displays`, `screenshot`, `wait`, `batch`, `ai_act_verify`).
        op: String,
        /// JSON object arguments for the pilot operation.
        #[arg(long, default_value = "{}")]
        args: String,
        /// Target backend (`auto`, `gpui`, `native`).
        #[arg(long, default_value = "auto")]
        target: String,
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
        #[cfg(feature = "shell-module")]
        Some(Command::Shell) => os_desktop::run_app_shell(),
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
                let i18n = os_ui::OsI18nCatalog::load_with_user_override().unwrap_or_default();
                let process_locale = os_session::detect_process_locale();
                let active_locale = i18n.resolve_tag(process_locale.as_str());
                let active_dir = i18n.direction(&active_locale);
                let payload = serde_json::json!({
                    "brand": brand::APP_NAME,
                    "version": env!("CARGO_PKG_VERSION"),
                    "hardware": hardware,
                    "environment": env_info,
                    "i18n": {
                        "locale": active_locale,
                        "dir": active_dir.as_str(),
                        "available_locales": i18n.available_locales(),
                        "w3c_specs": [
                            "https://www.w3.org/International/",
                            "https://www.w3.org/TR/international-specs/",
                            "https://www.w3.org/TR/string-meta/",
                            "https://www.w3.org/TR/charmod-norm/",
                            "https://www.w3.org/TR/ltli/"
                        ]
                    },
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
        Some(Command::Pilot { op, args, target }) => {
            let parsed_args: serde_json::Value = serde_json::from_str(&args)?;
            let mut engine = os_pilot::PilotEngine::new();
            let ctx = os_pilot::CommandContext {
                target: os_pilot::TargetBackend::parse(Some(&target)),
                ..Default::default()
            };
            let envelope = match engine.execute_tool(&op, &parsed_args, &ctx) {
                Ok(data) => os_pilot::CommandEnvelope::ok(op, data),
                Err(err) => os_pilot::CommandEnvelope::err(op, err),
            };
            println!("{}", serde_json::to_string_pretty(&envelope)?);
            if !envelope.ok {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}
