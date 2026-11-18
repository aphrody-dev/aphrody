//! Full-GPUI Retained FigCraft Studio (`FigCraftStudioView`):
//! - Pure `render()` obeying `gpui-fast` retention invariants (zero per-frame global writes,
//!   zero `cx.notify()` in `render`, dedicated `ScrollHandle`s, unique domain `ElementId`s)
//! - M3 Expressive Top Bar with 4-mode live switcher (`Light`, `Dark`, `NeuralObsidian`, `OledPitch`)
//! - Left Dock: Multi-page switcher, hierarchical Layer Tree, and 24+ M3 Expressive Kit Palette
//! - Interactive Infinite Zoom/Pan GPUI Canvas with snap guides, selection overlay, pointer drag,
//!   scroll wheel zoom/pan, and keyboard shortcuts
//! - Right Inspector Dock: Auto-Layout (Flex/Grid/Padding/Gap/Sizing), Tokens & Variants + WCAG AA,
//!   Spatial vs Effects Physics Springs + Google Sans Flex Axes, and Live Code Preview across all
//!   4 deterministic exporters with one-click Copy/Export.

use aphrody_design::design_md::{DesignMdDocument, load_bundled_root_design_md};
use aphrody_design::web_standards::audit_html_standards;
use aphrody_ui::google_design::{GoogleDesignTokens, GoogleSansFlexAxes, SpringSpec};
use aphrody_ui::parse_hex_hsla;
use gpui::{
    App, BoxShadow, ClipboardItem, Context, EventEmitter, FocusHandle, Focusable, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Point, Render, ScrollEvent,
    ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled as _, Window, actions, div,
    hsla, point, px,
};
use gpui_kit::material3::{M3AppearanceMode, m3_contrast_ratio};
use ui::{FluentBuilder as _, h_flex, v_flex};

use crate::figcraft_core::{
    AutoLayoutAxis, AutoLayoutMode, FigCraftDesignMode, FigCraftDocument, FigCraftExportTarget,
    FigCraftFrameNode, FigCraftNodeKind, PaddingEdges, PageKind, SizingConstraint, SnapAxis,
    SnapGuideLine, StarterWidgetTemplate, TemplateCategory, m3_expressive_starter_templates,
};

actions!(
    figcraft_studio,
    [
        /// Undo the last FigCraft document edit.
        FigCraftUndo,
        /// Redo the last undone FigCraft document edit.
        FigCraftRedo,
        /// Zoom in on the infinite FigCraft canvas.
        FigCraftZoomIn,
        /// Zoom out on the infinite FigCraft canvas.
        FigCraftZoomOut,
        /// Reset zoom and pan on the FigCraft canvas.
        FigCraftResetViewport,
        /// Duplicate the currently selected FigCraft node.
        FigCraftDuplicateNode,
        /// Delete the currently selected non-root FigCraft node.
        FigCraftDeleteNode,
        /// Copy generated code for the active export target.
        FigCraftCopyExport,
    ]
);

/// Events emitted by [`FigCraftStudioView`] so workspace hosts can synchronize sessions or parity modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FigCraftStudioEvent {
    DocumentMutated {
        title: String,
        node_count: usize,
        page_count: usize,
    },
    ModeChanged(FigCraftDesignMode),
    ExportCopied {
        target: FigCraftExportTarget,
        bytes: usize,
    },
    ParityToggleRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FigCraftInspectorTab {
    #[default]
    Layout,
    Tokens,
    Springs,
    Code,
}

impl FigCraftInspectorTab {
    pub const ALL: [Self; 4] = [Self::Layout, Self::Tokens, Self::Springs, Self::Code];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Layout => "Layout",
            Self::Tokens => "Tokens",
            Self::Springs => "Springs",
            Self::Code => "Code",
        }
    }

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Layout => "layout",
            Self::Tokens => "tokens",
            Self::Springs => "springs",
            Self::Code => "code",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FigCraftResolvedPalette {
    surface: Hsla,
    surface_dim: Hsla,
    surface_container_lowest: Hsla,
    surface_container_low: Hsla,
    surface_container: Hsla,
    surface_container_high: Hsla,
    surface_container_highest: Hsla,
    on_surface: Hsla,
    on_surface_variant: Hsla,
    primary: Hsla,
    on_primary: Hsla,
    primary_container: Hsla,
    on_primary_container: Hsla,
    secondary_container: Hsla,
    on_secondary_container: Hsla,
    tertiary: Hsla,
    tertiary_container: Hsla,
    on_tertiary_container: Hsla,
    outline: Hsla,
    outline_variant: Hsla,
    error: Hsla,
    on_error: Hsla,
}

impl FigCraftResolvedPalette {
    fn from_doc(doc: &FigCraftDocument) -> Self {
        let color = |role: &str, fallback: Hsla| {
            let hex = doc.resolve_role_hex(role);
            parse_hex_hsla(&hex, fallback)
        };
        let is_light = doc.active_mode == FigCraftDesignMode::Light;
        let bg_fb = if is_light {
            hsla(220.0 / 360.0, 0.4, 0.98, 1.0)
        } else {
            hsla(212.0 / 360.0, 0.32, 0.08, 1.0)
        };
        let fg_fb = if is_light {
            hsla(215.0 / 360.0, 0.25, 0.12, 1.0)
        } else {
            hsla(215.0 / 360.0, 0.32, 0.90, 1.0)
        };
        let primary_fb = hsla(203.0 / 360.0, 0.85, 0.55, 1.0);

        Self {
            surface: color("surface", bg_fb),
            surface_dim: color("surface-dim", bg_fb),
            surface_container_lowest: color("surface-container-lowest", bg_fb),
            surface_container_low: color("surface-container-low", bg_fb),
            surface_container: color("surface-container", bg_fb),
            surface_container_high: color("surface-container-high", bg_fb),
            surface_container_highest: color("surface-container-highest", bg_fb),
            on_surface: color("on-surface", fg_fb),
            on_surface_variant: color("on-surface-variant", fg_fb),
            primary: color("primary", primary_fb),
            on_primary: color("on-primary", bg_fb),
            primary_container: color("primary-container", primary_fb),
            on_primary_container: color("on-primary-container", fg_fb),
            secondary_container: color("secondary-container", bg_fb),
            on_secondary_container: color("on-secondary-container", fg_fb),
            tertiary: color("tertiary", primary_fb),
            tertiary_container: color("tertiary-container", bg_fb),
            on_tertiary_container: color("on-tertiary-container", fg_fb),
            outline: color("outline", fg_fb),
            outline_variant: color("outline-variant", fg_fb),
            error: color("error", hsla(0.0, 0.72, 0.52, 1.0)),
            on_error: color("on-error", bg_fb),
        }
    }
}

#[derive(Debug, Clone)]
enum CanvasDragState {
    Panning {
        start_mouse: Point<Pixels>,
        initial_pan_x: f32,
        initial_pan_y: f32,
    },
    DraggingNode {
        node_id: String,
        start_mouse: Point<Pixels>,
        initial_x_px: f32,
        initial_y_px: f32,
        preview_x_px: f32,
        preview_y_px: f32,
    },
}

/// Retained GPUI Material 3 Expressive FigCraft Studio View.
pub struct FigCraftStudioView {
    focus_handle: FocusHandle,
    doc: FigCraftDocument,
    design_md: DesignMdDocument,
    tokens: GoogleDesignTokens,
    palette: FigCraftResolvedPalette,
    zoom: f32,
    pan_x: f32,
    pan_y: f32,
    show_grid: bool,
    snap_enabled: bool,
    active_snap_guides: Vec<SnapGuideLine>,
    drag_state: Option<CanvasDragState>,
    active_category: Option<TemplateCategory>,
    inspector_tab: FigCraftInspectorTab,
    export_target: FigCraftExportTarget,
    cached_export_code: String,
    cached_lighthouse_score: Option<u8>,
    parity_label: SharedString,
    status_message: SharedString,
    canvas_scroll: ScrollHandle,
    left_dock_scroll: ScrollHandle,
    right_dock_scroll: ScrollHandle,
    code_preview_scroll: ScrollHandle,
}

impl FigCraftStudioView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let doc = FigCraftDocument::google_expressive_default().unwrap_or_else(|err| {
            log::warn!("FigCraft fallback document created: {err}");
            FigCraftDocument::from_design_md(
                "Google Neural Expressive Studio",
                &fallback_design_md(),
            )
        });
        Self::from_document(doc, cx)
    }

    pub fn from_document(doc: FigCraftDocument, cx: &mut Context<Self>) -> Self {
        let design_md = load_bundled_root_design_md().unwrap_or_else(|_| fallback_design_md());
        let tokens = GoogleDesignTokens::bundled().unwrap_or_else(|_| fallback_google_tokens());
        let palette = FigCraftResolvedPalette::from_doc(&doc);
        let export_target = FigCraftExportTarget::GpuiKitRs;
        let cached_export_code = doc.export_for_target(export_target, &design_md);
        let html = doc.export_for_target(FigCraftExportTarget::Lighthouse100Html, &design_md);
        let lighthouse = audit_html_standards("figcraft.html", &html, &design_md, Some(64_000));

        Self {
            focus_handle: cx.focus_handle(),
            doc,
            design_md,
            tokens,
            palette,
            zoom: 1.0,
            pan_x: 36.0,
            pan_y: 28.0,
            show_grid: true,
            snap_enabled: true,
            active_snap_guides: Vec::new(),
            drag_state: None,
            active_category: None,
            inspector_tab: FigCraftInspectorTab::Layout,
            export_target,
            cached_export_code,
            cached_lighthouse_score: Some(lighthouse.scores.overall),
            parity_label: SharedString::from("GPUI Google Design"),
            status_message: SharedString::from(
                "FigCraft Studio ready · 4 M3 modes · 24 templates · 4 deterministic exporters",
            ),
            canvas_scroll: ScrollHandle::new(),
            left_dock_scroll: ScrollHandle::new(),
            right_dock_scroll: ScrollHandle::new(),
            code_preview_scroll: ScrollHandle::new(),
        }
    }

    #[must_use]
    pub fn document(&self) -> &FigCraftDocument {
        &self.doc
    }

    #[must_use]
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    #[must_use]
    pub fn active_export_target(&self) -> FigCraftExportTarget {
        self.export_target
    }

    #[must_use]
    pub fn cached_export_code(&self) -> &str {
        &self.cached_export_code
    }

    #[must_use]
    pub fn inspector_tab(&self) -> FigCraftInspectorTab {
        self.inspector_tab
    }

    #[must_use]
    pub fn exported_code(&self, target: FigCraftExportTarget) -> String {
        self.doc.export_for_target(target, &self.design_md)
    }

    pub fn set_parity_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.parity_label = label.into();
        cx.notify();
    }

    pub fn replace_document(&mut self, doc: FigCraftDocument, cx: &mut Context<Self>) {
        self.doc = doc;
        self.refresh_derived_caches();
        cx.notify();
    }

    fn refresh_derived_caches(&mut self) {
        self.palette = FigCraftResolvedPalette::from_doc(&self.doc);
        self.cached_export_code = self.doc.export_for_target(self.export_target, &self.design_md);
        let html = self
            .doc
            .export_for_target(FigCraftExportTarget::Lighthouse100Html, &self.design_md);
        let report = audit_html_standards("figcraft.html", &html, &self.design_md, Some(64_000));
        self.cached_lighthouse_score = Some(report.scores.overall);
    }

    fn notify_document_mutated(&mut self, cx: &mut Context<Self>) {
        self.refresh_derived_caches();
        cx.emit(FigCraftStudioEvent::DocumentMutated {
            title: self.doc.title.clone(),
            node_count: self.doc.total_node_count(),
            page_count: self.doc.pages.len(),
        });
        cx.notify();
    }

    pub fn set_design_mode(&mut self, mode: FigCraftDesignMode, cx: &mut Context<Self>) {
        self.doc.set_design_mode(mode);
        self.status_message = SharedString::from(format!(
            "Switched M3 Expressive tokens to {} mode",
            mode.label()
        ));
        self.refresh_derived_caches();
        cx.emit(FigCraftStudioEvent::ModeChanged(mode));
        cx.emit(FigCraftStudioEvent::DocumentMutated {
            title: self.doc.title.clone(),
            node_count: self.doc.total_node_count(),
            page_count: self.doc.pages.len(),
        });
        cx.notify();
    }

    pub fn select_page(&mut self, page_id: &str, cx: &mut Context<Self>) {
        if self.doc.select_page(page_id) {
            self.active_snap_guides.clear();
            self.status_message = SharedString::from(format!("Active page: {page_id}"));
            self.notify_document_mutated(cx);
        }
    }

    pub fn add_page(&mut self, cx: &mut Context<Self>) {
        let next_index = self.doc.pages.len() + 1;
        let name = format!("Screen {next_index}");
        let page_id = self.doc.add_page(name.clone(), PageKind::Canvas);
        self.status_message = SharedString::from(format!("Added page '{name}' ({page_id})"));
        self.notify_document_mutated(cx);
    }

    pub fn select_node(&mut self, node_id: Option<String>, cx: &mut Context<Self>) {
        self.doc.select_node(node_id.clone());
        if let Some(id) = node_id {
            self.status_message = SharedString::from(format!("Selected `{id}`"));
        }
        cx.notify();
    }

    pub fn insert_template(&mut self, template_id: &str, cx: &mut Context<Self>) {
        let parent_id = self
            .doc
            .selected_node()
            .filter(|n| {
                matches!(
                    n.kind,
                    FigCraftNodeKind::Screen | FigCraftNodeKind::Frame | FigCraftNodeKind::Section
                )
            })
            .map(|n| n.id.clone())
            .or_else(|| {
                self.doc
                    .active_page()
                    .and_then(|p| p.root_frames.first())
                    .map(|r| r.id.clone())
            });
        if let Some(new_id) = self
            .doc
            .insert_template_into(parent_id.as_deref(), template_id)
        {
            self.status_message = SharedString::from(format!(
                "Inserted `{template_id}` as `{new_id}` (Ctrl+Z to undo)"
            ));
            self.notify_document_mutated(cx);
        }
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.doc.undo() {
            self.active_snap_guides.clear();
            self.status_message = SharedString::from("Undid last edit");
            self.notify_document_mutated(cx);
        }
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if self.doc.redo() {
            self.active_snap_guides.clear();
            self.status_message = SharedString::from("Redid edit");
            self.notify_document_mutated(cx);
        }
    }

    pub fn zoom_in(&mut self, cx: &mut Context<Self>) {
        self.zoom = (self.zoom + 0.15).clamp(0.25, 3.0);
        self.status_message = SharedString::from(format!("Zoom {:.0}%", self.zoom * 100.0));
        cx.notify();
    }

    pub fn zoom_out(&mut self, cx: &mut Context<Self>) {
        self.zoom = (self.zoom - 0.15).clamp(0.25, 3.0);
        self.status_message = SharedString::from(format!("Zoom {:.0}%", self.zoom * 100.0));
        cx.notify();
    }

    pub fn reset_viewport(&mut self, cx: &mut Context<Self>) {
        self.zoom = 1.0;
        self.pan_x = 36.0;
        self.pan_y = 28.0;
        self.canvas_scroll.set_offset(point(px(0.0), px(0.0)));
        self.status_message = SharedString::from("Viewport reset to 100%");
        cx.notify();
    }

    pub fn select_inspector_tab(&mut self, tab: FigCraftInspectorTab, cx: &mut Context<Self>) {
        self.inspector_tab = tab;
        cx.notify();
    }

    pub fn select_export_target(&mut self, target: FigCraftExportTarget, cx: &mut Context<Self>) {
        self.export_target = target;
        self.cached_export_code = self.doc.export_for_target(target, &self.design_md);
        self.status_message = SharedString::from(format!(
            "Generated {} ({} bytes)",
            target.label(),
            self.cached_export_code.len()
        ));
        cx.notify();
    }

    pub fn copy_active_export(&mut self, cx: &mut Context<Self>) {
        let code = self.doc.export_for_target(self.export_target, &self.design_md);
        let bytes = code.len();
        cx.write_to_clipboard(ClipboardItem::new_string(code));
        self.status_message = SharedString::from(format!(
            "Copied {} ({bytes} bytes) to clipboard",
            self.export_target.label()
        ));
        cx.emit(FigCraftStudioEvent::ExportCopied {
            target: self.export_target,
            bytes,
        });
        cx.notify();
    }

    fn toggle_grid(&mut self, cx: &mut Context<Self>) {
        self.show_grid = !self.show_grid;
        cx.notify();
    }

    fn toggle_snap(&mut self, cx: &mut Context<Self>) {
        self.snap_enabled = !self.snap_enabled;
        if !self.snap_enabled {
            self.active_snap_guides.clear();
        }
        cx.notify();
    }

    fn duplicate_selected(&mut self, cx: &mut Context<Self>) {
        let Some(selected_id) = self.doc.selected_node_id.clone() else {
            return;
        };
        if let Some(new_id) = self.doc.duplicate_node(&selected_id) {
            self.status_message = SharedString::from(format!("Duplicated `{selected_id}` -> `{new_id}`"));
            self.notify_document_mutated(cx);
        }
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        let Some(selected_id) = self.doc.selected_node_id.clone() else {
            return;
        };
        if self.doc.delete_node(&selected_id) {
            self.status_message = SharedString::from(format!("Deleted `{selected_id}`"));
            self.notify_document_mutated(cx);
        }
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let mods = event.keystroke.modifiers;
        let cmd_or_ctrl = mods.control || mods.platform;

        if cmd_or_ctrl && key.eq_ignore_ascii_case("z") {
            if mods.shift {
                self.redo(cx);
            } else {
                self.undo(cx);
            }
            return;
        }
        if cmd_or_ctrl && key.eq_ignore_ascii_case("y") {
            self.redo(cx);
            return;
        }
        if cmd_or_ctrl && key.eq_ignore_ascii_case("d") {
            self.duplicate_selected(cx);
            return;
        }
        if key == "backspace" || key == "delete" {
            self.delete_selected(cx);
            return;
        }
        if key == "=" || key == "+" {
            self.zoom_in(cx);
            return;
        }
        if key == "-" {
            self.zoom_out(cx);
            return;
        }
        if key == "0" && cmd_or_ctrl {
            self.reset_viewport(cx);
            return;
        }

        let step = if mods.shift { 10.0 } else { 2.0 };
        let delta = match key {
            "left" => Some((-step, 0.0)),
            "right" => Some((step, 0.0)),
            "up" => Some((0.0, -step)),
            "down" => Some((0.0, step)),
            _ => None,
        };
        if let Some((dx, dy)) = delta
            && let Some(selected_id) = self.doc.selected_node_id.clone()
        {
            let grid_step = if self.snap_enabled { Some(2.0) } else { None };
            if self.doc.move_node_by(&selected_id, dx, dy, grid_step) {
                self.notify_document_mutated(cx);
            }
        }
    }

    fn handle_canvas_scroll(&mut self, event: &ScrollEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(16.0));
        let dy = f32::from(delta.y);
        let dx = f32::from(delta.x);
        if event.modifiers.control || event.modifiers.platform {
            let zoom_delta = if dy < 0.0 { 0.1 } else { -0.1 };
            self.zoom = (self.zoom + zoom_delta).clamp(0.25, 3.0);
            self.status_message = SharedString::from(format!("Zoom {:.0}%", self.zoom * 100.0));
        } else {
            self.pan_x = (self.pan_x + dx).clamp(-2000.0, 2000.0);
            self.pan_y = (self.pan_y + dy).clamp(-2000.0, 2000.0);
        }
        cx.notify();
    }

    fn handle_canvas_mouse_down(&mut self, event: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.drag_state = Some(CanvasDragState::Panning {
            start_mouse: event.position,
            initial_pan_x: self.pan_x,
            initial_pan_y: self.pan_y,
        });
        cx.notify();
    }

    fn handle_canvas_mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag_state.clone() else {
            return;
        };
        match drag {
            CanvasDragState::Panning {
                start_mouse,
                initial_pan_x,
                initial_pan_y,
            } => {
                let dx = f32::from(event.position.x - start_mouse.x);
                let dy = f32::from(event.position.y - start_mouse.y);
                self.pan_x = (initial_pan_x + dx).clamp(-2000.0, 2000.0);
                self.pan_y = (initial_pan_y + dy).clamp(-2000.0, 2000.0);
                cx.notify();
            }
            CanvasDragState::DraggingNode {
                node_id,
                start_mouse,
                initial_x_px,
                initial_y_px,
                ..
            } => {
                let dx = f32::from(event.position.x - start_mouse.x) / self.zoom.max(0.25);
                let dy = f32::from(event.position.y - start_mouse.y) / self.zoom.max(0.25);
                let raw_x = initial_x_px + dx;
                let raw_y = initial_y_px + dy;
                let (next_x, next_y, guides) = if self.snap_enabled {
                    let res = self.doc.compute_snap_guides(&node_id, raw_x, raw_y, 8.0);
                    (res.snapped_x_px, res.snapped_y_px, res.guides)
                } else {
                    (raw_x, raw_y, Vec::new())
                };
                self.active_snap_guides = guides;
                self.drag_state = Some(CanvasDragState::DraggingNode {
                    node_id,
                    start_mouse,
                    initial_x_px,
                    initial_y_px,
                    preview_x_px: next_x,
                    preview_y_px: next_y,
                });
                cx.notify();
            }
        }
    }

    fn handle_canvas_mouse_up(&mut self, _event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag_state.take() else {
            return;
        };
        if let CanvasDragState::DraggingNode {
            node_id,
            initial_x_px,
            initial_y_px,
            preview_x_px,
            preview_y_px,
            ..
        } = drag
        {
            self.active_snap_guides.clear();
            let moved = (preview_x_px - initial_x_px).abs() > 0.5
                || (preview_y_px - initial_y_px).abs() > 0.5;
            if moved && self.doc.set_node_position(&node_id, preview_x_px, preview_y_px) {
                self.status_message = SharedString::from(format!(
                    "Moved `{node_id}` to ({preview_x_px:.0}, {preview_y_px:.0})"
                ));
                self.notify_document_mutated(cx);
                return;
            }
        }
        cx.notify();
    }

    fn resolve_node_hsla(&self, role: &str, fallback: Hsla) -> Hsla {
        let hex = self.doc.resolve_role_hex(role);
        parse_hex_hsla(&hex, fallback)
    }

    fn render_top_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette;
        let undo_count = self.doc.history.undo_stack.len();
        let redo_count = self.doc.history.redo_stack.len();
        let page_title = self
            .doc
            .active_page()
            .map_or("Overview", |page| page.name.as_str());

        h_flex()
            .w_full()
            .min_h(px(52.0))
            .px_3()
            .py_2()
            .gap_2()
            .flex_wrap()
            .items_center()
            .justify_between()
            .bg(p.surface_container)
            .border_b_1()
            .border_color(p.outline_variant)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .px_2p5()
                            .py_1()
                            .rounded(px(999.0))
                            .bg(p.primary)
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.on_primary)
                            .child("FigCraft M3"),
                    )
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.on_surface)
                                    .child(self.doc.title.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(p.on_surface_variant)
                                    .child(format!(
                                        "{page_title} · {} pages · {} nodes · {}",
                                        self.doc.pages.len(),
                                        self.doc.total_node_count(),
                                        self.doc.fig_binary_header_magic
                                    )),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .p_1()
                    .rounded(px(999.0))
                    .bg(p.surface_container_low)
                    .border_1()
                    .border_color(p.outline_variant)
                    .children(FigCraftDesignMode::ALL.into_iter().map(|mode| {
                        let active = self.doc.active_mode == mode;
                        let bg = if active { p.primary } else { p.surface_container_low };
                        let fg = if active { p.on_primary } else { p.on_surface_variant };
                        let mode_slug = format!("figcraft-mode-{}", mode.slug());
                        div()
                            .id(SharedString::from(mode_slug.clone()))
                            .debug_selector(move || mode_slug)
                            .px_2p5()
                            .py_1()
                            .rounded(px(999.0))
                            .bg(bg)
                            .text_xs()
                            .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                            .text_color(fg)
                            .cursor_pointer()
                            .child(mode.label())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_design_mode(mode, cx);
                            }))
                    })),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .flex_wrap()
                    .items_center()
                    .child(
                        div()
                            .id("figcraft-undo")
                            .debug_selector(|| "figcraft-undo".into())
                            .px_2p5()
                            .py_1()
                            .rounded(px(8.0))
                            .bg(if self.doc.can_undo() {
                                p.secondary_container
                            } else {
                                p.surface_container_low
                            })
                            .border_1()
                            .border_color(p.outline_variant)
                            .text_xs()
                            .text_color(if self.doc.can_undo() {
                                p.on_secondary_container
                            } else {
                                p.on_surface_variant
                            })
                            .cursor_pointer()
                            .child(format!("Undo ({undo_count})"))
                            .on_click(cx.listener(|this, _, _, cx| this.undo(cx))),
                    )
                    .child(
                        div()
                            .id("figcraft-redo")
                            .debug_selector(|| "figcraft-redo".into())
                            .px_2p5()
                            .py_1()
                            .rounded(px(8.0))
                            .bg(if self.doc.can_redo() {
                                p.secondary_container
                            } else {
                                p.surface_container_low
                            })
                            .border_1()
                            .border_color(p.outline_variant)
                            .text_xs()
                            .text_color(if self.doc.can_redo() {
                                p.on_secondary_container
                            } else {
                                p.on_surface_variant
                            })
                            .cursor_pointer()
                            .child(format!("Redo ({redo_count})"))
                            .on_click(cx.listener(|this, _, _, cx| this.redo(cx))),
                    )
                    .child(
                        div()
                            .id("figcraft-zoom-out")
                            .debug_selector(|| "figcraft-zoom-out".into())
                            .px_2()
                            .py_1()
                            .rounded(px(8.0))
                            .bg(p.surface_container_high)
                            .text_xs()
                            .text_color(p.on_surface)
                            .cursor_pointer()
                            .child("-")
                            .on_click(cx.listener(|this, _, _, cx| this.zoom_out(cx))),
                    )
                    .child(
                        div()
                            .id("figcraft-zoom-reset")
                            .debug_selector(|| "figcraft-zoom-reset".into())
                            .px_2()
                            .py_1()
                            .rounded(px(8.0))
                            .bg(p.surface_container_highest)
                            .border_1()
                            .border_color(p.outline_variant)
                            .text_xs()
                            .text_color(p.on_surface)
                            .cursor_pointer()
                            .child(format!("{:.0}%", self.zoom * 100.0))
                            .on_click(cx.listener(|this, _, _, cx| this.reset_viewport(cx))),
                    )
                    .child(
                        div()
                            .id("figcraft-zoom-in")
                            .debug_selector(|| "figcraft-zoom-in".into())
                            .px_2()
                            .py_1()
                            .rounded(px(8.0))
                            .bg(p.surface_container_high)
                            .text_xs()
                            .text_color(p.on_surface)
                            .cursor_pointer()
                            .child("+")
                            .on_click(cx.listener(|this, _, _, cx| this.zoom_in(cx))),
                    )
                    .child(
                        div()
                            .id("figcraft-toggle-snap")
                            .px_2()
                            .py_1()
                            .rounded(px(8.0))
                            .bg(if self.snap_enabled {
                                p.tertiary_container
                            } else {
                                p.surface_container_low
                            })
                            .text_xs()
                            .text_color(if self.snap_enabled {
                                p.on_tertiary_container
                            } else {
                                p.on_surface_variant
                            })
                            .cursor_pointer()
                            .child(if self.snap_enabled { "Snap ON" } else { "Snap OFF" })
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_snap(cx))),
                    )
                    .child(
                        div()
                            .id("figcraft-toggle-grid")
                            .px_2()
                            .py_1()
                            .rounded(px(8.0))
                            .bg(if self.show_grid {
                                p.tertiary_container
                            } else {
                                p.surface_container_low
                            })
                            .text_xs()
                            .text_color(if self.show_grid {
                                p.on_tertiary_container
                            } else {
                                p.on_surface_variant
                            })
                            .cursor_pointer()
                            .child(if self.show_grid { "Grid ON" } else { "Grid OFF" })
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_grid(cx))),
                    )
                    .child(
                        div()
                            .id("figcraft-parity-chip")
                            .px_2p5()
                            .py_1()
                            .rounded(px(999.0))
                            .bg(p.primary_container)
                            .text_xs()
                            .text_color(p.on_primary_container)
                            .cursor_pointer()
                            .child(self.parity_label.clone())
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.emit(FigCraftStudioEvent::ParityToggleRequested);
                            })),
                    )
                    .child(
                        div()
                            .id("figcraft-copy-export")
                            .debug_selector(|| "figcraft-copy-export".into())
                            .px_3()
                            .py_1()
                            .rounded(px(999.0))
                            .bg(p.primary)
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.on_primary)
                            .cursor_pointer()
                            .child("Copy Code")
                            .on_click(cx.listener(|this, _, _, cx| this.copy_active_export(cx))),
                    ),
            )
    }

    fn render_left_dock(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette;
        let active_page_id = self.doc.active_page_id.clone();
        let selected_node_id = self.doc.selected_node_id.clone();
        let templates = m3_expressive_starter_templates();
        let filtered_templates: Vec<&StarterWidgetTemplate> = templates
            .iter()
            .filter(|tpl| {
                self.active_category
                    .is_none_or(|category| tpl.category == category)
            })
            .collect();

        let mut layer_rows = Vec::new();
        if let Some(page) = self.doc.active_page() {
            for frame in &page.root_frames {
                collect_layer_rows(frame, 0, &mut layer_rows);
            }
        }

        v_flex()
            .id("figcraft-left-dock")
            .w(px(276.0))
            .h_full()
            .flex_shrink_0()
            .overflow_y_scroll()
            .track_scroll(&self.left_dock_scroll)
            .p_3()
            .gap_3()
            .bg(p.surface_container_low)
            .border_r_1()
            .border_color(p.outline_variant)
            .child(
                v_flex()
                    .gap_1p5()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.on_surface_variant)
                                    .child(format!("PAGES ({})", self.doc.pages.len())),
                            )
                            .child(
                                div()
                                    .id("figcraft-add-page")
                                    .debug_selector(|| "figcraft-add-page".into())
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(999.0))
                                    .bg(p.primary_container)
                                    .text_xs()
                                    .text_color(p.on_primary_container)
                                    .cursor_pointer()
                                    .child("+ Page")
                                    .on_click(cx.listener(|this, _, _, cx| this.add_page(cx))),
                            ),
                    )
                    .children(self.doc.pages.iter().map(|page| {
                        let page_id = page.id.clone();
                        let active = page.id == active_page_id;
                        div()
                            .id(SharedString::from(format!("figcraft-page-{}", page.id)))
                            .px_2p5()
                            .py_1p5()
                            .rounded(px(10.0))
                            .bg(if active {
                                p.primary_container
                            } else {
                                p.surface_container
                            })
                            .border_1()
                            .border_color(if active { p.primary } else { p.outline_variant })
                            .cursor_pointer()
                            .child(
                                h_flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(if active {
                                                FontWeight::SEMIBOLD
                                            } else {
                                                FontWeight::MEDIUM
                                            })
                                            .text_color(if active {
                                                p.on_primary_container
                                            } else {
                                                p.on_surface
                                            })
                                            .child(page.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(p.on_surface_variant)
                                            .child(page.kind.label()),
                                    ),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_page(&page_id, cx);
                            }))
                    })),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.on_surface_variant)
                            .child(format!("LAYER TREE ({})", layer_rows.len())),
                    )
                    .children(layer_rows.into_iter().map(|row| {
                        let selected = selected_node_id.as_deref() == Some(row.id.as_str());
                        let node_id = row.id.clone();
                        let node_id_vis = row.id.clone();
                        let node_id_lock = row.id.clone();
                        h_flex()
                            .id(SharedString::from(format!("figcraft-layer-{}", row.id)))
                            .pl(px(8.0 + (row.depth as f32) * 12.0))
                            .pr_2()
                            .py_1()
                            .rounded(px(8.0))
                            .justify_between()
                            .items_center()
                            .gap_1()
                            .bg(if selected {
                                p.secondary_container
                            } else {
                                p.surface_container
                            })
                            .border_1()
                            .border_color(if selected { p.primary } else { p.outline_variant })
                            .cursor_pointer()
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .items_center()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .px_1p5()
                                            .py_0p5()
                                            .rounded(px(4.0))
                                            .bg(p.surface_container_high)
                                            .text_xs()
                                            .text_color(p.primary)
                                            .child(row.kind_badge),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .truncate()
                                            .text_color(if selected {
                                                p.on_secondary_container
                                            } else {
                                                p.on_surface
                                            })
                                            .child(row.name),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "figcraft-vis-{}",
                                                row.id
                                            )))
                                            .px_1()
                                            .rounded(px(4.0))
                                            .bg(p.surface_container_high)
                                            .text_xs()
                                            .text_color(if row.visible {
                                                p.on_surface
                                            } else {
                                                p.error
                                            })
                                            .child(if row.visible { "V" } else { "H" })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                if this.doc.toggle_node_visibility(&node_id_vis) {
                                                    this.notify_document_mutated(cx);
                                                }
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "figcraft-lock-{}",
                                                row.id
                                            )))
                                            .px_1()
                                            .rounded(px(4.0))
                                            .bg(p.surface_container_high)
                                            .text_xs()
                                            .text_color(if row.locked {
                                                p.error
                                            } else {
                                                p.on_surface_variant
                                            })
                                            .child(if row.locked { "L" } else { "U" })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                if this.doc.toggle_node_locked(&node_id_lock) {
                                                    this.notify_document_mutated(cx);
                                                }
                                            })),
                                    ),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_node(Some(node_id.clone()), cx);
                            }))
                    })),
            )
            .child(
                v_flex()
                    .gap_1p5()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.on_surface_variant)
                            .child(format!("M3 EXPRESSIVE KIT ({})", templates.len())),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .flex_wrap()
                            .child(
                                div()
                                    .id("figcraft-cat-all")
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(999.0))
                                    .bg(if self.active_category.is_none() {
                                        p.primary
                                    } else {
                                        p.surface_container
                                    })
                                    .text_xs()
                                    .text_color(if self.active_category.is_none() {
                                        p.on_primary
                                    } else {
                                        p.on_surface_variant
                                    })
                                    .cursor_pointer()
                                    .child("All")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.active_category = None;
                                        cx.notify();
                                    })),
                            )
                            .children(TemplateCategory::ALL.into_iter().map(|category| {
                                let active = self.active_category == Some(category);
                                div()
                                    .id(SharedString::from(format!(
                                        "figcraft-cat-{}",
                                        category.label()
                                    )))
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(999.0))
                                    .bg(if active {
                                        p.primary
                                    } else {
                                        p.surface_container
                                    })
                                    .text_xs()
                                    .text_color(if active {
                                        p.on_primary
                                    } else {
                                        p.on_surface_variant
                                    })
                                    .cursor_pointer()
                                    .child(category.label())
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.active_category = Some(category);
                                        cx.notify();
                                    }))
                            })),
                    )
                    .children(filtered_templates.into_iter().map(|tpl| {
                        let tpl_id = tpl.id;
                        let swatch = self.resolve_node_hsla(tpl.fill_role, p.primary);
                        let sel_id = format!("figcraft-palette-{}", tpl.id);
                        div()
                            .id(SharedString::from(sel_id.clone()))
                            .debug_selector(move || sel_id)
                            .px_2p5()
                            .py_1p5()
                            .rounded(px(10.0))
                            .bg(p.surface_container)
                            .border_1()
                            .border_color(p.outline_variant)
                            .cursor_pointer()
                            .child(
                                h_flex()
                                    .justify_between()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child(
                                                div()
                                                    .w(px(10.0))
                                                    .h(px(10.0))
                                                    .rounded(px(999.0))
                                                    .bg(swatch),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.on_surface)
                                                    .child(tpl.name),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(p.primary)
                                            .child("+"),
                                    ),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.insert_template(tpl_id, cx);
                            }))
                    })),
            )
    }

    fn render_canvas_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette;
        let frames = self
            .doc
            .active_page()
            .map(|page| page.root_frames.clone())
            .unwrap_or_else(|| self.doc.root_frames.clone());
        let zoom = self.zoom;
        let pan_x = self.pan_x;
        let pan_y = self.pan_y;

        let mut canvas_stage = div()
            .id("figcraft-canvas-stage")
            .relative()
            .w(px(2400.0))
            .h(px(1600.0))
            .bg(p.surface_dim);

        if self.show_grid {
            canvas_stage = canvas_stage.children((0..12).map(|idx| {
                let offset = (idx as f32) * 160.0 * zoom + pan_x.rem_euclid(160.0 * zoom);
                div()
                    .absolute()
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .left(px(offset))
                    .w(px(1.0))
                    .bg(p.outline_variant.opacity(0.25))
            }));
            canvas_stage = canvas_stage.children((0..10).map(|idx| {
                let offset = (idx as f32) * 160.0 * zoom + pan_y.rem_euclid(160.0 * zoom);
                div()
                    .absolute()
                    .left(px(0.0))
                    .right(px(0.0))
                    .top(px(offset))
                    .h(px(1.0))
                    .bg(p.outline_variant.opacity(0.25))
            }));
        }

        for frame in &frames {
            canvas_stage = canvas_stage.child(self.render_canvas_node(frame, true, cx));
        }

        for guide in &self.active_snap_guides {
            let line = match guide.axis {
                SnapAxis::VerticalX => div()
                    .absolute()
                    .left(px(pan_x + guide.position_px * zoom))
                    .top(px(pan_y + guide.start_px * zoom))
                    .w(px(1.5))
                    .h(px(((guide.end_px - guide.start_px).abs() * zoom).max(24.0)))
                    .bg(p.tertiary),
                SnapAxis::HorizontalY => div()
                    .absolute()
                    .top(px(pan_y + guide.position_px * zoom))
                    .left(px(pan_x + guide.start_px * zoom))
                    .h(px(1.5))
                    .w(px(((guide.end_px - guide.start_px).abs() * zoom).max(24.0)))
                    .bg(p.tertiary),
            };
            canvas_stage = canvas_stage.child(line);
        }

        let selected_label = self
            .doc
            .selected_node()
            .map_or_else(|| "No selection".to_owned(), |n| {
                format!(
                    "{} · {} ({:.0}x{:.0} @ {:.0},{:.0})",
                    n.name, n.stitch_id, n.width_px, n.height_px, n.x_px, n.y_px
                )
            });

        div()
            .id("figcraft-canvas-viewport")
            .relative()
            .flex_1()
            .h_full()
            .min_w_0()
            .overflow_scroll()
            .track_scroll(&self.canvas_scroll)
            .bg(p.surface_dim)
            .on_scroll(cx.listener(Self::handle_canvas_scroll))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_canvas_mouse_down))
            .on_mouse_move(cx.listener(Self::handle_canvas_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_canvas_mouse_up))
            .child(canvas_stage)
            .child(
                h_flex()
                    .absolute()
                    .bottom(px(12.0))
                    .left(px(16.0))
                    .px_3()
                    .py_1p5()
                    .gap_3()
                    .rounded(px(999.0))
                    .bg(p.surface_container_high)
                    .border_1()
                    .border_color(p.outline_variant)
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.on_surface)
                            .child(selected_label),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.on_surface_variant)
                            .child(format!(
                                "Pan ({:.0}, {:.0}) · Zoom {:.0}%",
                                pan_x,
                                pan_y,
                                zoom * 100.0
                            )),
                    ),
            )
    }

    fn render_canvas_node(
        &self,
        node: &FigCraftFrameNode,
        is_root: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = self.palette;
        let zoom = self.zoom;
        let selected = self.doc.selected_node_id.as_deref() == Some(node.id.as_str());

        if !node.visible {
            return div().id(SharedString::from(format!(
                "figcraft-canvas-hidden-{}",
                node.id
            )));
        }

        let (eff_x, eff_y) = match &self.drag_state {
            Some(CanvasDragState::DraggingNode {
                node_id,
                preview_x_px,
                preview_y_px,
                ..
            }) if node_id == &node.id => (*preview_x_px, *preview_y_px),
            _ => (node.x_px, node.y_px),
        };

        let fill = self.resolve_node_hsla(&node.fill_role, p.surface_container);
        let text_col = self.resolve_node_hsla(&node.text_role, p.on_surface);
        let border_col = if selected {
            p.primary
        } else if let Some(ref srole) = node.stroke_role {
            self.resolve_node_hsla(srole, p.outline_variant)
        } else {
            p.outline_variant.opacity(0.35)
        };

        let mut container = div()
            .id(SharedString::from(format!("figcraft-canvas-node-{}", node.id)))
            .rounded(px((node.corner_radius_px * zoom).min(999.0)))
            .bg(fill)
            .opacity(node.opacity)
            .p(px((node.padding_px * zoom).max(4.0)))
            .gap(px((node.gap_px * zoom).max(2.0)))
            .cursor_pointer();

        if selected {
            container = container.border_2().border_color(p.primary);
        } else {
            container = container.border_1().border_color(border_col);
        }

        if node.elevation_level > 0 {
            let blur = (node.elevation_level as f32) * 5.0 * zoom;
            let offset_y = (node.elevation_level as f32) * 2.0 * zoom;
            container = container.shadow(vec![
                BoxShadow::new(px(0.0), px(offset_y), hsla(0.0, 0.0, 0.0, 0.28))
                    .blur_radius(px(blur)),
            ]);
        }

        if is_root {
            container = container
                .absolute()
                .left(px(self.pan_x + eff_x * zoom))
                .top(px(self.pan_y + eff_y * zoom))
                .w(px(node.width_px * zoom))
                .min_h(px(node.height_px * zoom));
        } else {
            match node.width_sizing {
                SizingConstraint::Fill => {
                    container = container.w_full();
                }
                SizingConstraint::Fixed(w) => {
                    container = container.w(px((w * zoom).max(40.0)));
                }
                SizingConstraint::Hug => {}
            }
            match node.height_sizing {
                SizingConstraint::Fixed(h) => {
                    container = container.min_h(px((h * zoom).max(24.0)));
                }
                SizingConstraint::Fill | SizingConstraint::Hug => {}
            }
        }

        container = match node.layout_mode {
            AutoLayoutMode::Grid => container.grid().grid_cols(node.grid_columns.max(1)),
            AutoLayoutMode::Flex | AutoLayoutMode::Freeform => match node.axis {
                AutoLayoutAxis::Horizontal => container.flex().flex_row().items_center(),
                AutoLayoutAxis::Vertical => container.flex().flex_col(),
            },
        };

        let node_id_select = node.id.clone();
        let node_id_drag = node.id.clone();
        let init_x = node.x_px;
        let init_y = node.y_px;
        let locked = node.locked;

        container = container
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                    cx.stop_propagation();
                    this.doc.select_node(Some(node_id_drag.clone()));
                    if !locked {
                        this.drag_state = Some(CanvasDragState::DraggingNode {
                            node_id: node_id_drag.clone(),
                            start_mouse: event.position,
                            initial_x_px: init_x,
                            initial_y_px: init_y,
                            preview_x_px: init_x,
                            preview_y_px: init_y,
                        });
                    }
                    cx.notify();
                }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.select_node(Some(node_id_select.clone()), cx);
            }));

        if is_root {
            container = container.child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .w_full()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.on_surface_variant)
                            .child(format!("{} · {}", node.name, node.stitch_id)),
                    )
                    .child(
                        div()
                            .px_1p5()
                            .py_0p5()
                            .rounded(px(999.0))
                            .bg(p.surface_container_high)
                            .text_xs()
                            .text_color(p.primary)
                            .child(format!("{:.0}x{:.0}", node.width_px, node.height_px)),
                    ),
            );
        }

        if let Some(ref text) = node.text_content {
            let weight = if node.flex_axes.wght >= 600.0 {
                FontWeight::SEMIBOLD
            } else if node.flex_axes.wght >= 500.0 {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            };
            container = container.child(
                div()
                    .text_sm()
                    .font_weight(weight)
                    .text_color(text_col)
                    .child(text.clone()),
            );
        }

        for child in &node.children {
            container = container.child(self.render_canvas_node(child, false, cx));
        }

        if selected {
            let handle_box = |id_suffix: &str| {
                div()
                    .id(SharedString::from(format!("figcraft-sel-handle-{}-{}", node.id, id_suffix)))
                    .w(px(7.0))
                    .h(px(7.0))
                    .rounded(px(2.0))
                    .bg(p.primary)
                    .border_1()
                    .border_color(p.on_primary)
            };
            container = container
                .child(handle_box("tl").absolute().top(px(-4.0)).left(px(-4.0)))
                .child(handle_box("tr").absolute().top(px(-4.0)).right(px(-4.0)))
                .child(handle_box("bl").absolute().bottom(px(-4.0)).left(px(-4.0)))
                .child(handle_box("br").absolute().bottom(px(-4.0)).right(px(-4.0)));
        }

        container
    }

    fn render_right_inspector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette;
        let selected = self.doc.selected_node().cloned();

        v_flex()
            .id("figcraft-right-inspector")
            .w(px(368.0))
            .h_full()
            .flex_shrink_0()
            .overflow_y_scroll()
            .track_scroll(&self.right_dock_scroll)
            .p_3()
            .gap_3()
            .bg(p.surface_container_low)
            .border_l_1()
            .border_color(p.outline_variant)
            .child(
                h_flex()
                    .gap_1()
                    .p_1()
                    .rounded(px(999.0))
                    .bg(p.surface_container)
                    .border_1()
                    .border_color(p.outline_variant)
                    .children(FigCraftInspectorTab::ALL.into_iter().map(|tab| {
                        let active = self.inspector_tab == tab;
                        let tab_sel = format!("figcraft-insp-tab-{}", tab.slug());
                        div()
                            .id(SharedString::from(tab_sel.clone()))
                            .debug_selector(move || tab_sel)
                            .flex_1()
                            .px_2()
                            .py_1()
                            .rounded(px(999.0))
                            .bg(if active { p.primary } else { p.surface_container })
                            .text_xs()
                            .text_center()
                            .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                            .text_color(if active { p.on_primary } else { p.on_surface_variant })
                            .cursor_pointer()
                            .child(tab.label())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_inspector_tab(tab, cx);
                            }))
                    })),
            )
            .when_some(selected.as_ref(), |col, node| {
                col.child(
                    v_flex()
                        .p_2p5()
                        .gap_1()
                        .rounded(px(12.0))
                        .bg(p.surface_container)
                        .border_1()
                        .border_color(p.outline_variant)
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.on_surface)
                                        .child(node.name.clone()),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded(px(999.0))
                                        .bg(p.tertiary_container)
                                        .text_xs()
                                        .text_color(p.on_tertiary_container)
                                        .child(node.kind.badge_label()),
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(p.on_surface_variant)
                                .child(format!(
                                    "data-stitch-id=\"{}\" · ({:.0}, {:.0}) · {:.0}x{:.0}",
                                    node.stitch_id, node.x_px, node.y_px, node.width_px, node.height_px
                                )),
                        ),
                )
            })
            .child(match self.inspector_tab {
                FigCraftInspectorTab::Layout => {
                    self.render_inspector_layout(selected.as_ref(), cx).into_any_element()
                }
                FigCraftInspectorTab::Tokens => {
                    self.render_inspector_tokens(selected.as_ref(), cx).into_any_element()
                }
                FigCraftInspectorTab::Springs => {
                    self.render_inspector_springs(selected.as_ref(), cx).into_any_element()
                }
                FigCraftInspectorTab::Code => self.render_inspector_code(cx).into_any_element(),
            })
    }

    fn render_inspector_layout(
        &self,
        selected: Option<&FigCraftFrameNode>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = self.palette;
        let Some(node) = selected.cloned() else {
            return v_flex().p_3().child(
                div()
                    .text_xs()
                    .text_color(p.on_surface_variant)
                    .child("Select a node on canvas or layer tree to inspect Auto-Layout."),
            );
        };

        let nid_axis = node.id.clone();
        let nid_mode = node.id.clone();
        let nid_gap_up = node.id.clone();
        let nid_gap_dn = node.id.clone();
        let nid_pad_up = node.id.clone();
        let nid_pad_dn = node.id.clone();
        let nid_cols = node.id.clone();
        let nid_radius = node.id.clone();
        let nid_sizing = node.id.clone();

        let node_for_axis = node.clone();
        let node_for_mode = node.clone();
        let node_for_gap_up = node.clone();
        let node_for_gap_dn = node.clone();
        let node_for_pad_up = node.clone();
        let node_for_pad_dn = node.clone();
        let node_for_cols = node.clone();
        let node_for_rad = node.clone();
        let node_for_sz = node.clone();

        v_flex()
            .gap_2p5()
            .child(
                v_flex()
                    .p_2p5()
                    .gap_2()
                    .rounded(px(12.0))
                    .bg(p.surface_container)
                    .border_1()
                    .border_color(p.outline_variant)
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.on_surface_variant)
                            .child("AUTO-LAYOUT & CONSTRAINTS"),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .children([
                                (AutoLayoutAxis::Horizontal, "Row"),
                                (AutoLayoutAxis::Vertical, "Column"),
                            ]
                            .into_iter()
                            .map(|(axis, label)| {
                                let active = node.axis == axis;
                                let nid = nid_axis.clone();
                                let n = node_for_axis.clone();
                                div()
                                    .id(SharedString::from(format!("figcraft-axis-{label}")))
                                    .flex_1()
                                    .py_1()
                                    .rounded(px(8.0))
                                    .bg(if active { p.primary } else { p.surface_container_high })
                                    .text_xs()
                                    .text_center()
                                    .text_color(if active { p.on_primary } else { p.on_surface })
                                    .cursor_pointer()
                                    .child(label)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if this.doc.update_node_layout(
                                            &nid,
                                            axis,
                                            n.layout_mode,
                                            n.gap_px,
                                            n.padding_edges,
                                            n.grid_columns,
                                            n.main_align,
                                            n.cross_align,
                                        ) {
                                            this.notify_document_mutated(cx);
                                        }
                                    }))
                            })),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .children([
                                (AutoLayoutMode::Flex, "Flex"),
                                (AutoLayoutMode::Grid, "Grid"),
                                (AutoLayoutMode::Freeform, "Free"),
                            ]
                            .into_iter()
                            .map(|(mode, label)| {
                                let active = node.layout_mode == mode;
                                let nid = nid_mode.clone();
                                let n = node_for_mode.clone();
                                div()
                                    .id(SharedString::from(format!("figcraft-lmode-{label}")))
                                    .flex_1()
                                    .py_1()
                                    .rounded(px(8.0))
                                    .bg(if active { p.primary } else { p.surface_container_high })
                                    .text_xs()
                                    .text_center()
                                    .text_color(if active { p.on_primary } else { p.on_surface })
                                    .cursor_pointer()
                                    .child(label)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if this.doc.update_node_layout(
                                            &nid,
                                            n.axis,
                                            mode,
                                            n.gap_px,
                                            n.padding_edges,
                                            n.grid_columns,
                                            n.main_align,
                                            n.cross_align,
                                        ) {
                                            this.notify_document_mutated(cx);
                                        }
                                    }))
                            })),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(p.on_surface)
                                    .child(format!("Gap: {:.0}px", node.gap_px)),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .id("figcraft-gap-minus")
                                            .px_2p5()
                                            .py_0p5()
                                            .rounded(px(6.0))
                                            .bg(p.surface_container_high)
                                            .text_xs()
                                            .text_color(p.on_surface)
                                            .cursor_pointer()
                                            .child("-4")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                let next = (node_for_gap_dn.gap_px - 4.0).max(0.0);
                                                if this.doc.update_node_layout(
                                                    &nid_gap_dn,
                                                    node_for_gap_dn.axis,
                                                    node_for_gap_dn.layout_mode,
                                                    next,
                                                    node_for_gap_dn.padding_edges,
                                                    node_for_gap_dn.grid_columns,
                                                    node_for_gap_dn.main_align,
                                                    node_for_gap_dn.cross_align,
                                                ) {
                                                    this.notify_document_mutated(cx);
                                                }
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id("figcraft-gap-plus")
                                            .px_2p5()
                                            .py_0p5()
                                            .rounded(px(6.0))
                                            .bg(p.surface_container_high)
                                            .text_xs()
                                            .text_color(p.on_surface)
                                            .cursor_pointer()
                                            .child("+4")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                let next = node_for_gap_up.gap_px + 4.0;
                                                if this.doc.update_node_layout(
                                                    &nid_gap_up,
                                                    node_for_gap_up.axis,
                                                    node_for_gap_up.layout_mode,
                                                    next,
                                                    node_for_gap_up.padding_edges,
                                                    node_for_gap_up.grid_columns,
                                                    node_for_gap_up.main_align,
                                                    node_for_gap_up.cross_align,
                                                ) {
                                                    this.notify_document_mutated(cx);
                                                }
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(p.on_surface)
                                    .child(format!("Padding: {:.0}px · Grid cols: {}", node.padding_px, node.grid_columns)),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .id("figcraft-pad-minus")
                                            .px_2()
                                            .py_0p5()
                                            .rounded(px(6.0))
                                            .bg(p.surface_container_high)
                                            .text_xs()
                                            .text_color(p.on_surface)
                                            .cursor_pointer()
                                            .child("-4p")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                let next = (node_for_pad_dn.padding_px - 4.0).max(0.0);
                                                if this.doc.update_node_layout(
                                                    &nid_pad_dn,
                                                    node_for_pad_dn.axis,
                                                    node_for_pad_dn.layout_mode,
                                                    node_for_pad_dn.gap_px,
                                                    PaddingEdges::uniform(next),
                                                    node_for_pad_dn.grid_columns,
                                                    node_for_pad_dn.main_align,
                                                    node_for_pad_dn.cross_align,
                                                ) {
                                                    this.notify_document_mutated(cx);
                                                }
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id("figcraft-pad-plus")
                                            .px_2()
                                            .py_0p5()
                                            .rounded(px(6.0))
                                            .bg(p.surface_container_high)
                                            .text_xs()
                                            .text_color(p.on_surface)
                                            .cursor_pointer()
                                            .child("+4p")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                let next = node_for_pad_up.padding_px + 4.0;
                                                if this.doc.update_node_layout(
                                                    &nid_pad_up,
                                                    node_for_pad_up.axis,
                                                    node_for_pad_up.layout_mode,
                                                    node_for_pad_up.gap_px,
                                                    PaddingEdges::uniform(next),
                                                    node_for_pad_up.grid_columns,
                                                    node_for_pad_up.main_align,
                                                    node_for_pad_up.cross_align,
                                                ) {
                                                    this.notify_document_mutated(cx);
                                                }
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id("figcraft-grid-cols-cycle")
                                            .px_2()
                                            .py_0p5()
                                            .rounded(px(6.0))
                                            .bg(p.secondary_container)
                                            .text_xs()
                                            .text_color(p.on_secondary_container)
                                            .cursor_pointer()
                                            .child("Cols+")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                let next_cols = (node_for_cols.grid_columns % 4) + 1;
                                                if this.doc.update_node_layout(
                                                    &nid_cols,
                                                    node_for_cols.axis,
                                                    AutoLayoutMode::Grid,
                                                    node_for_cols.gap_px,
                                                    node_for_cols.padding_edges,
                                                    next_cols,
                                                    node_for_cols.main_align,
                                                    node_for_cols.cross_align,
                                                ) {
                                                    this.notify_document_mutated(cx);
                                                }
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(p.on_surface_variant)
                                    .child(format!("Corner Radius ({:.0}px)", node.corner_radius_px)),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .flex_wrap()
                                    .children([0.0_f32, 8.0, 12.0, 16.0, 28.0, 9999.0].into_iter().map(
                                        |radius| {
                                            let active = (node.corner_radius_px - radius).abs() < 0.5;
                                            let nid = nid_radius.clone();
                                            let n = node_for_rad.clone();
                                            let lbl = if radius >= 9999.0 {
                                                "Pill".to_owned()
                                            } else {
                                                format!("{radius:.0}")
                                            };
                                            div()
                                                .id(SharedString::from(format!("figcraft-rad-{lbl}")))
                                                .px_2()
                                                .py_0p5()
                                                .rounded(px(6.0))
                                                .bg(if active {
                                                    p.primary
                                                } else {
                                                    p.surface_container_high
                                                })
                                                .text_xs()
                                                .text_color(if active {
                                                    p.on_primary
                                                } else {
                                                    p.on_surface
                                                })
                                                .cursor_pointer()
                                                .child(lbl)
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    if this.doc.update_node_sizing(
                                                        &nid,
                                                        n.width_px,
                                                        n.height_px,
                                                        n.width_sizing,
                                                        n.height_sizing,
                                                        radius,
                                                    ) {
                                                        this.notify_document_mutated(cx);
                                                    }
                                                }))
                                        },
                                    )),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .children([
                                (SizingConstraint::Fixed(node.width_px), "Fixed"),
                                (SizingConstraint::Hug, "Hug"),
                                (SizingConstraint::Fill, "Fill"),
                            ]
                            .into_iter()
                            .map(|(constraint, label)| {
                                let active = std::mem::discriminant(&node.width_sizing)
                                    == std::mem::discriminant(&constraint);
                                let nid = nid_sizing.clone();
                                let n = node_for_sz.clone();
                                div()
                                    .id(SharedString::from(format!("figcraft-width-{label}")))
                                    .flex_1()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .bg(if active { p.primary } else { p.surface_container_high })
                                    .text_xs()
                                    .text_center()
                                    .text_color(if active { p.on_primary } else { p.on_surface })
                                    .cursor_pointer()
                                    .child(format!("W: {label}"))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if this.doc.update_node_sizing(
                                            &nid,
                                            n.width_px,
                                            n.height_px,
                                            constraint,
                                            n.height_sizing,
                                            n.corner_radius_px,
                                        ) {
                                            this.notify_document_mutated(cx);
                                        }
                                    }))
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .id("figcraft-duplicate-node")
                            .flex_1()
                            .py_1p5()
                            .rounded(px(10.0))
                            .bg(p.secondary_container)
                            .text_xs()
                            .text_center()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.on_secondary_container)
                            .cursor_pointer()
                            .child("Duplicate (Ctrl+D)")
                            .on_click(cx.listener(|this, _, _, cx| this.duplicate_selected(cx))),
                    )
                    .child(
                        div()
                            .id("figcraft-delete-node")
                            .flex_1()
                            .py_1p5()
                            .rounded(px(10.0))
                            .bg(p.error)
                            .text_xs()
                            .text_center()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.on_error)
                            .cursor_pointer()
                            .child("Delete (Del)")
                            .on_click(cx.listener(|this, _, _, cx| this.delete_selected(cx))),
                    ),
            )
    }

    fn render_inspector_tokens(
        &self,
        selected: Option<&FigCraftFrameNode>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = self.palette;
        let Some(node) = selected.cloned() else {
            return v_flex().p_3().child(
                div()
                    .text_xs()
                    .text_color(p.on_surface_variant)
                    .child("Select a node to inspect live 4-mode M3 token bindings."),
            );
        };

        let contrast = self
            .doc
            .contrast_ratio_for_roles(&node.text_role, &node.fill_role);
        let contrast_text = contrast.map_or_else(
            || "N/A".to_owned(),
            |r| {
                let status = if r >= 4.5 { "WCAG AA PASS" } else { "CHECK PAIR" };
                format!("{r:.2}:1 · {status}")
            },
        );
        let contrast_ok = contrast.is_some_and(|r| r >= 4.5);

        let role_pairs = [
            ("surface", "on-surface"),
            ("surface-container-low", "on-surface"),
            ("surface-container", "on-surface"),
            ("surface-container-high", "on-surface"),
            ("primary", "on-primary"),
            ("primary-container", "on-primary-container"),
            ("secondary-container", "on-secondary-container"),
            ("tertiary-container", "on-tertiary-container"),
        ];

        let comp_def = node.component_ref.as_deref().and_then(|cid| {
            self.doc
                .component_definitions
                .iter()
                .find(|c| c.id == cid)
                .cloned()
        });

        v_flex()
            .gap_2p5()
            .child(
                v_flex()
                    .p_2p5()
                    .gap_2()
                    .rounded(px(12.0))
                    .bg(p.surface_container)
                    .border_1()
                    .border_color(p.outline_variant)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.on_surface_variant)
                                    .child(format!("4-MODE M3 TOKENS ({})", self.doc.active_mode.label())),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(999.0))
                                    .bg(if contrast_ok {
                                        p.primary_container
                                    } else {
                                        p.error
                                    })
                                    .text_xs()
                                    .text_color(if contrast_ok {
                                        p.on_primary_container
                                    } else {
                                        p.on_error
                                    })
                                    .child(contrast_text),
                            ),
                    )
                    .children(role_pairs.into_iter().map(|(fill_role, text_role)| {
                        let active = node.fill_role == fill_role;
                        let nid = node.id.clone();
                        let spring = node.spring_token.clone();
                        let elev = node.elevation_level;
                        let hex = self.doc.resolve_role_hex(fill_role);
                        let swatch = self.resolve_node_hsla(fill_role, p.surface_container);
                        div()
                            .id(SharedString::from(format!("figcraft-role-{fill_role}")))
                            .px_2()
                            .py_1()
                            .rounded(px(8.0))
                            .bg(if active {
                                p.secondary_container
                            } else {
                                p.surface_container_low
                            })
                            .border_1()
                            .border_color(if active { p.primary } else { p.outline_variant })
                            .cursor_pointer()
                            .child(
                                h_flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child(div().w(px(14.0)).h(px(14.0)).rounded(px(4.0)).bg(swatch))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(p.on_surface)
                                                    .child(fill_role),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(p.on_surface_variant)
                                            .child(hex),
                                    ),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this
                                    .doc
                                    .update_node_tokens(&nid, fill_role, text_role, &spring, elev)
                                {
                                    this.notify_document_mutated(cx);
                                }
                            }))
                    })),
            )
            .when_some(comp_def, |col, def| {
                col.child(
                    v_flex()
                        .p_2p5()
                        .gap_1p5()
                        .rounded(px(12.0))
                        .bg(p.surface_container)
                        .border_1()
                        .border_color(p.outline_variant)
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.on_surface_variant)
                                .child(format!("COMPONENT VARIANTS · {}", def.name)),
                        )
                        .child(
                            h_flex()
                                .gap_1p5()
                                .flex_wrap()
                                .children(def.variants.into_iter().map(|variant| {
                                    let active = node.active_variant.as_deref()
                                        == Some(variant.name.as_str());
                                    let nid = node.id.clone();
                                    let cid = def.id.clone();
                                    let vname = variant.name;
                                    div()
                                        .id(SharedString::from(format!("figcraft-variant-{vname}")))
                                        .px_2p5()
                                        .py_1()
                                        .rounded(px(999.0))
                                        .bg(if active {
                                            p.primary
                                        } else {
                                            p.surface_container_high
                                        })
                                        .text_xs()
                                        .text_color(if active {
                                            p.on_primary
                                        } else {
                                            p.on_surface
                                        })
                                        .cursor_pointer()
                                        .child(vname.clone())
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if this
                                                .doc
                                                .apply_component_variant(&nid, &cid, &vname)
                                            {
                                                this.notify_document_mutated(cx);
                                            }
                                        }))
                                })),
                        ),
                )
            })
    }

    fn render_inspector_springs(
        &self,
        selected: Option<&FigCraftFrameNode>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = self.palette;
        let Some(node) = selected.cloned() else {
            return v_flex().p_3().child(
                div()
                    .text_xs()
                    .text_color(p.on_surface_variant)
                    .child("Select a node to bind Spatial/Effects Springs & Google Sans Flex axes."),
            );
        };

        let spring_presets = [
            ("spatial_fast", "Spatial Fast (0.9 ζ)"),
            ("spatial_default", "Spatial Default (0.9 ζ)"),
            ("spatial_slow", "Spatial Slow (0.9 ζ)"),
            ("effects_fast", "Effects Fast (1.0 ζ)"),
            ("effects_default", "Effects Default (1.0 ζ)"),
            ("effects_slow", "Effects Slow (1.0 ζ)"),
        ];

        let active_spring: SpringSpec = self.doc.resolve_spring(&node.spring_token);
        let settle_150ms = active_spring.sample_position(0.15);
        let settle_300ms = active_spring.sample_position(0.30);

        let font_presets = [
            ("Display Hero", GoogleSansFlexAxes::display_hero()),
            ("Headline", GoogleSansFlexAxes::section_headline()),
            ("UI Control", GoogleSansFlexAxes::ui_control()),
            ("Tabular", GoogleSansFlexAxes::tabular_label()),
            ("Compact", GoogleSansFlexAxes::compact_mobile()),
            ("XR Glimmer", GoogleSansFlexAxes::xr_glimmer_glanceable()),
        ];

        v_flex()
            .gap_2p5()
            .child(
                v_flex()
                    .p_2p5()
                    .gap_1p5()
                    .rounded(px(12.0))
                    .bg(p.surface_container)
                    .border_1()
                    .border_color(p.outline_variant)
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.on_surface_variant)
                            .child("M3 EXPRESSIVE SPATIAL & EFFECTS SPRINGS"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.primary)
                            .child(format!(
                                "k={:.0}, ζ={:.2} | x(150ms)={:.3} · x(300ms)={:.3}",
                                active_spring.stiffness,
                                active_spring.damping_ratio,
                                settle_150ms,
                                settle_300ms
                            )),
                    )
                    .children(spring_presets.into_iter().map(|(token_name, label)| {
                        let active = node.spring_token == token_name;
                        let spec = self
                            .tokens
                            .spring(token_name)
                            .unwrap_or_else(|| self.doc.resolve_spring(token_name));
                        let nid = node.id.clone();
                        let fill = node.fill_role.clone();
                        let text = node.text_role.clone();
                        let elev = node.elevation_level;
                        div()
                            .id(SharedString::from(format!("figcraft-spring-{token_name}")))
                            .px_2()
                            .py_1()
                            .rounded(px(8.0))
                            .bg(if active {
                                p.primary_container
                            } else {
                                p.surface_container_low
                            })
                            .border_1()
                            .border_color(if active { p.primary } else { p.outline_variant })
                            .cursor_pointer()
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(if active {
                                                p.on_primary_container
                                            } else {
                                                p.on_surface
                                            })
                                            .child(label),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(p.on_surface_variant)
                                            .child(format!("k={:.0}", spec.stiffness)),
                                    ),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this
                                    .doc
                                    .update_node_tokens(&nid, &fill, &text, token_name, elev)
                                {
                                    this.notify_document_mutated(cx);
                                }
                            }))
                    })),
            )
            .child(
                v_flex()
                    .p_2p5()
                    .gap_1p5()
                    .rounded(px(12.0))
                    .bg(p.surface_container)
                    .border_1()
                    .border_color(p.outline_variant)
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.on_surface_variant)
                            .child("GOOGLE SANS FLEX VARIABLE AXES"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.on_surface)
                            .child(node.flex_axes.css_font_variation_settings()),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .flex_wrap()
                            .children(font_presets.into_iter().map(|(label, axes)| {
                                let active = (node.flex_axes.opsz - axes.opsz).abs() < 0.1
                                    && (node.flex_axes.wght - axes.wght).abs() < 0.1;
                                let nid = node.id.clone();
                                div()
                                    .id(SharedString::from(format!("figcraft-axes-{label}")))
                                    .px_2()
                                    .py_1()
                                    .rounded(px(8.0))
                                    .bg(if active {
                                        p.primary
                                    } else {
                                        p.surface_container_high
                                    })
                                    .text_xs()
                                    .text_color(if active {
                                        p.on_primary
                                    } else {
                                        p.on_surface
                                    })
                                    .cursor_pointer()
                                    .child(label)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if this.doc.update_node_flex_axes(&nid, axes) {
                                            this.notify_document_mutated(cx);
                                        }
                                    }))
                            })),
                    ),
            )
    }

    fn render_inspector_code(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette;
        let line_count = self.cached_export_code.lines().count();
        let lh_score = self.cached_lighthouse_score.unwrap_or(100);

        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .children(FigCraftExportTarget::ALL.into_iter().map(|target| {
                        let active = self.export_target == target;
                        let tab_sel = format!("figcraft-export-tab-{}", target.slug());
                        div()
                            .id(SharedString::from(tab_sel.clone()))
                            .debug_selector(move || tab_sel)
                            .px_2()
                            .py_1()
                            .rounded(px(8.0))
                            .bg(if active {
                                p.primary
                            } else {
                                p.surface_container_high
                            })
                            .text_xs()
                            .text_color(if active {
                                p.on_primary
                            } else {
                                p.on_surface
                            })
                            .cursor_pointer()
                            .child(target.label())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_export_target(target, cx);
                            }))
                    })),
            )
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.on_surface_variant)
                            .child(format!(
                                "{} · {line_count} lines · {} B",
                                self.export_target.file_extension(),
                                self.cached_export_code.len()
                            )),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .rounded(px(999.0))
                            .bg(p.primary_container)
                            .text_xs()
                            .text_color(p.on_primary_container)
                            .child(format!("Lighthouse {lh_score}/100")),
                    ),
            )
            .child(
                div()
                    .id("figcraft-code-preview-box")
                    .max_h(px(420.0))
                    .overflow_y_scroll()
                    .track_scroll(&self.code_preview_scroll)
                    .p_2p5()
                    .rounded(px(12.0))
                    .bg(p.surface_container_lowest)
                    .border_1()
                    .border_color(p.outline_variant)
                    .text_xs()
                    .text_color(p.on_surface)
                    .child(self.cached_export_code.clone()),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .id("figcraft-copy-export-panel")
                            .debug_selector(|| "figcraft-copy-export-panel".into())
                            .flex_1()
                            .py_1p5()
                            .rounded(px(999.0))
                            .bg(p.primary)
                            .text_xs()
                            .text_center()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.on_primary)
                            .cursor_pointer()
                            .child(format!("Copy {}", self.export_target.label()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.copy_active_export(cx);
                            })),
                    ),
            )
    }

    fn render_footer(&self) -> impl IntoElement {
        let p = self.palette;
        let m3_mode = match self.doc.active_mode {
            FigCraftDesignMode::Light => M3AppearanceMode::Light,
            FigCraftDesignMode::Dark => M3AppearanceMode::Dark,
            FigCraftDesignMode::NeuralObsidian => M3AppearanceMode::NeuralObsidian,
            FigCraftDesignMode::OledPitch => M3AppearanceMode::OledPitch,
        };
        let contrast_ratio = m3_contrast_ratio(p.on_surface, p.surface);
        let status_line = format!(
            "{} · Mode: {} · WCAG AA {contrast_ratio:.1}:1",
            self.status_message,
            self.doc.active_mode.label()
        );

        h_flex()
            .w_full()
            .px_3()
            .py_1p5()
            .justify_between()
            .items_center()
            .bg(p.surface_container)
            .border_t_1()
            .border_color(p.outline)
            .child(
                div()
                    .text_xs()
                    .text_color(p.on_surface_variant)
                    .child(status_line),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(p.primary)
                    .child(format!(
                        "gpui-fast + gpui-kit M3 ({}) · Lighthouse {}/100",
                        m3_mode.as_str(),
                        self.cached_lighthouse_score.unwrap_or(100)
                    )),
            )
    }
}

struct LayerRow {
    id: String,
    name: String,
    kind_badge: &'static str,
    depth: usize,
    visible: bool,
    locked: bool,
}

fn collect_layer_rows(node: &FigCraftFrameNode, depth: usize, out: &mut Vec<LayerRow>) {
    out.push(LayerRow {
        id: node.id.clone(),
        name: node.name.clone(),
        kind_badge: node.kind.badge_label(),
        depth,
        visible: node.visible,
        locked: node.locked,
    });
    for child in &node.children {
        collect_layer_rows(child, depth + 1, out);
    }
}

impl Focusable for FigCraftStudioView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<FigCraftStudioEvent> for FigCraftStudioView {}

impl Render for FigCraftStudioView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette;

        v_flex()
            .id("figcraft-studio-root")
            .size_full()
            .bg(p.surface)
            .text_color(p.on_surface)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .child(self.render_top_bar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(self.render_left_dock(cx))
                    .child(self.render_canvas_area(cx))
                    .child(self.render_right_inspector(cx)),
            )
            .child(self.render_footer())
    }
}

fn fallback_design_md() -> DesignMdDocument {
    DesignMdDocument {
        name: "Aphrody Google Design System".to_owned(),
        source_of_truth: "https://design.google/".to_owned(),
        system: "Material 3 Expressive".to_owned(),
        stitch_url: "https://stitch.withgoogle.com/".to_owned(),
        sans_flex_family: "Google Sans Flex".to_owned(),
        colors: Default::default(),
        typography_raw: Default::default(),
        rounded: Default::default(),
        spacing: Default::default(),
        body_markdown: String::new(),
        headings: Vec::new(),
        yaml_blocks: 0,
    }
}

fn fallback_google_tokens() -> GoogleDesignTokens {
    GoogleDesignTokens {
        schema_version: 1,
        source_of_truth: "https://design.google/".to_owned(),
        system: "Material 3 Expressive".to_owned(),
        spec_url: "https://m3.material.io/".to_owned(),
        fonts_url: "https://fonts.google.com/".to_owned(),
        stitch_url: "https://stitch.withgoogle.com/".to_owned(),
        sans_flex_family: "Google Sans Flex".to_owned(),
        synced_at: String::new(),
        seed_hex: "#135B82".to_owned(),
        variant: "expressive".to_owned(),
        principles: Vec::new(),
        reference: aphrody_ui::google_design::RefTokens {
            palette: Default::default(),
            typeface: Default::default(),
            variable_axes: Default::default(),
        },
        sys: aphrody_ui::google_design::SysTokens {
            color: aphrody_ui::google_design::ColorPaletteMap {
                light: Default::default(),
                dark: Default::default(),
                neural_obsidian: Default::default(),
                oled_pitch: Default::default(),
            },
            motion: aphrody_ui::google_design::MotionTokens {
                springs: Default::default(),
            },
            shape: aphrody_ui::ShapeScale {
                none_px: 0.0,
                extra_small_px: 4.0,
                small_px: 8.0,
                medium_px: 12.0,
                large_px: 16.0,
                large_increased_px: 20.0,
                extra_large_px: 28.0,
                full_px: 9999.0,
            },
            state: aphrody_ui::StateLayerOpacities {
                hover_opacity: 0.08,
                focus_opacity: 0.10,
                pressed_opacity: 0.10,
                dragged_opacity: 0.16,
                disabled_container_opacity: 0.12,
                disabled_content_opacity: 0.38,
                focus_ring_px: 2.0,
            },
        },
        comp: Default::default(),
    }
}

#[cfg(all(test, feature = "studio"))]
mod tests {
    use super::*;
    use gpui::{Modifiers, TestAppContext, size};

    #[gpui::test]
    fn figcraft_studio_view_renders_and_switches_4_m3_modes(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| FigCraftStudioView::new(cx));
        cx.simulate_resize(size(px(1440.0), px(900.0)));
        cx.run_until_parked();

        view.read_with(cx, |view, _| {
            assert_eq!(view.document().active_mode, FigCraftDesignMode::Dark);
            assert!(view.document().pages.len() >= 3);
            assert!(view.document().total_node_count() >= 10);
        });

        for (elem_id, expected_mode) in [
            ("figcraft-mode-light", FigCraftDesignMode::Light),
            (
                "figcraft-mode-neural-obsidian",
                FigCraftDesignMode::NeuralObsidian,
            ),
            ("figcraft-mode-oled-pitch", FigCraftDesignMode::OledPitch),
            ("figcraft-mode-dark", FigCraftDesignMode::Dark),
        ] {
            let bounds = cx
                .debug_bounds(elem_id)
                .unwrap_or_else(|| panic!("missing mode toggle bounds `{elem_id}`"));
            cx.simulate_mouse_move(bounds.center(), None, Modifiers::default());
            cx.simulate_click(bounds.center(), Modifiers::default());
            cx.run_until_parked();
            view.read_with(cx, |view, _| {
                assert_eq!(view.document().active_mode, expected_mode);
            });
        }
    }

    #[gpui::test]
    fn figcraft_studio_view_palette_insert_undo_redo_and_export_clipboard(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, cx| FigCraftStudioView::new(cx));
        cx.simulate_resize(size(px(1440.0), px(900.0)));
        cx.run_until_parked();

        let initial_nodes = view.read_with(cx, |view, _| view.document().total_node_count());

        let palette_bounds = cx
            .debug_bounds("figcraft-palette-m3-fab")
            .expect("palette m3-fab button rendered");
        cx.simulate_mouse_move(palette_bounds.center(), None, Modifiers::default());
        cx.simulate_click(palette_bounds.center(), Modifiers::default());
        cx.run_until_parked();

        view.read_with(cx, |view, _| {
            assert_eq!(view.document().total_node_count(), initial_nodes + 1);
            assert!(view.document().can_undo());
        });

        let undo_bounds = cx.debug_bounds("figcraft-undo").expect("undo button rendered");
        cx.simulate_click(undo_bounds.center(), Modifiers::default());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert_eq!(view.document().total_node_count(), initial_nodes);
            assert!(view.document().can_redo());
        });

        let redo_bounds = cx.debug_bounds("figcraft-redo").expect("redo button rendered");
        cx.simulate_click(redo_bounds.center(), Modifiers::default());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert_eq!(view.document().total_node_count(), initial_nodes + 1);
        });

        // Switch inspector to Code tab and test all 4 export targets + Copy to Clipboard.
        let code_tab = cx
            .debug_bounds("figcraft-insp-tab-code")
            .expect("code inspector tab rendered");
        cx.simulate_click(code_tab.center(), Modifiers::default());
        cx.run_until_parked();

        for (tab_id, target, needle) in [
            (
                "figcraft-export-tab-gpui-kit-rs",
                FigCraftExportTarget::GpuiKitRs,
                "gpui_kit::component",
            ),
            (
                "figcraft-export-tab-nsh-rs-tsx",
                FigCraftExportTarget::NshRsTsx,
                "data-stitch-id=",
            ),
            (
                "figcraft-export-tab-gpui-rs",
                FigCraftExportTarget::GpuiRs,
                "render_figcraft_screen",
            ),
            (
                "figcraft-export-tab-lighthouse100-html",
                FigCraftExportTarget::Lighthouse100Html,
                "<!doctype html>",
            ),
        ] {
            let tab_bounds = cx
                .debug_bounds(tab_id)
                .unwrap_or_else(|| panic!("missing export tab `{tab_id}`"));
            cx.simulate_click(tab_bounds.center(), Modifiers::default());
            cx.run_until_parked();
            view.read_with(cx, |view, _| {
                assert_eq!(view.active_export_target(), target);
                assert!(view.cached_export_code().contains(needle));
            });
        }

        let copy_bounds = cx
            .debug_bounds("figcraft-copy-export")
            .expect("copy export button rendered");
        assert!(copy_bounds.center().x < px(1440.0));
        cx.simulate_mouse_move(copy_bounds.center(), None, Modifiers::default());
        cx.simulate_click(copy_bounds.center(), Modifiers::default());
        cx.run_until_parked();
        let clip_text = cx
            .read_from_clipboard()
            .and_then(|item| item.text().map(|s| s.to_string()))
            .expect("clipboard contains exported code");
        assert!(clip_text.contains("<!doctype html>"));
    }

    #[gpui::test]
    fn figcraft_studio_view_zoom_and_page_addition_work(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| FigCraftStudioView::new(cx));
        cx.simulate_resize(size(px(1440.0), px(900.0)));
        cx.run_until_parked();

        let initial_pages = view.read_with(cx, |view, _| view.document().pages.len());

        let zoom_in = cx
            .debug_bounds("figcraft-zoom-in")
            .expect("zoom in button rendered");
        cx.simulate_click(zoom_in.center(), Modifiers::default());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert!(view.zoom() > 1.0);
        });

        let zoom_reset = cx
            .debug_bounds("figcraft-zoom-reset")
            .expect("zoom reset button rendered");
        cx.simulate_click(zoom_reset.center(), Modifiers::default());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert!((view.zoom() - 1.0).abs() < 1e-4);
        });

        let add_page = cx
            .debug_bounds("figcraft-add-page")
            .expect("add page button rendered");
        cx.simulate_click(add_page.center(), Modifiers::default());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert_eq!(view.document().pages.len(), initial_pages + 1);
        });
    }
}
