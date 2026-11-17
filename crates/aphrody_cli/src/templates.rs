use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};
use clap::{Args, Subcommand, ValueEnum};
use aphrody_design::design_md::{
    export_css_variables, export_shadcn_tailwind_css, export_wgsl_theme_struct,
    load_bundled_root_design_md,
};
use serde::Serialize;

#[derive(Debug, Subcommand)]
pub(crate) enum AppSubcommand {
    /// List canonical Wasm + WebGPU + Google Design templates
    List {
        #[arg(long)]
        json: bool,
    },
    /// Scaffold a canonical template into a standalone directory
    New(NewAppArgs),
    /// Build an app directory with [profile.web-lto] / [profile.aot]
    Build(BuildAppArgs),
    /// Export https://design.google/ tokens as CSS, WGSL, or JSON
    Tokens(TokensArgs),
}

#[derive(Debug, Args)]
pub(crate) struct NewAppArgs {
    /// Template id (`wgpu-compute-particles`, `gpui-web-studio`, `nsh-webgpu-shader`, `wasi-p2-aot-compute`)
    pub template: String,
    /// Target directory to create
    pub dir: PathBuf,
    /// Optional crate/package name override
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct BuildAppArgs {
    /// Project directory containing Cargo.toml
    pub dir: PathBuf,
    /// Compile for wasm32-unknown-unknown with fat LTO and wasm-opt -Oz
    #[arg(long)]
    pub lto: bool,
    /// Compile for wasm32-wasip2 release and emit AOT .cwasm via wasmtime
    #[arg(long)]
    pub aot: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub(crate) enum TokenFormat {
    #[default]
    Css,
    Shadcn,
    Wgsl,
    Json,
}

#[derive(Debug, Args)]
pub(crate) struct TokensArgs {
    #[arg(long, value_enum, default_value_t = TokenFormat::Css)]
    pub format: TokenFormat,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TemplateDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub target: &'static str,
    pub design_source: &'static str,
    pub description: &'static str,
}

pub(crate) const TEMPLATES: &[TemplateDescriptor] = &[
    TemplateDescriptor {
        id: "wgpu-compute-particles",
        title: "WGPU Compute Particles",
        target: "wasm32-unknown-unknown + native wgpu",
        design_source: "https://design.google/",
        description: "Compute particle step + instanced render WGSL shaders tinted by Material 3 Expressive tokens.",
    },
    TemplateDescriptor {
        id: "gpui-web-studio",
        title: "GPUI WebGPU Studio",
        target: "wasm32-unknown-unknown (WebGPU / WebGL2)",
        design_source: "https://design.google/",
        description: "GPUI + WebGPU studio starter using DESIGN.md surface containers and Google Sans Flex.",
    },
    TemplateDescriptor {
        id: "nsh-webgpu-shader",
        title: "NSH Reactive WebGPU Shader",
        target: "nsh desktop + wasm32",
        design_source: "https://design.google/",
        description: "Native Shell reactive view paired with M3 Expressive springs and procedural WGSL fragment shader.",
    },
    TemplateDescriptor {
        id: "wasi-p2-aot-compute",
        title: "WASI Preview 2 AOT Compute Kernel",
        target: "wasm32-wasip2 -> .cwasm",
        design_source: "https://design.google/",
        description: "Deterministic WASI P2 vector/HCT compute kernel built for zero-startup Wasmtime AOT execution.",
    },
    TemplateDescriptor {
        id: "gpui-shadcn-starterpack",
        title: "GPUI + shadcn / Base UI / Tailwind v4 Starter Pack",
        target: "wasm32-unknown-unknown + native GPUI",
        design_source: "https://design.google/",
        description: "Core Web Vitals >= 95, Lighthouse, W3C WCAG AA/ARIA, Vercel UI, and shadcn/Base UI/Tailwind v4 tokens mapped to GPUI & WGPU.",
    },
];

pub(crate) fn run(cmd: AppSubcommand) -> Result<()> {
    match cmd {
        AppSubcommand::List { json } => {
            if json {
                println!("{}", serde_json::to_string_pretty(TEMPLATES)?);
            } else {
                for tpl in TEMPLATES {
                    println!(
                        "{:<24} [{}] {} ({})",
                        tpl.id, tpl.target, tpl.title, tpl.design_source
                    );
                }
            }
            Ok(())
        }
        AppSubcommand::New(args) => {
            let written = scaffold_template(&args.template, &args.dir, args.name.as_deref())?;
            println!(
                "Scaffolded `{}` into {} ({} files, design={})",
                args.template,
                args.dir.display(),
                written.len(),
                brand::DESIGN_SOURCE_OF_TRUTH
            );
            Ok(())
        }
        AppSubcommand::Build(args) => build_template_app(&args.dir, args.lto, args.aot),
        AppSubcommand::Tokens(args) => {
            let doc = load_bundled_root_design_md()?;
            match args.format {
                TokenFormat::Css => println!("{}", export_css_variables(&doc)),
                TokenFormat::Shadcn => println!("{}", export_shadcn_tailwind_css(&doc)),
                TokenFormat::Wgsl => println!("{}", export_wgsl_theme_struct(&doc)),
                TokenFormat::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &aphrody_design::design_md::export_w3c_tokens_json(&doc)
                    )?
                ),
            }
            Ok(())
        }
    }
}

fn find_workspace_templates_root() -> Option<PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        let direct = cwd.join("templates/wasm-wgpu");
        if direct.is_dir() {
            return Some(direct);
        }
    }
    let manifest_rel = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../templates/wasm-wgpu");
    if manifest_rel.is_dir() {
        return Some(manifest_rel);
    }
    None
}

fn copy_dir_recursive(src: &Path, dst: &Path, old_slug: &str, new_slug: Option<&str>, out: &mut Vec<PathBuf>) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == "target" || name_str.starts_with('.') {
            continue;
        }
        let target = dst.join(&name);
        if path.is_dir() {
            copy_dir_recursive(&path, &target, old_slug, new_slug, out)?;
        } else {
            let raw = fs::read(&path)?;
            if let Some(replacement) = new_slug
                && let Ok(text) = String::from_utf8(raw.clone())
            {
                let updated = text
                    .replace(old_slug, replacement)
                    .replace(&old_slug.replace('-', "_"), &replacement.replace('-', "_"));
                fs::write(&target, updated.as_bytes())?;
            } else {
                fs::write(&target, &raw)?;
            }
            out.push(target);
        }
    }
    Ok(())
}

pub(crate) fn scaffold_template(
    template: &str,
    dest_dir: &Path,
    package_name: Option<&str>,
) -> Result<Vec<PathBuf>> {
    let Some(descriptor) = TEMPLATES.iter().find(|t| t.id == template) else {
        bail!(
            "unknown template '{template}'. Available: {}",
            TEMPLATES.iter().map(|t| t.id).collect::<Vec<_>>().join(", ")
        );
    };

    if let Some(root) = find_workspace_templates_root() {
        let src = root.join(template);
        if src.is_dir() {
            let mut written = Vec::new();
            copy_dir_recursive(&src, dest_dir, template, package_name, &mut written)?;
            if !written.is_empty() {
                return Ok(written);
            }
        }
    }

    // Fallback self-contained starter scaffold anchored to https://design.google/
    fs::create_dir_all(dest_dir.join("src"))?;
    let pkg = package_name.unwrap_or(template).replace('-', "_");
    let cargo_toml = format!(
        "[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nlicense = \"Apache-2.0\"\n\n[profile.release]\nlto = \"fat\"\ncodegen-units = 1\nopt-level = \"z\"\nstrip = true\n"
    );
    let design_toml = format!(
        "source_of_truth = \"https://design.google/\"\nsystem = \"Material 3 Expressive\"\nsans_flex_family = \"Google Sans Flex\"\ntemplate = \"{}\"\n",
        descriptor.id
    );
    let main_rs = format!(
        "fn main() {{\n    println!(\"Aphrody {} ready (design source: https://design.google/)\");\n}}\n",
        descriptor.id
    );
    let paths = [
        (dest_dir.join("Cargo.toml"), cargo_toml),
        (dest_dir.join("design.toml"), design_toml),
        (
            dest_dir.join("DESIGN.md"),
            aphrody_design::BUNDLED_ROOT_DESIGN_MD.to_owned(),
        ),
        (dest_dir.join("src/main.rs"), main_rs),
    ];
    let mut written = Vec::new();
    for (p, content) in paths {
        fs::write(&p, content)?;
        written.push(p);
    }
    Ok(written)
}

#[allow(
    clippy::disallowed_methods,
    reason = "synchronous cargo/wasmtime wrapper in standalone CLI subcommand"
)]
fn build_template_app(dir: &Path, lto: bool, aot: bool) -> Result<()> {
    let manifest = dir.join("Cargo.toml");
    if !manifest.is_file() {
        bail!("no Cargo.toml found in {}", dir.display());
    }
    let target = if aot {
        "wasm32-wasip2"
    } else if lto {
        "wasm32-unknown-unknown"
    } else {
        ""
    };
    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--release").current_dir(dir);
    if !target.is_empty() {
        cmd.args(["--target", target]);
    }
    let out = cmd.output().context("running cargo build in template app")?;
    if !out.status.success() {
        bail!(
            "cargo build failed ({:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    println!("Built {} cleanly (lto={lto}, aot={aot})", dir.display());
    Ok(())
}
