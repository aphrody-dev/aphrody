use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};
use aphrody_design::design_md::{
    diff_design_md, export_css_variables, export_shadcn_components_json, export_shadcn_tailwind_css,
    export_w3c_tokens_json, export_wgsl_theme_struct, lint_design_md, load_bundled_root_design_md,
    parse_design_md,
};
use aphrody_design::http_mirror::{
    fetch_or_fallback_article, mirror_design_google_sitemap, mirror_to_single_file_html,
};
use aphrody_design::modern_web_guidance;
use aphrody_design::screen_capture::capture_stitch_screen_png;
use aphrody_design::stitch_client::{StitchClient, stitch_screen_to_nsh_tsx};
use aphrody_design::web_standards::audit_project_standards;

const SUBMODULE_PARITY_MATRIX_TOML: &str =
    include_str!("../../../assets/os/submodule_parity_matrix.toml");

#[derive(Debug, Subcommand)]
pub(crate) enum DesignSubcommand {
    /// Pure-Rust live mirror of https://design.google/... (curl/wget/monolith replacement)
    Mirror(MirrorArgs),
    /// Inspect https://design.google/sitemap.xml entries
    Sitemap(SitemapArgs),
    /// Parse, lint, diff, or export DESIGN.md tokens
    DesignMd(DesignMdArgs),
    /// Google Stitch (stitch.withgoogle.com) routes, TSX synthesis, and PNG screen capture
    Stitch {
        #[command(subcommand)]
        action: StitchSubcommand,
    },
    /// Audit HTML/web files against GoogleChrome modern-web-guidance Baseline rules
    WebGuidance(WebGuidanceArgs),
    /// Audit HTML/web directories against Core Web Vitals, W3C, Google Style, Vercel & Lighthouse
    Lighthouse(LighthouseArgs),
    /// Print the Fork Submodule Parity Feature Matrix & CWV budgets
    ParityMatrix {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Args)]
pub(crate) struct MirrorArgs {
    #[arg(default_value = "https://design.google/library/code-is-a-design-material")]
    pub url: String,
    #[arg(long)]
    pub single_file: bool,
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub offline: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct SitemapArgs {
    #[arg(long)]
    pub filter: Option<String>,
    #[arg(long)]
    pub live: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct DesignMdArgs {
    /// Action: lint, css, w3c, wgsl, diff
    #[arg(default_value = "lint")]
    pub action: String,
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub against: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum StitchSubcommand {
    Sitemap {
        #[arg(long)]
        json: bool,
    },
    Generate {
        prompt: String,
        #[arg(long, default_value = "DESKTOP")]
        device: String,
        #[arg(long)]
        tsx: bool,
    },
    Capture {
        #[arg(default_value = "Aphrody Google Design Studio")]
        prompt: String,
        #[arg(short = 'o', long = "output")]
        output: PathBuf,
        #[arg(long, default_value_t = 1280)]
        width: u32,
        #[arg(long, default_value_t = 720)]
        height: u32,
    },
}

#[derive(Debug, Args)]
pub(crate) struct WebGuidanceArgs {
    #[arg(default_value = ".")]
    pub dir: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct LighthouseArgs {
    #[arg(default_value = "templates/wasm-wgpu")]
    pub dir: PathBuf,
    #[arg(long)]
    pub json: bool,
}

pub(crate) fn run(cmd: DesignSubcommand) -> Result<()> {
    match cmd {
        DesignSubcommand::Mirror(args) => {
            if args.single_file {
                let (html, article) = mirror_to_single_file_html(&args.url, None)?;
                if let Some(out) = args.output {
                    if let Some(parent) = out.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&out, html.as_bytes())?;
                    println!(
                        "Mirrored single-file HTML ({} bytes) -> {}",
                        html.len(),
                        out.display()
                    );
                } else if args.json {
                    println!("{}", serde_json::to_string_pretty(&article)?);
                } else {
                    println!("{} — {}", article.title, article.description);
                }
            } else {
                let article = fetch_or_fallback_article(&args.url, None, !args.offline)?;
                if let Some(out) = args.output {
                    if let Some(parent) = out.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&out, serde_json::to_string_pretty(&article)?)?;
                }
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&article)?);
                } else {
                    println!(
                        "{} (live={}): {}\nHeadings: {}",
                        article.title,
                        article.live_fetched,
                        article.description,
                        article.headings.join(" | ")
                    );
                }
            }
            Ok(())
        }
        DesignSubcommand::Sitemap(args) => {
            let mut urls = mirror_design_google_sitemap(None, args.live)?;
            if let Some(q) = args.filter.as_deref() {
                let q_low = q.to_ascii_lowercase();
                urls.retain(|u| u.loc.to_ascii_lowercase().contains(&q_low));
            }
            if args.json {
                println!("{}", serde_json::to_string_pretty(&urls)?);
            } else {
                println!("Sitemap URLs ({}):", urls.len());
                for u in urls.iter().take(25) {
                    println!("  {} [{}]", u.loc, u.lastmod);
                }
            }
            Ok(())
        }
        DesignSubcommand::DesignMd(args) => {
            let doc = match args.path {
                Some(ref p) => parse_design_md(&fs::read_to_string(p)?)?,
                None => load_bundled_root_design_md()?,
            };
            match args.action.as_str() {
                "lint" => {
                    let report = lint_design_md(&doc);
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                "css" => println!("{}", export_css_variables(&doc)),
                "shadcn" | "tailwind" => println!("{}", export_shadcn_tailwind_css(&doc)),
                "components-json" => println!(
                    "{}",
                    serde_json::to_string_pretty(&export_shadcn_components_json(&doc))?
                ),
                "w3c" => println!("{}", serde_json::to_string_pretty(&export_w3c_tokens_json(&doc))?),
                "wgsl" => println!("{}", export_wgsl_theme_struct(&doc)),
                "diff" => {
                    let other_raw = match args.against {
                        Some(p) => fs::read_to_string(p)?,
                        None => aphrody_design::BUNDLED_ROOT_DESIGN_MD.to_owned(),
                    };
                    let other = parse_design_md(&other_raw)?;
                    println!("{}", serde_json::to_string_pretty(&diff_design_md(&doc, &other))?);
                }
                other => anyhow::bail!("unknown design-md action `{other}` (expected lint|css|shadcn|tailwind|components-json|w3c|wgsl|diff)"),
            }
            Ok(())
        }
        DesignSubcommand::Stitch { action } => {
            let doc = load_bundled_root_design_md()?;
            let client = StitchClient::default();
            match action {
                StitchSubcommand::Sitemap { json } => {
                    let routes = StitchClient::stitch_sitemap_routes();
                    if json {
                        println!("{}", serde_json::to_string_pretty(&routes)?);
                    } else {
                        for r in routes {
                            println!("{:<28} {:<32} [{}]", r.path, r.title, r.capability);
                        }
                    }
                }
                StitchSubcommand::Generate { prompt, device, tsx } => {
                    let screen = client.synthesize_screen(&prompt, &device, &doc, false)?;
                    if tsx {
                        println!("{}", stitch_screen_to_nsh_tsx(&screen));
                    } else {
                        println!("{}", screen.html_code);
                    }
                }
                StitchSubcommand::Capture {
                    prompt,
                    output,
                    width,
                    height,
                } => {
                    let screen = client.synthesize_screen(&prompt, "DESKTOP", &doc, false)?;
                    capture_stitch_screen_png(&doc, &screen, width, height, &output)?;
                    println!(
                        "Captured Stitch screen `{}` -> {}",
                        screen.stitch_id,
                        output.display()
                    );
                }
            }
            Ok(())
        }
        DesignSubcommand::WebGuidance(args) => {
            let report = modern_web_guidance::audit_directory(&args.dir)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "modern-web-guidance: ok={} files_checked={} findings={}",
                    report.ok,
                    report.files_checked,
                    report.findings.len()
                );
            }
            if !report.ok {
                anyhow::bail!("modern-web-guidance found errors");
            }
            Ok(())
        }
        DesignSubcommand::Lighthouse(args) => {
            let doc = load_bundled_root_design_md()?;
            let report = audit_project_standards(&args.dir, &doc)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Lighthouse/CWV Audit ({} files): ok={} perf={} a11y={} best={} seo={} overall={}",
                    report.files_checked,
                    report.ok,
                    report.scores.performance,
                    report.scores.accessibility,
                    report.scores.best_practices,
                    report.scores.seo_agentic,
                    report.scores.overall,
                );
            }
            if !report.ok {
                anyhow::bail!("lighthouse/CWV audit did not pass threshold");
            }
            Ok(())
        }
        DesignSubcommand::ParityMatrix { json } => {
            if json {
                let parsed: toml::Value = toml::from_str(SUBMODULE_PARITY_MATRIX_TOML)?;
                println!("{}", serde_json::to_string_pretty(&parsed)?);
            } else {
                println!("{SUBMODULE_PARITY_MATRIX_TOML}");
            }
            Ok(())
        }
    }
}
