//! FigCraft Google Edition Core: multi-page auto-layout document tree, undo/redo command stack,
//! component variant definitions, 24+ Material 3 Expressive widget templates, `.fig` / `.sketch` /
//! `.penpot` container bridge, live 4-mode Google Design M3 token bindings, `data-stitch-id`
//! round-trip tracking, and 4 deterministic code exporters.

use std::collections::BTreeMap;

use anyhow::Result;
use aphrody_design::design_md::{DesignMdDocument, contrast_ratio_hex, load_bundled_root_design_md};
use aphrody_ui::AppearanceMode;
use aphrody_ui::google_design::{GoogleDesignTokens, GoogleSansFlexAxes, SpringSpec};
use serde::{Deserialize, Serialize};

const MAX_UNDO_HISTORY: usize = 128;

/// Live 4-mode Google Design Material 3 Expressive surface modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FigCraftDesignMode {
    Light,
    #[default]
    Dark,
    NeuralObsidian,
    OledPitch,
}

impl FigCraftDesignMode {
    pub const ALL: [Self; 4] = [
        Self::Light,
        Self::Dark,
        Self::NeuralObsidian,
        Self::OledPitch,
    ];

    #[must_use]
    pub const fn palette_key(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::NeuralObsidian => "neural_obsidian",
            Self::OledPitch => "oled_pitch",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::NeuralObsidian => "NeuralObsidian",
            Self::OledPitch => "OledPitch",
        }
    }

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::NeuralObsidian => "neural-obsidian",
            Self::OledPitch => "oled-pitch",
        }
    }

    #[must_use]
    pub const fn appearance_mode(self) -> AppearanceMode {
        match self {
            Self::Light => AppearanceMode::Light,
            Self::Dark | Self::NeuralObsidian | Self::OledPitch => AppearanceMode::Dark,
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::NeuralObsidian,
            Self::NeuralObsidian => Self::OledPitch,
            Self::OledPitch => Self::Light,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoLayoutAxis {
    Horizontal,
    #[default]
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoLayoutMode {
    #[default]
    Flex,
    Grid,
    Freeform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutAlignment {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    Stretch,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizingConstraint {
    Fixed(f32),
    #[default]
    Hug,
    Fill,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PaddingEdges {
    pub top_px: f32,
    pub right_px: f32,
    pub bottom_px: f32,
    pub left_px: f32,
}

impl PaddingEdges {
    #[must_use]
    pub const fn uniform(px: f32) -> Self {
        Self {
            top_px: px,
            right_px: px,
            bottom_px: px,
            left_px: px,
        }
    }

    #[must_use]
    pub const fn symmetric(vertical_px: f32, horizontal_px: f32) -> Self {
        Self {
            top_px: vertical_px,
            right_px: horizontal_px,
            bottom_px: vertical_px,
            left_px: horizontal_px,
        }
    }
}

impl Default for PaddingEdges {
    fn default() -> Self {
        Self::uniform(16.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FigCraftNodeKind {
    #[default]
    Frame,
    Screen,
    Section,
    Text,
    Button,
    Fab,
    Card,
    Chip,
    Input,
    Switch,
    Slider,
    NavRail,
    TopBar,
    BottomBar,
    Dialog,
    Badge,
    Progress,
    ListItem,
    KpiTile,
    SegmentedBar,
    SpectrumBanner,
}

impl FigCraftNodeKind {
    #[must_use]
    pub const fn badge_label(self) -> &'static str {
        match self {
            Self::Frame => "FRAME",
            Self::Screen => "SCREEN",
            Self::Section => "SECTION",
            Self::Text => "TEXT",
            Self::Button => "BTN",
            Self::Fab => "FAB",
            Self::Card => "CARD",
            Self::Chip => "CHIP",
            Self::Input => "INPUT",
            Self::Switch => "SWITCH",
            Self::Slider => "SLIDER",
            Self::NavRail => "RAIL",
            Self::TopBar => "APPBAR",
            Self::BottomBar => "NAVBAR",
            Self::Dialog => "DIALOG",
            Self::Badge => "BADGE",
            Self::Progress => "PROG",
            Self::ListItem => "LIST",
            Self::KpiTile => "KPI",
            Self::SegmentedBar => "SEG",
            Self::SpectrumBanner => "GEMINI",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FigTokenBinding {
    pub property: String,
    pub design_md_role: String,
    pub resolved_value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FigCraftFrameNode {
    pub id: String,
    pub stitch_id: String,
    pub name: String,
    pub axis: AutoLayoutAxis,
    pub width_px: f32,
    pub height_px: f32,
    pub gap_px: f32,
    pub padding_px: f32,
    pub corner_radius_px: f32,
    pub fill_role: String,
    pub text_content: Option<String>,
    pub bindings: Vec<FigTokenBinding>,
    pub children: Vec<FigCraftFrameNode>,
    #[serde(default)]
    pub kind: FigCraftNodeKind,
    #[serde(default)]
    pub x_px: f32,
    #[serde(default)]
    pub y_px: f32,
    #[serde(default)]
    pub layout_mode: AutoLayoutMode,
    #[serde(default = "default_grid_columns")]
    pub grid_columns: u16,
    #[serde(default)]
    pub padding_edges: PaddingEdges,
    #[serde(default)]
    pub main_align: LayoutAlignment,
    #[serde(default)]
    pub cross_align: LayoutAlignment,
    #[serde(default)]
    pub width_sizing: SizingConstraint,
    #[serde(default)]
    pub height_sizing: SizingConstraint,
    #[serde(default = "default_text_role")]
    pub text_role: String,
    #[serde(default)]
    pub stroke_role: Option<String>,
    #[serde(default)]
    pub stroke_width_px: f32,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub elevation_level: u8,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub component_ref: Option<String>,
    #[serde(default)]
    pub active_variant: Option<String>,
    #[serde(default = "default_spring_token")]
    pub spring_token: String,
    #[serde(default)]
    pub flex_axes: GoogleSansFlexAxes,
}

/// Alias preserving both `FigCraftFrameNode` and `FigCraftNode` naming across callers.
pub type FigCraftNode = FigCraftFrameNode;

fn default_grid_columns() -> u16 {
    2
}

fn default_text_role() -> String {
    "on-surface".to_owned()
}

fn default_opacity() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_spring_token() -> String {
    "spatial_default".to_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageKind {
    #[default]
    Canvas,
    Components,
    Tokens,
    ResponsiveMobile,
}

impl PageKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Canvas => "Canvas",
            Self::Components => "Components",
            Self::Tokens => "Tokens",
            Self::ResponsiveMobile => "Compact <600px",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FigCraftPage {
    pub id: String,
    pub name: String,
    pub kind: PageKind,
    pub root_frames: Vec<FigCraftFrameNode>,
    pub viewport_x: f32,
    pub viewport_y: f32,
    pub zoom: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentVariantSpec {
    pub name: String,
    pub fill_role: String,
    pub text_role: String,
    pub stroke_role: Option<String>,
    pub corner_radius_px: f32,
    pub spring_token: String,
    pub elevation_level: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentVariantDefinition {
    pub id: String,
    pub name: String,
    pub category: String,
    pub default_variant: String,
    pub variants: Vec<ComponentVariantSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FigCraftSnapshot {
    pub pages: Vec<FigCraftPage>,
    pub active_page_id: String,
    pub selected_node_id: Option<String>,
    pub active_mode: FigCraftDesignMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FigCraftCommand {
    pub label: String,
    pub before: FigCraftSnapshot,
    pub after: FigCraftSnapshot,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UndoRedoHistory {
    pub undo_stack: Vec<FigCraftCommand>,
    pub redo_stack: Vec<FigCraftCommand>,
}

impl UndoRedoHistory {
    pub fn record(&mut self, command: FigCraftCommand) {
        self.redo_stack.clear();
        self.undo_stack.push(command);
        if self.undo_stack.len() > MAX_UNDO_HISTORY {
            let overflow = self.undo_stack.len() - MAX_UNDO_HISTORY;
            self.undo_stack.drain(0..overflow);
        }
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    #[must_use]
    pub fn undo_labels(&self) -> Vec<&str> {
        self.undo_stack
            .iter()
            .rev()
            .map(|c| c.label.as_str())
            .collect()
    }

    #[must_use]
    pub fn redo_labels(&self) -> Vec<&str> {
        self.redo_stack
            .iter()
            .rev()
            .map(|c| c.label.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapAxis {
    VerticalX,
    HorizontalY,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapGuideLine {
    pub axis: SnapAxis,
    pub position_px: f32,
    pub start_px: f32,
    pub end_px: f32,
    pub target_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapGuideResult {
    pub snapped_x_px: f32,
    pub snapped_y_px: f32,
    pub guides: Vec<SnapGuideLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCategory {
    Actions,
    Cards,
    Navigation,
    Inputs,
    Feedback,
    Layout,
    Expressive,
}

impl TemplateCategory {
    pub const ALL: [Self; 7] = [
        Self::Actions,
        Self::Cards,
        Self::Navigation,
        Self::Inputs,
        Self::Feedback,
        Self::Layout,
        Self::Expressive,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Actions => "Actions",
            Self::Cards => "Cards",
            Self::Navigation => "Nav",
            Self::Inputs => "Inputs",
            Self::Feedback => "Feedback",
            Self::Layout => "Layout",
            Self::Expressive => "Expressive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StarterWidgetTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub category: TemplateCategory,
    pub description: &'static str,
    pub kind: FigCraftNodeKind,
    pub axis: AutoLayoutAxis,
    pub layout_mode: AutoLayoutMode,
    pub default_width_px: f32,
    pub default_height_px: f32,
    pub gap_px: f32,
    pub padding_px: f32,
    pub corner_radius_px: f32,
    pub fill_role: &'static str,
    pub text_role: &'static str,
    pub stroke_role: Option<&'static str>,
    pub elevation_level: u8,
    pub spring_token: &'static str,
    pub sample_text: &'static str,
    pub gpui_kit_component: &'static str,
    pub nsh_component: &'static str,
    pub html_tag: &'static str,
}

const STARTER_WIDGET_TEMPLATES: [StarterWidgetTemplate; 24] = [
    StarterWidgetTemplate {
        id: "m3-filled-button",
        name: "M3 Filled Button",
        category: TemplateCategory::Actions,
        description: "High-emphasis pill CTA bound to primary / on-primary.",
        kind: FigCraftNodeKind::Button,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 196.0,
        default_height_px: 44.0,
        gap_px: 8.0,
        padding_px: 18.0,
        corner_radius_px: 9999.0,
        fill_role: "primary",
        text_role: "on-primary",
        stroke_role: None,
        elevation_level: 1,
        spring_token: "spatial_fast",
        sample_text: "Sync Design Tokens",
        gpui_kit_component: "Button::new(\"m3-filled\").primary()",
        nsh_component: "View role=\"button\" class=\"rounded-full bg-primary\"",
        html_tag: "button",
    },
    StarterWidgetTemplate {
        id: "m3-tonal-button",
        name: "M3 Tonal Button",
        category: TemplateCategory::Actions,
        description: "Medium-emphasis pill bound to secondary-container.",
        kind: FigCraftNodeKind::Button,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 184.0,
        default_height_px: 44.0,
        gap_px: 8.0,
        padding_px: 16.0,
        corner_radius_px: 9999.0,
        fill_role: "secondary-container",
        text_role: "on-secondary-container",
        stroke_role: None,
        elevation_level: 0,
        spring_token: "spatial_fast",
        sample_text: "Export GPUI Kit",
        gpui_kit_component: "Button::new(\"m3-tonal\").secondary()",
        nsh_component: "View role=\"button\" class=\"rounded-full bg-secondary\"",
        html_tag: "button",
    },
    StarterWidgetTemplate {
        id: "m3-outlined-button",
        name: "M3 Outlined Button",
        category: TemplateCategory::Actions,
        description: "Outlined action pill with 1px outline border.",
        kind: FigCraftNodeKind::Button,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 172.0,
        default_height_px: 44.0,
        gap_px: 8.0,
        padding_px: 16.0,
        corner_radius_px: 9999.0,
        fill_role: "surface",
        text_role: "primary",
        stroke_role: Some("outline"),
        elevation_level: 0,
        spring_token: "effects_fast",
        sample_text: "Inspect Layer",
        gpui_kit_component: "Button::new(\"m3-outlined\").outline()",
        nsh_component: "View role=\"button\" class=\"rounded-full border border-border\"",
        html_tag: "button",
    },
    StarterWidgetTemplate {
        id: "m3-text-button",
        name: "M3 Text Action",
        category: TemplateCategory::Actions,
        description: "Low-emphasis inline text button meeting 44px hit target.",
        kind: FigCraftNodeKind::Button,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 132.0,
        default_height_px: 44.0,
        gap_px: 6.0,
        padding_px: 12.0,
        corner_radius_px: 9999.0,
        fill_role: "surface-container-low",
        text_role: "primary",
        stroke_role: None,
        elevation_level: 0,
        spring_token: "effects_fast",
        sample_text: "Learn More",
        gpui_kit_component: "Button::new(\"m3-text\").ghost()",
        nsh_component: "View role=\"button\" class=\"rounded-full px-3 py-2\"",
        html_tag: "button",
    },
    StarterWidgetTemplate {
        id: "m3-fab",
        name: "M3 Expressive FAB",
        category: TemplateCategory::Actions,
        description: "56x56 square-rounded Floating Action Button.",
        kind: FigCraftNodeKind::Fab,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 56.0,
        default_height_px: 56.0,
        gap_px: 0.0,
        padding_px: 16.0,
        corner_radius_px: 16.0,
        fill_role: "primary-container",
        text_role: "on-primary-container",
        stroke_role: None,
        elevation_level: 3,
        spring_token: "spatial_default",
        sample_text: "+",
        gpui_kit_component: "Button::new(\"m3-fab\").primary()",
        nsh_component: "View role=\"button\" class=\"w-14 h-14 rounded-2xl bg-primary\"",
        html_tag: "button",
    },
    StarterWidgetTemplate {
        id: "m3-extended-fab",
        name: "M3 Extended FAB",
        category: TemplateCategory::Actions,
        description: "Labeled primary floating action container.",
        kind: FigCraftNodeKind::Fab,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 204.0,
        default_height_px: 56.0,
        gap_px: 12.0,
        padding_px: 20.0,
        corner_radius_px: 16.0,
        fill_role: "primary-container",
        text_role: "on-primary-container",
        stroke_role: None,
        elevation_level: 3,
        spring_token: "spatial_default",
        sample_text: "+ New Component",
        gpui_kit_component: "Button::new(\"m3-ext-fab\").primary()",
        nsh_component: "View role=\"button\" class=\"h-14 rounded-2xl bg-primary px-5\"",
        html_tag: "button",
    },
    StarterWidgetTemplate {
        id: "m3-segmented-buttons",
        name: "M3 Segmented Bar",
        category: TemplateCategory::Actions,
        description: "Connected pill group for mode & density switching.",
        kind: FigCraftNodeKind::SegmentedBar,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 340.0,
        default_height_px: 44.0,
        gap_px: 4.0,
        padding_px: 4.0,
        corner_radius_px: 9999.0,
        fill_role: "surface-container",
        text_role: "on-surface",
        stroke_role: Some("outline-variant"),
        elevation_level: 0,
        spring_token: "spatial_fast",
        sample_text: "Light · Dark · Obsidian · OLED",
        gpui_kit_component: "h_flex().rounded_full().p_1()",
        nsh_component: "View class=\"flex-row rounded-full bg-muted p-1\"",
        html_tag: "div",
    },
    StarterWidgetTemplate {
        id: "m3-filter-chip",
        name: "M3 Filter Chip",
        category: TemplateCategory::Inputs,
        description: "8px rounded selection chip with subtle variant stroke.",
        kind: FigCraftNodeKind::Chip,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 144.0,
        default_height_px: 36.0,
        gap_px: 6.0,
        padding_px: 12.0,
        corner_radius_px: 8.0,
        fill_role: "secondary-container",
        text_role: "on-secondary-container",
        stroke_role: Some("outline-variant"),
        elevation_level: 0,
        spring_token: "effects_fast",
        sample_text: "WCAG AA 4.5+",
        gpui_kit_component: "Tag::secondary()",
        nsh_component: "View class=\"rounded-lg bg-secondary px-3 py-1\"",
        html_tag: "button",
    },
    StarterWidgetTemplate {
        id: "m3-assist-chip",
        name: "M3 Assist Chip",
        category: TemplateCategory::Actions,
        description: "Contextual suggestion chip with 8px M3 small shape.",
        kind: FigCraftNodeKind::Chip,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 168.0,
        default_height_px: 36.0,
        gap_px: 8.0,
        padding_px: 12.0,
        corner_radius_px: 8.0,
        fill_role: "surface-container-low",
        text_role: "on-surface",
        stroke_role: Some("outline"),
        elevation_level: 1,
        spring_token: "effects_fast",
        sample_text: "Bind DESIGN.md",
        gpui_kit_component: "Tag::outline()",
        nsh_component: "View class=\"rounded-lg border border-border px-3 py-1\"",
        html_tag: "button",
    },
    StarterWidgetTemplate {
        id: "m3-elevated-card",
        name: "M3 Elevated Card",
        category: TemplateCategory::Cards,
        description: "Surface-container-low card with elevation 1 and 16px corners.",
        kind: FigCraftNodeKind::Card,
        axis: AutoLayoutAxis::Vertical,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 320.0,
        default_height_px: 156.0,
        gap_px: 10.0,
        padding_px: 18.0,
        corner_radius_px: 16.0,
        fill_role: "surface-container-low",
        text_role: "on-surface",
        stroke_role: Some("outline-variant"),
        elevation_level: 1,
        spring_token: "spatial_default",
        sample_text: "Material 3 Expressive Surface",
        gpui_kit_component: "v_flex().rounded(px(16.0)).p_4()",
        nsh_component: "View class=\"rounded-2xl bg-card p-4 border border-border\"",
        html_tag: "article",
    },
    StarterWidgetTemplate {
        id: "m3-filled-card",
        name: "M3 Filled Card",
        category: TemplateCategory::Cards,
        description: "High-tonal filled card on surface-container-highest.",
        kind: FigCraftNodeKind::Card,
        axis: AutoLayoutAxis::Vertical,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 320.0,
        default_height_px: 156.0,
        gap_px: 10.0,
        padding_px: 18.0,
        corner_radius_px: 16.0,
        fill_role: "surface-container-highest",
        text_role: "on-surface",
        stroke_role: None,
        elevation_level: 0,
        spring_token: "spatial_default",
        sample_text: "Tokenized Filled Container",
        gpui_kit_component: "v_flex().rounded(px(16.0)).p_4()",
        nsh_component: "View class=\"rounded-2xl bg-muted p-4\"",
        html_tag: "article",
    },
    StarterWidgetTemplate {
        id: "m3-outlined-card",
        name: "M3 Outlined Card",
        category: TemplateCategory::Cards,
        description: "Clean flat card with outline-variant hairline border.",
        kind: FigCraftNodeKind::Card,
        axis: AutoLayoutAxis::Vertical,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 320.0,
        default_height_px: 156.0,
        gap_px: 10.0,
        padding_px: 18.0,
        corner_radius_px: 16.0,
        fill_role: "surface",
        text_role: "on-surface",
        stroke_role: Some("outline-variant"),
        elevation_level: 0,
        spring_token: "effects_default",
        sample_text: "Outlined Inspector Card",
        gpui_kit_component: "v_flex().rounded(px(16.0)).border_1().p_4()",
        nsh_component: "View class=\"rounded-2xl border border-border bg-background p-4\"",
        html_tag: "article",
    },
    StarterWidgetTemplate {
        id: "m3-top-app-bar",
        name: "M3 Top App Bar",
        category: TemplateCategory::Navigation,
        description: "64px expressive header bar with title & quick actions.",
        kind: FigCraftNodeKind::TopBar,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 960.0,
        default_height_px: 64.0,
        gap_px: 16.0,
        padding_px: 16.0,
        corner_radius_px: 12.0,
        fill_role: "surface-container",
        text_role: "on-surface",
        stroke_role: Some("outline-variant"),
        elevation_level: 1,
        spring_token: "effects_default",
        sample_text: "FigCraft Neural Expressive",
        gpui_kit_component: "h_flex().h(px(64.0)).px_4()",
        nsh_component: "View class=\"h-16 flex-row items-center justify-between px-4\"",
        html_tag: "header",
    },
    StarterWidgetTemplate {
        id: "m3-navigation-rail",
        name: "M3 Navigation Rail",
        category: TemplateCategory::Navigation,
        description: "80px compact vertical navigation rail for desktop & tablet.",
        kind: FigCraftNodeKind::NavRail,
        axis: AutoLayoutAxis::Vertical,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 80.0,
        default_height_px: 480.0,
        gap_px: 12.0,
        padding_px: 12.0,
        corner_radius_px: 16.0,
        fill_role: "surface-container-low",
        text_role: "on-surface",
        stroke_role: Some("outline-variant"),
        elevation_level: 0,
        spring_token: "spatial_default",
        sample_text: "Canvas · Kit · Tokens",
        gpui_kit_component: "v_flex().w(px(80.0)).gap_3()",
        nsh_component: "View class=\"w-20 flex-col items-center gap-3 p-3\"",
        html_tag: "nav",
    },
    StarterWidgetTemplate {
        id: "m3-navigation-bar",
        name: "M3 Bottom Nav Bar",
        category: TemplateCategory::Navigation,
        description: "Compact <600px mobile bottom bar with >=44px destinations.",
        kind: FigCraftNodeKind::BottomBar,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 390.0,
        default_height_px: 68.0,
        gap_px: 8.0,
        padding_px: 12.0,
        corner_radius_px: 20.0,
        fill_role: "surface-container",
        text_role: "on-surface",
        stroke_role: Some("outline-variant"),
        elevation_level: 2,
        spring_token: "spatial_fast",
        sample_text: "Studio · Layers · Tokens · Export",
        gpui_kit_component: "h_flex().min_h(px(64.0)).px_3()",
        nsh_component: "View class=\"h-16 flex-row items-center justify-around px-3\"",
        html_tag: "nav",
    },
    StarterWidgetTemplate {
        id: "m3-search-omnibar",
        name: "M3 Search Omnibar",
        category: TemplateCategory::Expressive,
        description: "Full-pill Neural Expressive command & search bar.",
        kind: FigCraftNodeKind::Input,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 640.0,
        default_height_px: 56.0,
        gap_px: 12.0,
        padding_px: 16.0,
        corner_radius_px: 9999.0,
        fill_role: "surface-container-high",
        text_role: "on-surface",
        stroke_role: Some("primary"),
        elevation_level: 2,
        spring_token: "spatial_fast",
        sample_text: "+ Ask Gemini · Search M3 Kit or DESIGN.md tokens",
        gpui_kit_component: "Input::new(&search_state).rounded_full()",
        nsh_component: "View class=\"h-14 rounded-full bg-card px-4 border border-primary\"",
        html_tag: "search",
    },
    StarterWidgetTemplate {
        id: "m3-expressive-switch",
        name: "M3 Expressive Switch",
        category: TemplateCategory::Inputs,
        description: "52x32 toggle switch with spring thumb transition.",
        kind: FigCraftNodeKind::Switch,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 180.0,
        default_height_px: 44.0,
        gap_px: 12.0,
        padding_px: 10.0,
        corner_radius_px: 9999.0,
        fill_role: "primary-container",
        text_role: "on-primary-container",
        stroke_role: Some("primary"),
        elevation_level: 0,
        spring_token: "spatial_fast",
        sample_text: "Snap Guides: ON",
        gpui_kit_component: "Switch::new(\"m3-switch\").checked(true)",
        nsh_component: "Switch checked={true} label=\"Snap Guides\"",
        html_tag: "button",
    },
    StarterWidgetTemplate {
        id: "m3-slider",
        name: "M3 Continuous Slider",
        category: TemplateCategory::Inputs,
        description: "Expressive spring slider for gap, radius, and variable axes.",
        kind: FigCraftNodeKind::Slider,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 260.0,
        default_height_px: 44.0,
        gap_px: 10.0,
        padding_px: 12.0,
        corner_radius_px: 9999.0,
        fill_role: "surface-container-high",
        text_role: "on-surface",
        stroke_role: Some("outline-variant"),
        elevation_level: 0,
        spring_token: "spatial_fast",
        sample_text: "Corner Radius: 16px",
        gpui_kit_component: "Slider::new(&slider_state)",
        nsh_component: "Slider min={0} max={32} value={16}",
        html_tag: "div",
    },
    StarterWidgetTemplate {
        id: "m3-linear-progress",
        name: "M3 Linear Progress",
        category: TemplateCategory::Feedback,
        description: "Deterministic progress bar bound to primary / surface-container-highest.",
        kind: FigCraftNodeKind::Progress,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 280.0,
        default_height_px: 28.0,
        gap_px: 8.0,
        padding_px: 8.0,
        corner_radius_px: 9999.0,
        fill_role: "surface-container-highest",
        text_role: "on-surface",
        stroke_role: None,
        elevation_level: 0,
        spring_token: "effects_default",
        sample_text: "Lighthouse Audit 100%",
        gpui_kit_component: "Progress::new().value(100.0)",
        nsh_component: "Progress value={100}",
        html_tag: "div",
    },
    StarterWidgetTemplate {
        id: "m3-dialog-sheet",
        name: "M3 Dialog Sheet",
        category: TemplateCategory::Feedback,
        description: "28px extra-large corner dialog surface with spatial slow enter.",
        kind: FigCraftNodeKind::Dialog,
        axis: AutoLayoutAxis::Vertical,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 380.0,
        default_height_px: 210.0,
        gap_px: 14.0,
        padding_px: 24.0,
        corner_radius_px: 28.0,
        fill_role: "surface-container-high",
        text_role: "on-surface",
        stroke_role: Some("outline-variant"),
        elevation_level: 4,
        spring_token: "spatial_slow",
        sample_text: "Publish FigCraft Bundle",
        gpui_kit_component: "v_flex().rounded(px(28.0)).p_6()",
        nsh_component: "View role=\"dialog\" class=\"rounded-3xl bg-popover p-6\"",
        html_tag: "section",
    },
    StarterWidgetTemplate {
        id: "m3-badge-counter",
        name: "M3 Status Badge",
        category: TemplateCategory::Feedback,
        description: "High-contrast status pill bound to tertiary-container.",
        kind: FigCraftNodeKind::Badge,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 132.0,
        default_height_px: 28.0,
        gap_px: 6.0,
        padding_px: 10.0,
        corner_radius_px: 9999.0,
        fill_role: "tertiary-container",
        text_role: "on-tertiary-container",
        stroke_role: None,
        elevation_level: 0,
        spring_token: "effects_fast",
        sample_text: "GPUI Fast OK",
        gpui_kit_component: "Tag::primary()",
        nsh_component: "View class=\"rounded-full bg-accent px-2.5 py-0.5\"",
        html_tag: "span",
    },
    StarterWidgetTemplate {
        id: "m3-list-item",
        name: "M3 Two-Line List Item",
        category: TemplateCategory::Layout,
        description: "Structured list row with >=44px height and subtle divider.",
        kind: FigCraftNodeKind::ListItem,
        axis: AutoLayoutAxis::Horizontal,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 340.0,
        default_height_px: 56.0,
        gap_px: 12.0,
        padding_px: 14.0,
        corner_radius_px: 12.0,
        fill_role: "surface-container-low",
        text_role: "on-surface",
        stroke_role: Some("outline-variant"),
        elevation_level: 0,
        spring_token: "effects_fast",
        sample_text: "Google Sans Flex · opsz 14 · wght 500",
        gpui_kit_component: "h_flex().min_h(px(56.0)).px_3()",
        nsh_component: "View class=\"min-h-14 flex-row items-center px-3\"",
        html_tag: "div",
    },
    StarterWidgetTemplate {
        id: "m3-kpi-metric-tile",
        name: "M3 Telemetry KPI Tile",
        category: TemplateCategory::Layout,
        description: "Tabular-nums metric card designed for Grid auto-layout.",
        kind: FigCraftNodeKind::KpiTile,
        axis: AutoLayoutAxis::Vertical,
        layout_mode: AutoLayoutMode::Grid,
        default_width_px: 220.0,
        default_height_px: 112.0,
        gap_px: 8.0,
        padding_px: 16.0,
        corner_radius_px: 16.0,
        fill_role: "surface-container",
        text_role: "on-surface",
        stroke_role: Some("outline-variant"),
        elevation_level: 1,
        spring_token: "spatial_fast",
        sample_text: "144 FPS · 0.73ms Retained",
        gpui_kit_component: "v_flex().rounded(px(16.0)).p_4()",
        nsh_component: "View class=\"rounded-2xl bg-card p-4 tabular-nums\"",
        html_tag: "article",
    },
    StarterWidgetTemplate {
        id: "m3-gemini-spectrum-banner",
        name: "Gemini Spectrum Banner",
        category: TemplateCategory::Expressive,
        description: "Neural Expressive gradient-accented AI canvas card.",
        kind: FigCraftNodeKind::SpectrumBanner,
        axis: AutoLayoutAxis::Vertical,
        layout_mode: AutoLayoutMode::Flex,
        default_width_px: 460.0,
        default_height_px: 124.0,
        gap_px: 8.0,
        padding_px: 18.0,
        corner_radius_px: 20.0,
        fill_role: "tertiary-container",
        text_role: "on-tertiary-container",
        stroke_role: Some("primary"),
        elevation_level: 2,
        spring_token: "spatial_default",
        sample_text: "Gemini · Code is a Design Material",
        gpui_kit_component: "v_flex().rounded(px(20.0)).p_4()",
        nsh_component: "View class=\"rounded-2xl bg-accent p-4\"",
        html_tag: "section",
    },
];

/// Returns the 24 built-in Material 3 Expressive & Neural Expressive starter widget templates.
#[must_use]
pub const fn m3_expressive_starter_templates() -> &'static [StarterWidgetTemplate] {
    &STARTER_WIDGET_TEMPLATES
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FigCraftExportTarget {
    #[default]
    GpuiKitRs,
    NshRsTsx,
    GpuiRs,
    Lighthouse100Html,
}

impl FigCraftExportTarget {
    pub const ALL: [Self; 4] = [
        Self::GpuiKitRs,
        Self::NshRsTsx,
        Self::GpuiRs,
        Self::Lighthouse100Html,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::GpuiKitRs => "gpui-kit .rs",
            Self::NshRsTsx => "nsh .rs/.tsx",
            Self::GpuiRs => "gpui .rs",
            Self::Lighthouse100Html => "HTML5 100",
        }
    }

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::GpuiKitRs => "gpui-kit-rs",
            Self::NshRsTsx => "nsh-rs-tsx",
            Self::GpuiRs => "gpui-rs",
            Self::Lighthouse100Html => "lighthouse100-html",
        }
    }

    #[must_use]
    pub const fn file_extension(self) -> &'static str {
        match self {
            Self::GpuiKitRs | Self::GpuiRs => "rs",
            Self::NshRsTsx => "tsx",
            Self::Lighthouse100Html => "html",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FigCraftDocument {
    pub title: String,
    pub design_source: String,
    pub fig_binary_header_magic: String,
    pub root_frames: Vec<FigCraftFrameNode>,
    pub selected_node_id: Option<String>,
    #[serde(default)]
    pub pages: Vec<FigCraftPage>,
    #[serde(default)]
    pub active_page_id: String,
    #[serde(default)]
    pub active_mode: FigCraftDesignMode,
    #[serde(default)]
    pub token_palette: BTreeMap<String, String>,
    #[serde(default)]
    pub component_definitions: Vec<ComponentVariantDefinition>,
    #[serde(default)]
    pub history: UndoRedoHistory,
    #[serde(default = "default_next_id_seq")]
    next_id_seq: u64,
}

fn default_next_id_seq() -> u64 {
    100
}

impl FigCraftDocument {
    /// Synthesizes a multi-page Google Neural Expressive FigCraft workspace bound to root `DESIGN.md` tokens.
    pub fn google_expressive_default() -> Result<Self> {
        let design_md = load_bundled_root_design_md()?;
        Ok(Self::from_design_md("Google Neural Expressive Studio", &design_md))
    }

    #[must_use]
    pub fn from_design_md(title: &str, design_md: &DesignMdDocument) -> Self {
        let mode = FigCraftDesignMode::Dark;
        let token_palette = build_palette_for_mode(mode, design_md);
        let component_definitions = default_component_definitions();

        let omnibar = FigCraftFrameNode {
            id: "node-omnibar".to_owned(),
            stitch_id: "stitch-figcraft-omnibar".to_owned(),
            name: "Neural Expressive Omnibar".to_owned(),
            axis: AutoLayoutAxis::Horizontal,
            width_px: 640.0,
            height_px: 56.0,
            gap_px: 12.0,
            padding_px: 16.0,
            corner_radius_px: 9999.0,
            fill_role: "surface-container-high".to_owned(),
            text_content: Some("+ Ask Gemini · Carbon High · Import .fig / DESIGN.md".to_owned()),
            bindings: Vec::new(),
            children: Vec::new(),
            kind: FigCraftNodeKind::Input,
            x_px: 32.0,
            y_px: 88.0,
            layout_mode: AutoLayoutMode::Flex,
            grid_columns: 1,
            padding_edges: PaddingEdges::symmetric(12.0, 16.0),
            main_align: LayoutAlignment::SpaceBetween,
            cross_align: LayoutAlignment::Center,
            width_sizing: SizingConstraint::Fill,
            height_sizing: SizingConstraint::Fixed(56.0),
            text_role: "on-surface".to_owned(),
            stroke_role: Some("primary".to_owned()),
            stroke_width_px: 1.0,
            opacity: 1.0,
            elevation_level: 2,
            visible: true,
            locked: false,
            component_ref: Some("comp-omnibar".to_owned()),
            active_variant: Some("elevated".to_owned()),
            spring_token: "spatial_fast".to_owned(),
            flex_axes: GoogleSansFlexAxes::ui_control(),
        };

        let cta = FigCraftFrameNode {
            id: "node-primary-cta".to_owned(),
            stitch_id: "stitch-figcraft-cta".to_owned(),
            name: "Primary Expressive Pill".to_owned(),
            axis: AutoLayoutAxis::Horizontal,
            width_px: 232.0,
            height_px: 44.0,
            gap_px: 8.0,
            padding_px: 18.0,
            corner_radius_px: 9999.0,
            fill_role: "primary".to_owned(),
            text_content: Some("Sync Stitch & GPUI".to_owned()),
            bindings: Vec::new(),
            children: Vec::new(),
            kind: FigCraftNodeKind::Button,
            x_px: 32.0,
            y_px: 164.0,
            layout_mode: AutoLayoutMode::Flex,
            grid_columns: 1,
            padding_edges: PaddingEdges::symmetric(10.0, 18.0),
            main_align: LayoutAlignment::Center,
            cross_align: LayoutAlignment::Center,
            width_sizing: SizingConstraint::Fixed(232.0),
            height_sizing: SizingConstraint::Fixed(44.0),
            text_role: "on-primary".to_owned(),
            stroke_role: None,
            stroke_width_px: 0.0,
            opacity: 1.0,
            elevation_level: 1,
            visible: true,
            locked: false,
            component_ref: Some("comp-m3-button".to_owned()),
            active_variant: Some("filled".to_owned()),
            spring_token: "spatial_fast".to_owned(),
            flex_axes: GoogleSansFlexAxes::ui_control(),
        };

        let kpi_frame = FigCraftFrameNode {
            id: "node-kpi-grid".to_owned(),
            stitch_id: "stitch-figcraft-kpi-grid".to_owned(),
            name: "M3 Telemetry Grid".to_owned(),
            axis: AutoLayoutAxis::Horizontal,
            width_px: 640.0,
            height_px: 120.0,
            gap_px: 12.0,
            padding_px: 16.0,
            corner_radius_px: 16.0,
            fill_role: "surface-container-low".to_owned(),
            text_content: Some("144 FPS GPUI Fast · 4 M3 Modes · WCAG AA >= 4.5:1".to_owned()),
            bindings: Vec::new(),
            children: Vec::new(),
            kind: FigCraftNodeKind::KpiTile,
            x_px: 32.0,
            y_px: 228.0,
            layout_mode: AutoLayoutMode::Grid,
            grid_columns: 3,
            padding_edges: PaddingEdges::uniform(16.0),
            main_align: LayoutAlignment::SpaceBetween,
            cross_align: LayoutAlignment::Stretch,
            width_sizing: SizingConstraint::Fill,
            height_sizing: SizingConstraint::Hug,
            text_role: "on-surface".to_owned(),
            stroke_role: Some("outline-variant".to_owned()),
            stroke_width_px: 1.0,
            opacity: 1.0,
            elevation_level: 1,
            visible: true,
            locked: false,
            component_ref: None,
            active_variant: None,
            spring_token: "effects_default".to_owned(),
            flex_axes: GoogleSansFlexAxes::tabular_label(),
        };

        let hero_frame = FigCraftFrameNode {
            id: "node-root-screen".to_owned(),
            stitch_id: "stitch-figcraft-root".to_owned(),
            name: title.to_owned(),
            axis: AutoLayoutAxis::Vertical,
            width_px: 720.0,
            height_px: 440.0,
            gap_px: 20.0,
            padding_px: 28.0,
            corner_radius_px: 24.0,
            fill_role: "surface".to_owned(),
            text_content: Some("Where should we start?".to_owned()),
            bindings: Vec::new(),
            children: vec![omnibar, cta, kpi_frame],
            kind: FigCraftNodeKind::Screen,
            x_px: 48.0,
            y_px: 40.0,
            layout_mode: AutoLayoutMode::Flex,
            grid_columns: 1,
            padding_edges: PaddingEdges::uniform(28.0),
            main_align: LayoutAlignment::Start,
            cross_align: LayoutAlignment::Stretch,
            width_sizing: SizingConstraint::Fixed(720.0),
            height_sizing: SizingConstraint::Fixed(440.0),
            text_role: "on-surface".to_owned(),
            stroke_role: Some("outline-variant".to_owned()),
            stroke_width_px: 1.0,
            opacity: 1.0,
            elevation_level: 0,
            visible: true,
            locked: false,
            component_ref: None,
            active_variant: None,
            spring_token: "spatial_default".to_owned(),
            flex_axes: GoogleSansFlexAxes::display_hero(),
        };

        let kit_showcase_frame = FigCraftFrameNode {
            id: "node-kit-board".to_owned(),
            stitch_id: "stitch-figcraft-kit-board".to_owned(),
            name: "M3 Expressive Component Matrix".to_owned(),
            axis: AutoLayoutAxis::Vertical,
            width_px: 680.0,
            height_px: 360.0,
            gap_px: 16.0,
            padding_px: 24.0,
            corner_radius_px: 20.0,
            fill_role: "surface-container-low".to_owned(),
            text_content: Some("24 Expressive Primitives & Variant Matrix".to_owned()),
            bindings: Vec::new(),
            children: vec![
                node_from_template("node-kit-card", "m3-elevated-card", 0.0, 0.0),
                node_from_template("node-kit-chip", "m3-filter-chip", 0.0, 0.0),
                node_from_template("node-kit-switch", "m3-expressive-switch", 0.0, 0.0),
            ],
            kind: FigCraftNodeKind::Section,
            x_px: 60.0,
            y_px: 48.0,
            layout_mode: AutoLayoutMode::Flex,
            grid_columns: 2,
            padding_edges: PaddingEdges::uniform(24.0),
            main_align: LayoutAlignment::Start,
            cross_align: LayoutAlignment::Start,
            width_sizing: SizingConstraint::Fixed(680.0),
            height_sizing: SizingConstraint::Fixed(360.0),
            text_role: "on-surface".to_owned(),
            stroke_role: Some("outline-variant".to_owned()),
            stroke_width_px: 1.0,
            opacity: 1.0,
            elevation_level: 1,
            visible: true,
            locked: false,
            component_ref: None,
            active_variant: None,
            spring_token: "spatial_default".to_owned(),
            flex_axes: GoogleSansFlexAxes::section_headline(),
        };

        let mobile_frame = FigCraftFrameNode {
            id: "node-mobile-screen".to_owned(),
            stitch_id: "stitch-figcraft-mobile".to_owned(),
            name: "Compact Mobile Shell (390x640)".to_owned(),
            axis: AutoLayoutAxis::Vertical,
            width_px: 390.0,
            height_px: 520.0,
            gap_px: 16.0,
            padding_px: 20.0,
            corner_radius_px: 28.0,
            fill_role: "surface".to_owned(),
            text_content: Some("Aphrody Compact WindowClass".to_owned()),
            bindings: Vec::new(),
            children: vec![
                node_from_template("node-mobile-banner", "m3-gemini-spectrum-banner", 0.0, 0.0),
                node_from_template("node-mobile-nav", "m3-navigation-bar", 0.0, 0.0),
            ],
            kind: FigCraftNodeKind::Screen,
            x_px: 120.0,
            y_px: 48.0,
            layout_mode: AutoLayoutMode::Flex,
            grid_columns: 1,
            padding_edges: PaddingEdges::uniform(20.0),
            main_align: LayoutAlignment::SpaceBetween,
            cross_align: LayoutAlignment::Stretch,
            width_sizing: SizingConstraint::Fixed(390.0),
            height_sizing: SizingConstraint::Fixed(520.0),
            text_role: "on-surface".to_owned(),
            stroke_role: Some("outline".to_owned()),
            stroke_width_px: 1.0,
            opacity: 1.0,
            elevation_level: 1,
            visible: true,
            locked: false,
            component_ref: None,
            active_variant: None,
            spring_token: "spatial_fast".to_owned(),
            flex_axes: GoogleSansFlexAxes::compact_mobile(),
        };

        let pages = vec![
            FigCraftPage {
                id: "page-overview".to_owned(),
                name: "Overview Canvas".to_owned(),
                kind: PageKind::Canvas,
                root_frames: vec![hero_frame],
                viewport_x: 0.0,
                viewport_y: 0.0,
                zoom: 1.0,
            },
            FigCraftPage {
                id: "page-components".to_owned(),
                name: "M3 Kit & Variants".to_owned(),
                kind: PageKind::Components,
                root_frames: vec![kit_showcase_frame],
                viewport_x: 0.0,
                viewport_y: 0.0,
                zoom: 1.0,
            },
            FigCraftPage {
                id: "page-mobile".to_owned(),
                name: "Compact Mobile".to_owned(),
                kind: PageKind::ResponsiveMobile,
                root_frames: vec![mobile_frame],
                viewport_x: 0.0,
                viewport_y: 0.0,
                zoom: 1.0,
            },
        ];

        #[cfg(feature = "storytold-suite")]
        let magic = String::from_utf8_lossy(craft_fig::DESIGN_HEADER).into_owned();
        #[cfg(not(feature = "storytold-suite"))]
        let magic = "fig-kiwi".to_owned();

        let mut doc = Self {
            title: title.to_owned(),
            design_source: design_md.source_of_truth.clone(),
            fig_binary_header_magic: magic,
            root_frames: Vec::new(),
            selected_node_id: Some("node-omnibar".to_owned()),
            pages,
            active_page_id: "page-overview".to_owned(),
            active_mode: mode,
            token_palette,
            component_definitions,
            history: UndoRedoHistory::default(),
            next_id_seq: 100,
        };
        doc.rebind_all_tokens();
        doc.sync_root_frames_from_active_page();
        doc
    }

    #[must_use]
    pub fn total_node_count(&self) -> usize {
        fn count_rec(node: &FigCraftFrameNode) -> usize {
            1 + node.children.iter().map(count_rec).sum::<usize>()
        }
        if self.pages.is_empty() {
            return self.root_frames.iter().map(count_rec).sum();
        }
        self.pages
            .iter()
            .flat_map(|page| page.root_frames.iter())
            .map(count_rec)
            .sum()
    }

    #[must_use]
    pub fn active_page(&self) -> Option<&FigCraftPage> {
        self.pages
            .iter()
            .find(|p| p.id == self.active_page_id)
            .or_else(|| self.pages.first())
    }

    pub fn active_page_mut(&mut self) -> Option<&mut FigCraftPage> {
        let target_id = self.active_page_id.clone();
        if let Some(pos) = self.pages.iter().position(|p| p.id == target_id) {
            self.pages.get_mut(pos)
        } else {
            self.pages.first_mut()
        }
    }

    fn sync_root_frames_from_active_page(&mut self) {
        if let Some(page) = self.active_page() {
            self.root_frames = page.root_frames.clone();
        }
    }

    fn snapshot(&self) -> FigCraftSnapshot {
        FigCraftSnapshot {
            pages: self.pages.clone(),
            active_page_id: self.active_page_id.clone(),
            selected_node_id: self.selected_node_id.clone(),
            active_mode: self.active_mode,
        }
    }

    fn restore_snapshot(&mut self, snap: FigCraftSnapshot) {
        self.pages = snap.pages;
        self.active_page_id = snap.active_page_id;
        self.selected_node_id = snap.selected_node_id;
        self.active_mode = snap.active_mode;
        self.refresh_palette_for_active_mode();
        self.rebind_all_tokens();
        self.sync_root_frames_from_active_page();
    }

    fn commit_mutation<R>(&mut self, label: impl Into<String>, mutate: impl FnOnce(&mut Self) -> Option<R>) -> Option<R> {
        let before = self.snapshot();
        let result = mutate(self)?;
        self.rebind_all_tokens();
        self.sync_root_frames_from_active_page();
        let after = self.snapshot();
        self.history.record(FigCraftCommand {
            label: label.into(),
            before,
            after,
        });
        Some(result)
    }

    pub fn undo(&mut self) -> bool {
        let Some(cmd) = self.history.undo_stack.pop() else {
            return false;
        };
        self.restore_snapshot(cmd.before.clone());
        self.history.redo_stack.push(cmd);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(cmd) = self.history.redo_stack.pop() else {
            return false;
        };
        self.restore_snapshot(cmd.after.clone());
        self.history.undo_stack.push(cmd);
        true
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn set_design_mode(&mut self, mode: FigCraftDesignMode) {
        if self.active_mode == mode {
            return;
        }
        let label = format!("Switch mode to {}", mode.label());
        if self
            .commit_mutation(label, |doc| {
                doc.active_mode = mode;
                doc.refresh_palette_for_active_mode();
                Some(())
            })
            .is_none()
        {
            log::debug!("Mode switch to {} produced no snapshot", mode.label());
        }
    }

    fn refresh_palette_for_active_mode(&mut self) {
        if let Ok(tokens) = GoogleDesignTokens::bundled() {
            let map = tokens.palette_by_name(self.active_mode.palette_key());
            let mut next = BTreeMap::new();
            for (k, v) in map {
                next.insert(k.clone(), v.clone());
                next.insert(k.replace('_', "-"), v.clone());
            }
            if !next.is_empty() {
                self.token_palette = next;
                return;
            }
        }
        if let Ok(design_md) = load_bundled_root_design_md() {
            self.token_palette = build_palette_for_mode(self.active_mode, &design_md);
        }
    }

    /// Resolves any Material 3 role (`surface-container` or `surface_container`) under the active mode.
    #[must_use]
    pub fn resolve_role_hex(&self, role: &str) -> String {
        let normalized_hyphen = role.replace('_', "-");
        let normalized_under = role.replace('-', "_");
        if let Some(hex) = self
            .token_palette
            .get(role)
            .or_else(|| self.token_palette.get(&normalized_hyphen))
            .or_else(|| self.token_palette.get(&normalized_under))
        {
            return hex.clone();
        }
        fallback_hex_for_mode(self.active_mode, &normalized_hyphen).to_owned()
    }

    #[must_use]
    pub fn contrast_ratio_for_roles(&self, fg_role: &str, bg_role: &str) -> Option<f32> {
        let fg = self.resolve_role_hex(fg_role);
        let bg = self.resolve_role_hex(bg_role);
        contrast_ratio_hex(&fg, &bg)
    }

    pub fn rebind_all_tokens(&mut self) {
        let palette = self.token_palette.clone();
        let mode = self.active_mode;
        fn rebind_node(node: &mut FigCraftFrameNode, palette: &BTreeMap<String, String>, mode: FigCraftDesignMode) {
            let resolve = |role: &str| -> String {
                let hyphen = role.replace('_', "-");
                let under = role.replace('-', "_");
                palette
                    .get(role)
                    .or_else(|| palette.get(&hyphen))
                    .or_else(|| palette.get(&under))
                    .cloned()
                    .unwrap_or_else(|| fallback_hex_for_mode(mode, &hyphen).to_owned())
            };

            let mut bindings = vec![
                FigTokenBinding {
                    property: "fill".to_owned(),
                    design_md_role: format!("colors.{}", node.fill_role),
                    resolved_value: resolve(&node.fill_role),
                },
                FigTokenBinding {
                    property: "text".to_owned(),
                    design_md_role: format!("colors.{}", node.text_role),
                    resolved_value: resolve(&node.text_role),
                },
            ];
            if let Some(ref stroke_role) = node.stroke_role {
                bindings.push(FigTokenBinding {
                    property: "stroke".to_owned(),
                    design_md_role: format!("colors.{stroke_role}"),
                    resolved_value: resolve(stroke_role),
                });
            }
            node.bindings = bindings;
            for child in &mut node.children {
                rebind_node(child, palette, mode);
            }
        }
        for page in &mut self.pages {
            for frame in &mut page.root_frames {
                rebind_node(frame, &palette, mode);
            }
        }
        for frame in &mut self.root_frames {
            rebind_node(frame, &palette, mode);
        }
    }

    pub fn add_page(&mut self, name: impl Into<String>, kind: PageKind) -> String {
        let page_name = name.into();
        let next_seq = self.allocate_id_seq();
        let page_id = format!("page-{next_seq}");
        let frame_id = format!("node-frame-{next_seq}");
        let stitch_id = format!("stitch-page-{next_seq}");
        let new_page_id = page_id.clone();

        if self
            .commit_mutation(format!("Add page '{page_name}'"), |doc| {
                let root = FigCraftFrameNode {
                    id: frame_id.clone(),
                    stitch_id,
                    name: page_name.clone(),
                    axis: AutoLayoutAxis::Vertical,
                    width_px: if kind == PageKind::ResponsiveMobile { 390.0 } else { 720.0 },
                    height_px: 460.0,
                    gap_px: 16.0,
                    padding_px: 24.0,
                    corner_radius_px: 24.0,
                    fill_role: "surface".to_owned(),
                    text_content: Some(page_name.clone()),
                    bindings: Vec::new(),
                    children: Vec::new(),
                    kind: FigCraftNodeKind::Screen,
                    x_px: 48.0,
                    y_px: 40.0,
                    layout_mode: AutoLayoutMode::Flex,
                    grid_columns: 1,
                    padding_edges: PaddingEdges::uniform(24.0),
                    main_align: LayoutAlignment::Start,
                    cross_align: LayoutAlignment::Stretch,
                    width_sizing: SizingConstraint::Fixed(if kind == PageKind::ResponsiveMobile {
                        390.0
                    } else {
                        720.0
                    }),
                    height_sizing: SizingConstraint::Fixed(460.0),
                    text_role: "on-surface".to_owned(),
                    stroke_role: Some("outline-variant".to_owned()),
                    stroke_width_px: 1.0,
                    opacity: 1.0,
                    elevation_level: 0,
                    visible: true,
                    locked: false,
                    component_ref: None,
                    active_variant: None,
                    spring_token: "spatial_default".to_owned(),
                    flex_axes: GoogleSansFlexAxes::section_headline(),
                };
                doc.pages.push(FigCraftPage {
                    id: page_id.clone(),
                    name: page_name,
                    kind,
                    root_frames: vec![root],
                    viewport_x: 0.0,
                    viewport_y: 0.0,
                    zoom: 1.0,
                });
                doc.active_page_id = page_id;
                doc.selected_node_id = Some(frame_id);
                Some(())
            })
            .is_none()
        {
            log::debug!("Page creation returned empty snapshot");
        }

        new_page_id
    }

    pub fn select_page(&mut self, page_id: &str) -> bool {
        if self.active_page_id == page_id || !self.pages.iter().any(|p| p.id == page_id) {
            return false;
        }
        let target_id = page_id.to_owned();
        self.commit_mutation(format!("Switch to page {target_id}"), |doc| {
            doc.active_page_id = target_id.clone();
            let first_node = doc
                .pages
                .iter()
                .find(|p| p.id == target_id)
                .and_then(|p| p.root_frames.first())
                .map(|n| n.id.clone());
            doc.selected_node_id = first_node;
            Some(())
        })
        .is_some()
    }

    pub fn rename_page(&mut self, page_id: &str, new_name: impl Into<String>) -> bool {
        let new_name = new_name.into();
        if new_name.trim().is_empty() {
            return false;
        }
        let target_id = page_id.to_owned();
        self.commit_mutation(format!("Rename page to '{new_name}'"), |doc| {
            let page = doc.pages.iter_mut().find(|p| p.id == target_id)?;
            page.name = new_name;
            Some(())
        })
        .is_some()
    }

    pub fn select_node(&mut self, node_id: Option<String>) {
        self.selected_node_id = node_id;
    }

    #[must_use]
    pub fn find_node(&self, node_id: &str) -> Option<&FigCraftFrameNode> {
        fn find_rec<'a>(nodes: &'a [FigCraftFrameNode], id: &str) -> Option<&'a FigCraftFrameNode> {
            for node in nodes {
                if node.id == id {
                    return Some(node);
                }
                if let Some(found) = find_rec(&node.children, id) {
                    return Some(found);
                }
            }
            None
        }
        if let Some(page) = self.active_page()
            && let Some(found) = find_rec(&page.root_frames, node_id)
        {
            return Some(found);
        }
        for page in &self.pages {
            if let Some(found) = find_rec(&page.root_frames, node_id) {
                return Some(found);
            }
        }
        find_rec(&self.root_frames, node_id)
    }

    fn find_node_mut_in_active(&mut self, node_id: &str) -> Option<&mut FigCraftFrameNode> {
        fn find_mut<'a>(nodes: &'a mut [FigCraftFrameNode], id: &str) -> Option<&'a mut FigCraftFrameNode> {
            for node in nodes {
                if node.id == id {
                    return Some(node);
                }
                if let Some(found) = find_mut(&mut node.children, id) {
                    return Some(found);
                }
            }
            None
        }
        let page = self.active_page_mut()?;
        find_mut(&mut page.root_frames, node_id)
    }

    #[must_use]
    pub fn selected_node(&self) -> Option<&FigCraftFrameNode> {
        let id = self.selected_node_id.as_deref()?;
        self.find_node(id)
    }

    fn allocate_id_seq(&mut self) -> u64 {
        self.next_id_seq = self.next_id_seq.saturating_add(1);
        self.next_id_seq
    }

    /// Inserts one of the 24 M3 Expressive templates into `parent_id` (or the active page's primary screen).
    pub fn insert_template_into(&mut self, parent_id: Option<&str>, template_id: &str) -> Option<String> {
        let template = STARTER_WIDGET_TEMPLATES
            .iter()
            .find(|t| t.id == template_id)?;
        let seq = self.allocate_id_seq();
        let new_id = format!("node-{}-{}", template.id, seq);
        let target_parent = parent_id.map(str::to_owned);
        let new_id_ret = new_id.clone();

        self.commit_mutation(format!("Insert {}", template.name), |doc| {
            let page = doc.active_page_mut()?;
            let mut node = node_from_template(&new_id, template.id, 24.0, 24.0);
            node.stitch_id = format!("stitch-{}-{seq}", template.id);

            let inserted = if let Some(ref pid) = target_parent {
                insert_child_by_id(&mut page.root_frames, pid, node.clone())
            } else {
                false
            };
            if !inserted {
                if let Some(first_screen) = page.root_frames.first_mut() {
                    let y_offset = 40.0 + (first_screen.children.len() as f32) * 56.0;
                    node.x_px = 28.0;
                    node.y_px = y_offset;
                    first_screen.children.push(node);
                } else {
                    node.x_px = 48.0;
                    node.y_px = 48.0;
                    page.root_frames.push(node);
                }
            }
            doc.selected_node_id = Some(new_id);
            Some(new_id_ret)
        })
    }

    /// Moves a top-level or nested node by `(dx, dy)`, optionally snapping to grid or sibling edges.
    pub fn move_node_by(&mut self, node_id: &str, dx: f32, dy: f32, snap_grid_px: Option<f32>) -> bool {
        let Some(current) = self.find_node(node_id) else {
            return false;
        };
        if current.locked {
            return false;
        }
        let mut next_x = current.x_px + dx;
        let mut next_y = current.y_px + dy;
        if let Some(step) = snap_grid_px
            && step > 0.0
        {
            next_x = (next_x / step).round() * step;
            next_y = (next_y / step).round() * step;
        }
        self.set_node_position(node_id, next_x, next_y)
    }

    pub fn set_node_position(&mut self, node_id: &str, x_px: f32, y_px: f32) -> bool {
        let target = node_id.to_owned();
        self.commit_mutation(format!("Move {target}"), |doc| {
            let node = doc.find_node_mut_in_active(&target)?;
            if node.locked {
                return None;
            }
            node.x_px = x_px;
            node.y_px = y_px;
            Some(())
        })
        .is_some()
    }

    pub fn update_node_layout(
        &mut self,
        node_id: &str,
        axis: AutoLayoutAxis,
        layout_mode: AutoLayoutMode,
        gap_px: f32,
        padding: PaddingEdges,
        grid_columns: u16,
        main_align: LayoutAlignment,
        cross_align: LayoutAlignment,
    ) -> bool {
        let target = node_id.to_owned();
        self.commit_mutation(format!("Update layout on {target}"), |doc| {
            let node = doc.find_node_mut_in_active(&target)?;
            node.axis = axis;
            node.layout_mode = layout_mode;
            node.gap_px = gap_px.clamp(0.0, 128.0);
            node.padding_edges = padding;
            node.padding_px = padding.left_px;
            node.grid_columns = grid_columns.clamp(1, 12);
            node.main_align = main_align;
            node.cross_align = cross_align;
            Some(())
        })
        .is_some()
    }

    pub fn update_node_sizing(
        &mut self,
        node_id: &str,
        width_px: f32,
        height_px: f32,
        width_sizing: SizingConstraint,
        height_sizing: SizingConstraint,
        corner_radius_px: f32,
    ) -> bool {
        let target = node_id.to_owned();
        self.commit_mutation(format!("Update sizing on {target}"), |doc| {
            let node = doc.find_node_mut_in_active(&target)?;
            node.width_px = width_px.clamp(16.0, 4096.0);
            node.height_px = height_px.clamp(16.0, 4096.0);
            node.width_sizing = width_sizing;
            node.height_sizing = height_sizing;
            node.corner_radius_px = corner_radius_px.clamp(0.0, 9999.0);
            Some(())
        })
        .is_some()
    }

    pub fn update_node_tokens(
        &mut self,
        node_id: &str,
        fill_role: &str,
        text_role: &str,
        spring_token: &str,
        elevation_level: u8,
    ) -> bool {
        let target = node_id.to_owned();
        let fill = fill_role.to_owned();
        let text = text_role.to_owned();
        let spring = spring_token.to_owned();
        self.commit_mutation(format!("Bind tokens on {target}"), |doc| {
            let node = doc.find_node_mut_in_active(&target)?;
            node.fill_role = fill;
            node.text_role = text;
            node.spring_token = spring;
            node.elevation_level = elevation_level.min(5);
            Some(())
        })
        .is_some()
    }

    pub fn update_node_flex_axes(&mut self, node_id: &str, axes: GoogleSansFlexAxes) -> bool {
        let target = node_id.to_owned();
        self.commit_mutation(format!("Update typography axes on {target}"), |doc| {
            let node = doc.find_node_mut_in_active(&target)?;
            node.flex_axes = axes;
            Some(())
        })
        .is_some()
    }

    pub fn update_node_text(
        &mut self,
        node_id: &str,
        name: Option<String>,
        text_content: Option<String>,
    ) -> bool {
        let target = node_id.to_owned();
        self.commit_mutation(format!("Edit text on {target}"), |doc| {
            let node = doc.find_node_mut_in_active(&target)?;
            if let Some(next_name) = name {
                node.name = next_name;
            }
            if let Some(next_text) = text_content {
                node.text_content = if next_text.is_empty() { None } else { Some(next_text) };
            }
            Some(())
        })
        .is_some()
    }

    pub fn toggle_node_visibility(&mut self, node_id: &str) -> bool {
        let target = node_id.to_owned();
        self.commit_mutation(format!("Toggle visibility {target}"), |doc| {
            let node = doc.find_node_mut_in_active(&target)?;
            node.visible = !node.visible;
            Some(())
        })
        .is_some()
    }

    pub fn toggle_node_locked(&mut self, node_id: &str) -> bool {
        let target = node_id.to_owned();
        self.commit_mutation(format!("Toggle lock {target}"), |doc| {
            let node = doc.find_node_mut_in_active(&target)?;
            node.locked = !node.locked;
            Some(())
        })
        .is_some()
    }

    pub fn duplicate_node(&mut self, node_id: &str) -> Option<String> {
        let original = self.find_node(node_id)?.clone();
        let seq = self.allocate_id_seq();
        let mut cloned = original;
        let new_id = format!("{}-copy-{seq}", cloned.id);
        cloned.id = new_id.clone();
        cloned.stitch_id = format!("{}-copy-{seq}", cloned.stitch_id);
        cloned.name = format!("{} Copy", cloned.name);
        cloned.x_px += 24.0;
        cloned.y_px += 24.0;
        re_id_children(&mut cloned.children, seq);

        let target = node_id.to_owned();
        self.commit_mutation(format!("Duplicate {target}"), |doc| {
            let page = doc.active_page_mut()?;
            if duplicate_sibling_rec(&mut page.root_frames, &target, cloned) {
                doc.selected_node_id = Some(new_id.clone());
                Some(new_id)
            } else {
                None
            }
        })
    }

    pub fn delete_node(&mut self, node_id: &str) -> bool {
        let target = node_id.to_owned();
        self.commit_mutation(format!("Delete {target}"), |doc| {
            let page = doc.active_page_mut()?;
            // Preserve at least one root screen frame per page.
            if page.root_frames.len() <= 1 && page.root_frames.first().is_some_and(|r| r.id == target) {
                return None;
            }
            if remove_node_rec(&mut page.root_frames, &target) {
                let fallback_id = page.root_frames.first().map(|r| r.id.clone());
                if doc.selected_node_id.as_deref() == Some(target.as_str()) {
                    doc.selected_node_id = fallback_id;
                }
                Some(())
            } else {
                None
            }
        })
        .is_some()
    }

    pub fn apply_component_variant(
        &mut self,
        node_id: &str,
        component_id: &str,
        variant_name: &str,
    ) -> bool {
        let Some(comp) = self
            .component_definitions
            .iter()
            .find(|c| c.id == component_id)
            .cloned()
        else {
            return false;
        };
        let Some(variant) = comp.variants.iter().find(|v| v.name == variant_name).cloned() else {
            return false;
        };
        let target = node_id.to_owned();
        let comp_id = component_id.to_owned();
        let var_name = variant_name.to_owned();
        self.commit_mutation(format!("Apply variant {comp_id}:{var_name}"), |doc| {
            let node = doc.find_node_mut_in_active(&target)?;
            node.component_ref = Some(comp_id);
            node.active_variant = Some(var_name);
            node.fill_role = variant.fill_role;
            node.text_role = variant.text_role;
            node.stroke_role = variant.stroke_role;
            node.corner_radius_px = variant.corner_radius_px;
            node.spring_token = variant.spring_token;
            node.elevation_level = variant.elevation_level;
            Some(())
        })
        .is_some()
    }

    /// Computes vertical & horizontal smart snap guides against sibling & frame edges/centers.
    #[must_use]
    pub fn compute_snap_guides(
        &self,
        moving_node_id: &str,
        proposed_x: f32,
        proposed_y: f32,
        threshold_px: f32,
    ) -> SnapGuideResult {
        let Some(moving) = self.find_node(moving_node_id) else {
            return SnapGuideResult {
                snapped_x_px: proposed_x,
                snapped_y_px: proposed_y,
                guides: Vec::new(),
            };
        };
        let Some(page) = self.active_page() else {
            return SnapGuideResult {
                snapped_x_px: proposed_x,
                snapped_y_px: proposed_y,
                guides: Vec::new(),
            };
        };

        let mut candidates = Vec::new();
        collect_nodes_flat(&page.root_frames, moving_node_id, &mut candidates);

        let mut snapped_x = proposed_x;
        let mut snapped_y = proposed_y;
        let mut best_dx = threshold_px.max(0.5);
        let mut best_dy = threshold_px.max(0.5);
        let mut x_guide: Option<SnapGuideLine> = None;
        let mut y_guide: Option<SnapGuideLine> = None;

        let mw = moving.width_px;
        let mh = moving.height_px;

        for other in candidates {
            let ox_anchors = [other.x_px, other.x_px + other.width_px * 0.5, other.x_px + other.width_px];
            let mx_offsets = [0.0, mw * 0.5, mw];
            for &oa in &ox_anchors {
                for &mo in &mx_offsets {
                    let diff = (proposed_x + mo - oa).abs();
                    if diff < best_dx {
                        best_dx = diff;
                        snapped_x = oa - mo;
                        x_guide = Some(SnapGuideLine {
                            axis: SnapAxis::VerticalX,
                            position_px: oa,
                            start_px: proposed_y.min(other.y_px),
                            end_px: (proposed_y + mh).max(other.y_px + other.height_px),
                            target_node_id: other.id.clone(),
                        });
                    }
                }
            }

            let oy_anchors = [other.y_px, other.y_px + other.height_px * 0.5, other.y_px + other.height_px];
            let my_offsets = [0.0, mh * 0.5, mh];
            for &oa in &oy_anchors {
                for &mo in &my_offsets {
                    let diff = (proposed_y + mo - oa).abs();
                    if diff < best_dy {
                        best_dy = diff;
                        snapped_y = oa - mo;
                        y_guide = Some(SnapGuideLine {
                            axis: SnapAxis::HorizontalY,
                            position_px: oa,
                            start_px: proposed_x.min(other.x_px),
                            end_px: (proposed_x + mw).max(other.x_px + other.width_px),
                            target_node_id: other.id.clone(),
                        });
                    }
                }
            }
        }

        let mut guides = Vec::new();
        if let Some(g) = x_guide {
            guides.push(g);
        }
        if let Some(g) = y_guide {
            guides.push(g);
        }

        SnapGuideResult {
            snapped_x_px: snapped_x,
            snapped_y_px: snapped_y,
            guides,
        }
    }

    /// Resolves the M3 Expressive spring physics token (`spatial_*` or `effects_*`) attached to a node.
    #[must_use]
    pub fn resolve_spring(&self, spring_name: &str) -> SpringSpec {
        if let Ok(tokens) = GoogleDesignTokens::bundled()
            && let Some(spec) = tokens.spring(spring_name)
        {
            return spec;
        }
        match spring_name {
            "spatial_fast" => SpringSpec {
                stiffness: 1400.0,
                damping_ratio: 0.9,
            },
            "spatial_slow" => SpringSpec {
                stiffness: 300.0,
                damping_ratio: 0.9,
            },
            "effects_fast" => SpringSpec {
                stiffness: 3800.0,
                damping_ratio: 1.0,
            },
            "effects_default" => SpringSpec {
                stiffness: 1600.0,
                damping_ratio: 1.0,
            },
            "effects_slow" => SpringSpec {
                stiffness: 800.0,
                damping_ratio: 1.0,
            },
            _ => SpringSpec {
                stiffness: 700.0,
                damping_ratio: 0.9,
            },
        }
    }

    #[must_use]
    pub fn export_for_target(
        &self,
        target: FigCraftExportTarget,
        design_md: &DesignMdDocument,
    ) -> String {
        match target {
            FigCraftExportTarget::GpuiKitRs => export_to_gpui_kit_rs(self),
            FigCraftExportTarget::NshRsTsx => export_to_nsh_rs_tsx(self),
            FigCraftExportTarget::GpuiRs => export_to_gpui_rs(self),
            FigCraftExportTarget::Lighthouse100Html => export_to_lighthouse100_html(self, design_md),
        }
    }

    /// Inspects raw `.fig`, `.sketch`, or `.penpot` archive bytes via `craft-fig`, `craft-sketch`, and `craft-penpot`.
    #[must_use]
    pub fn inspect_external_design_bytes(bytes: &[u8]) -> String {
        #[cfg(feature = "storytold-suite")]
        {
            if craft_fig::is_fig(bytes) {
                return match craft_fig::Archive::open_with(bytes, craft_fig::Limits::default()) {
                    Ok(arch) => format!(
                        "FigCraft .fig archive OK (canvas={} B, file_name={})",
                        arch.canvas.len(),
                        arch.file_name.as_deref().unwrap_or("<unnamed>")
                    ),
                    Err(err) => format!("FigCraft .fig error: {err}"),
                };
            }
            if let Ok(sketch) =
                craft_sketch::Archive::open_with(bytes, craft_sketch::Limits::default())
            {
                return format!(
                    "Sketch v48+ archive OK (pages={}, warnings={})",
                    sketch.pages.len(),
                    sketch.warnings.len()
                );
            }
            if let Ok(penpot) = craft_penpot::Archive::open(bytes) {
                return format!(
                    "Penpot 2.x archive OK (files={})",
                    penpot.files.len()
                );
            }
            format!(
                "Raw design buffer ({} bytes, magic header expected `{}`)",
                bytes.len(),
                String::from_utf8_lossy(craft_fig::DESIGN_HEADER)
            )
        }
        #[cfg(not(feature = "storytold-suite"))]
        {
            format!(
                "Raw design buffer ({} bytes, enable `storytold-suite` for .fig/.sketch/.penpot decoding)",
                bytes.len()
            )
        }
    }
}

fn collect_nodes_flat<'a>(
    nodes: &'a [FigCraftFrameNode],
    exclude_id: &str,
    out: &mut Vec<&'a FigCraftFrameNode>,
) {
    for node in nodes {
        if node.id != exclude_id && node.visible {
            out.push(node);
        }
        collect_nodes_flat(&node.children, exclude_id, out);
    }
}

fn insert_child_by_id(
    nodes: &mut [FigCraftFrameNode],
    parent_id: &str,
    child: FigCraftFrameNode,
) -> bool {
    for node in nodes {
        if node.id == parent_id {
            node.children.push(child);
            return true;
        }
        if insert_child_by_id(&mut node.children, parent_id, child.clone()) {
            return true;
        }
    }
    false
}

fn duplicate_sibling_rec(
    nodes: &mut Vec<FigCraftFrameNode>,
    target_id: &str,
    cloned: FigCraftFrameNode,
) -> bool {
    if let Some(pos) = nodes.iter().position(|n| n.id == target_id) {
        nodes.insert(pos + 1, cloned);
        return true;
    }
    for node in nodes {
        if duplicate_sibling_rec(&mut node.children, target_id, cloned.clone()) {
            return true;
        }
    }
    false
}

fn remove_node_rec(nodes: &mut Vec<FigCraftFrameNode>, target_id: &str) -> bool {
    if let Some(pos) = nodes.iter().position(|n| n.id == target_id) {
        nodes.remove(pos);
        return true;
    }
    for node in nodes {
        if remove_node_rec(&mut node.children, target_id) {
            return true;
        }
    }
    false
}

fn re_id_children(children: &mut [FigCraftFrameNode], seq: u64) {
    for (idx, child) in children.iter_mut().enumerate() {
        child.id = format!("{}-c{seq}-{idx}", child.id);
        child.stitch_id = format!("{}-c{seq}-{idx}", child.stitch_id);
        re_id_children(&mut child.children, seq);
    }
}

#[must_use]
fn node_from_template(id: &str, template_id: &str, x_px: f32, y_px: f32) -> FigCraftFrameNode {
    let tpl = STARTER_WIDGET_TEMPLATES
        .iter()
        .find(|t| t.id == template_id)
        .unwrap_or(&STARTER_WIDGET_TEMPLATES[0]);
    FigCraftFrameNode {
        id: id.to_owned(),
        stitch_id: format!("stitch-{}", tpl.id),
        name: tpl.name.to_owned(),
        axis: tpl.axis,
        width_px: tpl.default_width_px,
        height_px: tpl.default_height_px,
        gap_px: tpl.gap_px,
        padding_px: tpl.padding_px,
        corner_radius_px: tpl.corner_radius_px,
        fill_role: tpl.fill_role.to_owned(),
        text_content: Some(tpl.sample_text.to_owned()),
        bindings: Vec::new(),
        children: Vec::new(),
        kind: tpl.kind,
        x_px,
        y_px,
        layout_mode: tpl.layout_mode,
        grid_columns: if tpl.layout_mode == AutoLayoutMode::Grid { 2 } else { 1 },
        padding_edges: PaddingEdges::uniform(tpl.padding_px),
        main_align: LayoutAlignment::Center,
        cross_align: LayoutAlignment::Center,
        width_sizing: SizingConstraint::Fixed(tpl.default_width_px),
        height_sizing: SizingConstraint::Fixed(tpl.default_height_px),
        text_role: tpl.text_role.to_owned(),
        stroke_role: tpl.stroke_role.map(str::to_owned),
        stroke_width_px: if tpl.stroke_role.is_some() { 1.0 } else { 0.0 },
        opacity: 1.0,
        elevation_level: tpl.elevation_level,
        visible: true,
        locked: false,
        component_ref: None,
        active_variant: None,
        spring_token: tpl.spring_token.to_owned(),
        flex_axes: GoogleSansFlexAxes::ui_control(),
    }
}

fn default_component_definitions() -> Vec<ComponentVariantDefinition> {
    vec![
        ComponentVariantDefinition {
            id: "comp-m3-button".to_owned(),
            name: "M3 Expressive Button".to_owned(),
            category: "Actions".to_owned(),
            default_variant: "filled".to_owned(),
            variants: vec![
                ComponentVariantSpec {
                    name: "filled".to_owned(),
                    fill_role: "primary".to_owned(),
                    text_role: "on-primary".to_owned(),
                    stroke_role: None,
                    corner_radius_px: 9999.0,
                    spring_token: "spatial_fast".to_owned(),
                    elevation_level: 1,
                },
                ComponentVariantSpec {
                    name: "tonal".to_owned(),
                    fill_role: "secondary-container".to_owned(),
                    text_role: "on-secondary-container".to_owned(),
                    stroke_role: None,
                    corner_radius_px: 9999.0,
                    spring_token: "spatial_fast".to_owned(),
                    elevation_level: 0,
                },
                ComponentVariantSpec {
                    name: "outlined".to_owned(),
                    fill_role: "surface".to_owned(),
                    text_role: "primary".to_owned(),
                    stroke_role: Some("outline".to_owned()),
                    corner_radius_px: 9999.0,
                    spring_token: "effects_fast".to_owned(),
                    elevation_level: 0,
                },
            ],
        },
        ComponentVariantDefinition {
            id: "comp-omnibar".to_owned(),
            name: "Neural Expressive Omnibar".to_owned(),
            category: "Expressive".to_owned(),
            default_variant: "elevated".to_owned(),
            variants: vec![
                ComponentVariantSpec {
                    name: "elevated".to_owned(),
                    fill_role: "surface-container-high".to_owned(),
                    text_role: "on-surface".to_owned(),
                    stroke_role: Some("primary".to_owned()),
                    corner_radius_px: 9999.0,
                    spring_token: "spatial_fast".to_owned(),
                    elevation_level: 2,
                },
                ComponentVariantSpec {
                    name: "tonal".to_owned(),
                    fill_role: "surface-container".to_owned(),
                    text_role: "on-surface".to_owned(),
                    stroke_role: Some("outline-variant".to_owned()),
                    corner_radius_px: 9999.0,
                    spring_token: "effects_default".to_owned(),
                    elevation_level: 0,
                },
            ],
        },
    ]
}

fn build_palette_for_mode(mode: FigCraftDesignMode, design_md: &DesignMdDocument) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if let Ok(tokens) = GoogleDesignTokens::bundled() {
        for (k, v) in tokens.palette_by_name(mode.palette_key()) {
            map.insert(k.clone(), v.clone());
            map.insert(k.replace('_', "-"), v.clone());
        }
    }
    if mode == FigCraftDesignMode::Dark {
        for (k, v) in &design_md.colors {
            map.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    map
}

fn fallback_hex_for_mode(mode: FigCraftDesignMode, role: &str) -> &'static str {
    match mode {
        FigCraftDesignMode::Light => match role {
            "surface" | "background" => "#F7F9FF",
            "surface-container-lowest" => "#FFFFFF",
            "surface-container-low" => "#EEF3FA",
            "surface-container" => "#E7EDF6",
            "surface-container-high" => "#E0E7F1",
            "surface-container-highest" => "#D9E1EC",
            "on-surface" | "on-background" => "#161C24",
            "on-surface-variant" => "#3D4754",
            "primary" => "#006399",
            "on-primary" => "#FFFFFF",
            "primary-container" => "#CEE5FF",
            "on-primary-container" => "#004972",
            "secondary" => "#4B6078",
            "secondary-container" => "#D2E4FF",
            "on-secondary-container" => "#34485F",
            "tertiary-container" => "#E2DFFF",
            "on-tertiary-container" => "#434077",
            "outline" => "#6E7886",
            "outline-variant" => "#BEC7D4",
            "error" => "#BA1A1A",
            "on-error" => "#FFFFFF",
            _ => "#E7EDF6",
        },
        FigCraftDesignMode::NeuralObsidian => match role {
            "surface" | "background" => "#0B0E14",
            "surface-container-lowest" => "#06080C",
            "surface-container-low" => "#13161F",
            "surface-container" => "#181C26",
            "surface-container-high" => "#1C202B",
            "surface-container-highest" => "#262C3A",
            "on-surface" | "on-background" => "#E4EAF4",
            "on-surface-variant" => "#BAC4D6",
            "primary" => "#8FD0FF",
            "on-primary" => "#003352",
            "primary-container" => "#004B75",
            "on-primary-container" => "#CEE5FF",
            "secondary" => "#B4C9E5",
            "secondary-container" => "#34485F",
            "on-secondary-container" => "#D2E4FF",
            "tertiary-container" => "#434077",
            "on-tertiary-container" => "#E2DFFF",
            "outline" => "#8792A6",
            "outline-variant" => "#384152",
            "error" => "#FFB4AB",
            "on-error" => "#690005",
            _ => "#181C26",
        },
        FigCraftDesignMode::OledPitch => match role {
            "surface" | "background" => "#000000",
            "surface-container-lowest" => "#000000",
            "surface-container-low" => "#0A0E14",
            "surface-container" => "#121720",
            "surface-container-high" => "#1B222E",
            "surface-container-highest" => "#252E3D",
            "on-surface" | "on-background" => "#EDF2FA",
            "on-surface-variant" => "#C2CCDC",
            "primary" => "#8FD0FF",
            "on-primary" => "#003352",
            "primary-container" => "#004B75",
            "on-primary-container" => "#CEE5FF",
            "secondary" => "#B4C9E5",
            "secondary-container" => "#34485F",
            "on-secondary-container" => "#D2E4FF",
            "tertiary-container" => "#434077",
            "on-tertiary-container" => "#E2DFFF",
            "outline" => "#8C96A8",
            "outline-variant" => "#2B3545",
            "error" => "#FFB4AB",
            "on-error" => "#690005",
            _ => "#121720",
        },
        FigCraftDesignMode::Dark => match role {
            "surface" | "background" => "#0E141B",
            "surface-container-lowest" => "#080D13",
            "surface-container-low" => "#161D25",
            "surface-container" => "#1B222B",
            "surface-container-high" => "#252C36",
            "surface-container-highest" => "#303843",
            "on-surface" | "on-background" => "#E0E6F0",
            "on-surface-variant" => "#C0CAD7",
            "primary" => "#8FD0FF",
            "on-primary" => "#003352",
            "primary-container" => "#004B75",
            "on-primary-container" => "#CEE5FF",
            "secondary" => "#B4C9E5",
            "secondary-container" => "#34485F",
            "on-secondary-container" => "#D2E4FF",
            "tertiary-container" => "#434077",
            "on-tertiary-container" => "#E2DFFF",
            "outline" => "#8A94A2",
            "outline-variant" => "#404956",
            "error" => "#FFB4AB",
            "on-error" => "#690005",
            _ => "#1B222B",
        },
    }
}

fn escape_rust_str(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn escape_html_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

fn active_or_root_frames(doc: &FigCraftDocument) -> &[FigCraftFrameNode] {
    if let Some(page) = doc.active_page()
        && !page.root_frames.is_empty()
    {
        return &page.root_frames;
    }
    &doc.root_frames
}

/// Exporter 1: Deterministic `gpui-kit` 0.10.0 Rust code generator bound to M3 Expressive tokens.
#[must_use]
pub fn export_to_gpui_kit_rs(doc: &FigCraftDocument) -> String {
    let mut out = String::from(
        "use gpui_kit::component::button::{Button, ButtonVariants as _};\n\
         use gpui_kit::component::label::Label;\n\
         use gpui_kit::component::tag::Tag;\n\
         use gpui_kit::{Context, IntoElement, ParentElement as _, Render, Styled as _, Window, div, px, rgb};\n\n",
    );

    let mode_label = doc.active_mode.label();
    let page_name = doc
        .active_page()
        .map_or("Overview Canvas", |p| p.name.as_str());
    out.push_str(&format!(
        "// Generated deterministically by FigCraft Studio ({mode_label} · Page `{page_name}`)\n\
         pub struct FigCraftKitScreen;\n\n\
         impl Render for FigCraftKitScreen {{\n\
         \x20   fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {{\n\
         \x20       div().flex().flex_col().gap(px(16.0)).p(px(24.0))\n"
    ));

    for frame in active_or_root_frames(doc) {
        emit_gpui_kit_node(&mut out, doc, frame, 3);
    }

    out.push_str("    }\n}\n");
    out
}

fn emit_gpui_kit_node(
    out: &mut String,
    doc: &FigCraftDocument,
    node: &FigCraftFrameNode,
    depth: usize,
) {
    if !node.visible {
        return;
    }
    let indent = "    ".repeat(depth);
    let fill_hex = doc.resolve_role_hex(&node.fill_role);
    let text_hex = doc.resolve_role_hex(&node.text_role);
    let fill_u32 = hex_to_u32(&fill_hex);
    let text_u32 = hex_to_u32(&text_hex);
    let id_esc = escape_rust_str(&node.stitch_id);
    let text_esc = escape_rust_str(node.text_content.as_deref().unwrap_or(&node.name));

    let layout_chain = match node.layout_mode {
        AutoLayoutMode::Grid => format!(".grid().grid_cols({})", node.grid_columns.max(1)),
        AutoLayoutMode::Flex | AutoLayoutMode::Freeform => match node.axis {
            AutoLayoutAxis::Horizontal => ".flex().flex_row()".to_owned(),
            AutoLayoutAxis::Vertical => ".flex().flex_col()".to_owned(),
        },
    };

    out.push_str(&format!(
        "{indent}.child(\n\
         {indent}    div()\n\
         {indent}        .id(\"{id_esc}\")\n\
         {indent}        {layout_chain}\n\
         {indent}        .gap(px({:.1}))\n\
         {indent}        .p(px({:.1}))\n\
         {indent}        .rounded(px({:.1}))\n\
         {indent}        .bg(rgb(0x{fill_u32:06X}))\n\
         {indent}        .text_color(rgb(0x{text_u32:06X}))\n",
        node.gap_px, node.padding_px, node.corner_radius_px
    ));

    match node.kind {
        FigCraftNodeKind::Button | FigCraftNodeKind::Fab => {
            let variant = if node.fill_role == "primary" || node.fill_role == "primary-container" {
                ".primary()"
            } else if node.stroke_role.is_some() {
                ".outline()"
            } else {
                ".secondary()"
            };
            out.push_str(&format!(
                "{indent}        .child(Button::new(\"{id_esc}-btn\"){variant}.label(\"{text_esc}\"))\n"
            ));
        }
        FigCraftNodeKind::Chip | FigCraftNodeKind::Badge => {
            out.push_str(&format!(
                "{indent}        .child(Tag::secondary().child(Label::new(\"{text_esc}\")))\n"
            ));
        }
        _ => {
            if node.text_content.is_some() || node.children.is_empty() {
                out.push_str(&format!(
                    "{indent}        .child(Label::new(\"{text_esc}\"))\n"
                ));
            }
        }
    }

    for child in &node.children {
        emit_gpui_kit_node(out, doc, child, depth + 2);
    }
    out.push_str(&format!("{indent})\n"));
}

/// Exporter 2: Deterministic NSH TSX + Rust bridge emitter preserving `data-stitch-id` & M3 tokens.
#[must_use]
pub fn export_to_nsh_rs_tsx(doc: &FigCraftDocument) -> String {
    let mut out = String::from(
        "import { mount, signal, ScrollView, Text, View } from \"nsh\";\n\n\
         // Rust host token sync: `aphrody_nsh::material3_kit::nsh_m3_theme_from_gpui`\n",
    );
    out.push_str(&format!(
        "const ACTIVE_M3_MODE = \"{}\";\n\n\
         export function FigCraftGoogleScreen() {{\n\
         \x20 const activeMode = signal(ACTIVE_M3_MODE);\n\
         \x20 return (\n\
         \x20   <ScrollView data-m3-mode={{activeMode()}} class=\"flex-1 bg-background\">\n",
        doc.active_mode.slug()
    ));

    for frame in active_or_root_frames(doc) {
        emit_nsh_tsx_node(&mut out, doc, frame, 3);
    }

    out.push_str(
        "    </ScrollView>\n  );\n}\n\nmount(FigCraftGoogleScreen);\n",
    );
    out
}

fn emit_nsh_tsx_node(
    out: &mut String,
    doc: &FigCraftDocument,
    node: &FigCraftFrameNode,
    depth: usize,
) {
    if !node.visible {
        return;
    }
    let indent = "  ".repeat(depth);
    let fill_hex = doc.resolve_role_hex(&node.fill_role);
    let layout_cls = match node.layout_mode {
        AutoLayoutMode::Grid => format!("grid grid-cols-{} gap-3", node.grid_columns.max(1)),
        AutoLayoutMode::Flex | AutoLayoutMode::Freeform => match node.axis {
            AutoLayoutAxis::Horizontal => "flex-row items-center gap-3".to_owned(),
            AutoLayoutAxis::Vertical => "flex-col gap-4".to_owned(),
        },
    };
    let role_attr = match node.kind {
        FigCraftNodeKind::Button | FigCraftNodeKind::Fab | FigCraftNodeKind::Chip => " role=\"button\"",
        FigCraftNodeKind::Dialog => " role=\"dialog\"",
        _ => "",
    };
    let radius_cls = if node.corner_radius_px >= 999.0 {
        "rounded-full"
    } else if node.corner_radius_px >= 20.0 {
        "rounded-3xl"
    } else if node.corner_radius_px >= 12.0 {
        "rounded-xl"
    } else {
        "rounded-lg"
    };
    let label = escape_html_text(node.text_content.as_deref().unwrap_or(&node.name));
    out.push_str(&format!(
        "{indent}<View data-stitch-id=\"{}\" data-fill=\"{fill_hex}\"{role_attr} class=\"{layout_cls} {radius_cls} p-4\">\n",
        escape_html_text(&node.stitch_id)
    ));
    if node.text_content.is_some() || node.children.is_empty() {
        out.push_str(&format!(
            "{indent}  <Text class=\"text-sm font-medium text-foreground\">{label}</Text>\n"
        ));
    }
    for child in &node.children {
        emit_nsh_tsx_node(out, doc, child, depth + 1);
    }
    out.push_str(&format!("{indent}</View>\n"));
}

/// Exporter 3: Pure GPUI + `ui` Rust emitter consuming `GoogleDesignTokens` & auto-layout primitives.
#[must_use]
pub fn export_to_gpui_rs(doc: &FigCraftDocument) -> String {
    let mut out = String::from(
        "use gpui::{IntoElement, ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px, rgb};\n\
         use ui::{Label, h_flex, v_flex};\n\n",
    );
    out.push_str(&format!(
        "// Active M3 Mode: {}\n\
         pub fn render_figcraft_screen() -> impl IntoElement {{\n\
         \x20   v_flex().gap_5().p_6()\n",
        doc.active_mode.label()
    ));

    for frame in active_or_root_frames(doc) {
        emit_pure_gpui_node(&mut out, doc, frame, 2);
    }

    out.push_str("}\n");
    out
}

fn emit_pure_gpui_node(
    out: &mut String,
    doc: &FigCraftDocument,
    node: &FigCraftFrameNode,
    depth: usize,
) {
    if !node.visible {
        return;
    }
    let indent = "    ".repeat(depth);
    let fill_u32 = hex_to_u32(&doc.resolve_role_hex(&node.fill_role));
    let text_u32 = hex_to_u32(&doc.resolve_role_hex(&node.text_role));
    let id_esc = escape_rust_str(&node.stitch_id);
    let text_esc = escape_rust_str(node.text_content.as_deref().unwrap_or(&node.name));

    let container = match node.layout_mode {
        AutoLayoutMode::Grid => format!("div().grid().grid_cols({})", node.grid_columns.max(1)),
        AutoLayoutMode::Flex | AutoLayoutMode::Freeform => match node.axis {
            AutoLayoutAxis::Horizontal => "h_flex()".to_owned(),
            AutoLayoutAxis::Vertical => "v_flex()".to_owned(),
        },
    };

    out.push_str(&format!(
        "{indent}.child(\n\
         {indent}    {container}\n\
         {indent}        .id(\"{id_esc}\")\n\
         {indent}        .gap(px({:.1}))\n\
         {indent}        .p(px({:.1}))\n\
         {indent}        .rounded(px({:.1}))\n\
         {indent}        .bg(rgb(0x{fill_u32:06X}))\n\
         {indent}        .text_color(rgb(0x{text_u32:06X}))\n",
        node.gap_px, node.padding_px, node.corner_radius_px
    ));
    if node.text_content.is_some() || node.children.is_empty() {
        out.push_str(&format!(
            "{indent}        .child(Label::new(\"{text_esc}\"))\n"
        ));
    }
    for child in &node.children {
        emit_pure_gpui_node(out, doc, child, depth + 2);
    }
    out.push_str(&format!("{indent})\n"));
}

/// Exporter 4: Baseline-compliant HTML5 passing `modern-web-guidance` & Lighthouse 100/100.
#[must_use]
pub fn export_to_lighthouse100_html(doc: &FigCraftDocument, design_md: &DesignMdDocument) -> String {
    let pick_mode_color = |role: &str, fallback: &str| -> String {
        let resolved = doc.resolve_role_hex(role);
        if !resolved.is_empty() {
            return resolved;
        }
        design_md
            .colors
            .get(role)
            .cloned()
            .unwrap_or_else(|| fallback.to_owned())
    };

    let surface = pick_mode_color("surface", "#0E141B");
    let on_surface = pick_mode_color("on-surface", "#E0E6F0");
    let primary = pick_mode_color("primary", "#8FD0FF");
    let on_primary = pick_mode_color("on-primary", "#003352");
    let container = pick_mode_color("surface-container", "#1B222B");
    let outline_variant = pick_mode_color("outline-variant", "#404956");

    let mut sections = String::new();
    for frame in active_or_root_frames(doc) {
        emit_html_node(&mut sections, doc, frame, 2);
    }

    format!(
        r#"<!doctype html>
<html lang="en" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="description" content="Aphrody Google FigCraft &amp; Stitch Studio synthesized screen.">
  <title>{title}</title>
  <style>
    :root {{
      --md-sys-color-surface: {surface};
      --md-sys-color-on-surface: {on_surface};
      --md-sys-color-primary: {primary};
      --md-sys-color-on-primary: {on_primary};
      --md-sys-color-surface-container: {container};
      --md-sys-color-outline-variant: {outline_variant};
      --md-ref-typeface-brand: "Google Sans Flex", system-ui, sans-serif;
    }}
    body {{ margin: 0; font-family: var(--md-ref-typeface-brand); background: var(--md-sys-color-surface); color: var(--md-sys-color-on-surface); padding: 24px; }}
    .frame {{ display: grid; gap: 16px; max-width: 1024px; margin: 0 auto 24px; padding: 24px; border-radius: 24px; background: var(--md-sys-color-surface-container); border: 1px solid var(--md-sys-color-outline-variant); }}
    .row {{ display: flex; flex-wrap: wrap; gap: 12px; align-items: center; }}
    .pill {{ min-height: 44px; border-radius: 9999px; border: 0; padding: 10px 20px; background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); font-weight: 600; cursor: pointer; transition: opacity 120ms ease; }}
    .pill:focus-visible {{ outline: 2px solid var(--md-sys-color-on-surface); outline-offset: 2px; }}
    @media (prefers-reduced-motion: reduce) {{ .pill {{ transition-duration: 0.01ms !important; }} }}
  </style>
</head>
<body>
  <header><h1>{title}</h1></header>
  <main data-slot="figcraft-canvas" data-m3-mode="{mode}" data-design-source="{src}">
{sections}  </main>
</body>
</html>
"#,
        title = escape_html_text(&doc.title),
        mode = doc.active_mode.slug(),
        src = escape_html_text(&doc.design_source),
    )
}

fn emit_html_node(
    out: &mut String,
    doc: &FigCraftDocument,
    node: &FigCraftFrameNode,
    depth: usize,
) {
    if !node.visible {
        return;
    }
    let indent = "  ".repeat(depth);
    let stitch_id = escape_html_text(&node.stitch_id);
    let text = escape_html_text(node.text_content.as_deref().unwrap_or(&node.name));
    let fill_hex = doc.resolve_role_hex(&node.fill_role);

    match node.kind {
        FigCraftNodeKind::Button
        | FigCraftNodeKind::Fab
        | FigCraftNodeKind::Chip
        | FigCraftNodeKind::Switch => {
            out.push_str(&format!(
                "{indent}<button type=\"button\" data-stitch-id=\"{stitch_id}\" data-slot=\"action\" class=\"pill\">{text}</button>\n"
            ));
        }
        FigCraftNodeKind::Screen | FigCraftNodeKind::Section | FigCraftNodeKind::Frame => {
            out.push_str(&format!(
                "{indent}<section data-stitch-id=\"{stitch_id}\" data-slot=\"frame\" style=\"border-radius: {:.0}px;\" class=\"frame\"><h2>{}</h2>\n",
                node.corner_radius_px.min(32.0),
                escape_html_text(&node.name)
            ));
            if let Some(ref body) = node.text_content {
                out.push_str(&format!("{indent}  <p>{}</p>\n", escape_html_text(body)));
            }
            for child in &node.children {
                emit_html_node(out, doc, child, depth + 1);
            }
            out.push_str(&format!("{indent}</section>\n"));
        }
        _ => {
            out.push_str(&format!(
                "{indent}<div data-stitch-id=\"{stitch_id}\" data-fill=\"{fill_hex}\" class=\"row\"><span>{text}</span></div>\n"
            ));
            for child in &node.children {
                emit_html_node(out, doc, child, depth + 1);
            }
        }
    }
}

/// Convenience helper loading bundled root `DESIGN.md` and exporting Lighthouse 100/100 HTML5.
pub fn export_to_lighthouse100_html_bundled(doc: &FigCraftDocument) -> Result<String> {
    let design_md = load_bundled_root_design_md()?;
    Ok(export_to_lighthouse100_html(doc, &design_md))
}

/// Exports a [`FigCraftDocument`] to NSH TSX preserving `data-stitch-id` attributes.
#[must_use]
pub fn export_figcraft_nsh_tsx(doc: &FigCraftDocument) -> String {
    export_to_nsh_rs_tsx(doc)
}

/// Exports a [`FigCraftDocument`] to idiomatic GPUI Rust consuming `ShadcnGpuiTheme` & `GoogleDesignTokens`.
#[must_use]
pub fn export_figcraft_gpui_rust(doc: &FigCraftDocument) -> String {
    export_to_gpui_rs(doc)
}

/// Exports a [`FigCraftDocument`] to Baseline-compliant HTML5 passing `modern-web-guidance` & Lighthouse 100/100.
#[must_use]
pub fn export_figcraft_baseline_html(doc: &FigCraftDocument, design_md: &DesignMdDocument) -> String {
    export_to_lighthouse100_html(doc, design_md)
}

fn hex_to_u32(hex: &str) -> u32 {
    let trimmed = hex.trim().trim_start_matches('#');
    if trimmed.len() >= 6 {
        u32::from_str_radix(&trimmed[..6], 16).unwrap_or(0x1B222B)
    } else {
        0x1B222B
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aphrody_design::web_standards::audit_html_standards;

    #[test]
    fn default_workspace_has_multi_page_tree_and_24_m3_templates() -> Result<()> {
        let doc = FigCraftDocument::google_expressive_default()?;
        assert!(doc.pages.len() >= 3);
        assert!(doc.total_node_count() >= 10);
        assert!(m3_expressive_starter_templates().len() >= 20);
        assert!(!doc.component_definitions.is_empty());
        Ok(())
    }

    #[test]
    fn document_editing_and_undo_redo_stack_round_trip() -> Result<()> {
        let mut doc = FigCraftDocument::google_expressive_default()?;
        let initial_pages = doc.pages.len();
        let initial_nodes = doc.total_node_count();

        let inserted_id = doc
            .insert_template_into(Some("node-root-screen"), "m3-fab")
            .expect("template insertion succeeds");
        assert_eq!(doc.total_node_count(), initial_nodes + 1);
        assert!(doc.can_undo());
        assert!(!doc.can_redo());

        assert!(doc.move_node_by(&inserted_id, 40.0, 24.0, Some(8.0)));
        assert!(doc.update_node_layout(
            &inserted_id,
            AutoLayoutAxis::Horizontal,
            AutoLayoutMode::Grid,
            12.0,
            PaddingEdges::uniform(12.0),
            3,
            LayoutAlignment::Center,
            LayoutAlignment::Center,
        ));

        let new_page_id = doc.add_page("Checkout Flow", PageKind::Canvas);
        assert_eq!(doc.pages.len(), initial_pages + 1);
        assert_eq!(doc.active_page_id, new_page_id);

        assert!(doc.undo());
        assert_eq!(doc.pages.len(), initial_pages);
        assert!(doc.can_redo());

        assert!(doc.redo());
        assert_eq!(doc.pages.len(), initial_pages + 1);

        // Undo back to initial state.
        while doc.undo() {}
        assert_eq!(doc.pages.len(), initial_pages);
        assert_eq!(doc.total_node_count(), initial_nodes);
        Ok(())
    }

    #[test]
    fn four_mode_m3_tokens_maintain_wcag_aa_contrast_and_rebind_nodes() -> Result<()> {
        let mut doc = FigCraftDocument::google_expressive_default()?;
        for mode in FigCraftDesignMode::ALL {
            doc.set_design_mode(mode);
            assert_eq!(doc.active_mode, mode);
            let ratio_surface = doc
                .contrast_ratio_for_roles("on-surface", "surface")
                .expect("surface contrast ratio computed");
            assert!(
                ratio_surface >= 4.5,
                "{mode:?} on-surface/surface contrast {ratio_surface:.2} < 4.5"
            );
            let ratio_primary = doc
                .contrast_ratio_for_roles("on-primary", "primary")
                .expect("primary contrast ratio computed");
            assert!(
                ratio_primary >= 4.5,
                "{mode:?} on-primary/primary contrast {ratio_primary:.2} < 4.5"
            );
            let cta = doc.find_node("node-primary-cta").expect("cta exists");
            assert!(!cta.bindings.is_empty());
        }
        Ok(())
    }

    #[test]
    fn snap_guides_and_component_variants_work_deterministically() -> Result<()> {
        let mut doc = FigCraftDocument::google_expressive_default()?;
        let snap = doc.compute_snap_guides("node-primary-cta", 33.0, 165.0, 6.0);
        assert_eq!(snap.snapped_x_px, 32.0);
        assert!(!snap.guides.is_empty());

        assert!(doc.apply_component_variant("node-primary-cta", "comp-m3-button", "outlined"));
        let cta = doc.find_node("node-primary-cta").expect("cta present");
        assert_eq!(cta.active_variant.as_deref(), Some("outlined"));
        assert_eq!(cta.stroke_role.as_deref(), Some("outline"));
        Ok(())
    }

    #[test]
    fn all_four_exporters_emit_valid_deterministic_outputs() -> Result<()> {
        let mut doc = FigCraftDocument::google_expressive_default()?;
        let design_md = load_bundled_root_design_md()?;

        for mode in FigCraftDesignMode::ALL {
            doc.set_design_mode(mode);

            let gpui_kit_code = export_to_gpui_kit_rs(&doc);
            assert!(gpui_kit_code.contains("use gpui_kit::component::button"));
            assert!(gpui_kit_code.contains("FigCraftKitScreen"));

            let nsh_code = export_to_nsh_rs_tsx(&doc);
            assert!(nsh_code.contains("data-stitch-id="));
            assert!(nsh_code.contains("FigCraftGoogleScreen"));
            assert!(nsh_code.contains(mode.slug()));

            let gpui_code = export_to_gpui_rs(&doc);
            assert!(gpui_code.contains("render_figcraft_screen"));
            assert!(gpui_code.contains("use ui::{Label"));

            let html = export_to_lighthouse100_html(&doc, &design_md);
            let report = audit_html_standards("figcraft.html", &html, &design_md, Some(64_000));
            assert!(report.ok, "{mode:?} Lighthouse findings: {:?}", report.findings);
            assert_eq!(report.scores.overall, 100);
        }
        Ok(())
    }
}
