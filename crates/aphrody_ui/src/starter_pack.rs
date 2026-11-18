//! Pure-Rust GPUI Starter Pack uniting shadcn/ui semantic color roles, Base UI headless
//! primitive state contracts, Tailwind v4 `@theme` radius/spacing tokens, Material 3 Expressive
//! (`https://design.google/`), Core Web Vitals / Lighthouse budgets, and Fork Submodule Parity.

use anyhow::{Context as _, Result, ensure};
use gpui::{ClickEvent, FontWeight, Hsla, Role, hsla};
use serde::{Deserialize, Serialize};
use ui::prelude::*;

use crate::cosmic_theme::{AppearanceMode, OsTheme, contrast_ratio};
use crate::google_design::{GoogleDesignTokens, ShapeScale};

/// Embedded submodule parity & CWV matrix (`assets/os/submodule_parity_matrix.toml`).
pub const BUNDLED_SUBMODULE_PARITY_TOML: &str =
    include_str!("../../../assets/os/submodule_parity_matrix.toml");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoreWebVitalsBudgetSpec {
    pub lcp_good_ms: u32,
    pub inp_good_ms: u32,
    pub cls_good_max: f32,
    pub fcp_good_ms: u32,
    pub ttfb_good_ms: u32,
    pub gpu_frame_60hz_ms: f32,
    pub gpu_frame_120hz_ms: f32,
    pub gpui_input_latency_ms: f32,
    pub wasm_critical_brotli_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonorepoAuditSummary {
    pub ts_files_scanned: usize,
    pub rust_files_scanned: usize,
    pub ts_compile_errors: usize,
    pub rust_mod_rs_violations: usize,
    pub rust_prod_unwrap_violations: usize,
    pub machine_path_leaks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmoduleParityEntry {
    pub id: String,
    pub label: String,
    pub repo_or_submodule: String,
    pub present_on_disk: bool,
    pub manifest_present: bool,
    pub cargo_patched: bool,
    pub wasm32_unknown: bool,
    pub wasip2_aot: bool,
    pub wgpu_v30_webgpu: bool,
    pub taffy_layout: bool,
    pub accesskit_aria: bool,
    pub google_design_tailwind: bool,
    pub shadcn_base_ui_starter: bool,
    pub ai_agent_mcp: bool,
    pub module_file_count: usize,
    pub role: String,
}

/// Complete parsed parity feature matrix comparing `aphrody-core` with all audited `vendor/*` submodules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmoduleParityMatrix {
    pub schema_version: u32,
    pub design_source_of_truth: String,
    pub starter_pack_crate: String,
    pub web_standards_crate: String,
    pub core_web_vitals_budgets: CoreWebVitalsBudgetSpec,
    pub monorepo_audit_summary: MonorepoAuditSummary,
    #[serde(default)]
    pub frameworks: Vec<SubmoduleParityEntry>,
}

impl SubmoduleParityMatrix {
    pub fn bundled() -> Result<Self> {
        toml::from_str(BUNDLED_SUBMODULE_PARITY_TOML)
            .context("parsing bundled submodule_parity_matrix.toml")
    }
}

/// Semantic shadcn/ui + Tailwind v4 theme roles derived deterministically from [`GoogleDesignTokens`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadcnGpuiTheme {
    pub mode: AppearanceMode,
    pub background: Hsla,
    pub foreground: Hsla,
    pub card: Hsla,
    pub card_foreground: Hsla,
    pub popover: Hsla,
    pub popover_foreground: Hsla,
    pub primary: Hsla,
    pub primary_foreground: Hsla,
    pub secondary: Hsla,
    pub secondary_foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    pub destructive: Hsla,
    pub destructive_foreground: Hsla,
    pub border: Hsla,
    pub input: Hsla,
    pub ring: Hsla,
    pub radius_sm_px: f32,
    pub radius_md_px: f32,
    pub radius_lg_px: f32,
    pub radius_xl_px: f32,
}

impl ShadcnGpuiTheme {
    /// Builds a WCAG 2.2 AA theme from bundled `GoogleDesignTokens` for `mode`.
    pub fn from_bundled(mode: AppearanceMode) -> Result<Self> {
        let tokens = GoogleDesignTokens::bundled()?;
        Ok(Self::from_google_design(&tokens, mode))
    }

    /// Maps Material 3 Expressive reference roles to shadcn/Tailwind v4 semantic tokens.
    #[must_use]
    pub fn from_google_design(tokens: &GoogleDesignTokens, mode: AppearanceMode) -> Self {
        let color_or = |role: &str, fallback: Hsla| tokens.hsla(mode, role).unwrap_or(fallback);
        let fallback_bg = match mode {
            AppearanceMode::Light => hsla(220.0 / 360.0, 0.4, 0.98, 1.0),
            AppearanceMode::Dark => hsla(212.0 / 360.0, 0.32, 0.08, 1.0),
        };
        let fallback_fg = match mode {
            AppearanceMode::Light => hsla(215.0 / 360.0, 0.25, 0.12, 1.0),
            AppearanceMode::Dark => hsla(215.0 / 360.0, 0.32, 0.90, 1.0),
        };
        let fallback_accent = hsla(203.0 / 360.0, 0.85, 0.55, 1.0);
        let fallback_error = hsla(0.0, 0.72, 0.52, 1.0);

        let background = color_or("surface", fallback_bg);
        let foreground = color_or("on_surface", fallback_fg);
        let card = color_or("surface_container_low", background);
        let popover = color_or("surface_container_high", card);
        let primary = color_or("primary", fallback_accent);
        let primary_foreground = color_or("on_primary", fallback_bg);
        let secondary = color_or("secondary_container", background);
        let secondary_foreground = color_or("on_secondary_container", foreground);
        let muted = color_or("surface_container", card);
        let muted_foreground = color_or("on_surface_variant", foreground);
        let accent = color_or("tertiary_container", secondary);
        let accent_foreground = color_or("on_tertiary_container", foreground);
        let destructive = color_or("error", fallback_error);
        let destructive_foreground = color_or("on_error", fallback_bg);
        let border = color_or("outline_variant", muted_foreground);

        Self {
            mode,
            background,
            foreground,
            card,
            card_foreground: foreground,
            popover,
            popover_foreground: foreground,
            primary,
            primary_foreground,
            secondary,
            secondary_foreground,
            muted,
            muted_foreground,
            accent,
            accent_foreground,
            destructive,
            destructive_foreground,
            border,
            input: border,
            ring: primary,
            radius_sm_px: tokens.sys.shape.small_px,
            radius_md_px: tokens.sys.shape.medium_px,
            radius_lg_px: tokens.sys.shape.large_px,
            radius_xl_px: tokens.sys.shape.extra_large_px,
        }
    }

    /// Maps the active desktop theme (`themes.json` entry and mode) onto the same semantic roles,
    /// so components follow theme switches instead of the bundled Google Design palette.
    #[must_use]
    pub fn from_os_theme(theme: &OsTheme, shape: &ShapeScale) -> Self {
        let colors = &theme.colors;
        Self {
            mode: theme.mode,
            background: colors.surface,
            foreground: colors.on_surface,
            card: colors.surface_container_low,
            card_foreground: colors.on_surface,
            popover: colors.surface_container_high,
            popover_foreground: colors.on_surface,
            primary: colors.primary,
            primary_foreground: colors.on_primary,
            secondary: colors.secondary_container,
            secondary_foreground: colors.on_secondary_container,
            muted: colors.surface_container,
            muted_foreground: colors.on_surface_variant,
            accent: colors.primary_container,
            accent_foreground: colors.on_primary_container,
            destructive: colors.error,
            destructive_foreground: colors.on_error,
            border: colors.outline_variant,
            input: colors.outline_variant,
            ring: colors.primary,
            radius_sm_px: shape.small_px,
            radius_md_px: shape.medium_px,
            radius_lg_px: shape.large_px,
            radius_xl_px: shape.extra_large_px,
        }
    }

    /// Verifies WCAG 2.2 AA (`>= 4.5:1`) text contrast across primary, card, muted, and destructive tokens.
    pub fn verify_wcag_aa(&self) -> Result<()> {
        for (name, fg, bg) in [
            ("foreground/background", self.foreground, self.background),
            ("card_foreground/card", self.card_foreground, self.card),
            ("primary_foreground/primary", self.primary_foreground, self.primary),
            (
                "secondary_foreground/secondary",
                self.secondary_foreground,
                self.secondary,
            ),
            ("muted_foreground/muted", self.muted_foreground, self.muted),
            (
                "destructive_foreground/destructive",
                self.destructive_foreground,
                self.destructive,
            ),
        ] {
            let ratio = contrast_ratio(fg, bg);
            ensure!(
                ratio >= 4.5,
                "{:?} `{name}` contrast {ratio:.2}:1 < WCAG AA 4.5:1",
                self.mode
            );
        }
        Ok(())
    }
}

/// Visual variants matching shadcn/ui component contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StarterVariant {
    #[default]
    Default,
    Secondary,
    Outline,
    Ghost,
    Destructive,
}

/// Spacing/sizing scale aligned to Material 3 density & Vercel 44px touch target guidelines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StarterSize {
    Sm,
    #[default]
    Md,
    Lg,
    Icon,
}

/// Deterministic AI-agent locator and Base UI headless accessibility contract attached to components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentSlotMeta {
    pub slot: &'static str,
    pub headless_primitive: &'static str,
    pub tailwind_recipe: &'static str,
    pub a11y_role: &'static str,
    pub wcag_target_min_px: f32,
}

/// Re-exported catalog entry describing the 16 Base UI + shadcn + Tailwind + GPUI primitives.
pub type StarterPrimitiveSpec = aphrody_design::web_standards::StarterPrimitiveSummary;

pub struct StarterPackCatalog;

impl StarterPackCatalog {
    #[must_use]
    pub const fn primitives() -> &'static [StarterPrimitiveSpec] {
        aphrody_design::web_standards::starter_primitive_summaries()
    }
}

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Accessible shadcn/Base UI styled GPUI Button (`RenderOnce`) with focus ring & variant tokens.
#[derive(IntoElement)]
pub struct StarterButton {
    id: ElementId,
    label: SharedString,
    variant: StarterVariant,
    size: StarterSize,
    disabled: bool,
    theme: ShadcnGpuiTheme,
    on_click: Option<ClickHandler>,
}

impl StarterButton {
    #[must_use]
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        theme: &ShadcnGpuiTheme,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: StarterVariant::Default,
            size: StarterSize::Md,
            disabled: false,
            theme: *theme,
            on_click: None,
        }
    }

    #[must_use]
    pub fn variant(mut self, variant: StarterVariant) -> Self {
        self.variant = variant;
        self
    }

    #[must_use]
    pub fn size(mut self, size: StarterSize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    #[must_use]
    pub const fn agent_meta() -> AgentSlotMeta {
        AgentSlotMeta {
            slot: "button",
            headless_primitive: "gpui::InteractiveElement + Role::Button",
            tailwind_recipe: "inline-flex items-center justify-center rounded-md text-sm font-medium",
            a11y_role: "button",
            wcag_target_min_px: 36.0,
        }
    }
}

impl RenderOnce for StarterButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (bg, fg, border_c) = match self.variant {
            StarterVariant::Default => (
                self.theme.primary,
                self.theme.primary_foreground,
                self.theme.primary,
            ),
            StarterVariant::Secondary => (
                self.theme.secondary,
                self.theme.secondary_foreground,
                self.theme.secondary,
            ),
            StarterVariant::Outline => (
                self.theme.background,
                self.theme.foreground,
                self.theme.border,
            ),
            StarterVariant::Ghost => (
                hsla(0.0, 0.0, 0.0, 0.0),
                self.theme.foreground,
                hsla(0.0, 0.0, 0.0, 0.0),
            ),
            StarterVariant::Destructive => (
                self.theme.destructive,
                self.theme.destructive_foreground,
                self.theme.destructive,
            ),
        };

        let (height_px, pad_x_px) = match self.size {
            StarterSize::Sm => (32.0_f32, 12.0_f32),
            StarterSize::Md => (36.0_f32, 16.0_f32),
            StarterSize::Lg => (44.0_f32, 20.0_f32),
            StarterSize::Icon => (36.0_f32, 8.0_f32),
        };

        let focus_ring = self.theme.ring;
        let mut base = div()
            .id(self.id)
            .h(rems_from_px(height_px))
            .px(rems_from_px(pad_x_px))
            .rounded(rems_from_px(self.theme.radius_sm_px))
            .border_1()
            .border_color(border_c)
            .bg(bg)
            .flex()
            .items_center()
            .justify_center()
            .role(Role::Button)
            .aria_label(self.label.clone())
            .when(self.disabled, |el| el.opacity(0.45))
            .when(!self.disabled, |el| {
                el.cursor_pointer()
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.border_2().border_color(focus_ring))
            })
            .child(
                div()
                    .text_color(fg)
                    .font_weight(FontWeight::MEDIUM)
                    .child(Label::new(self.label).size(LabelSize::Small).color(Color::Custom(fg))),
            );

        if !self.disabled
            && let Some(on_click) = self.on_click
        {
            base = base.on_click(move |ev, window, cx| on_click(ev, window, cx));
        }

        base
    }
}

/// Pill-shaped semantic status badge (`shadcn Badge`).
#[derive(IntoElement)]
pub struct StarterBadge {
    label: SharedString,
    variant: StarterVariant,
    theme: ShadcnGpuiTheme,
}

impl StarterBadge {
    #[must_use]
    pub fn new(label: impl Into<SharedString>, theme: &ShadcnGpuiTheme) -> Self {
        Self {
            label: label.into(),
            variant: StarterVariant::Secondary,
            theme: *theme,
        }
    }

    #[must_use]
    pub fn variant(mut self, variant: StarterVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl RenderOnce for StarterBadge {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (bg, fg, border_c) = match self.variant {
            StarterVariant::Default => (
                self.theme.primary,
                self.theme.primary_foreground,
                self.theme.primary,
            ),
            StarterVariant::Secondary => (
                self.theme.secondary,
                self.theme.secondary_foreground,
                self.theme.secondary,
            ),
            StarterVariant::Outline => (self.theme.card, self.theme.foreground, self.theme.border),
            StarterVariant::Ghost => (self.theme.muted, self.theme.muted_foreground, self.theme.muted),
            StarterVariant::Destructive => (
                self.theme.destructive,
                self.theme.destructive_foreground,
                self.theme.destructive,
            ),
        };
        div()
            .px(rems_from_px(10.0_f32))
            .py(rems_from_px(2.0_f32))
            .rounded(rems_from_px(999.0_f32))
            .border_1()
            .border_color(border_c)
            .bg(bg)
            .child(Label::new(self.label).size(LabelSize::XSmall).color(Color::Custom(fg)))
    }
}

/// Composable container card (`shadcn Card`) using semantic border, card surface, and radius tokens.
#[derive(IntoElement)]
pub struct StarterCard {
    title: SharedString,
    description: Option<SharedString>,
    footer: Option<SharedString>,
    body_children: Vec<AnyElement>,
    theme: ShadcnGpuiTheme,
}

impl StarterCard {
    #[must_use]
    pub fn new(title: impl Into<SharedString>, theme: &ShadcnGpuiTheme) -> Self {
        Self {
            title: title.into(),
            description: None,
            footer: None,
            body_children: Vec::new(),
            theme: *theme,
        }
    }

    #[must_use]
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    #[must_use]
    pub fn footer(mut self, footer: impl Into<SharedString>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    #[must_use]
    pub fn child(mut self, element: impl IntoElement) -> Self {
        self.body_children.push(element.into_any_element());
        self
    }
}

impl RenderOnce for StarterCard {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        v_flex()
            .w_full()
            .p_4()
            .gap_3()
            .rounded(rems_from_px(self.theme.radius_lg_px))
            .border_1()
            .border_color(self.theme.border)
            .bg(self.theme.card)
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div().font_weight(FontWeight::SEMIBOLD).child(
                            Label::new(self.title)
                                .size(LabelSize::Default)
                                .color(Color::Custom(self.theme.card_foreground)),
                        ),
                    )
                    .when_some(self.description, |col, desc| {
                        col.child(
                            Label::new(desc)
                                .size(LabelSize::Small)
                                .color(Color::Custom(self.theme.muted_foreground)),
                        )
                    }),
            )
            .children(self.body_children)
            .when_some(self.footer, |col, footer| {
                col.child(
                    div().pt_2().border_t_1().border_color(self.theme.border).child(
                        Label::new(footer)
                            .size(LabelSize::XSmall)
                            .color(Color::Custom(self.theme.muted_foreground)),
                    ),
                )
            })
    }
}

/// Core Web Vitals / Lighthouse KPI card with large value, threshold badge, and semantic framing.
#[derive(IntoElement)]
pub struct StarterKpiCard {
    title: SharedString,
    value: SharedString,
    status: SharedString,
    healthy: bool,
    theme: ShadcnGpuiTheme,
}

impl StarterKpiCard {
    #[must_use]
    pub fn new(
        title: impl Into<SharedString>,
        value: impl Into<SharedString>,
        status: impl Into<SharedString>,
        healthy: bool,
        theme: &ShadcnGpuiTheme,
    ) -> Self {
        Self {
            title: title.into(),
            value: value.into(),
            status: status.into(),
            healthy,
            theme: *theme,
        }
    }
}

impl RenderOnce for StarterKpiCard {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let badge_variant = if self.healthy {
            StarterVariant::Default
        } else {
            StarterVariant::Destructive
        };
        StarterCard::new(self.title, &self.theme)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .child(
                        div().font_weight(FontWeight::BOLD).child(
                            Label::new(self.value)
                                .size(LabelSize::Large)
                                .color(Color::Custom(self.theme.card_foreground)),
                        ),
                    )
                    .child(StarterBadge::new(self.status, &self.theme).variant(badge_variant)),
            )
    }
}

/// Inline semantic alert banner (`Role::Alert`).
#[derive(IntoElement)]
pub struct StarterAlert {
    title: SharedString,
    message: SharedString,
    destructive: bool,
    theme: ShadcnGpuiTheme,
}

impl StarterAlert {
    #[must_use]
    pub fn new(
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        destructive: bool,
        theme: &ShadcnGpuiTheme,
    ) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            destructive,
            theme: *theme,
        }
    }
}

impl RenderOnce for StarterAlert {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (bg, fg, border_c) = if self.destructive {
            (
                self.theme.destructive,
                self.theme.destructive_foreground,
                self.theme.destructive,
            )
        } else {
            (self.theme.muted, self.theme.foreground, self.theme.border)
        };
        v_flex()
            .w_full()
            .p_3()
            .gap_1()
            .rounded(rems_from_px(self.theme.radius_md_px))
            .border_1()
            .border_color(border_c)
            .bg(bg)
            .child(
                div().font_weight(FontWeight::SEMIBOLD).child(
                    Label::new(self.title).size(LabelSize::Small).color(Color::Custom(fg)),
                ),
            )
            .child(Label::new(self.message).size(LabelSize::XSmall).color(Color::Custom(fg)))
    }
}

/// CLS-safe reserved loading placeholder (`StarterSkeleton`) matching Base UI / shadcn geometry.
#[derive(IntoElement)]
pub struct StarterSkeleton {
    width_px: f32,
    height_px: f32,
    theme: ShadcnGpuiTheme,
}

impl StarterSkeleton {
    #[must_use]
    pub fn new(width_px: f32, height_px: f32, theme: &ShadcnGpuiTheme) -> Self {
        Self {
            width_px,
            height_px,
            theme: *theme,
        }
    }
}

impl RenderOnce for StarterSkeleton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let _role = Role::GenericContainer;
        div()
            .w(rems_from_px(self.width_px))
            .h(rems_from_px(self.height_px))
            .rounded(rems_from_px(self.theme.radius_sm_px))
            .bg(self.theme.muted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadcn_gpui_themes_meet_wcag_aa_in_light_and_dark_modes() -> Result<()> {
        for mode in [AppearanceMode::Light, AppearanceMode::Dark] {
            let theme = ShadcnGpuiTheme::from_bundled(mode)?;
            theme.verify_wcag_aa()?;
        }
        assert_eq!(StarterButton::agent_meta().slot, "button");
        assert!(StarterPackCatalog::primitives().len() >= 16);
        Ok(())
    }

    #[test]
    fn every_bundled_desktop_theme_maps_to_wcag_aa_starter_roles() -> Result<()> {
        let shape = GoogleDesignTokens::bundled()?.sys.shape;
        let catalog = crate::cosmic_theme::OsThemeCatalog::bundled()?;
        for id in catalog.ids() {
            for mode in [AppearanceMode::Light, AppearanceMode::Dark] {
                let os_theme = catalog
                    .resolve(&id, Some(mode))
                    .with_context(|| format!("theme {id} missing {mode:?}"))?;
                ShadcnGpuiTheme::from_os_theme(&os_theme, &shape)
                    .verify_wcag_aa()
                    .with_context(|| format!("theme {id}"))?;
            }
        }
        Ok(())
    }

    #[test]
    fn bundled_submodule_parity_matrix_parses_cleanly() -> Result<()> {
        let matrix = SubmoduleParityMatrix::bundled()?;
        assert_eq!(matrix.design_source_of_truth, "https://design.google/");
        assert!(matrix.frameworks.len() >= 11);
        assert!(matrix.core_web_vitals_budgets.lcp_good_ms <= 2500);
        assert!(matrix.core_web_vitals_budgets.inp_good_ms <= 200);
        assert_eq!(matrix.monorepo_audit_summary.rust_prod_unwrap_violations, 0);
        Ok(())
    }
}
