//! Native core Aphrody (`zed`) Workspace studio integrating real-time `https://design.google/`
//! mirroring (*Code is a Design Material*, *Google Sans Flex*), `DESIGN.md` editing & lint/WCAG
//! checks, Chrome `modern-web-guidance` Baseline audits, and `https://stitch.withgoogle.com/`
//! sitemap + screen-capture panels directly into GPUI workspace panes.

use std::path::PathBuf;

use google_design_stitch::design_md::{
    DesignMdDocument, DesignMdLintReport, export_css_variables, export_shadcn_tailwind_css,
    lint_design_md, load_bundled_root_design_md,
};
use google_design_stitch::http_mirror::{
    DesignArticleSnapshot, DesignSitemapSnapshot, SitemapUrlEntry, fetch_or_fallback_article,
    mirror_design_google_sitemap,
};
use google_design_stitch::modern_web_guidance::{WebGuidanceFinding, audit_html_source};
use google_design_stitch::screen_capture::capture_stitch_screen_png;
use google_design_stitch::stitch_client::{StitchClient, StitchScreen, stitch_screen_to_nsh_tsx};
use google_design_stitch::web_standards::{LighthouseAuditReport, audit_html_standards};
use gpui_ui::ActiveTheme as _;
use gpui_ui::gpui::{
    self, App, AppContext as _, Context, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled, Window, actions,
    div,
};
use gpui_ui::ui::{
    FluentBuilder as _, Icon, IconName, InteractiveElement as _, Label, LabelCommon as _, h_flex,
    v_flex,
};
use os_ui::AppearanceMode;
use os_ui::google_design::GoogleSansFlexAxes;
use os_ui::starter_pack::{
    ShadcnGpuiTheme, StarterBadge, StarterButton, StarterCard, StarterKpiCard,
    StarterPackCatalog, StarterSize, StarterVariant, SubmoduleParityMatrix,
};
use workspace::Workspace;
use workspace::item::{Item, ItemEvent};

actions!(
    google_design_studio,
    [
        /// Opens a live GPUI panel mirroring `https://design.google/` (*Code is a Design Material*, *Google Sans Flex*, and sitemap URLs).
        OpenGoogleDesignMirror,
        /// Opens the native Google Stitch (`https://stitch.withgoogle.com/`) & `DESIGN.md` studio workspace tab.
        OpenStitchStudio,
        /// Opens the Core Web Vitals, Lighthouse, W3C, Vercel & shadcn/Base UI/Tailwind GPUI Starter Pack studio tab.
        OpenGpuiStarterPack,
        /// Renders and saves an offscreen PNG screen capture of the active Google Design / Stitch studio surface.
        CaptureDesignScreen,
    ]
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioMode {
    GoogleDesignMirror,
    StitchStudio,
    GpuiStarterPack,
}

struct LiveMirrorPayload {
    sitemap: Vec<SitemapUrlEntry>,
    articles: Vec<DesignArticleSnapshot>,
    live_count: usize,
}

pub struct DesignStitchStudioItem {
    focus_handle: FocusHandle,
    mode: StudioMode,
    worktree_root: Option<PathBuf>,
    sitemap: Vec<SitemapUrlEntry>,
    articles: Vec<DesignArticleSnapshot>,
    design_md: DesignMdDocument,
    lint_report: DesignMdLintReport,
    axes: GoogleSansFlexAxes,
    active_screen: Option<StitchScreen>,
    guidance_findings: Vec<WebGuidanceFinding>,
    lighthouse_report: Option<LighthouseAuditReport>,
    parity_matrix: Option<SubmoduleParityMatrix>,
    status_message: SharedString,
    last_capture: Option<PathBuf>,
    is_busy: bool,
}

impl DesignStitchStudioItem {
    pub fn new(mode: StudioMode, worktree_root: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let snapshot = DesignSitemapSnapshot::bundled().ok();
        let sitemap = snapshot.as_ref().map_or_else(Vec::new, |s| s.urls.clone());
        let articles = snapshot
            .as_ref()
            .map_or_else(Vec::new, |s| s.featured_articles.clone());
        let design_md = load_bundled_root_design_md().unwrap_or_else(|_| DesignMdDocument {
            name: "Aphrody Google Design System".to_owned(),
            source_of_truth: brand::DESIGN_SOURCE_OF_TRUTH.to_owned(),
            system: brand::DESIGN_SYSTEM.to_owned(),
            stitch_url: brand::DESIGN_STITCH_URL.to_owned(),
            sans_flex_family: brand::DESIGN_SANS_FLEX_FAMILY.to_owned(),
            colors: Default::default(),
            typography_raw: Default::default(),
            rounded: Default::default(),
            spacing: Default::default(),
            body_markdown: String::new(),
            headings: Vec::new(),
            yaml_blocks: 0,
        });
        let lint_report = lint_design_md(&design_md);
        let client = StitchClient::default();
        let active_screen = client
            .synthesize_screen("Aphrody Native Stitch Studio", "DESKTOP", &design_md, false)
            .ok();
        let guidance_findings = active_screen
            .as_ref()
            .map_or_else(Vec::new, |s| audit_html_source("stitch.html", &s.html_code));
        let lighthouse_report = active_screen
            .as_ref()
            .map(|s| audit_html_standards("stitch.html", &s.html_code, &design_md, Some(96_000)));
        let parity_matrix = SubmoduleParityMatrix::bundled().ok();

        let status_message = SharedString::from(format!(
            "Synced from {} ({} sitemap routes, WCAG {})",
            brand::DESIGN_SOURCE_OF_TRUTH,
            sitemap_len(&snapshot),
            lint_report.contrast_ratio_primary
        ));

        Self {
            focus_handle: cx.focus_handle(),
            mode,
            worktree_root,
            sitemap,
            articles,
            design_md,
            lint_report,
            axes: GoogleSansFlexAxes::default(),
            active_screen,
            guidance_findings,
            lighthouse_report,
            parity_matrix,
            status_message,
            last_capture: None,
            is_busy: false,
        }
    }

    pub fn refresh_live_mirror(&mut self, cx: &mut Context<Self>) {
        if self.is_busy {
            return;
        }
        self.is_busy = true;
        self.status_message = SharedString::from("Mirroring https://design.google/ in real time...");
        cx.notify();

        let cache_dir = self
            .worktree_root
            .as_ref()
            .map(|r| r.join(brand::LOCAL_SETTINGS_FOLDER).join("design-cache"));
        let bg = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let result = bg
                .spawn(async move {
                    let sitemap = mirror_design_google_sitemap(cache_dir.as_deref(), true)?;
                    let a1 = fetch_or_fallback_article(
                        "https://design.google/library/code-is-a-design-material",
                        cache_dir.as_deref(),
                        true,
                    )?;
                    let a2 = fetch_or_fallback_article(
                        "https://design.google/library/google-sans-flex-font",
                        cache_dir.as_deref(),
                        true,
                    )?;
                    let live_count = [a1.live_fetched, a2.live_fetched]
                        .into_iter()
                        .filter(|x| *x)
                        .count();
                    anyhow::Ok(LiveMirrorPayload {
                        sitemap,
                        articles: vec![a1, a2],
                        live_count,
                    })
                })
                .await;

            if let Err(err) = this.update(cx, |this, cx| {
                this.is_busy = false;
                match result {
                    Ok(payload) => {
                        this.status_message = SharedString::from(format!(
                            "Live mirror complete: {} sitemap URLs, {} live HTML5 articles fetched",
                            payload.sitemap.len(),
                            payload.live_count
                        ));
                        this.sitemap = payload.sitemap;
                        this.articles = payload.articles;
                    }
                    Err(err) => {
                        this.status_message =
                            SharedString::from(format!("Live mirror fallback active: {err}"));
                    }
                }
                cx.notify();
            }) {
                log::debug!("Design mirror view dropped before refresh finished: {err}");
            }
        })
        .detach();
    }

    pub fn capture_png(&mut self, cx: &mut Context<Self>) {
        if self.is_busy {
            return;
        }
        let Some(screen) = self.active_screen.clone() else {
            return;
        };
        let design_md = self.design_md.clone();
        let out_dir = self
            .worktree_root
            .clone()
            .unwrap_or_else(std::env::temp_dir)
            .join(brand::LOCAL_SETTINGS_FOLDER)
            .join("captures");
        let slug = match self.mode {
            StudioMode::GoogleDesignMirror => "design-google-mirror.png",
            StudioMode::StitchStudio => "stitch-studio-capture.png",
            StudioMode::GpuiStarterPack => "gpui-starter-pack-capture.png",
        };
        let out_path = out_dir.join(slug);
        self.is_busy = true;
        self.status_message = SharedString::from("Capturing Stitch PNG screen preview...");
        cx.notify();

        let bg = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let save_res = bg
                .spawn(async move { capture_stitch_screen_png(&design_md, &screen, 1280, 720, &out_path).map(|()| out_path) })
                .await;
            if let Err(err) = this.update(cx, |this, cx| {
                this.is_busy = false;
                match save_res {
                    Ok(path) => {
                        this.status_message = SharedString::from(format!(
                            "Captured PNG screen preview: {}",
                            path.display()
                        ));
                        this.last_capture = Some(path);
                    }
                    Err(err) => {
                        this.status_message = SharedString::from(format!("Capture failed: {err}"));
                    }
                }
                cx.notify();
            }) {
                log::debug!("Design studio dropped before PNG capture completed: {err}");
            }
        })
        .detach();
    }

    pub fn register(
        workspace: &mut Workspace,
        _window: &mut Window,
        _cx: &mut Context<Workspace>,
    ) {
        workspace.register_action(|workspace, _: &OpenGoogleDesignMirror, window, cx| {
            let root = workspace
                .worktrees(cx)
                .next()
                .map(|w| w.read(cx).abs_path().to_path_buf());
            let item = cx.new(|cx| Self::new(StudioMode::GoogleDesignMirror, root, cx));
            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
        });

        workspace.register_action(|workspace, _: &OpenStitchStudio, window, cx| {
            let root = workspace
                .worktrees(cx)
                .next()
                .map(|w| w.read(cx).abs_path().to_path_buf());
            let item = cx.new(|cx| Self::new(StudioMode::StitchStudio, root, cx));
            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
        });

        workspace.register_action(|workspace, _: &OpenGpuiStarterPack, window, cx| {
            let root = workspace
                .worktrees(cx)
                .next()
                .map(|w| w.read(cx).abs_path().to_path_buf());
            let item = cx.new(|cx| Self::new(StudioMode::GpuiStarterPack, root, cx));
            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
        });

        workspace.register_action(|workspace, _: &CaptureDesignScreen, _window, cx| {
            if let Some(active) = workspace.active_item(cx)
                && let Some(studio) = active.act_as::<Self>(cx)
            {
                studio.update(cx, |studio, cx| studio.capture_png(cx));
            }
        });
    }
}

fn sitemap_len(snapshot: &Option<DesignSitemapSnapshot>) -> usize {
    snapshot.as_ref().map_or(0, |s| s.total_urls)
}

impl Focusable for DesignStitchStudioItem {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<ItemEvent> for DesignStitchStudioItem {}

impl Render for DesignStitchStudioItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        let mode_title = match self.mode {
            StudioMode::GoogleDesignMirror => "Google Design Live Mirror (design.google)",
            StudioMode::StitchStudio => "Google Stitch & DESIGN.md Studio",
            StudioMode::GpuiStarterPack => {
                "GPUI Starter Pack (CWV + Lighthouse + W3C + Vercel + shadcn/Base UI/Tailwind)"
            }
        };
        let tsx_preview = self
            .active_screen
            .as_ref()
            .map(stitch_screen_to_nsh_tsx)
            .unwrap_or_default();
        let css_vars = export_css_variables(&self.design_md);
        let tailwind_css = export_shadcn_tailwind_css(&self.design_md);
        let starter_theme = ShadcnGpuiTheme::from_bundled(AppearanceMode::Dark).unwrap_or_else(|_| {
            ShadcnGpuiTheme::from_google_design(
                &os_ui::google_design::GoogleDesignTokens {
                    schema_version: 1,
                    source_of_truth: brand::DESIGN_SOURCE_OF_TRUTH.to_owned(),
                    system: brand::DESIGN_SYSTEM.to_owned(),
                    spec_url: String::new(),
                    fonts_url: String::new(),
                    stitch_url: brand::DESIGN_STITCH_URL.to_owned(),
                    sans_flex_family: brand::DESIGN_SANS_FLEX_FAMILY.to_owned(),
                    synced_at: String::new(),
                    seed_hex: String::new(),
                    variant: String::new(),
                    principles: Vec::new(),
                    reference: os_ui::google_design::RefTokens {
                        palette: Default::default(),
                        typeface: Default::default(),
                        variable_axes: Default::default(),
                    },
                    sys: os_ui::google_design::SysTokens {
                        color: os_ui::google_design::ColorPaletteMap {
                            light: Default::default(),
                            dark: Default::default(),
                        },
                        motion: os_ui::google_design::MotionTokens {
                            springs: Default::default(),
                        },
                        shape: os_ui::google_design::ShapeScale {
                            none_px: 0.0,
                            extra_small_px: 4.0,
                            small_px: 8.0,
                            medium_px: 12.0,
                            large_px: 16.0,
                            large_increased_px: 20.0,
                            extra_large_px: 28.0,
                            full_px: 9999.0,
                        },
                        state: os_ui::google_design::StateLayerOpacities {
                            hover_opacity: 0.08,
                            focus_opacity: 0.1,
                            pressed_opacity: 0.1,
                            dragged_opacity: 0.16,
                            disabled_container_opacity: 0.12,
                            disabled_content_opacity: 0.38,
                            focus_ring_px: 2.0,
                        },
                    },
                },
                AppearanceMode::Dark,
            )
        });
        let framework_count = self
            .parity_matrix
            .as_ref()
            .map_or(0, |matrix| matrix.frameworks.len());

        v_flex()
            .id("google-design-stitch-studio")
            .size_full()
            .p_4()
            .gap_3()
            .bg(colors.editor_background)
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(Label::new(mode_title))
                            .child(
                                Label::new(self.status_message.clone())
                                    .size(gpui_ui::ui::LabelSize::Small),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .id("refresh-live-design-google")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(colors.element_background)
                                    .cursor_pointer()
                                    .child(Label::new("Sync Live Mirror").size(gpui_ui::ui::LabelSize::Small))
                                    .on_click(cx.listener(|this, _, _, cx| this.refresh_live_mirror(cx))),
                            )
                            .child(
                                div()
                                    .id("capture-stitch-png")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(colors.element_background)
                                    .cursor_pointer()
                                    .child(Label::new("Capture PNG").size(gpui_ui::ui::LabelSize::Small))
                                    .on_click(cx.listener(|this, _, _, cx| this.capture_png(cx))),
                            ),
                    ),
            )
            .child(
                div()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.surface_background)
                    .child(Label::new(format!(
                        "DESIGN.md: {} | {} colors | Contrast {} | Font: {} ({}) | Baseline findings: {}",
                        self.design_md.source_of_truth,
                        self.lint_report.color_count,
                        self.lint_report.contrast_ratio_primary,
                        self.design_md.sans_flex_family,
                        self.axes.css_font_variation_settings(),
                        self.guidance_findings.len(),
                    )).size(gpui_ui::ui::LabelSize::Small)),
            )
            .child(
                v_flex()
                    .gap_2()
                    .children(self.articles.iter().take(3).map(|article| {
                        div()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(colors.border)
                            .bg(colors.panel_background)
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(Label::new(article.title.clone()))
                                    .child(
                                        Label::new(article.description.clone())
                                            .size(gpui_ui::ui::LabelSize::Small),
                                    ),
                            )
                    })),
            )
            .when(!tsx_preview.is_empty(), |col| {
                col.child(
                    div()
                        .p_3()
                        .rounded_md()
                        .border_1()
                        .border_color(colors.border)
                        .bg(colors.editor_background)
                        .child(
                            Label::new(format!(
                                "CSS tokens lines: {} | Tailwind v4 lines: {} | Stitch NSH TSX bytes: {} | Sitemap routes: {} | Parity stacks: {} | Starter primitives: {}",
                                css_vars.lines().count(),
                                tailwind_css.lines().count(),
                                tsx_preview.len(),
                                self.sitemap.len(),
                                framework_count,
                                StarterPackCatalog::primitives().len()
                            ))
                            .size(gpui_ui::ui::LabelSize::Small),
                        ),
                )
            })
            .when_some(self.lighthouse_report.as_ref(), |col, lh| {
                col.child(
                    StarterCard::new("Lighthouse & Core Web Vitals Readiness", &starter_theme)
                        .description("Live W3C, Google Style, Vercel, and shadcn/Base UI/Tailwind parity scores")
                        .footer(format!(
                            "LCP <= {}ms | INP <= {}ms | CLS <= {:.2} | Critical Wasm <= {}B",
                            lh.cwv.budgets.lcp_good_ms,
                            lh.cwv.budgets.inp_good_ms,
                            lh.cwv.budgets.cls_good_max,
                            lh.cwv.budgets.wasm_critical_brotli_bytes,
                        ))
                        .child(
                            h_flex()
                                .gap_2()
                                .child(StarterKpiCard::new(
                                    "Performance",
                                    format!("{}/100", lh.scores.performance),
                                    if lh.cwv.lcp_non_blocking_ok { "GOOD" } else { "WARN" },
                                    lh.cwv.lcp_non_blocking_ok,
                                    &starter_theme,
                                ))
                                .child(StarterKpiCard::new(
                                    "Accessibility",
                                    format!("{}/100", lh.scores.accessibility),
                                    "WCAG AA",
                                    lh.scores.accessibility >= 90,
                                    &starter_theme,
                                ))
                                .child(StarterKpiCard::new(
                                    "Best Practices",
                                    format!("{}/100", lh.scores.best_practices),
                                    "M3 + shadcn",
                                    lh.scores.best_practices >= 90,
                                    &starter_theme,
                                ))
                                .child(StarterKpiCard::new(
                                    "SEO & Agentic",
                                    format!("{}/100", lh.scores.seo_agentic),
                                    "data-slot",
                                    lh.scores.seo_agentic >= 90,
                                    &starter_theme,
                                )),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(StarterBadge::new(
                                    format!("{framework_count} Audited Stacks"),
                                    &starter_theme,
                                ))
                                .child(
                                    StarterButton::new(
                                        "starter-sync-mirror",
                                        "Sync Google Design Mirror",
                                        &starter_theme,
                                    )
                                    .variant(StarterVariant::Secondary)
                                    .size(StarterSize::Sm)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.refresh_live_mirror(cx);
                                    })),
                                ),
                        ),
                )
            })
    }
}

impl Item for DesignStitchStudioItem {
    type Event = ItemEvent;

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::BoltFilled))
    }

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        match self.mode {
            StudioMode::GoogleDesignMirror => SharedString::from("Google Design Mirror"),
            StudioMode::StitchStudio => SharedString::from("Stitch DESIGN.md Studio"),
            StudioMode::GpuiStarterPack => SharedString::from("GPUI Starter Pack"),
        }
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("google design studio: open")
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(ItemEvent)) {
        f(*event);
    }
}
