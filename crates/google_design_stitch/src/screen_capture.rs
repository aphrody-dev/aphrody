//! Pure-Rust deterministic offscreen PNG screen capture for Google Design (`DESIGN.md`)
//! cards and Google Stitch (`StitchScreen`) layouts using `image::RgbaImage`.

use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};
use image::{ImageFormat, Rgba, RgbaImage};

use crate::design_md::{DesignMdDocument, parse_hex_rgb};
use crate::stitch_client::StitchScreen;

fn hex_to_rgba(doc: &DesignMdDocument, key: &str, fallback: [u8; 4]) -> Rgba<u8> {
    let rgba = doc
        .colors
        .get(key)
        .and_then(|hex| parse_hex_rgb(hex))
        .map_or(fallback, |(r, g, b)| [r, g, b, 255]);
    Rgba(rgba)
}

fn fill_rounded_rect(
    img: &mut RgbaImage,
    x0: u32,
    y0: u32,
    w: u32,
    h: u32,
    radius: u32,
    color: Rgba<u8>,
) {
    let max_x = (x0 + w).min(img.width());
    let max_y = (y0 + h).min(img.height());
    let r = radius.min(w / 2).min(h / 2) as i32;

    for y in y0..max_y {
        for x in x0..max_x {
            if r > 0 {
                let dx = if x < x0 + r as u32 {
                    (x0 + r as u32) as i32 - x as i32
                } else if x >= max_x.saturating_sub(r as u32) {
                    x as i32 - (max_x - r as u32 - 1) as i32
                } else {
                    0
                };
                let dy = if y < y0 + r as u32 {
                    (y0 + r as u32) as i32 - y as i32
                } else if y >= max_y.saturating_sub(r as u32) {
                    y as i32 - (max_y - r as u32 - 1) as i32
                } else {
                    0
                };
                if dx * dx + dy * dy > r * r {
                    continue;
                }
            }
            img.put_pixel(x, y, color);
        }
    }
}

/// Renders a Material 3 Expressive visual preview + color/container hierarchy PNG for a Stitch screen.
pub fn capture_stitch_screen_png(
    doc: &DesignMdDocument,
    _screen: &StitchScreen,
    width: u32,
    height: u32,
    out_path: &Path,
) -> Result<()> {
    let w = width.max(320);
    let h = height.max(200);
    let bg = hex_to_rgba(doc, "surface", [14, 20, 27, 255]);
    let container_low = hex_to_rgba(doc, "surface-container-low", [22, 29, 37, 255]);
    let container_high = hex_to_rgba(doc, "surface-container-high", [37, 44, 54, 255]);
    let primary = hex_to_rgba(doc, "primary", [143, 208, 255, 255]);
    let secondary = hex_to_rgba(doc, "secondary", [180, 201, 229, 255]);
    let tertiary = hex_to_rgba(doc, "tertiary", [198, 194, 255, 255]);
    let on_surface = hex_to_rgba(doc, "on-surface", [224, 230, 240, 255]);

    let mut img = RgbaImage::from_pixel(w, h, bg);

    // Top Material 3 navigation rail + app bar card
    let rail_w = (w / 7).clamp(56, 96);
    fill_rounded_rect(&mut img, 16, 16, rail_w, h.saturating_sub(32), 16, container_low);
    fill_rounded_rect(&mut img, 28, 32, rail_w.saturating_sub(24), 40, 20, primary);
    fill_rounded_rect(&mut img, 28, 88, rail_w.saturating_sub(24), 40, 20, secondary);

    let main_x = rail_w + 32;
    let main_w = w.saturating_sub(main_x + 16);
    fill_rounded_rect(&mut img, main_x, 16, main_w, 64, 16, container_high);
    fill_rounded_rect(&mut img, main_x + 20, 36, (main_w / 3).max(80), 22, 11, on_surface);
    fill_rounded_rect(&mut img, w.saturating_sub(148), 28, 112, 40, 20, primary);

    // Hero expressive container + swatch strip
    let hero_y = 96;
    let hero_h = h.saturating_sub(hero_y + 24);
    fill_rounded_rect(&mut img, main_x, hero_y, main_w, hero_h, 24, container_low);

    let swatch_w = ((main_w.saturating_sub(64)) / 4).max(32);
    let swatch_y = hero_y + 24;
    let swatch_h = (hero_h / 2).max(48);
    fill_rounded_rect(&mut img, main_x + 16, swatch_y, swatch_w, swatch_h, 16, primary);
    fill_rounded_rect(&mut img, main_x + 28 + swatch_w, swatch_y, swatch_w, swatch_h, 16, secondary);
    fill_rounded_rect(
        &mut img,
        main_x + 40 + swatch_w * 2,
        swatch_y,
        swatch_w,
        swatch_h,
        16,
        tertiary,
    );
    fill_rounded_rect(
        &mut img,
        main_x + 52 + swatch_w * 3,
        swatch_y,
        swatch_w.saturating_sub(4),
        swatch_h,
        16,
        container_high,
    );

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    img.save_with_format(out_path, ImageFormat::Png)
        .with_context(|| format!("writing PNG capture to {}", out_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design_md::load_bundled_root_design_md;
    use crate::stitch_client::StitchClient;

    #[test]
    fn captures_valid_png_screen_preview() -> Result<()> {
        let doc = load_bundled_root_design_md()?;
        let client = StitchClient::new("https://stitch.withgoogle.com", None);
        let screen = client.synthesize_screen("Preview", "DESKTOP", &doc, false)?;
        let dir = tempfile::tempdir()?;
        let out = dir.path().join("capture.png");
        capture_stitch_screen_png(&doc, &screen, 640, 360, &out)?;
        let bytes = fs::read(&out)?;
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.len() > 200);
        Ok(())
    }
}
