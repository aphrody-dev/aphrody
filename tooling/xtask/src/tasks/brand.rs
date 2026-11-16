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
#[allow(dead_code, reason = "Validates all required sections of root product.toml")]
pub struct ProductManifest {
    pub product: ProductSection,
    pub paths: PathsSection,
    pub urls: UrlsSection,
    pub env: EnvSection,
    pub fonts: FontsSection,
    pub assets: AssetsSection,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code, reason = "Validates all required fields of [product] in product.toml")]
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
    pub server: String,
    pub extension_registry: String,
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

    /// Background gradient top/bottom RGBA and foreground glyph tint per release channel.
    fn palette(self) -> ([u8; 4], [u8; 4], [u8; 4]) {
        match self {
            // Stable: deep obsidian-indigo surface with crisp ice-white glyph
            Self::Stable => ([18, 22, 36, 255], [10, 12, 20, 255], [236, 244, 255, 255]),
            // Preview: royal violet surface with soft lavender-white glyph
            Self::Preview => ([38, 20, 62, 255], [18, 10, 32, 255], [233, 213, 255, 255]),
            // Nightly: warm midnight-amber surface with golden-amber glyph
            Self::Nightly => ([42, 28, 14, 255], [20, 14, 8, 255], [253, 224, 138, 255]),
            // Dev: slate-emerald surface with mint-cyan glyph
            Self::Dev => ([14, 38, 36, 255], [8, 20, 20, 255], [153, 246, 228, 255]),
        }
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
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn apply_brand(root: &Path, manifest: &ProductManifest, generate_icons: bool) -> Result<()> {
    let mut updated_files = Vec::new();

    sync_text_file(
        &root.join("Cargo.toml"),
        |current| render_root_cargo_toml(current, manifest),
        true,
        &mut updated_files,
    )?;

    sync_text_file(
        &root.join("crates/zed/Cargo.toml"),
        |current| render_app_cargo_toml(current, manifest),
        true,
        &mut updated_files,
    )?;

    sync_text_file(
        &root.join("crates/zed/resources/zed.desktop.in"),
        |current| Ok(render_desktop_entry(current, manifest)),
        true,
        &mut updated_files,
    )?;

    sync_text_file(
        &root.join("crates/zed/resources/windows/zed.iss"),
        |current| Ok(render_inno_setup(current, manifest)),
        true,
        &mut updated_files,
    )?;

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
    let mut drifted = Vec::new();

    sync_text_file(
        &root.join("Cargo.toml"),
        |current| render_root_cargo_toml(current, manifest),
        false,
        &mut drifted,
    )?;

    sync_text_file(
        &root.join("crates/zed/Cargo.toml"),
        |current| render_app_cargo_toml(current, manifest),
        false,
        &mut drifted,
    )?;

    sync_text_file(
        &root.join("crates/zed/resources/zed.desktop.in"),
        |current| Ok(render_desktop_entry(current, manifest)),
        false,
        &mut drifted,
    )?;

    sync_text_file(
        &root.join("crates/zed/resources/windows/zed.iss"),
        |current| Ok(render_inno_setup(current, manifest)),
        false,
        &mut drifted,
    )?;

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
        ws_pkg.insert("homepage", toml_edit::value(manifest.urls.homepage.as_str()));
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
    let line_ending = if current.contains("\r\n") { "\r\n" } else { "\n" };
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

fn generate_all_channel_icons(root: &Path, manifest: &ProductManifest) -> Result<Vec<PathBuf>> {
    let svg_path = root.join(&manifest.assets.logo_svg);
    let svg_bytes = fs::read(&svg_path)
        .with_context(|| format!("failed to read brand SVG at {}", svg_path.display()))?;

    let mut written = Vec::new();

    for channel in ChannelVariant::ALL {
        let suffix = channel.file_suffix();
        let img_1024 = render_channel_icon_rgba(&svg_bytes, 1024, channel)?;
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
) -> Result<RgbaImage> {
    let (bg_top, bg_bottom, fg_tint) = channel.palette();
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

    let offset = ((canvas_size - target_glyph_px) / 2) as u32;
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
        let frame = IcoFrame::as_png(
            resized.as_raw(),
            size,
            size,
            ExtendedColorType::Rgba8,
        )
        .with_context(|| format!("failed to encode {size}x{size} ICO frame"))?;
        frames.push(frame);
    }

    let mut buffer = Cursor::new(Vec::new());
    IcoEncoder::new(&mut buffer)
        .encode_images(&frames)
        .context("failed to encode multi-resolution ICO")?;
    Ok(buffer.into_inner())
}

fn write_png_if_changed(
    path: &Path,
    img: &RgbaImage,
    written: &mut Vec<PathBuf>,
) -> Result<()> {
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

fn write_bytes_if_changed(
    path: &Path,
    bytes: &[u8],
    written: &mut Vec<PathBuf>,
) -> Result<()> {
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
                repository: "https://github.com/aphrody-dev/gpui-os".into(),
                docs: "https://docs.aphrody.com".into(),
                server: "https://zed.dev".into(),
                extension_registry: "https://api.zed.dev".into(),
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
        let rgba = render_channel_icon_rgba(minimal_svg, 256, ChannelVariant::Stable)?;
        assert_eq!(rgba.dimensions(), (256, 256));
        let ico = encode_multi_size_ico(&rgba)?;
        assert!(ico.starts_with(&[0, 0, 1, 0]), "valid ICO magic header");
        Ok(())
    }

    #[test]
    fn test_sync_text_file_is_crlf_resilient() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let cargo_path = dir.path().join("Cargo.toml");
        let lf = "[workspace.package]\nhomepage = \"https://aphrody.com\"\nrepository = \"https://github.com/aphrody-dev/gpui-os\"\n";
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
