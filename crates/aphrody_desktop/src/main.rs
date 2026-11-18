use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use clap::{Parser, Subcommand};
use aphrody_craft::{CraftCatalog, McpServerState, inspect_path, serve_http, serve_stdio};
use aphrody_session::{
    DistBundleSpec, DistFormat, HardwareCapabilities, SessionEnvironment, SessionManifest,
    discover_system_apps,
};
use aphrody_ui::{AppearanceMode, OsThemeCatalog};

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
    /// Execute an AI OS Pilot computer-use step (`status`, `snapshot`, `click`, `type`, `act_verify`, `batch`, `browser_*`) returning JSON envelope 2.6.
    Pilot {
        /// Pilot operation name (`status`, `permissions`, `snapshot`, `find`, `click`, `type`, `windows`, `displays`, `screenshot`, `wait`, `batch`, `act_verify`, `browser_*`).
        op: String,
        /// JSON object arguments for the pilot operation (pass `-` to read from stdin).
        #[arg(long, default_value = "{}")]
        args: String,
        /// Target backend (`auto`, `gpui`, `native`).
        #[arg(long, default_value = "auto")]
        target: String,
        /// Optional pilot session scope identifier.
        #[arg(long)]
        session: Option<String>,
        /// Optional operation deadline in milliseconds.
        #[arg(long)]
        timeout_ms: Option<u64>,
        /// Optional target application name filter.
        #[arg(long)]
        app: Option<String>,
        /// Optional target window identifier.
        #[arg(long)]
        window_id: Option<String>,
        /// Stop batch execution on the first failing step (`batch` / `browser_batch`).
        #[arg(long)]
        stop_on_error: Option<bool>,
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
        .filter_module("aphrody_desktop", log::LevelFilter::Info)
        .try_init()
        .ok();

    let cli = Cli::parse();
    match cli.command {
        None | Some(Command::Desktop) => {
            let opts = aphrody_desktop::DesktopOptions {
                initial_theme: cli.theme,
                initial_mode: parse_mode(cli.mode.as_deref()),
                initial_app: cli.app,
                inspect_path: cli.inspect,
                ..Default::default()
            };
            aphrody_desktop::run_desktop_app(opts)?;
        }
        #[cfg(feature = "shell-module")]
        Some(Command::Shell) => aphrody_desktop::run_app_shell(),
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
                let i18n = aphrody_ui::OsI18nCatalog::load_with_user_override().unwrap_or_default();
                let process_locale = aphrody_session::detect_process_locale();
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
        Some(Command::Pilot {
            op,
            args,
            target,
            session,
            timeout_ms,
            app,
            window_id,
            stop_on_error,
        }) => {
            let raw_args = if args.trim() == "-" {
                use std::io::Read as _;
                let mut stdin_buf = String::new();
                std::io::stdin().read_to_string(&mut stdin_buf)?;
                stdin_buf
            } else {
                args
            };
            let envelope = run_pilot_cli_command(
                &op,
                &raw_args,
                &target,
                session.as_deref(),
                timeout_ms,
                app,
                window_id,
                stop_on_error,
            )?;
            println!("{}", serde_json::to_string_pretty(&envelope)?);
            if !envelope.ok {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_pilot_cli_command(
    op: &str,
    raw_args: &str,
    target: &str,
    session: Option<&str>,
    timeout_ms: Option<u64>,
    app: Option<String>,
    window_id: Option<String>,
    stop_on_error: Option<bool>,
) -> anyhow::Result<aphrody_pilot::CommandEnvelope> {
    let mut parsed_args: serde_json::Value = serde_json::from_str(raw_args)?;
    if let (Some(stop), Some(obj)) = (stop_on_error, parsed_args.as_object_mut()) {
        obj.entry("stop_on_error")
            .or_insert(serde_json::Value::Bool(stop));
    }
    let mut engine = aphrody_pilot::PilotEngine::new();
    if let Some(session_id) = session
        && let Err(err) = engine.set_session(session_id)
    {
        return Ok(aphrody_pilot::CommandEnvelope::err(op, err));
    }
    let ctx = aphrody_pilot::CommandContext {
        target: aphrody_pilot::TargetBackend::parse(Some(target)),
        timeout_ms,
        app,
        window_id,
        ..Default::default()
    };
    Ok(match engine.execute_tool(op, &parsed_args, &ctx) {
        Ok(data) => aphrody_pilot::CommandEnvelope::ok(op, data),
        Err(err) => aphrody_pilot::CommandEnvelope::err(op, err),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pilot_cli_parses_flags_and_executes_batch_stop_on_error() {
        let parsed = Cli::try_parse_from([
            "aphrody-os",
            "pilot",
            "desktop_batch",
            "--target",
            "gpui",
            "--session",
            "agent-session-1",
            "--timeout-ms",
            "5000",
            "--stop-on-error",
            "true",
            "--args",
            r#"{"steps":[{"op":"desktop_status","args":{}},{"op":"desktop_bad_step","args":{}},{"op":"desktop_status","args":{}}]}"#,
        ])
        .expect("cli parse succeeds");

        let Some(Command::Pilot {
            op,
            args,
            target,
            session,
            timeout_ms,
            app,
            window_id,
            stop_on_error,
        }) = parsed.command
        else {
            panic!("expected Pilot subcommand");
        };

        let envelope = run_pilot_cli_command(
            &op,
            &args,
            &target,
            session.as_deref(),
            timeout_ms,
            app,
            window_id,
            stop_on_error,
        )
        .expect("cli execution ok");
        assert!(envelope.ok);
        assert_eq!(envelope.version, aphrody_pilot::ENVELOPE_VERSION);
        let data = envelope.data.expect("batch report present");
        assert_eq!(data["stopped_early"], true);
        assert_eq!(data["not_started"], 1);
    }
}
