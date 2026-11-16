use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail, ensure};
use clap::{Parser, ValueEnum};

use crate::workspace::load_workspace;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AppTemplate {
    /// Lightweight GPU UI window using `gpui_ui` (`gpui` + `ui` + `theme` + `icons`).
    Minimal,
    /// Standalone code editor + Tree-sitter + LSP/Git/Worktree/Terminal using `gpui_ui` + `gpui_code`.
    Code,
    /// Full multi-workspace docked studio application using `gpui_app_shell` + `AppModule`.
    Studio,
    /// Multi-workspace studio fused with Native Shell (`nsh` TSX/Rust surfaces + live TSX preview + `GpuiCodeEditor`).
    NshStudio,
}

#[derive(Parser, Debug)]
pub struct NewAppArgs {
    /// Crate and binary name for the new GPUI application (e.g. `my-data-studio`).
    pub name: String,

    /// Target directory where the project folder will be created (defaults to `../<name>`).
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Starter template to generate.
    #[arg(long, value_enum, default_value_t = AppTemplate::Studio)]
    pub template: AppTemplate,
}

pub fn run_new_app(args: NewAppArgs) -> Result<()> {
    let crate_name = sanitize_crate_name(&args.name)?;
    let workspace = load_workspace()?;
    let workspace_root = workspace.workspace_root.as_std_path();

    let target_dir = match args.output {
        Some(output) => output,
        None => workspace_root
            .parent()
            .unwrap_or(workspace_root)
            .join(&args.name),
    };

    if target_dir.exists() {
        bail!(
            "Target directory '{}' already exists. Choose a different name or `--output` path.",
            target_dir.display()
        );
    }

    let gpui_os_path = normalize_toml_path(workspace_root);
    generate_app_files(&target_dir, &crate_name, &args.name, &gpui_os_path, args.template)?;

    println!(
        "Created GPUI application '{}' ({:?} template) at {}",
        crate_name,
        args.template,
        target_dir.display()
    );
    println!("\nNext steps:");
    println!("  cd \"{}\"", target_dir.display());
    println!("  cargo run");

    Ok(())
}

pub fn generate_app_files(
    target_dir: &Path,
    crate_name: &str,
    display_title: &str,
    gpui_os_path: &str,
    template: AppTemplate,
) -> Result<()> {
    let src_dir = target_dir.join("src");
    fs::create_dir_all(&src_dir)
        .with_context(|| format!("failed to create directory {}", src_dir.display()))?;

    let cargo_toml = render_cargo_toml(crate_name, gpui_os_path, template);
    let main_rs = render_main_rs(crate_name, display_title, template);
    let readme_md = render_readme(crate_name, display_title, template);
    let cargo_config = render_cargo_config();

    fs::write(target_dir.join("Cargo.toml"), cargo_toml)
        .context("failed to write Cargo.toml")?;
    fs::write(src_dir.join("main.rs"), main_rs).context("failed to write src/main.rs")?;
    fs::write(target_dir.join("README.md"), readme_md).context("failed to write README.md")?;

    let dot_cargo_dir = target_dir.join(".cargo");
    fs::create_dir_all(&dot_cargo_dir).context("failed to create .cargo directory")?;
    fs::write(dot_cargo_dir.join("config.toml"), cargo_config)
        .context("failed to write .cargo/config.toml")?;

    Ok(())
}

fn sanitize_crate_name(raw_name: &str) -> Result<String> {
    let trimmed = raw_name.trim();
    ensure!(!trimmed.is_empty(), "Application name cannot be empty");

    let mut normalized = String::with_capacity(trimmed.len());
    for character in trimmed.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            normalized.push(character.to_ascii_lowercase());
        } else if character == '-' || character == ' ' {
            normalized.push('_');
        } else {
            bail!(
                "Invalid character '{character}' in application name '{raw_name}'. Use ASCII letters, digits, '-' or '_'."
            );
        }
    }

    if normalized
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit())
    {
        bail!("Application name '{raw_name}' cannot start with a digit");
    }

    Ok(normalized)
}

fn normalize_toml_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn render_cargo_config() -> &'static str {
    r#"[target.'cfg(all())']
rustflags = ["-C", "symbol-mangling-version=v0", "--cfg", "tokio_unstable"]

[target.'cfg(target_os = "windows")']
rustflags = ["--cfg", "windows_slim_errors", "-C", "target-feature=+crt-static"]
"#
}

fn render_cargo_toml(crate_name: &str, gpui_os_path: &str, template: AppTemplate) -> String {
    let dependencies = match template {
        AppTemplate::Minimal => format!(
            r#"gpui_ui = {{ path = "{gpui_os_path}/crates/gpui_ui" }}"#
        ),
        AppTemplate::Code => format!(
            r#"gpui_ui = {{ path = "{gpui_os_path}/crates/gpui_ui" }}
gpui_code = {{ path = "{gpui_os_path}/crates/gpui_code" }}"#
        ),
        AppTemplate::Studio => format!(
            r#"gpui_ui = {{ path = "{gpui_os_path}/crates/gpui_ui" }}
gpui_code = {{ path = "{gpui_os_path}/crates/gpui_code", features = ["full"] }}
gpui_app_shell = {{ path = "{gpui_os_path}/crates/gpui_app_shell" }}"#
        ),
        AppTemplate::NshStudio => format!(
            r#"gpui_ui = {{ path = "{gpui_os_path}/crates/gpui_ui", features = ["nsh-dev", "nsh-shader", "nsh-math", "nsh-tui"] }}
gpui_code = {{ path = "{gpui_os_path}/crates/gpui_code", features = ["full"] }}
gpui_app_shell = {{ path = "{gpui_os_path}/crates/gpui_app_shell", features = ["nsh"] }}"#
        ),
    };

    format!(
        r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2024"
publish = false

[[bin]]
name = "{crate_name}"
path = "src/main.rs"

[dependencies]
{dependencies}
"#
    )
}

fn render_main_rs(crate_name: &str, display_title: &str, template: AppTemplate) -> String {
    match template {
        AppTemplate::Minimal => format!(
            r#"use gpui_ui::gpui::TitlebarOptions;
use gpui_ui::prelude::*;

struct MainView {{
    counter: usize,
}}

impl Render for MainView {{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {{
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(cx.theme().colors().background)
            .child(Label::new("{display_title}").size(LabelSize::Large))
            .child(
                Label::new(format!("Clicks: {{}}", self.counter))
                    .color(Color::Muted),
            )
            .child(
                Button::new("increment", "Increment")
                    .style(ButtonStyle::Filled)
                    .on_click(cx.listener(|this, _event, _window, cx| {{
                        this.counter += 1;
                        cx.notify();
                    }})),
            )
    }}
}}

fn main() {{
    gpui_ui::application().run(|cx| {{
        gpui_ui::init(cx);

        let window_options = WindowOptions {{
            titlebar: Some(TitlebarOptions {{
                title: Some("{display_title}".into()),
                ..Default::default()
            }}),
            ..Default::default()
        }};

        if let Err(error) = cx.open_window(window_options, |_window, cx| {{
            cx.new(|_cx| MainView {{ counter: 0 }})
        }}) {{
            eprintln!("Failed to open window for {crate_name}: {{error:#}}");
        }}
    }});
}}
"#
        ),
        AppTemplate::Code => format!(
            r#"use gpui_code::Rope;
use gpui_ui::gpui::TitlebarOptions;
use gpui_ui::prelude::*;

struct CodePreviewView {{
    rope: Rope,
}}

impl Render for CodePreviewView {{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {{
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .bg(cx.theme().colors().editor_background)
            .child(Label::new("{display_title} — gpui_code Engine").size(LabelSize::Large))
            .child(
                Label::new(format!(
                    "Rope length: {{}} bytes ({{}} lines)",
                    self.rope.len(),
                    self.rope.max_point().row + 1
                ))
                .color(Color::Accent),
            )
            .child(Label::new(self.rope.to_string()).color(Color::Default))
    }}
}}

fn main() {{
    gpui_ui::application().run(|cx| {{
        gpui_ui::init(cx);
        let _languages = gpui_code::create_language_registry(cx);

        let window_options = WindowOptions {{
            titlebar: Some(TitlebarOptions {{
                title: Some("{display_title}".into()),
                ..Default::default()
            }}),
            ..Default::default()
        }};

        if let Err(error) = cx.open_window(window_options, |_window, cx| {{
            cx.new(|_cx| CodePreviewView {{
                rope: Rope::from("fn main() {{\n    println!(\"Powered by gpui_code!\");\n}}\n"),
            }})
        }}) {{
            eprintln!("Failed to open window for {crate_name}: {{error:#}}");
        }}
    }});
}}
"#
        ),
        AppTemplate::Studio => format!(
            r#"use std::sync::Arc;

use gpui_app_shell::gpui_ui::gpui;
use gpui_app_shell::gpui_ui::prelude::*;
use gpui_app_shell::workspace::dock::{{DockPosition, Panel, PanelEvent}};
use gpui_app_shell::workspace::{{AppState, Workspace}};
use gpui_app_shell::{{AppModule, AppShellBuilder, create_code_editor}};

actions!(
    {crate_name},
    [
        /// Toggles the custom domain panel.
        ToggleDomainPanel,
    ]
);

struct DomainPanel {{
    focus_handle: FocusHandle,
    position: DockPosition,
}}

impl DomainPanel {{
    fn new(cx: &mut Context<Self>) -> Self {{
        Self {{
            focus_handle: cx.focus_handle(),
            position: DockPosition::Right,
        }}
    }}
}}

impl EventEmitter<PanelEvent> for DomainPanel {{}}

impl Focusable for DomainPanel {{
    fn focus_handle(&self, _cx: &App) -> FocusHandle {{
        self.focus_handle.clone()
    }}
}}

impl Panel for DomainPanel {{
    fn persistent_name() -> &'static str {{
        "DomainPanel"
    }}

    fn panel_key() -> &'static str {{
        "{crate_name}_domain_panel"
    }}

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {{
        self.position
    }}

    fn position_is_valid(&self, position: DockPosition) -> bool {{
        matches!(
            position,
            DockPosition::Left | DockPosition::Right | DockPosition::Bottom
        )
    }}

    fn set_position(
        &mut self,
        position: DockPosition,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {{
        self.position = position;
        cx.notify();
    }}

    fn default_size(&self, _window: &Window, _cx: &App) -> Pixels {{
        px(320.0)
    }}

    fn icon(&self, _window: &Window, _cx: &App) -> Option<IconName> {{
        Some(IconName::BoltFilled)
    }}

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {{
        Some("{display_title} Inspector")
    }}

    fn toggle_action(&self) -> Box<dyn Action> {{
        ToggleDomainPanel.boxed_clone()
    }}

    fn activation_priority(&self) -> u32 {{
        10
    }}
}}

impl Render for DomainPanel {{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {{
        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .p_3()
            .gap_2()
            .bg(cx.theme().colors().panel_background)
            .child(Label::new("{display_title}").size(LabelSize::Large))
            .child(
                Label::new("Custom domain dock panel powered by gpui_app_shell.")
                    .color(Color::Muted)
                    .size(LabelSize::Small),
            )
    }}
}}

struct CustomDomainModule;

impl AppModule for CustomDomainModule {{
    fn name(&self) -> &'static str {{
        "{crate_name}-module"
    }}

    fn init(&self, _app_state: &Arc<AppState>, cx: &mut App) {{
        cx.bind_keys([KeyBinding::new("ctrl-alt-d", ToggleDomainPanel, None)]);
    }}

    fn configure_workspace(
        &self,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {{
        let panel = cx.new(DomainPanel::new);
        workspace.add_panel(panel, window, cx);
        workspace.register_action(|workspace, _: &ToggleDomainPanel, window, cx| {{
            workspace.toggle_panel_focus::<DomainPanel>(window, cx);
        }});

        let editor = create_code_editor(
            "// Welcome to {display_title}!\nfn main() {{\n    println!(\"Ready.\");\n}}\n",
            Some("Rust"),
            &workspace.app_state().languages.clone(),
            window,
            cx,
        );
        workspace.add_item_to_active_pane(Box::new(editor), None, true, window, cx);
    }}
}}

fn main() {{
    let custom_data_dir = std::env::temp_dir().join("{crate_name}-data");
    AppShellBuilder::new("{display_title}")
        .data_dir(custom_data_dir)
        .with_module(CustomDomainModule)
        .run();
}}
"#
        ),
        AppTemplate::NshStudio => format!(
            r#"use gpui_app_shell::AppShellBuilder;
use gpui_app_shell::nsh::NshModule;

fn main() {{
    let custom_data_dir = std::env::temp_dir().join("{crate_name}-data");
    AppShellBuilder::new("{display_title}")
        .data_dir(custom_data_dir)
        .with_module(NshModule::with_showcase())
        .run();
}}
"#
        ),
    }
}

fn render_readme(crate_name: &str, display_title: &str, template: AppTemplate) -> String {
    format!(
        r#"# {display_title} (`{crate_name}`)

Generated with `cargo xtask new-app --template {template:?}` from `gpui-os`.

## Run

```sh
cargo run
```
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_crate_name() {
        assert_eq!(sanitize_crate_name("My-Data Studio").unwrap(), "my_data_studio");
        assert!(sanitize_crate_name("").is_err());
        assert!(sanitize_crate_name("123app").is_err());
    }

    #[test]
    fn test_generate_app_templates() {
        let temp = tempfile::tempdir().unwrap();
        for (index, template) in [
            AppTemplate::Minimal,
            AppTemplate::Code,
            AppTemplate::Studio,
            AppTemplate::NshStudio,
        ]
        .into_iter()
        .enumerate()
        {
            let app_dir = temp.path().join(format!("sample_{index}"));
            generate_app_files(
                &app_dir,
                "sample_app",
                "Sample App",
                "C:/gpui-os",
                template,
            )
            .unwrap();
            assert!(app_dir.join("Cargo.toml").exists());
            assert!(app_dir.join("src/main.rs").exists());
            assert!(app_dir.join(".cargo/config.toml").exists());
        }
    }
}
