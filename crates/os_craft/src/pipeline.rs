use std::path::Path;

#[cfg(feature = "native-inspect")]
use object::{Object as _, ObjectSection as _, ObjectSymbol as _};
use serde::{Deserialize, Serialize};
#[cfg(feature = "wasm-inspect")]
use wasmparser::{Parser as WasmParser, Payload as WasmPayload};

const MAX_INSPECT_BYTES: usize = 64 * 1024 * 1024;
const PREVIEW_CAP_BYTES: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerSummary {
    pub name: String,
    pub visible: bool,
    pub width: u32,
    pub height: u32,
    pub blend_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryInspection {
    pub container: String,
    pub arch: String,
    pub entry_point: u64,
    pub sections: Vec<String>,
    pub symbol_count: usize,
    pub sample_symbols: Vec<String>,
    pub wasm_exports: Vec<String>,
    pub wasm_imports: Vec<String>,
}

/// Unified inspection verdict produced by `craft-sniff`, `craft-psd`, `craft-exr`, `craft-plist`, `image`, `usvg`, `object`, and `wasmparser`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionReport {
    pub source_name: String,
    pub byte_len: usize,
    pub format_name: String,
    pub family: String,
    pub mime_type: String,
    pub preferred_extension: String,
    pub shannon_entropy: f64,
    pub dimensions: Option<(u32, u32)>,
    pub layers: Vec<LayerSummary>,
    pub binary: Option<BinaryInspection>,
    pub plist_summary: Option<String>,
    pub hex_head: String,
    pub recommended_app: Option<String>,
    pub notes: Vec<String>,
}

/// Inspects a file on disk cleanly within `MAX_INSPECT_BYTES`.
pub fn inspect_path(
    path: &Path,
    catalog: Option<&crate::catalog::CraftCatalog>,
) -> anyhow::Result<InspectionReport> {
    let meta = std::fs::metadata(path)?;
    if meta.len() as usize > MAX_INSPECT_BYTES {
        let mut f = std::fs::File::open(path)?;
        let mut head = vec![0_u8; craft_sniff::HEAD.max(8192)];
        use std::io::Read as _;
        let read = f.read(&mut head)?;
        head.truncate(read);
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let mut report = inspect_bytes(&name, &head, Some(path), catalog);
        report.byte_len = meta.len() as usize;
        report
            .notes
            .push("Header-only inspection applied (file exceeds 64 MiB limit)".to_owned());
        return Ok(report);
    }

    let bytes = std::fs::read(path)?;
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    Ok(inspect_bytes(&name, &bytes, Some(path), catalog))
}

/// Runs pure-Rust magic identification, structural decoding, and entropy calculation over arbitrary bytes.
#[must_use]
pub fn inspect_bytes(
    name: &str,
    bytes: &[u8],
    path_hint: Option<&Path>,
    catalog: Option<&crate::catalog::CraftCatalog>,
) -> InspectionReport {
    let kind = craft_sniff::identify(bytes);
    #[cfg(feature = "codecs")]
    let (mut format_name, mut family, mut mime_type, mut ext) =
        classify_bytes(bytes, kind, path_hint);
    #[cfg(not(feature = "codecs"))]
    let (format_name, family, mime_type, ext) = classify_bytes(bytes, kind, path_hint);

    #[cfg(feature = "codecs")]
    let mut dimensions: Option<(u32, u32)> = None;
    #[cfg(not(feature = "codecs"))]
    let dimensions: Option<(u32, u32)> = None;

    #[cfg(feature = "codecs")]
    let mut layers: Vec<LayerSummary> = Vec::new();
    #[cfg(not(feature = "codecs"))]
    let layers: Vec<LayerSummary> = Vec::new();

    #[cfg(any(feature = "native-inspect", feature = "wasm-inspect"))]
    let mut binary: Option<BinaryInspection> = None;
    #[cfg(not(any(feature = "native-inspect", feature = "wasm-inspect")))]
    let binary: Option<BinaryInspection> = None;

    #[cfg(feature = "codecs")]
    let mut plist_summary: Option<String> = None;
    #[cfg(not(feature = "codecs"))]
    let plist_summary: Option<String> = None;

    #[cfg(feature = "codecs")]
    let mut notes: Vec<String> = Vec::new();
    #[cfg(not(feature = "codecs"))]
    let notes: Vec<String> = Vec::new();

    #[cfg(feature = "codecs")]
    {
        if format_name == "Psd"
            && let Ok(psd) = craft_psd::PsdFile::from_bytes(bytes)
        {
            dimensions = Some((psd.header.width, psd.header.height));
            for layer in psd.layers() {
                layers.push(LayerSummary {
                    name: layer.name().to_owned(),
                    visible: layer.is_visible(),
                    width: layer.rect.right.saturating_sub(layer.rect.left).max(0) as u32,
                    height: layer.rect.bottom.saturating_sub(layer.rect.top).max(0) as u32,
                    blend_mode: format!("{:?}", layer.blend_mode),
                });
            }
            notes.push(format!(
                "PSD v{:?} channels={} depth={:?}",
                psd.header.version, psd.header.channels, psd.header.depth
            ));
        }

        if format_name == "OpenExr"
            && let Ok(exr_parts) =
                craft_exr::decode::parts(bytes, &craft_exr::decode::Limits::default())
        {
            if let Some(first) = exr_parts.first() {
                dimensions = Some((first.display_window.width, first.display_window.height));
                notes.push(format!(
                    "OpenEXR parts={} compression={} deep={}",
                    exr_parts.len(),
                    first.compression,
                    first.deep
                ));
            }
            for part in exr_parts {
                let channel_names: Vec<&str> =
                    part.channels.iter().map(|c| c.name.as_str()).collect();
                layers.push(LayerSummary {
                    name: part.name.unwrap_or_else(|| {
                        format!("part_{} ({})", part.index, channel_names.join(","))
                    }),
                    visible: true,
                    width: part.data_window.width,
                    height: part.data_window.height,
                    blend_mode: part.compression,
                });
            }
        }

        if format_name == "BinaryPlist" || bytes.starts_with(b"bplist") {
            if let Ok(val) = craft_plist::parse_binary(bytes) {
                plist_summary = Some(format!("{val:?}"));
            }
        } else if bytes.trim_ascii_start().starts_with(b"<?xml")
            && let Ok(val) = craft_plist::parse_xml(bytes)
        {
            plist_summary = Some(format!("{val:?}"));
        }

        if family == "vector" || is_probable_svg(bytes) {
            let opts = usvg::Options::default();
            if let Ok(tree) = usvg::Tree::from_data(bytes, &opts) {
                let sz = tree.size();
                dimensions = Some((sz.width().round() as u32, sz.height().round() as u32));
                format_name = "Svg".to_owned();
                family = "vector".to_owned();
                mime_type = "image/svg+xml".to_owned();
                ext = "svg".to_owned();
                notes.push(format!("SVG viewBox {:.1}x{:.1}", sz.width(), sz.height()));
            }
        }

        if dimensions.is_none()
            && let Ok(reader) =
                image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()
            && let Ok(dims) = reader.into_dimensions()
        {
            dimensions = Some(dims);
        }
    }

    #[cfg(feature = "native-inspect")]
    if matches!(
        format_name.as_str(),
        "Elf" | "Pe" | "MachO" | "MachOFat" | "DosExecutable"
    ) || matches!(family.as_str(), "executable")
    {
        binary = parse_native_binary(bytes);
    }

    #[cfg(feature = "wasm-inspect")]
    if format_name == "Wasm" || bytes.starts_with(b"\0asm") {
        binary = parse_wasm_details(bytes).or(binary);
    }

    let lookup_path = path_hint
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from(format!("input.{ext}")));
    let recommended_app = catalog
        .and_then(|cat| cat.resolve_for_file(&lookup_path, Some(&family)))
        .map(|spec| spec.id.clone());

    InspectionReport {
        source_name: name.to_owned(),
        byte_len: bytes.len(),
        format_name,
        family,
        mime_type,
        preferred_extension: ext,
        shannon_entropy: shannon_entropy(bytes),
        dimensions,
        layers,
        binary,
        plist_summary,
        hex_head: format_hex_preview(bytes.get(..bytes.len().min(64)).unwrap_or(bytes)),
        recommended_app,
        notes,
    }
}

/// Rasterizes bounded SVG data directly into an RGBA8 buffer via `resvg`/`tiny-skia` for GPU preview thumbnails.
#[cfg(feature = "codecs")]
pub fn render_svg_rgba(bytes: &[u8], max_edge: u32) -> anyhow::Result<(u32, u32, Vec<u8>)> {
    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_data(bytes, &opts)?;
    let size = tree.size();
    let w = size.width().max(1.0);
    let h = size.height().max(1.0);
    let scale = (max_edge.max(1) as f32 / w.max(h)).min(4.0);
    let out_w = (w * scale).ceil().max(1.0) as u32;
    let out_h = (h * scale).ceil().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(out_w, out_h)
        .ok_or_else(|| anyhow::anyhow!("failed to allocate {out_w}x{out_h} SVG pixmap"))?;
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok((out_w, out_h, pixmap.take()))
}

fn classify_bytes(
    bytes: &[u8],
    kind: Option<craft_sniff::Kind>,
    path_hint: Option<&Path>,
) -> (String, String, String, String) {
    match kind {
        Some(k) => (
            format!("{k:?}"),
            family_slug(k.family()).to_owned(),
            k.mime_type().to_owned(),
            k.extension().to_owned(),
        ),
        None => {
            if is_probable_svg(bytes) {
                (
                    "Svg".to_owned(),
                    "vector".to_owned(),
                    "image/svg+xml".to_owned(),
                    "svg".to_owned(),
                )
            } else if std::str::from_utf8(bytes.get(..bytes.len().min(4096)).unwrap_or(bytes))
                .is_ok()
            {
                (
                    "PlainText".to_owned(),
                    "text".to_owned(),
                    "text/plain".to_owned(),
                    path_hint
                        .and_then(|p| p.extension())
                        .and_then(|e| e.to_str())
                        .unwrap_or("txt")
                        .to_ascii_lowercase(),
                )
            } else {
                (
                    "UnknownBinary".to_owned(),
                    "executable".to_owned(),
                    "application/octet-stream".to_owned(),
                    "bin".to_owned(),
                )
            }
        }
    }
}

fn family_slug(f: craft_sniff::Family) -> &'static str {
    use craft_sniff::Family;
    match f {
        Family::Image => "image",
        Family::Audio => "audio",
        Family::Video => "video",
        Family::Model => "model",
        Family::Archive | Family::Compressed => "archive",
        Family::Executable => "executable",
        Family::Database => "database",
        Family::Document => "document",
        _ => "binary",
    }
}

fn is_probable_svg(bytes: &[u8]) -> bool {
    let head = bytes
        .get(..bytes.len().min(PREVIEW_CAP_BYTES))
        .unwrap_or(bytes);
    std::str::from_utf8(head).is_ok_and(|text| text.contains("<svg"))
}

#[cfg(feature = "native-inspect")]
fn parse_native_binary(bytes: &[u8]) -> Option<BinaryInspection> {
    let file = object::File::parse(bytes).ok()?;
    let sections = file
        .sections()
        .take(32)
        .filter_map(|s| s.name().ok().map(str::to_owned))
        .filter(|n| !n.is_empty())
        .collect();
    let symbol_count = file.symbols().count();
    let sample_symbols = file
        .symbols()
        .filter_map(|s| s.name().ok().map(str::to_owned))
        .filter(|n| !n.is_empty())
        .take(16)
        .collect();
    Some(BinaryInspection {
        container: format!("{:?}", file.format()),
        arch: format!("{:?}", file.architecture()),
        entry_point: file.entry(),
        sections,
        symbol_count,
        sample_symbols,
        wasm_exports: Vec::new(),
        wasm_imports: Vec::new(),
    })
}

#[cfg(feature = "wasm-inspect")]
fn parse_wasm_details(bytes: &[u8]) -> Option<BinaryInspection> {
    let mut sections = Vec::new();
    let mut exports = Vec::new();
    let mut imports = Vec::new();

    for payload in WasmParser::new(0).parse_all(bytes) {
        let Ok(payload) = payload else {
            break;
        };
        match payload {
            WasmPayload::TypeSection(_) => sections.push("type".to_owned()),
            WasmPayload::ImportSection(reader) => {
                sections.push("import".to_owned());
                for item in reader.into_imports().take(24).flatten() {
                    imports.push(format!("{}::{}", item.module, item.name));
                }
            }
            WasmPayload::FunctionSection(_) => sections.push("function".to_owned()),
            WasmPayload::ExportSection(reader) => {
                sections.push("export".to_owned());
                for exp in reader.into_iter().take(24).flatten() {
                    exports.push(exp.name.to_owned());
                }
            }
            WasmPayload::CodeSectionStart { .. } => sections.push("code".to_owned()),
            WasmPayload::CustomSection(c) => sections.push(format!("custom:{}", c.name())),
            _ => {}
        }
    }

    if sections.is_empty() {
        return None;
    }

    Some(BinaryInspection {
        container: "WebAssembly".to_owned(),
        arch: "wasm32".to_owned(),
        entry_point: 0,
        sections,
        symbol_count: exports.len() + imports.len(),
        sample_symbols: Vec::new(),
        wasm_exports: exports,
        wasm_imports: imports,
    })
}

#[must_use]
pub fn shannon_entropy(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut counts = [0_u64; 256];
    for &b in bytes {
        counts[b as usize] += 1;
    }
    let total = bytes.len() as f64;
    let mut entropy = 0.0_f64;
    for count in counts {
        if count == 0 {
            continue;
        }
        let p = count as f64 / total;
        entropy -= p * p.log2();
    }
    entropy
}

fn format_hex_preview(slice: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(slice.len() * 3);
    for (idx, &byte) in slice.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_and_decodes_psd_svg_and_wasm_cleanly() {
        let catalog = crate::catalog::CraftCatalog::bundled().expect("catalog parses");

        let mut psd_builder = craft_psd::PsdBuilder::new(4, 4);
        psd_builder.push_layer(craft_psd::LayerSpec::new(
            "Background",
            0,
            0,
            4,
            4,
            craft_psd::PixelData::Rgba8(vec![255; 4 * 4 * 4]),
        ));
        psd_builder.composite(craft_psd::PixelData::Rgba8(vec![255; 4 * 4 * 4]));
        let psd_bytes = psd_builder.to_bytes().expect("psd builds");
        let psd_report = inspect_bytes("banner.psd", &psd_bytes, None, Some(&catalog));
        assert_eq!(psd_report.format_name, "Psd");
        assert_eq!(psd_report.dimensions, Some((4, 4)));
        assert_eq!(psd_report.layers.len(), 1);
        assert_eq!(psd_report.recommended_app.as_deref(), Some("photocraft"));

        let svg_bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"><rect width="120" height="80"/></svg>"#;
        let svg_report = inspect_bytes("icon.svg", svg_bytes, None, Some(&catalog));
        assert_eq!(svg_report.format_name, "Svg");
        assert_eq!(svg_report.dimensions, Some((120, 80)));
        assert_eq!(svg_report.recommended_app.as_deref(), Some("vectorcraft"));

        let (rw, rh, rgba) = render_svg_rgba(svg_bytes, 64).expect("svg rasterizes cleanly");
        assert!(rw == 64 && rh > 0 && rgba.len() == (rw * rh * 4) as usize);

        let wasm_bytes = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let wasm_report = inspect_bytes("module.wasm", &wasm_bytes, None, Some(&catalog));
        assert_eq!(wasm_report.format_name, "Wasm");
        assert_eq!(
            wasm_report.recommended_app.as_deref(),
            Some("craft-anything")
        );
    }
}
