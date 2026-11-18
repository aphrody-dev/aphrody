mod tasks;
mod workspace;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo xtask")]
struct Args {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Runs `cargo clippy`.
    Clippy(tasks::clippy::ClippyArgs),
    Compliance(tasks::compliance::ComplianceArgs),
    Licenses(tasks::licenses::LicensesArgs),
    /// Checks that packages conform to a set of standards.
    PackageConformity(tasks::package_conformity::PackageConformityArgs),
    /// Scaffolds a new standalone GPUI application powered by `gpui_ui`, `gpui_code`, and `gpui_app_shell`.
    NewApp(tasks::new_app::NewAppArgs),
    /// Applies or verifies declarative product branding (`product.toml`) across manifests and icons.
    #[command(subcommand)]
    Brand(tasks::brand::BrandCommand),
    /// Inspect the GPUI publish plan and its dependencies.
    #[command(subcommand)]
    Gpui(tasks::gpui::GpuiCommand),
    /// Consolidated Google Design, Storytold 82-repo, egui/Bevy parity & NSH session studio workflows.
    #[command(subcommand)]
    Studio(tasks::studio::StudioCommand),
    /// Runs the sandbox_linux NixOS VM tests.
    SandboxTests(tasks::sandbox_tests::SandboxTestsArgs),
    /// Runs the Windows WSL Bubblewrap sandbox behavior tests.
    WslSandboxTests(tasks::wsl_sandbox_tests::WslSandboxTestsArgs),
    /// Downloads the pinned `webrtc-sys` release and configures `LK_CUSTOM_WEBRTC`.
    SetupWebrtc(tasks::setup_webrtc::SetupWebrtcArgs),
    /// Builds GPUI web examples and serves them.
    WebExamples(tasks::web_examples::WebExamplesArgs),
    Workflows(tasks::workflows::GenerateWorkflowArgs),
    CheckWorkflows(tasks::workflow_checks::WorkflowValidationArgs),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        CliCommand::Clippy(args) => tasks::clippy::run_clippy(args),
        CliCommand::Compliance(args) => tasks::compliance::check_compliance(args),
        CliCommand::Licenses(args) => tasks::licenses::run_licenses(args),
        CliCommand::PackageConformity(args) => {
            tasks::package_conformity::run_package_conformity(args)
        }
        CliCommand::NewApp(args) => tasks::new_app::run_new_app(args),
        CliCommand::Brand(cmd) => tasks::brand::run_brand(cmd),
        CliCommand::Gpui(args) => tasks::gpui::run_gpui(args),
        CliCommand::Studio(cmd) => tasks::studio::run_studio(cmd),
        CliCommand::SandboxTests(args) => tasks::sandbox_tests::run_sandbox_tests(args),
        CliCommand::WslSandboxTests(args) => tasks::wsl_sandbox_tests::run_wsl_sandbox_tests(args),
        CliCommand::SetupWebrtc(args) => tasks::setup_webrtc::run_setup_webrtc(args),
        CliCommand::WebExamples(args) => tasks::web_examples::run_web_examples(args),
        CliCommand::Workflows(args) => tasks::workflows::run_workflows(args),
        CliCommand::CheckWorkflows(args) => tasks::workflow_checks::validate(args),
    }
}
