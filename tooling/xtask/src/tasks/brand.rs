use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use clap::{Args, Subcommand};
use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, RgbaImage, imageops};
use serde::Deserialize;
use toml_edit::{Array, DocumentMut, Item, Value};

use crate::workspace::load_workspace;

#[derive(Subcommand, Debug)]
pub enum BrandCommand {
    /// Synchronize all static manifests, bundle metadata, desktop files, and multi-channel icons from `product.toml`.
    Apply(BrandApplyArgs),
    /// Verify in read-only mode that all manifests and icons match `product.toml` (CI check).
    Check(BrandCheckArgs),
}

#[derive(Args, Debug, Default)]
pub struct BrandApplyArgs {
    /// Skip regenerating PNG and ICO icon assets.
    #[arg(long)]
    pub skip_icons: bool,
}

#[derive(Args, Debug, Default)]
pub struct BrandCheckArgs {}

#[derive(Debug, Clone, Deserialize)]
#[allow(
    dead_code,
    reason = "Validates all required sections of root product.toml"
)]
pub struct ProductManifest {
    pub product: ProductSection,
    pub paths: PathsSection,
    pub urls: UrlsSection,
    pub env: EnvSection,
    pub fonts: FontsSection,
    pub assets: AssetsSection,
    pub plugin: PluginSection,
    pub git: GitSection,
    pub features: FeaturesSection,
    pub design: DesignSection,
    pub colors: ColorsSection,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DesignSection {
    pub source_of_truth: String,
    pub system: String,
    pub spec_url: String,
    pub fonts_url: String,
    pub stitch_url: String,
    pub sans_flex_family: String,
    pub design_md: String,
    pub tokens_manifest: String,
    pub themes_manifest: String,
    pub seed_hex: String,
    pub variant: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(
    dead_code,
    reason = "Validates all required fields of [product] in product.toml"
)]
pub struct ProductSection {
    pub name: String,
    pub slug: String,
    pub tagline: String,
    pub description: String,
    pub authors: Vec<String>,
    pub company_name: String,
    pub copyright: String,
    pub identifier_prefix: String,
    pub windows_app_id_prefix: String,
    pub url_scheme: String,
    pub cli_url_scheme: String,
    pub dock_action_scheme: String,
    pub legacy_url_schemes: Vec<String>,
    pub legacy_cli_url_schemes: Vec<String>,
    pub legacy_dock_action_schemes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct PathsSection {
    pub local_settings_folder: String,
    pub legacy_local_settings_folder: String,
    pub remote_server_folder: String,
    pub legacy_remote_server_folder: String,
    pub remote_wsl_server_folder: String,
    pub legacy_remote_wsl_server_folder: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct UrlsSection {
    pub homepage: String,
    pub repository: String,
    pub docs: String,
    pub status: String,
    pub store: String,
    pub server: String,
    pub extension_registry: String,
    #[serde(default)]
    pub releases: Option<String>,
    #[serde(default)]
    pub download: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct EnvSection {
    pub prefix: String,
    pub legacy_prefix: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct FontsSection {
    pub sans_alias: String,
    pub mono_alias: String,
    pub legacy_sans_alias: String,
    pub legacy_mono_alias: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetsSection {
    pub logo_svg: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginSection {
    pub name: String,
    pub version: String,
    pub scope: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitSection {
    pub default_branch: String,
    pub upstream_zed: String,
    pub upstream_zed_branch: String,
    pub upstream_nsh: String,
    pub upstream_gpui_kit: String,
    pub upstream_wgpu: String,
    #[serde(default)]
    pub upstream_awesome_gpui: String,
    #[serde(default)]
    pub upstream_extensions: String,
    #[serde(default)]
    pub upstream_winget_pkgs: String,
    #[serde(default)]
    pub upstream_modern_web_guidance: String,
    #[serde(default)]
    pub upstream_design_md: String,
    #[serde(default)]
    pub upstream_stitch_sdk: String,
    #[serde(default)]
    pub upstream_stitch_skills: String,
    #[serde(default)]
    pub upstream_agent_browser: String,
    #[serde(default)]
    pub upstream_agent_desktop: String,
    pub submodule_bun: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeaturesSection {
    pub default_app_features: Vec<String>,
    pub modular_ui_crate: String,
    pub modular_ui_features: String,
    pub modular_code_crate: String,
    pub modular_shell_crate: String,
    pub modular_shell_features: String,
    pub wgpu_cli_package: String,
    pub wgpu_cli_bin: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ColorsSection {
    pub channels: ChannelsColorSection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelsColorSection {
    #[serde(default)]
    pub lts: Option<ChannelPaletteConfig>,
    pub stable: ChannelPaletteConfig,
    pub preview: ChannelPaletteConfig,
    #[serde(default)]
    pub canary: Option<ChannelPaletteConfig>,
    pub nightly: ChannelPaletteConfig,
    pub dev: ChannelPaletteConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelPaletteConfig {
    pub top: [u8; 4],
    pub bottom: [u8; 4],
    pub foreground: [u8; 4],
}

#[derive(Clone, Copy)]
pub enum ChannelVariant {
    Stable,
    Preview,
    Nightly,
    Dev,
}

impl ChannelVariant {
    const ALL: [Self; 4] = [Self::Stable, Self::Preview, Self::Nightly, Self::Dev];

    fn bundle_section_name(self) -> &'static str {
        match self {
            Self::Stable => "bundle-stable",
            Self::Preview => "bundle-preview",
            Self::Nightly => "bundle-nightly",
            Self::Dev => "bundle-dev",
        }
    }

    fn file_suffix(self) -> &'static str {
        match self {
            Self::Stable => "",
            Self::Preview => "-preview",
            Self::Nightly => "-nightly",
            Self::Dev => "-dev",
        }
    }

    fn display_name(self, base_name: &str) -> String {
        match self {
            Self::Stable => base_name.to_owned(),
            Self::Preview => format!("{base_name} Preview"),
            Self::Nightly => format!("{base_name} Nightly"),
            Self::Dev => format!("{base_name} Dev"),
        }
    }

    fn identifier(self, id_prefix: &str) -> String {
        match self {
            Self::Stable => id_prefix.to_owned(),
            Self::Preview => format!("{id_prefix}-Preview"),
            Self::Nightly => format!("{id_prefix}-Nightly"),
            Self::Dev => format!("{id_prefix}-Dev"),
        }
    }

    /// Background gradient top/bottom RGBA and foreground glyph tint configured per channel in `product.toml`.
    fn palette(self, channels: &ChannelsColorSection) -> ([u8; 4], [u8; 4], [u8; 4]) {
        let cfg = match self {
            Self::Stable => &channels.stable,
            Self::Preview => &channels.preview,
            Self::Nightly => &channels.nightly,
            Self::Dev => &channels.dev,
        };
        (cfg.top, cfg.bottom, cfg.foreground)
    }
}

pub fn run_brand(command: BrandCommand) -> Result<()> {
    let workspace = load_workspace()?;
    let root = workspace.workspace_root.as_std_path();
    let manifest = load_product_manifest(root)?;

    match command {
        BrandCommand::Apply(args) => apply_brand(root, &manifest, !args.skip_icons),
        BrandCommand::Check(_args) => check_brand(root, &manifest),
    }
}

pub fn load_product_manifest(workspace_root: &Path) -> Result<ProductManifest> {
    let path = workspace_root.join("product.toml");
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let manifest: ProductManifest =
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
    for (name, palette) in [
        ("lts", manifest.colors.channels.lts.as_ref()),
        ("canary", manifest.colors.channels.canary.as_ref()),
    ] {
        if let Some(cfg) = palette {
            anyhow::ensure!(
                cfg.top[3] > 0 && cfg.bottom[3] > 0 && cfg.foreground[3] > 0,
                "[colors.channels.{name}] alpha channels must be non-zero"
            );
        }
    }
    Ok(manifest)
}

fn sync_manifest_files(
    root: &Path,
    manifest: &ProductManifest,
    write_changes: bool,
    changed: &mut Vec<PathBuf>,
) -> Result<()> {
    sync_text_file(
        &root.join("Cargo.toml"),
        |current| render_root_cargo_toml(current, manifest),
        write_changes,
        changed,
    )?;

    sync_text_file(
        &root.join("crates/zed/Cargo.toml"),
        |current| render_app_cargo_toml(current, manifest),
        write_changes,
        changed,
    )?;

    sync_text_file(
        &root.join("crates/zed/resources/zed.desktop.in"),
        |current| Ok(render_desktop_entry(current, manifest)),
        write_changes,
        changed,
    )?;

    sync_text_file(
        &root.join("crates/zed/resources/windows/zed.iss"),
        |current| Ok(render_inno_setup(current, manifest)),
        write_changes,
        changed,
    )?;

    let plugin_json = root.join(format!(
        ".agents/plugins/{}/plugin.json",
        manifest.plugin.name
    ));
    if plugin_json.is_file() {
        sync_text_file(
            &plugin_json,
            |current| render_plugin_manifest(current, manifest),
            write_changes,
            changed,
        )?;
    }

    let issue_config = root.join(".github/ISSUE_TEMPLATE/config.yml");
    if issue_config.is_file() {
        sync_text_file(
            &issue_config,
            |current| Ok(render_issue_template_config(current, manifest)),
            write_changes,
            changed,
        )?;
    }

    Ok(())
}

fn validate_manifest(manifest: &ProductManifest) -> Result<()> {
    let required = [
        ("urls.status", manifest.urls.status.as_str()),
        ("urls.store", manifest.urls.store.as_str()),
        ("plugin.name", manifest.plugin.name.as_str()),
        ("plugin.version", manifest.plugin.version.as_str()),
        ("plugin.scope", manifest.plugin.scope.as_str()),
        ("git.default_branch", manifest.git.default_branch.as_str()),
        ("git.upstream_zed", manifest.git.upstream_zed.as_str()),
        (
            "git.upstream_zed_branch",
            manifest.git.upstream_zed_branch.as_str(),
        ),
        ("git.upstream_nsh", manifest.git.upstream_nsh.as_str()),
        (
            "git.upstream_gpui_kit",
            manifest.git.upstream_gpui_kit.as_str(),
        ),
        ("git.upstream_wgpu", manifest.git.upstream_wgpu.as_str()),
        (
            "git.upstream_awesome_gpui",
            manifest.git.upstream_awesome_gpui.as_str(),
        ),
        (
            "git.upstream_extensions",
            manifest.git.upstream_extensions.as_str(),
        ),
        (
            "git.upstream_winget_pkgs",
            manifest.git.upstream_winget_pkgs.as_str(),
        ),
        (
            "git.upstream_modern_web_guidance",
            manifest.git.upstream_modern_web_guidance.as_str(),
        ),
        (
            "git.upstream_design_md",
            manifest.git.upstream_design_md.as_str(),
        ),
        (
            "git.upstream_stitch_sdk",
            manifest.git.upstream_stitch_sdk.as_str(),
        ),
        (
            "git.upstream_stitch_skills",
            manifest.git.upstream_stitch_skills.as_str(),
        ),
        ("git.submodule_bun", manifest.git.submodule_bun.as_str()),
        (
            "features.modular_ui_crate",
            manifest.features.modular_ui_crate.as_str(),
        ),
        (
            "features.modular_ui_features",
            manifest.features.modular_ui_features.as_str(),
        ),
        (
            "features.modular_code_crate",
            manifest.features.modular_code_crate.as_str(),
        ),
        (
            "features.modular_shell_crate",
            manifest.features.modular_shell_crate.as_str(),
        ),
        (
            "features.modular_shell_features",
            manifest.features.modular_shell_features.as_str(),
        ),
        (
            "features.wgpu_cli_package",
            manifest.features.wgpu_cli_package.as_str(),
        ),
        (
            "features.wgpu_cli_bin",
            manifest.features.wgpu_cli_bin.as_str(),
        ),
    ];
    for (field, value) in required {
        anyhow::ensure!(
            !value.trim().is_empty(),
            "product.toml field '{field}' cannot be empty"
        );
    }
    anyhow::ensure!(
        !manifest.features.default_app_features.is_empty(),
        "product.toml field 'features.default_app_features' cannot be empty"
    );
    anyhow::ensure!(
        manifest.design.source_of_truth == "https://design.google/",
        "product.toml field 'design.source_of_truth' must be 'https://design.google/'"
    );
    Ok(())
}

fn apply_brand(root: &Path, manifest: &ProductManifest, generate_icons: bool) -> Result<()> {
    validate_manifest(manifest)?;
    anyhow::ensure!(
        root.join(&manifest.design.tokens_manifest).is_file(),
        "missing design tokens manifest: {}",
        manifest.design.tokens_manifest
    );
    anyhow::ensure!(
        root.join(&manifest.design.themes_manifest).is_file(),
        "missing design themes manifest: {}",
        manifest.design.themes_manifest
    );
    anyhow::ensure!(
        root.join(&manifest.design.design_md).is_file(),
        "missing root DESIGN.md manifest: {}",
        manifest.design.design_md
    );
    let mut updated_files = Vec::new();
    sync_manifest_files(root, manifest, true, &mut updated_files)?;

    if generate_icons {
        let icon_paths = generate_all_channel_icons(root, manifest)?;
        updated_files.extend(icon_paths);
    }

    println!(
        "Brand '{}' ({}) synchronized across {} file(s).",
        manifest.product.name,
        manifest.product.slug,
        updated_files.len()
    );
    Ok(())
}

fn check_brand(root: &Path, manifest: &ProductManifest) -> Result<()> {
    validate_manifest(manifest)?;
    let mut drifted = Vec::new();
    sync_manifest_files(root, manifest, false, &mut drifted)?;

    let tokens_path = root.join(&manifest.design.tokens_manifest);
    if !tokens_path.is_file() {
        drifted.push(tokens_path);
    }
    let themes_path = root.join(&manifest.design.themes_manifest);
    if !themes_path.is_file() {
        drifted.push(themes_path);
    }
    let design_md_path = root.join(&manifest.design.design_md);
    if !design_md_path.is_file() {
        drifted.push(design_md_path);
    }

    let svg_path = root.join(&manifest.assets.logo_svg);
    if !svg_path.is_file() {
        drifted.push(svg_path);
    }

    for channel in ChannelVariant::ALL {
        let suffix = channel.file_suffix();
        let png_1x = root.join(format!("crates/zed/resources/app-icon{suffix}.png"));
        let png_2x = root.join(format!("crates/zed/resources/app-icon{suffix}@2x.png"));
        let ico = root.join(format!("crates/zed/resources/windows/app-icon{suffix}.ico"));
        for required in [png_1x, png_2x, ico] {
            if !required.is_file() {
                drifted.push(required);
            }
        }
    }

    if !drifted.is_empty() {
        let list = drifted
            .iter()
            .map(|path| format!("  - {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "Brand drift detected relative to `product.toml` in {} file(s):\n{}\nRun `cargo xtask brand apply` to synchronize.",
            drifted.len(),
            list
        );
    }

    println!(
        "Brand check OK: all manifests and icons match '{}' (`product.toml`).",
        manifest.product.name
    );
    Ok(())
}

fn sync_text_file(
    path: &Path,
    transform: impl FnOnce(&str) -> Result<String>,
    write_changes: bool,
    changed: &mut Vec<PathBuf>,
) -> Result<()> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let uses_crlf = raw.contains("\r\n");
    let current = raw.replace("\r\n", "\n");
    let expected = transform(&current)?.replace("\r\n", "\n");
    if current != expected {
        if write_changes {
            let formatted = if uses_crlf {
                expected.replace('\n', "\r\n")
            } else {
                expected
            };
            fs::write(path, formatted)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
        changed.push(path.to_path_buf());
    }
    Ok(())
}

pub fn render_root_cargo_toml(current: &str, manifest: &ProductManifest) -> Result<String> {
    let mut doc: DocumentMut = current
        .parse()
        .context("failed to parse root Cargo.toml with toml_edit")?;

    if let Some(ws_pkg) = doc
        .get_mut("workspace")
        .and_then(Item::as_table_like_mut)
        .and_then(|ws| ws.get_mut("package"))
        .and_then(Item::as_table_like_mut)
    {
        ws_pkg.insert(
            "homepage",
            toml_edit::value(manifest.urls.homepage.as_str()),
        );
        ws_pkg.insert(
            "repository",
            toml_edit::value(manifest.urls.repository.as_str()),
        );
    }

    Ok(doc.to_string())
}

pub fn render_app_cargo_toml(current: &str, manifest: &ProductManifest) -> Result<String> {
    let mut doc: DocumentMut = current
        .parse()
        .context("failed to parse crates/zed/Cargo.toml with toml_edit")?;

    if let Some(pkg) = doc.get_mut("package").and_then(Item::as_table_like_mut) {
        pkg.insert(
            "description",
            toml_edit::value(manifest.product.description.as_str()),
        );
        let mut authors_arr = Array::new();
        for author in &manifest.product.authors {
            authors_arr.push(author.as_str());
        }
        pkg.insert("authors", Item::Value(Value::Array(authors_arr)));
        pkg.insert(
            "default-run",
            toml_edit::value(manifest.product.slug.as_str()),
        );
    }

    if let Some(bins) = doc.get_mut("bin").and_then(Item::as_array_of_tables_mut) {
        for bin_table in bins.iter_mut() {
            let is_main = bin_table
                .get("path")
                .and_then(Item::as_str)
                .is_some_and(|p| p == "src/main.rs");
            if is_main {
                bin_table.insert("name", toml_edit::value(manifest.product.slug.as_str()));
            }
        }
    }

    let mut all_schemes = Vec::with_capacity(1 + manifest.product.legacy_url_schemes.len());
    all_schemes.push(manifest.product.url_scheme.clone());
    for legacy in &manifest.product.legacy_url_schemes {
        if !all_schemes.contains(legacy) {
            all_schemes.push(legacy.clone());
        }
    }

    if let Some(metadata) = doc
        .get_mut("package")
        .and_then(Item::as_table_like_mut)
        .and_then(|pkg| pkg.get_mut("metadata"))
        .and_then(Item::as_table_like_mut)
    {
        for channel in ChannelVariant::ALL {
            if let Some(bundle_tbl) = metadata
                .get_mut(channel.bundle_section_name())
                .and_then(Item::as_table_like_mut)
            {
                let id = channel.identifier(&manifest.product.identifier_prefix);
                let display = channel.display_name(&manifest.product.name);
                bundle_tbl.insert("identifier", toml_edit::value(id));
                bundle_tbl.insert("name", toml_edit::value(display));

                let mut schemes_arr = Array::new();
                for scheme in &all_schemes {
                    schemes_arr.push(scheme.as_str());
                }
                bundle_tbl.insert("osx_url_schemes", Item::Value(Value::Array(schemes_arr)));
            }
        }
    }

    Ok(doc.to_string())
}

fn render_desktop_entry(current: &str, manifest: &ProductManifest) -> String {
    let mut schemes = vec![format!("x-scheme-handler/{}", manifest.product.url_scheme)];
    for legacy in &manifest.product.legacy_url_schemes {
        let handler = format!("x-scheme-handler/{legacy}");
        if !schemes.contains(&handler) {
            schemes.push(handler);
        }
    }
    let mime_line = format!(
        "MimeType=text/plain;application/x-zerosize;{};",
        schemes.join(";")
    );
    let mut keywords = vec![manifest.product.slug.clone()];
    for legacy in &manifest.product.legacy_url_schemes {
        if !keywords.contains(legacy) {
            keywords.push(legacy.clone());
        }
    }
    let keywords_line = format!("Keywords={};", keywords.join(";"));
    let comment_line = format!("Comment={}", manifest.product.description);

    let mut out_lines = Vec::new();
    for line in current.lines() {
        if line.starts_with("Comment=") {
            out_lines.push(comment_line.clone());
        } else if line.starts_with("Keywords=") {
            out_lines.push(keywords_line.clone());
        } else if line.starts_with("MimeType=") {
            out_lines.push(mime_line.clone());
        } else {
            out_lines.push(line.to_owned());
        }
    }
    let mut joined = out_lines.join("\n");
    if current.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

fn render_inno_setup(current: &str, manifest: &ProductManifest) -> String {
    let line_ending = if current.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut out_lines = Vec::new();
    for line in current.lines() {
        if line.starts_with("AppPublisher=") {
            out_lines.push(format!("AppPublisher={}", manifest.product.company_name));
        } else if line.starts_with("AppPublisherURL=") {
            out_lines.push(format!("AppPublisherURL={}", manifest.urls.homepage));
        } else if line.starts_with("AppSupportURL=") {
            out_lines.push(format!("AppSupportURL={}", manifest.urls.docs));
        } else if line.starts_with("AppUpdatesURL=") {
            out_lines.push(format!("AppUpdatesURL={}", manifest.urls.homepage));
        } else {
            out_lines.push(line.to_owned());
        }
    }
    let mut joined = out_lines.join(line_ending);
    if current.ends_with('\n') {
        joined.push_str(line_ending);
    }
    joined
}

fn render_plugin_manifest(current: &str, manifest: &ProductManifest) -> Result<String> {
    let mut json: serde_json::Value =
        serde_json::from_str(current).context("failed to parse plugin.json")?;
    if let Some(obj) = json.as_object_mut() {
        obj.insert(
            "name".to_owned(),
            serde_json::Value::String(manifest.plugin.name.clone()),
        );
        obj.insert(
            "version".to_owned(),
            serde_json::Value::String(manifest.plugin.version.clone()),
        );
        obj.insert(
            "homepage".to_owned(),
            serde_json::Value::String(manifest.urls.homepage.clone()),
        );
        let author = obj.entry("author").or_insert_with(|| serde_json::json!({}));
        if let Some(author_obj) = author.as_object_mut() {
            author_obj.insert(
                "name".to_owned(),
                serde_json::Value::String(manifest.product.company_name.clone()),
            );
            author_obj.insert(
                "url".to_owned(),
                serde_json::Value::String(manifest.urls.homepage.clone()),
            );
        }
        let repo = obj
            .entry("repository")
            .or_insert_with(|| serde_json::json!({"type": "git"}));
        if let Some(repo_obj) = repo.as_object_mut() {
            repo_obj.insert(
                "type".to_owned(),
                serde_json::Value::String("git".to_owned()),
            );
            repo_obj.insert(
                "url".to_owned(),
                serde_json::Value::String(manifest.urls.repository.clone()),
            );
        }
    }
    let mut rendered = serde_json::to_string_pretty(&json)?;
    rendered.push('\n');
    Ok(rendered)
}

fn render_issue_template_config(current: &str, manifest: &ProductManifest) -> String {
    let repo = manifest.urls.repository.trim_end_matches('/');
    let mut out_lines = Vec::new();
    let mut last_name = String::new();
    for line in current.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("- name:") {
            last_name = name.trim().to_owned();
            out_lines.push(line.to_owned());
        } else if trimmed.starts_with("url:") && last_name.eq_ignore_ascii_case("Discussions") {
            let indent = &line[..line.len() - line.trim_start().len()];
            out_lines.push(format!("{indent}url: {repo}/discussions"));
        } else if trimmed.starts_with("url:") && last_name.eq_ignore_ascii_case("Project Wiki") {
            let indent = &line[..line.len() - line.trim_start().len()];
            out_lines.push(format!("{indent}url: {repo}/wiki"));
        } else {
            out_lines.push(line.to_owned());
        }
    }
    let mut joined = out_lines.join("\n");
    if current.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

fn generate_all_channel_icons(root: &Path, manifest: &ProductManifest) -> Result<Vec<PathBuf>> {
    let svg_path = root.join(&manifest.assets.logo_svg);
    let svg_bytes = fs::read(&svg_path)
        .with_context(|| format!("failed to read brand SVG at {}", svg_path.display()))?;

    let mut written = Vec::new();

    for channel in ChannelVariant::ALL {
        let suffix = channel.file_suffix();
        let img_1024 =
            render_channel_icon_rgba(&svg_bytes, 1024, channel, &manifest.colors.channels)?;
        let img_512 = imageops::resize(&img_1024, 512, 512, imageops::FilterType::Lanczos3);

        let png_1x_path = root.join(format!("crates/zed/resources/app-icon{suffix}.png"));
        let png_2x_path = root.join(format!("crates/zed/resources/app-icon{suffix}@2x.png"));
        write_png_if_changed(&png_1x_path, &img_512, &mut written)?;
        write_png_if_changed(&png_2x_path, &img_1024, &mut written)?;

        let ico_bytes = encode_multi_size_ico(&img_1024)?;
        let ico_path = root.join(format!("crates/zed/resources/windows/app-icon{suffix}.ico"));
        write_bytes_if_changed(&ico_path, &ico_bytes, &mut written)?;

        if matches!(channel, ChannelVariant::Stable) {
            let helper_ico = root.join("crates/auto_update_helper/app-icon.ico");
            if helper_ico.parent().is_some_and(Path::exists) {
                write_bytes_if_changed(&helper_ico, &ico_bytes, &mut written)?;
            }
        }
    }

    Ok(written)
}

pub fn render_channel_icon_rgba(
    svg_bytes: &[u8],
    canvas_size: u32,
    channel: ChannelVariant,
    channels: &ChannelsColorSection,
) -> Result<RgbaImage> {
    let (bg_top, bg_bottom, fg_tint) = channel.palette(channels);
    let mut canvas = RgbaImage::new(canvas_size, canvas_size);

    let size_f = canvas_size as f32;
    let margin = size_f * 0.0625;
    let rect_min = margin;
    let rect_max = size_f - margin;
    let corner_radius = size_f * 0.215;

    for y in 0..canvas_size {
        let t = (y as f32) / (canvas_size.saturating_sub(1).max(1) as f32);
        let r = lerp_u8(bg_top[0], bg_bottom[0], t);
        let g = lerp_u8(bg_top[1], bg_bottom[1], t);
        let b = lerp_u8(bg_top[2], bg_bottom[2], t);

        for x in 0..canvas_size {
            let coverage = rounded_rect_coverage(
                x as f32 + 0.5,
                y as f32 + 0.5,
                rect_min,
                rect_max,
                corner_radius,
            );
            if coverage > 0.0 {
                let alpha = (coverage * 255.0).round().clamp(0.0, 255.0) as u8;
                canvas.put_pixel(x, y, image::Rgba([r, g, b, alpha]));
            }
        }
    }

    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg_bytes, &options)
        .context("failed to parse brand logo SVG via usvg")?;
    let svg_size = tree.size();

    let target_glyph_px = (size_f * 0.62).round().max(1.0) as u32;
    let mut glyph_pixmap = tiny_skia::Pixmap::new(target_glyph_px, target_glyph_px)
        .context("failed to allocate tiny_skia Pixmap for icon glyph")?;
    let scale_x = (target_glyph_px as f32) / svg_size.width();
    let scale_y = (target_glyph_px as f32) / svg_size.height();
    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);
    resvg::render(&tree, transform, &mut glyph_pixmap.as_mut());

    let offset = (canvas_size - target_glyph_px) / 2;
    let glyph_data = glyph_pixmap.data();
    for gy in 0..target_glyph_px {
        for gx in 0..target_glyph_px {
            let idx = ((gy * target_glyph_px + gx) * 4) as usize;
            let src_a = glyph_data[idx + 3];
            if src_a == 0 {
                continue;
            }
            let alpha_norm = (src_a as f32) / 255.0;
            let dst_x = offset + gx;
            let dst_y = offset + gy;
            let dst = canvas.get_pixel_mut(dst_x, dst_y);
            let out_r = lerp_u8(dst[0], fg_tint[0], alpha_norm);
            let out_g = lerp_u8(dst[1], fg_tint[1], alpha_norm);
            let out_b = lerp_u8(dst[2], fg_tint[2], alpha_norm);
            let out_a = dst[3].max(src_a);
            *dst = image::Rgba([out_r, out_g, out_b, out_a]);
        }
    }

    Ok(canvas)
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    ((a as f32) * (1.0 - t) + (b as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn rounded_rect_coverage(px: f32, py: f32, min_xy: f32, max_xy: f32, radius: f32) -> f32 {
    let cx = (min_xy + max_xy) * 0.5;
    let cy = (min_xy + max_xy) * 0.5;
    let half = (max_xy - min_xy) * 0.5 - radius;
    let dx = (px - cx).abs() - half;
    let dy = (py - cy).abs() - half;
    let outside = (dx.max(0.0).hypot(dy.max(0.0))) + dx.max(dy).min(0.0) - radius;
    (0.5 - outside).clamp(0.0, 1.0)
}

pub fn encode_multi_size_ico(base_rgba: &RgbaImage) -> Result<Vec<u8>> {
    let sizes = [16_u32, 32, 48, 256];
    let mut frames = Vec::with_capacity(sizes.len());
    for size in sizes {
        let resized = imageops::resize(base_rgba, size, size, imageops::FilterType::Lanczos3);
        let frame = IcoFrame::as_png(resized.as_raw(), size, size, ExtendedColorType::Rgba8)
            .with_context(|| format!("failed to encode {size}x{size} ICO frame"))?;
        frames.push(frame);
    }

    let mut buffer = Cursor::new(Vec::new());
    IcoEncoder::new(&mut buffer)
        .encode_images(&frames)
        .context("failed to encode multi-resolution ICO")?;
    Ok(buffer.into_inner())
}

fn write_png_if_changed(path: &Path, img: &RgbaImage, written: &mut Vec<PathBuf>) -> Result<()> {
    let mut png_bytes = Vec::new();
    PngEncoder::new(&mut png_bytes)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::Rgba8,
        )
        .with_context(|| format!("failed to encode PNG for {}", path.display()))?;
    write_bytes_if_changed(path, &png_bytes, written)
}

fn write_bytes_if_changed(path: &Path, bytes: &[u8], written: &mut Vec<PathBuf>) -> Result<()> {
    let unchanged = fs::read(path).is_ok_and(|existing| existing == bytes);
    if !unchanged {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
        written.push(path.to_path_buf());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manifest() -> ProductManifest {
        ProductManifest {
            product: ProductSection {
                name: "Aphrody".into(),
                slug: "aphrody".into(),
                tagline: "GPU-native code & UI studio".into(),
                description: "Ultra-fast studio".into(),
                authors: vec!["Aphrody Labs <dev@aphrody.com>".into()],
                company_name: "Aphrody Labs".into(),
                copyright: "Copyright 2026 Aphrody Labs".into(),
                identifier_prefix: "com.aphrody.Aphrody".into(),
                windows_app_id_prefix: "Aphrody-Studio".into(),
                url_scheme: "aphrody".into(),
                cli_url_scheme: "aphrody-cli".into(),
                dock_action_scheme: "aphrody-dock-action".into(),
                legacy_url_schemes: vec!["zed".into()],
                legacy_cli_url_schemes: vec!["zed-cli".into()],
                legacy_dock_action_schemes: vec!["zed-dock-action".into()],
            },
            paths: PathsSection {
                local_settings_folder: ".aphrody".into(),
                legacy_local_settings_folder: ".zed".into(),
                remote_server_folder: ".aphrody_server".into(),
                legacy_remote_server_folder: ".zed_server".into(),
                remote_wsl_server_folder: ".aphrody_wsl_server".into(),
                legacy_remote_wsl_server_folder: ".zed_wsl_server".into(),
            },
            urls: UrlsSection {
                homepage: "https://aphrody.com".into(),
                repository: "https://github.com/aphrody-dev/aphrody".into(),
                docs: "https://docs.aphrody.com".into(),
                status: "https://status.aphrody.com".into(),
                store: "https://aphrody.com/store".into(),
                server: "https://zed.dev".into(),
                extension_registry: "https://api.zed.dev".into(),
                releases: Some("https://releases.aphrody.com".into()),
                download: Some("https://aphrody.com/download".into()),
            },
            env: EnvSection {
                prefix: "APHRODY".into(),
                legacy_prefix: "ZED".into(),
            },
            fonts: FontsSection {
                sans_alias: ".AphrodySans".into(),
                mono_alias: ".AphrodyMono".into(),
                legacy_sans_alias: ".ZedSans".into(),
                legacy_mono_alias: ".ZedMono".into(),
            },
            assets: AssetsSection {
                logo_svg: "assets/images/aphrody_logo.svg".into(),
            },
            plugin: PluginSection {
                name: "aphrody".into(),
                version: "1.30.0".into(),
                scope: "@aphrody".into(),
            },
            git: GitSection {
                default_branch: "main".into(),
                upstream_zed: "https://github.com/zed-industries/zed.git".into(),
                upstream_zed_branch: "main".into(),
                upstream_nsh: "https://github.com/longbridge/nsh.git".into(),
                upstream_gpui_kit: "https://github.com/longbridge/gpui-kit.git".into(),
                upstream_wgpu: "https://github.com/gfx-rs/wgpu.git".into(),
                upstream_awesome_gpui: "https://github.com/zed-industries/awesome-gpui.git".into(),
                upstream_extensions: "https://github.com/zed-industries/extensions.git".into(),
                upstream_winget_pkgs: "https://github.com/zed-industries/winget-pkgs.git".into(),
                upstream_modern_web_guidance: "https://github.com/GoogleChrome/modern-web-guidance-src.git".into(),
                upstream_design_md: "https://github.com/google-labs-code/design-md.git".into(),
                upstream_stitch_sdk: "https://github.com/google-labs-code/stitch-sdk.git".into(),
                upstream_stitch_skills: "https://github.com/google-labs-code/stitch-skills.git".into(),
                submodule_bun: "https://github.com/aphrody-labs/bun".into(),
            },
            features: FeaturesSection {
                default_app_features: vec!["full".into()],
                modular_ui_crate: "gpui_ui".into(),
                modular_ui_features: "nsh-full".into(),
                modular_code_crate: "gpui_code".into(),
                modular_shell_crate: "gpui_app_shell".into(),
                modular_shell_features: "nsh".into(),
                wgpu_cli_package: "aphrody_cli".into(),
                wgpu_cli_bin: "gpui-wgpu".into(),
            },
            design: DesignSection {
                source_of_truth: "https://design.google/".into(),
                system: "Material 3 Expressive".into(),
                spec_url: "https://m3.material.io/".into(),
                fonts_url: "https://fonts.google.com/".into(),
                stitch_url: "https://stitch.withgoogle.com/".into(),
                sans_flex_family: "Google Sans Flex".into(),
                design_md: "DESIGN.md".into(),
                tokens_manifest: "assets/os/google_design_tokens.toml".into(),
                themes_manifest: "assets/os/themes.json".into(),
                seed_hex: "#135B82".into(),
                variant: "expressive".into(),
            },
            colors: ColorsSection {
                channels: ChannelsColorSection {
                    lts: Some(ChannelPaletteConfig {
                        top: [14, 30, 48, 255],
                        bottom: [8, 16, 28, 255],
                        foreground: [186, 230, 253, 255],
                    }),
                    stable: ChannelPaletteConfig {
                        top: [18, 22, 36, 255],
                        bottom: [10, 12, 20, 255],
                        foreground: [236, 244, 255, 255],
                    },
                    preview: ChannelPaletteConfig {
                        top: [38, 20, 62, 255],
                        bottom: [18, 10, 32, 255],
                        foreground: [233, 213, 255, 255],
                    },
                    canary: Some(ChannelPaletteConfig {
                        top: [54, 18, 36, 255],
                        bottom: [26, 8, 18, 255],
                        foreground: [251, 207, 232, 255],
                    }),
                    nightly: ChannelPaletteConfig {
                        top: [42, 28, 14, 255],
                        bottom: [20, 14, 8, 255],
                        foreground: [253, 224, 138, 255],
                    },
                    dev: ChannelPaletteConfig {
                        top: [14, 38, 36, 255],
                        bottom: [8, 20, 20, 255],
                        foreground: [153, 246, 228, 255],
                    },
                },
            },
        }
    }

    #[test]
    fn test_app_cargo_toml_branding_is_idempotent_and_preserves_zed_scheme() -> Result<()> {
        let sample_manifest = test_manifest();

        let input = r#"[package]
name = "zed"
version = "1.30.0"
description = "Old"
authors = ["Old"]
default-run = "zed"

[[bin]]
name = "zed"
path = "src/main.rs"

[package.metadata.bundle-stable]
identifier = "dev.zed.Zed"
name = "Zed"
osx_url_schemes = ["zed"]
"#;

        let once = render_app_cargo_toml(input, &sample_manifest)?;
        let twice = render_app_cargo_toml(&once, &sample_manifest)?;
        assert_eq!(once, twice, "render_app_cargo_toml must be idempotent");
        assert!(once.contains(r#"default-run = "aphrody""#));
        assert!(once.contains(r#"name = "aphrody""#));
        assert!(once.contains(r#"identifier = "com.aphrody.Aphrody""#));
        assert!(once.contains(r#""aphrody", "zed""#));
        Ok(())
    }

    #[test]
    fn test_svg_to_rgba_and_ico_generation() -> Result<()> {
        let minimal_svg = br#"<svg width="96" height="96" viewBox="0 0 96 96" xmlns="http://www.w3.org/2000/svg"><circle cx="48" cy="48" r="32" fill="black"/></svg>"#;
        let manifest = test_manifest();
        let rgba = render_channel_icon_rgba(
            minimal_svg,
            256,
            ChannelVariant::Stable,
            &manifest.colors.channels,
        )?;
        assert_eq!(rgba.dimensions(), (256, 256));
        let ico = encode_multi_size_ico(&rgba)?;
        assert!(ico.starts_with(&[0, 0, 1, 0]), "valid ICO magic header");
        Ok(())
    }

    #[test]
    fn test_plugin_and_issue_config_render_from_manifest() -> Result<()> {
        let manifest = test_manifest();
        validate_manifest(&manifest)?;
        let plugin = render_plugin_manifest(
            r#"{"name":"old","version":"0.0.1","author":{"name":"Old","url":"https://old"},"homepage":"https://old","repository":{"type":"git","url":"https://old"}}"#,
            &manifest,
        )?;
        assert!(plugin.contains("https://github.com/aphrody-dev/aphrody"));
        assert!(plugin.contains("Aphrody Labs"));

        let issue_cfg = render_issue_template_config(
            "contact_links:\n  - name: Discussions\n    url: https://old/discussions\n  - name: Project Wiki\n    url: https://old/wiki\n",
            &manifest,
        );
        assert!(issue_cfg.contains("https://github.com/aphrody-dev/aphrody/discussions"));
        assert!(issue_cfg.contains("https://github.com/aphrody-dev/aphrody/wiki"));
        Ok(())
    }

    #[test]
    fn test_sync_text_file_is_crlf_resilient() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let cargo_path = dir.path().join("Cargo.toml");
        let lf = "[workspace.package]\nhomepage = \"https://aphrody.com\"\nrepository = \"https://github.com/aphrody-dev/aphrody\"\n";
        let manifest = test_manifest();
        fs::write(&cargo_path, lf.replace('\n', "\r\n"))?;
        let mut drifted = Vec::new();
        sync_text_file(
            &cargo_path,
            |current| render_root_cargo_toml(current, &manifest),
            false,
            &mut drifted,
        )?;
        assert!(
            drifted.is_empty(),
            "CRLF checkout must not report brand drift"
        );
        Ok(())
    }
}
