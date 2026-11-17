//! Headless, Wasm-safe, and GPUI-embeddable `egui 0.37` host, IME JSON wire adapter,
//! tessellation profiler, and deterministic RGBA8 rasterizer.

use anyhow::Result;
use ecolor::Color32;
use egui::{ClippedPrimitive, Context, Event, FullOutput, ImeEvent, Pos2, RawInput, Rect, Ui};
use epaint::Primitive;
use aphrody_ui::AppearanceMode;
use serde::{Deserialize, Serialize};
use web_time::Instant;

use crate::theme_bridge::apply_google_design_theme;

/// JSON-serializable input & IME wire event compatible with `craft-ui` / `storytold` IME test suites.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImeWireEvent {
    Preedit {
        text: String,
        cursor_start: Option<usize>,
        cursor_end: Option<usize>,
    },
    Commit {
        text: String,
    },
    DeleteSurrounding {
        before_chars: usize,
        after_chars: usize,
    },
    PointerMoved {
        x: f32,
        y: f32,
    },
    PointerPrimaryClick {
        x: f32,
        y: f32,
        pressed: bool,
    },
    Scroll {
        dx: f32,
        dy: f32,
    },
}

impl ImeWireEvent {
    #[must_use]
    pub fn into_egui_event(self) -> Event {
        match self {
            Self::Preedit {
                text,
                cursor_start,
                cursor_end,
            } => {
                let active_range_chars = match (cursor_start, cursor_end) {
                    (Some(s), Some(e)) => Some(s..e),
                    _ => None,
                };
                Event::Ime(ImeEvent::Preedit {
                    text,
                    active_range_chars,
                })
            }
            Self::Commit { text } => Event::Ime(ImeEvent::Commit(text)),
            Self::DeleteSurrounding {
                before_chars,
                after_chars,
            } => Event::Ime(ImeEvent::DeleteSurrounding {
                before_chars,
                after_chars,
            }),
            Self::PointerMoved { x, y } => Event::PointerMoved(Pos2::new(x, y)),
            Self::PointerPrimaryClick { x, y, pressed } => Event::PointerButton {
                pos: Pos2::new(x, y),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
            Self::Scroll { dx, dy } => Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: emath::vec2(dx, dy),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::NONE,
            },
        }
    }
}

/// Per-frame telemetry emitted by [`EguiGpuiHost`], directly comparable against GPUI frame stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EguiFrameStats {
    pub frame_index: u64,
    pub build_and_tessellate_us: u64,
    pub draw_calls: usize,
    pub vertices: usize,
    pub indices: usize,
    pub triangles: usize,
    pub texture_uploads: usize,
    pub accesskit_nodes: usize,
    pub width_px: u32,
    pub height_px: u32,
}

/// Complete output of one `egui` pass hosted inside Zed / Aphrody.
pub struct EguiFrameArtifact {
    pub stats: EguiFrameStats,
    pub primitives: Vec<ClippedPrimitive>,
    pub output_summary_cursor: String,
}

/// Retained `egui::Context` wrapper synchronized to Google Design tokens and embeddable in GPUI views.
#[derive(Clone)]
pub struct EguiGpuiHost {
    ctx: Context,
    mode: AppearanceMode,
    frame_index: u64,
    queued_events: Vec<Event>,
}

impl EguiGpuiHost {
    /// Creates a new `egui 0.37` host pre-configured with Material 3 Expressive tokens and AccessKit enabled.
    pub fn new(mode: AppearanceMode) -> Result<Self> {
        let ctx = Context::default();
        ctx.enable_accesskit();
        apply_google_design_theme(&ctx, mode)?;
        Ok(Self {
            ctx,
            mode,
            frame_index: 0,
            queued_events: Vec::new(),
        })
    }

    #[must_use]
    pub fn context(&self) -> &Context {
        &self.ctx
    }

    #[must_use]
    pub fn appearance(&self) -> AppearanceMode {
        self.mode
    }

    pub fn set_appearance(&mut self, mode: AppearanceMode) -> Result<()> {
        if self.mode != mode {
            self.mode = mode;
            apply_google_design_theme(&self.ctx, mode)?;
        }
        Ok(())
    }

    pub fn push_wire_event(&mut self, event: ImeWireEvent) {
        self.queued_events.push(event.into_egui_event());
    }

    pub fn push_wire_events_json(&mut self, json_payload: &str) -> Result<usize> {
        let parsed: Vec<ImeWireEvent> = serde_json::from_str(json_payload)?;
        let count = parsed.len();
        for ev in parsed {
            self.push_wire_event(ev);
        }
        Ok(count)
    }

    /// Runs one immediate-mode UI frame at `(width_px, height_px)` and tessellates the resulting shapes.
    pub fn run_frame(
        &mut self,
        width_px: u32,
        height_px: u32,
        pixels_per_point: f32,
        build_ui: impl FnMut(&mut Ui),
    ) -> EguiFrameArtifact {
        let ppp = pixels_per_point.clamp(0.5, 4.0);
        let logical_w = (width_px.max(1) as f32) / ppp;
        let logical_h = (height_px.max(1) as f32) / ppp;

        let mut raw_input = RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                emath::vec2(logical_w, logical_h),
            )),
            events: std::mem::take(&mut self.queued_events),
            time: Some(self.frame_index as f64 * (1.0 / 60.0)),
            ..RawInput::default()
        };
        raw_input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(ppp);

        let started = Instant::now();
        let FullOutput {
            platform_output,
            mut textures_delta,
            shapes,
            pixels_per_point: out_ppp,
            ..
        } = self.ctx.run_ui(raw_input, build_ui);
        let texture_uploads = textures_delta.set.len();
        textures_delta.clear();

        let primitives = self.ctx.tessellate(shapes, out_ppp);
        let elapsed_us = started.elapsed().as_micros() as u64;
        self.frame_index = self.frame_index.saturating_add(1);

        let mut draw_calls = 0usize;
        let mut vertices = 0usize;
        let mut indices = 0usize;
        for prim in &primitives {
            if let Primitive::Mesh(mesh) = &prim.primitive {
                draw_calls += 1;
                vertices += mesh.vertices.len();
                indices += mesh.indices.len();
            }
        }

        let accesskit_nodes = platform_output
            .accesskit_update
            .as_ref()
            .map_or(0, |u| u.nodes.len());

        EguiFrameArtifact {
            stats: EguiFrameStats {
                frame_index: self.frame_index,
                build_and_tessellate_us: elapsed_us,
                draw_calls,
                vertices,
                indices,
                triangles: indices / 3,
                texture_uploads,
                accesskit_nodes,
                width_px: width_px.max(1),
                height_px: height_px.max(1),
            },
            primitives,
            output_summary_cursor: format!("{:?}", platform_output.cursor_icon),
        }
    }
}

/// Deterministic pure-Rust barycentric rasterizer converting tessellated `egui` meshes into an
/// RGBA8 image buffer (`width * height * 4`) for headless CI verification and preview surfaces.
#[must_use]
pub fn rasterize_primitives_rgba8(
    primitives: &[ClippedPrimitive],
    width_px: u32,
    height_px: u32,
    pixels_per_point: f32,
    clear_color: Color32,
) -> Vec<u8> {
    let w = width_px.clamp(1, 2048) as usize;
    let h = height_px.clamp(1, 2048) as usize;
    let ppp = pixels_per_point.clamp(0.5, 4.0);
    let mut rgba = vec![0_u8; w * h * 4];
    let bg = clear_color.to_srgba_unmultiplied();
    for px_chunk in rgba.chunks_exact_mut(4) {
        px_chunk.copy_from_slice(&bg);
    }

    let edge = |ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32| -> f32 {
        (px - ax) * (by - ay) - (py - ay) * (bx - ax)
    };

    for clipped in primitives {
        let Primitive::Mesh(mesh) = &clipped.primitive else {
            continue;
        };
        let clip_min_x = ((clipped.clip_rect.min.x * ppp).floor() as isize).clamp(0, w as isize);
        let clip_min_y = ((clipped.clip_rect.min.y * ppp).floor() as isize).clamp(0, h as isize);
        let clip_max_x = ((clipped.clip_rect.max.x * ppp).ceil() as isize).clamp(0, w as isize);
        let clip_max_y = ((clipped.clip_rect.max.y * ppp).ceil() as isize).clamp(0, h as isize);
        if clip_min_x >= clip_max_x || clip_min_y >= clip_max_y {
            continue;
        }

        for tri in mesh.indices.chunks_exact(3) {
            let (Some(v0), Some(v1), Some(v2)) = (
                mesh.vertices.get(tri[0] as usize),
                mesh.vertices.get(tri[1] as usize),
                mesh.vertices.get(tri[2] as usize),
            ) else {
                continue;
            };
            let x0 = v0.pos.x * ppp;
            let y0 = v0.pos.y * ppp;
            let x1 = v1.pos.x * ppp;
            let y1 = v1.pos.y * ppp;
            let x2 = v2.pos.x * ppp;
            let y2 = v2.pos.y * ppp;

            let area = edge(x0, y0, x1, y1, x2, y2);
            if area.abs() < 1e-4 {
                continue;
            }
            let min_x = (x0.min(x1).min(x2).floor() as isize).max(clip_min_x);
            let max_x = (x0.max(x1).max(x2).ceil() as isize).min(clip_max_x);
            let min_y = (y0.min(y1).min(y2).floor() as isize).max(clip_min_y);
            let max_y = (y0.max(y1).max(y2).ceil() as isize).min(clip_max_y);

            let inv_area = 1.0 / area;
            let c0 = v0.color.to_srgba_unmultiplied();
            let c1 = v1.color.to_srgba_unmultiplied();
            let c2 = v2.color.to_srgba_unmultiplied();

            for py in min_y..max_y {
                let sample_y = py as f32 + 0.5;
                for px in min_x..max_x {
                    let sample_x = px as f32 + 0.5;
                    let w0 = edge(x1, y1, x2, y2, sample_x, sample_y) * inv_area;
                    let w1 = edge(x2, y2, x0, y0, sample_x, sample_y) * inv_area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                        let src_a = (c0[3] as f32 * w0 + c1[3] as f32 * w1 + c2[3] as f32 * w2)
                            .clamp(0.0, 255.0)
                            / 255.0;
                        if src_a <= 0.002 {
                            continue;
                        }
                        let src_r = c0[0] as f32 * w0 + c1[0] as f32 * w1 + c2[0] as f32 * w2;
                        let src_g = c0[1] as f32 * w0 + c1[1] as f32 * w1 + c2[1] as f32 * w2;
                        let src_b = c0[2] as f32 * w0 + c1[2] as f32 * w1 + c2[2] as f32 * w2;
                        let idx = (py as usize * w + px as usize) * 4;
                        if let Some(dst) = rgba.get_mut(idx..idx + 4) {
                            let inv_a = 1.0 - src_a;
                            dst[0] = (src_r * src_a + dst[0] as f32 * inv_a).round() as u8;
                            dst[1] = (src_g * src_a + dst[1] as f32 * inv_a).round() as u8;
                            dst[2] = (src_b * src_a + dst[2] as f32 * inv_a).round() as u8;
                            dst[3] = 255;
                        }
                    }
                }
            }
        }
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_egui_037_frame_with_google_design_tokens_and_ime_json() -> Result<()> {
        let mut host = EguiGpuiHost::new(AppearanceMode::Dark)?;
        let count = host.push_wire_events_json(
            r#"[
                {"kind": "pointer_moved", "x": 48.0, "y": 32.0},
                {"kind": "preedit", "text": "Fig", "cursor_start": 0, "cursor_end": 3},
                {"kind": "commit", "text": "FigCraft"}
            ]"#,
        )?;
        assert_eq!(count, 3);

        let mut search = String::from("craft");
        let artifact = host.run_frame(480, 320, 1.0, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.heading("Aphrody × egui 0.37 Google Design Surface");
                ui.text_edit_singleline(&mut search);
                let _clicked = ui.button("Inspect .fig Canvas").clicked();
            });
        });

        assert_eq!(artifact.stats.frame_index, 1);
        assert!(artifact.stats.draw_calls > 0);
        assert!(artifact.stats.vertices > 12);
        assert!(artifact.stats.triangles > 4);
        assert!(artifact.stats.accesskit_nodes > 0);

        let rgba = rasterize_primitives_rgba8(
            &artifact.primitives,
            240,
            160,
            0.5,
            Color32::from_rgb(14, 20, 27),
        );
        assert_eq!(rgba.len(), 240 * 160 * 4);
        let non_bg_pixels = rgba
            .chunks_exact(4)
            .filter(|px| px[0] != 14 || px[1] != 20 || px[2] != 27)
            .count();
        assert!(non_bg_pixels > 100);

        host.set_appearance(AppearanceMode::Light)?;
        assert_eq!(host.appearance(), AppearanceMode::Light);
        Ok(())
    }
}
