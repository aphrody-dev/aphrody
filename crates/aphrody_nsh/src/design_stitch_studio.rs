//! Native core Aphrody (`zed`) Workspace studio integrating real-time `https://design.google/`
//! mirroring (*Code is a Design Material*, *Google Sans Flex*), `DESIGN.md` editing & lint/WCAG
//! checks, Chrome `modern-web-guidance` Baseline audits, and `https://stitch.withgoogle.com/`
//! sitemap + screen-capture panels directly into GPUI workspace panes.

use std::path::PathBuf;

use aphrody_craft::{
    CraftStudioSession, FigCraftStudioEvent, FigCraftStudioView, ParityRenderMode, StudioPillar,
    render_gpui_studio_panel, spawn_figcraft_studio_view,
};
use aphrody_design::design_md::{
    DesignMdDocument, DesignMdLintReport, export_css_variables, export_shadcn_tailwind_css,
    lint_design_md, load_bundled_root_design_md,
};
use aphrody_design::http_mirror::{
    DesignArticleSnapshot, DesignSitemapSnapshot, SitemapUrlEntry, fetch_or_fallback_article,
    mirror_design_google_sitemap,
};
use aphrody_design::modern_web_guidance::{WebGuidanceFinding, audit_html_source};
use aphrody_design::screen_capture::capture_stitch_screen_png;
use aphrody_design::stitch_client::{
    StitchClient, StitchMaquetteSpec, StitchScreen, bundled_maquette_catalog,
    stitch_screen_to_nsh_tsx,
};
use aphrody_design::web_standards::{LighthouseAuditReport, audit_html_standards};
use gpui_ui::ActiveTheme as _;
use gpui_ui::gpui::{
    self, AnyElement, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled,
    Subscription, Window, actions, div, px,
};
use gpui_ui::kit::{
    COMPOSE_M3_VERSION, M3AppearanceMode, M3Compose15Catalog, M3MaterialShape,
    M3ParametricPrototyperView, apply_kit_m3_theme,
};
use gpui_ui::ui::{
    FluentBuilder as _, Icon, IconName, InteractiveElement as _, Label, LabelCommon as _, h_flex,
    v_flex,
};
use aphrody_ui::AppearanceMode;
use aphrody_ui::google_design::{GoogleSansFlexAxes, MirrorAccentPalette};
use aphrody_ui::google_design_mirror::{ReferenceGallery, ReferenceGalleryState};
use aphrody_ui::material_glass::{
    GlassMaterialPreset, LUCIDE_SHOWCASE_ICONS, M3_DYNAMIC_SEEDS, M3DynamicVariant,
    MATERIAL_SYMBOL_NAMES, MaterialSymbolStyle, compute_m3_dynamic_scheme, lucide_icon_svg,
    m3_rgb_to_hsla, material_symbol_svg,
};
use aphrody_ui::starter_pack::{
    ShadcnGpuiTheme, StarterBadge, StarterButton, StarterCard, StarterKpiCard,
    StarterPackCatalog, StarterSize, StarterVariant, SubmoduleParityMatrix,
};
use workspace::Workspace;
use workspace::item::{Item, ItemEvent};

actions!(
    google_design_studio,
    [
        /// Opens the consolidated Aphrody Suite Hub uniting all 12 fork pillars, Material 3 modes, and studio launchers.
        OpenAphrodySuite,
        /// Opens the live Compose Material 3 1.5.0 Stable & Parametric Prototyping Studio powered by GPUI Kit.
        OpenMaterial3ComposeStudio,
        /// Opens a live GPUI panel mirroring `https://design.google/` (*Code is a Design Material*, *Google Sans Flex*, and sitemap URLs).
        OpenGoogleDesignMirror,
        /// Opens the native Google Stitch (`https://stitch.withgoogle.com/`) & `DESIGN.md` studio workspace tab.
        OpenStitchStudio,
        /// Opens the Core Web Vitals, Lighthouse, W3C, Vercel & shadcn/Base UI/Tailwind GPUI Starter Pack studio tab.
        OpenGpuiStarterPack,
        /// Opens the Google FigCraft + Stitch Studio tab (shared GUI core: GPUI Google Design vs egui 0.37).
        OpenGoogleFigCraftStudio,
        /// Opens the Google Craft Anything Workbench tab (clean-room X64/A64 disasm, oxc JS AST & SHA-256 evidence).
        OpenGoogleCraftAnythingStudio,
        /// Opens the Unreal Engine 5.8 (Bevy 0.20 + Nanite Meshlets + Solari GI + MaterialX->WGSL) studio tab.
        OpenUnrealBevyStudio,
        /// Cycles the active Studio GUI parity mode between GPUI Google Design, egui 0.37/bevy_egui, and 50/50 Split.
        ToggleStudioGuiParity,
        /// Renders and saves an offscreen PNG screen capture of the active Google Design / Stitch studio surface.
        CaptureDesignScreen,
    ]
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioMode {
    AphrodySuiteHub,
    Material3ComposeStudio,
    GoogleDesignMirror,
    StitchStudio,
    GpuiStarterPack,
    GoogleFigCraftStudio,
    GoogleCraftAnythingStudio,
    UnrealBevyStudio,
}

impl StudioMode {
    pub const ALL: [Self; 8] = [
        Self::AphrodySuiteHub,
        Self::Material3ComposeStudio,
        Self::GoogleDesignMirror,
        Self::StitchStudio,
        Self::GpuiStarterPack,
        Self::GoogleFigCraftStudio,
        Self::GoogleCraftAnythingStudio,
        Self::UnrealBevyStudio,
    ];

    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::AphrodySuiteHub => "Aphrody Hub",
            Self::Material3ComposeStudio => "Material 3 1.5",
            Self::GoogleDesignMirror => "Design Mirror",
            Self::StitchStudio => "Stitch",
            Self::GpuiStarterPack => "Starter Pack",
            Self::GoogleFigCraftStudio => "Canvas",
            Self::GoogleCraftAnythingStudio => "Inspector",
            Self::UnrealBevyStudio => "Bevy 5.8",
        }
    }
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
    maquette_catalog: Vec<StitchMaquetteSpec>,
    guidance_findings: Vec<WebGuidanceFinding>,
    lighthouse_report: Option<LighthouseAuditReport>,
    parity_matrix: Option<SubmoduleParityMatrix>,
    craft_session: Option<CraftStudioSession>,
    figcraft_view: Option<Entity<FigCraftStudioView>>,
    m3_prototyper_view: Option<Entity<M3ParametricPrototyperView>>,
    m3_mode: M3AppearanceMode,
    m3_seed_rgb: [u8; 3],
    m3_variant: M3DynamicVariant,
    glass_preset: GlassMaterialPreset,
    status_message: SharedString,
    last_capture: Option<PathBuf>,
    is_busy: bool,
    gallery: ReferenceGalleryState,
    gallery_accents: Option<MirrorAccentPalette>,
    _subscriptions: Vec<Subscription>,
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
        let maquette_catalog = bundled_maquette_catalog();
        let guidance_findings = active_screen
            .as_ref()
            .map_or_else(Vec::new, |s| audit_html_source("stitch.html", &s.html_code));
        let lighthouse_report = active_screen
            .as_ref()
            .map(|s| audit_html_standards("stitch.html", &s.html_code, &design_md, Some(96_000)));
        let parity_matrix = SubmoduleParityMatrix::bundled().ok();
        let mut craft_session = CraftStudioSession::bootstrap(AppearanceMode::Dark).ok();
        if let Some(ref mut session) = craft_session {
            session.active_pillar = match mode {
                StudioMode::GoogleFigCraftStudio => StudioPillar::FigCraftGoogle,
                StudioMode::GoogleCraftAnythingStudio => StudioPillar::CraftAnythingGoogle,
                StudioMode::UnrealBevyStudio => StudioPillar::UnrealBevy58,
                _ => StudioPillar::StorytoldSuiteCatalog,
            };
            if let Err(err) = session.run_parity_benchmark(640, 420) {
                log::warn!("Initial studio parity benchmark warning: {err}");
            }
        }

        let mut subscriptions = Vec::new();
        let figcraft_view = if mode == StudioMode::GoogleFigCraftStudio {
            let view = spawn_figcraft_studio_view(craft_session.as_ref(), cx);
            let sub = cx.subscribe(
                &view,
                |this: &mut Self, view: Entity<FigCraftStudioView>, event: &FigCraftStudioEvent, cx| {
                    match event {
                        FigCraftStudioEvent::DocumentMutated {
                            title,
                            node_count,
                            page_count,
                        } => {
                            let updated_doc = view.read(cx).document().clone();
                            if let Some(ref mut session) = this.craft_session
                                && let Err(err) = session.sync_figcraft_document(updated_doc)
                            {
                                log::warn!("FigCraft session sync warning: {err}");
                            }
                            this.status_message = SharedString::from(format!(
                                "{title}: {page_count} pages · {node_count} nodes"
                            ));
                            cx.notify();
                        }
                        FigCraftStudioEvent::ModeChanged(mode) => {
                            if let Some(ref mut session) = this.craft_session
                                && let Err(err) = session.set_appearance(mode.appearance_mode())
                            {
                                log::warn!("FigCraft appearance sync warning: {err}");
                            }
                            this.status_message = SharedString::from(format!(
                                "FigCraft M3 theme: {}",
                                mode.label()
                            ));
                            cx.notify();
                        }
                        FigCraftStudioEvent::ExportCopied { target, bytes } => {
                            this.status_message = SharedString::from(format!(
                                "Copied {} ({bytes} B)",
                                target.label()
                            ));
                            cx.notify();
                        }
                        FigCraftStudioEvent::ParityToggleRequested => {
                            this.cycle_parity_mode(cx);
                        }
                    }
                },
            );
            subscriptions.push(sub);
            Some(view)
        } else {
            None
        };

        let m3_prototyper_view = if mode == StudioMode::Material3ComposeStudio {
            Some(cx.new(M3ParametricPrototyperView::new))
        } else {
            None
        };

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
            maquette_catalog,
            guidance_findings,
            lighthouse_report,
            parity_matrix,
            craft_session,
            figcraft_view,
            m3_prototyper_view,
            m3_mode: M3AppearanceMode::NeuralObsidian,
            m3_seed_rgb: M3_DYNAMIC_SEEDS[0].rgb,
            m3_variant: M3DynamicVariant::Expressive,
            glass_preset: GlassMaterialPreset::LiquidGlass,
            status_message,
            last_capture: None,
            is_busy: false,
            gallery: ReferenceGalleryState::default(),
            gallery_accents: match MirrorAccentPalette::bundled() {
                Ok(accents) => Some(accents),
                Err(err) => {
                    log::warn!("Mirrored reference tokens unavailable: {err:#}");
                    None
                }
            },
            _subscriptions: subscriptions,
        }
    }

    pub fn switch_mode(&mut self, next: StudioMode, cx: &mut Context<Self>) {
        self.mode = next;
        if next == StudioMode::Material3ComposeStudio && self.m3_prototyper_view.is_none() {
            self.m3_prototyper_view = Some(cx.new(M3ParametricPrototyperView::new));
        }
        if next == StudioMode::GoogleFigCraftStudio && self.figcraft_view.is_none() {
            let view = spawn_figcraft_studio_view(self.craft_session.as_ref(), cx);
            self.figcraft_view = Some(view);
        }
        if let Some(ref mut session) = self.craft_session {
            session.active_pillar = match next {
                StudioMode::GoogleFigCraftStudio => StudioPillar::FigCraftGoogle,
                StudioMode::GoogleCraftAnythingStudio => StudioPillar::CraftAnythingGoogle,
                StudioMode::UnrealBevyStudio => StudioPillar::UnrealBevy58,
                _ => StudioPillar::StorytoldSuiteCatalog,
            };
        }
        self.status_message = SharedString::from(format!(
            "Active studio: {}",
            studio_mode_tab_title(next)
        ));
        cx.notify();
    }

    pub fn set_m3_mode(&mut self, mode: M3AppearanceMode, cx: &mut Context<Self>) {
        self.m3_mode = mode;
        apply_kit_m3_theme(mode, cx);
        self.status_message = SharedString::from(format!(
            "Material 3 mode: {} (Compose {COMPOSE_M3_VERSION})",
            mode.display_name()
        ));
        cx.notify();
    }

    pub fn cycle_parity_mode(&mut self, cx: &mut Context<Self>) {
        if let Some(ref mut session) = self.craft_session {
            match session.cycle_parity_mode() {
                Ok(next_mode) => {
                    if let Some(ref view) = self.figcraft_view {
                        view.update(cx, |view, cx| {
                            view.set_parity_label(next_mode.label(), cx);
                        });
                    }
                    self.status_message =
                        SharedString::from(format!("Studio parity mode: {}", next_mode.label()));
                    cx.notify();
                }
                Err(err) => {
                    log::warn!("Parity benchmark refresh warning: {err}");
                }
            }
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
                    // Figma serves an authenticated app shell, so its curated snapshot entry is kept.
                    let mut articles = Vec::new();
                    for curated in DesignSitemapSnapshot::bundled()?.featured_articles {
                        if curated.url.starts_with("https://www.figma.com/") {
                            articles.push(curated);
                        } else {
                            articles.push(fetch_or_fallback_article(
                                &curated.url,
                                cache_dir.as_deref(),
                                true,
                            )?);
                        }
                    }
                    let live_count = articles.iter().filter(|article| article.live_fetched).count();
                    anyhow::Ok(LiveMirrorPayload {
                        sitemap,
                        articles,
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
            StudioMode::AphrodySuiteHub => "aphrody-suite-hub.png",
            StudioMode::Material3ComposeStudio => "m3-compose15-studio.png",
            StudioMode::GoogleDesignMirror => "design-google-mirror.png",
            StudioMode::StitchStudio => "stitch-studio-capture.png",
            StudioMode::GpuiStarterPack => "gpui-starter-pack-capture.png",
            StudioMode::GoogleFigCraftStudio => "google-figcraft-studio.png",
            StudioMode::GoogleCraftAnythingStudio => "google-craft-anything-studio.png",
            StudioMode::UnrealBevyStudio => "unreal-bevy-58-studio.png",
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
        workspace.register_action(|workspace, _: &OpenAphrodySuite, window, cx| {
            let root = workspace
                .worktrees(cx)
                .next()
                .map(|w| w.read(cx).abs_path().to_path_buf());
            let item = cx.new(|cx| Self::new(StudioMode::AphrodySuiteHub, root, cx));
            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
        });

        workspace.register_action(|workspace, _: &OpenMaterial3ComposeStudio, window, cx| {
            let root = workspace
                .worktrees(cx)
                .next()
                .map(|w| w.read(cx).abs_path().to_path_buf());
            let item = cx.new(|cx| Self::new(StudioMode::Material3ComposeStudio, root, cx));
            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
        });

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

        workspace.register_action(|workspace, _: &OpenGoogleFigCraftStudio, window, cx| {
            let root = workspace
                .worktrees(cx)
                .next()
                .map(|w| w.read(cx).abs_path().to_path_buf());
            let item = cx.new(|cx| Self::new(StudioMode::GoogleFigCraftStudio, root, cx));
            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
        });

        workspace.register_action(|workspace, _: &OpenGoogleCraftAnythingStudio, window, cx| {
            let root = workspace
                .worktrees(cx)
                .next()
                .map(|w| w.read(cx).abs_path().to_path_buf());
            let item = cx.new(|cx| Self::new(StudioMode::GoogleCraftAnythingStudio, root, cx));
            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
        });

        workspace.register_action(|workspace, _: &OpenUnrealBevyStudio, window, cx| {
            let root = workspace
                .worktrees(cx)
                .next()
                .map(|w| w.read(cx).abs_path().to_path_buf());
            let item = cx.new(|cx| Self::new(StudioMode::UnrealBevyStudio, root, cx));
            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
        });

        workspace.register_action(|workspace, _: &ToggleStudioGuiParity, _window, cx| {
            if let Some(active) = workspace.active_item(cx)
                && let Some(studio) = active.act_as::<Self>(cx)
            {
                studio.update(cx, |studio, cx| {
                    studio.cycle_parity_mode(cx);
                });
            }
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

impl DesignStitchStudioItem {
    fn render_reference_gallery(
        &self,
        theme: &ShadcnGpuiTheme,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let accents = self.gallery_accents?;
        Some(
            ReferenceGallery::new("studio-reference-gallery", &self.gallery, theme, &accents)
                .thinking(self.is_busy)
                .on_change(cx.listener(|this, change, _, cx| {
                    if this.gallery.apply(change) {
                        cx.notify();
                    }
                }))
                .into_any_element(),
        )
    }
    fn render_mode_switcher_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors();
        h_flex()
            .id("aphrody-unified-studio-switcher")
            .w_full()
            .flex_wrap()
            .gap_1p5()
            .px_2()
            .py_1p5()
            .rounded_full()
            .bg(colors.toolbar_background)
            .border_1()
            .border_color(colors.border)
            .children(StudioMode::ALL.into_iter().map(|candidate| {
                let active = self.mode == candidate;
                div()
                    .id(SharedString::from(format!(
                        "studio-mode-{}",
                        candidate.short_label()
                    )))
                    .px_3()
                    .py_1()
                    .rounded_full()
                    .cursor_pointer()
                    .bg(if active {
                        colors.element_selected
                    } else {
                        colors.element_background
                    })
                    .border_1()
                    .border_color(if active {
                        colors.border_focused
                    } else {
                        colors.border_transparent
                    })
                    .child(
                        Label::new(candidate.short_label())
                            .size(gpui_ui::ui::LabelSize::Small),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.switch_mode(candidate, cx);
                    }))
            }))
            .into_any_element()
    }

    fn render_material_glass_lab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors();
        let dark = self.m3_mode != M3AppearanceMode::Light;
        let scheme = compute_m3_dynamic_scheme(
            (self.m3_seed_rgb[0], self.m3_seed_rgb[1], self.m3_seed_rgb[2]),
            dark,
            self.m3_variant,
            0.0,
        );
        let glass_plan = self.glass_preset.resolve_gpu_plan(2.0, 16.0);
        let md_count = MATERIAL_SYMBOL_NAMES
            .iter()
            .filter(|name| material_symbol_svg(name, MaterialSymbolStyle::Rounded).is_some())
            .count();
        let lucide_count = LUCIDE_SHOWCASE_ICONS
            .iter()
            .filter(|(_, icon)| lucide_icon_svg(*icon).contains("<svg"))
            .count();

        let tonal_swatches = [
            ("Primary", m3_rgb_to_hsla(scheme.primary)),
            ("Container", m3_rgb_to_hsla(scheme.primary_container)),
            ("Secondary", m3_rgb_to_hsla(scheme.secondary)),
            ("Tertiary", m3_rgb_to_hsla(scheme.tertiary)),
            ("Surface", m3_rgb_to_hsla(scheme.surface_container_high)),
            ("Error", m3_rgb_to_hsla(scheme.error)),
        ];

        v_flex()
            .w_full()
            .p_3()
            .gap_2()
            .rounded_md()
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface_background)
            .child(
                h_flex()
                    .justify_between()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Label::new(format!(
                            "Material 3 HCT (Spec2025 · {}) · {} ({:.0}px · {} Kawase · Fresnel {:.0}%) · Icons: {} md-icons + {} Lucide",
                            self.m3_variant.label(),
                            self.glass_preset.label(),
                            glass_plan.physical_radius_px,
                            glass_plan.kawase_levels,
                            glass_plan.specular_intensity * 100.0,
                            md_count,
                            lucide_count,
                        ))
                        .size(gpui_ui::ui::LabelSize::Small),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .children(tonal_swatches.into_iter().map(|(label, swatch)| {
                                h_flex()
                                    .gap_1()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_full()
                                    .border_1()
                                    .border_color(colors.border)
                                    .items_center()
                                    .child(div().size(px(10.0)).rounded_full().bg(swatch))
                                    .child(Label::new(label).size(gpui_ui::ui::LabelSize::XSmall))
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .flex_wrap()
                    .children(M3_DYNAMIC_SEEDS.iter().enumerate().map(|(idx, seed)| {
                        let selected = self.m3_seed_rgb == seed.rgb;
                        let rgb = seed.rgb;
                        let swatch: gpui_ui::gpui::Hsla = gpui_ui::gpui::Rgba {
                            r: f32::from(rgb[0]) / 255.0,
                            g: f32::from(rgb[1]) / 255.0,
                            b: f32::from(rgb[2]) / 255.0,
                            a: 1.0,
                        }
                        .into();
                        div()
                            .id(("nsh-m3-seed", idx))
                            .px_2()
                            .py_1()
                            .rounded_full()
                            .cursor_pointer()
                            .bg(if selected {
                                colors.element_selected
                            } else {
                                colors.element_background
                            })
                            .border_1()
                            .border_color(if selected {
                                colors.border_focused
                            } else {
                                colors.border
                            })
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .child(div().size(px(10.0)).rounded_full().bg(swatch))
                                    .child(
                                        Label::new(seed.name).size(gpui_ui::ui::LabelSize::XSmall),
                                    ),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.m3_seed_rgb = rgb;
                                this.status_message = SharedString::from(format!(
                                    "M3 HCT seed {} ({})",
                                    seed.name,
                                    this.m3_variant.label()
                                ));
                                cx.notify();
                            }))
                    }))
                    .children(GlassMaterialPreset::ALL.into_iter().enumerate().map(|(idx, preset)| {
                        let selected = self.glass_preset == preset;
                        div()
                            .id(("nsh-glass-preset", idx))
                            .px_2()
                            .py_1()
                            .rounded_full()
                            .cursor_pointer()
                            .bg(if selected {
                                colors.element_selected
                            } else {
                                colors.element_background
                            })
                            .border_1()
                            .border_color(if selected {
                                colors.border_focused
                            } else {
                                colors.border
                            })
                            .child(Label::new(preset.label()).size(gpui_ui::ui::LabelSize::XSmall))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.glass_preset = preset;
                                this.status_message = SharedString::from(format!(
                                    "Glass preset: {}",
                                    preset.label()
                                ));
                                cx.notify();
                            }))
                    })),
            )
            .into_any_element()
    }

    fn render_material3_compose_workspace(&self, cx: &mut Context<Self>) -> AnyElement {
        let switcher = self.render_mode_switcher_bar(cx);
        let material_glass_bar = self.render_material_glass_lab_bar(cx);
        let colors = cx.theme().colors();
        let catalog = M3Compose15Catalog::canonical();
        v_flex()
            .id("aphrody-m3-compose15-studio-workspace")
            .size_full()
            .p_3()
            .gap_3()
            .bg(colors.editor_background)
            .child(switcher)
            .child(material_glass_bar)
            .child(
                h_flex()
                    .w_full()
                    .flex_wrap()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.toolbar_background)
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(Label::new(format!(
                                "{} Material 3 Compose {COMPOSE_M3_VERSION} & Parametric Studio",
                                brand::APP_NAME
                            )))
                            .child(
                                Label::new(format!(
                                    "{} parity entries · {} MaterialShapes WGSL SDFs · {}",
                                    catalog.parity_entries.len(),
                                    M3MaterialShape::ALL_35.len(),
                                    self.status_message
                                ))
                                .size(gpui_ui::ui::LabelSize::Small),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .flex_wrap()
                            .children(M3AppearanceMode::ALL.into_iter().map(|mode| {
                                let selected = self.m3_mode == mode;
                                div()
                                    .id(SharedString::from(format!("studio-m3-mode-{}", mode.slug())))
                                    .px_2p5()
                                    .py_1()
                                    .rounded_full()
                                    .cursor_pointer()
                                    .bg(if selected {
                                        colors.element_selected
                                    } else {
                                        colors.element_background
                                    })
                                    .border_1()
                                    .border_color(if selected {
                                        colors.border_focused
                                    } else {
                                        colors.border
                                    })
                                    .child(Label::new(mode.label()).size(gpui_ui::ui::LabelSize::Small))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.set_m3_mode(mode, cx);
                                    }))
                            })),
                    ),
            )
            .child(
                div()
                    .id("m3-compose-studio-scroll")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scroll()
                    .when_some(self.m3_prototyper_view.clone(), |pane, view| {
                        pane.child(view)
                    }),
            )
            .into_any_element()
    }

    fn render_aphrody_suite_hub(&self, cx: &mut Context<Self>) -> AnyElement {
        let switcher = self.render_mode_switcher_bar(cx);
        let material_glass_bar = self.render_material_glass_lab_bar(cx);
        let colors = cx.theme().colors();
        let catalog = M3Compose15Catalog::canonical();
        v_flex()
            .id("aphrody-consolidated-suite-hub")
            .size_full()
            .overflow_y_scroll()
            .p_4()
            .gap_3()
            .bg(colors.editor_background)
            .child(switcher)
            .child(
                h_flex()
                    .w_full()
                    .flex_wrap()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.surface_background)
                    .child(
                        v_flex()
                            .gap_1()
                            .child(Label::new(format!(
                                "{} Hub ({} Capabilities)",
                                brand::APP_NAME,
                                brand::suite::APHRODY_PILLARS.len()
                            )))
                            .child(
                                Label::new(format!(
                                    "Single feature `aphrody` · Compose M3 {COMPOSE_M3_VERSION} ({} entries, {} shapes) · WCAG {:.1}:1 · {}",
                                    catalog.parity_entries.len(),
                                    M3MaterialShape::ALL_35.len(),
                                    brand::design::WCAG_AA_MIN_CONTRAST,
                                    brand::DESIGN_SOURCE_OF_TRUTH,
                                ))
                                .size(gpui_ui::ui::LabelSize::Small),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .flex_wrap()
                            .children(M3AppearanceMode::ALL.into_iter().map(|mode| {
                                let selected = self.m3_mode == mode;
                                div()
                                    .id(SharedString::from(format!("hub-m3-mode-{}", mode.slug())))
                                    .px_2p5()
                                    .py_1()
                                    .rounded_full()
                                    .cursor_pointer()
                                    .bg(if selected {
                                        colors.element_selected
                                    } else {
                                        colors.element_background
                                    })
                                    .border_1()
                                    .border_color(if selected {
                                        colors.border_focused
                                    } else {
                                        colors.border
                                    })
                                    .child(Label::new(mode.label()).size(gpui_ui::ui::LabelSize::Small))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.set_m3_mode(mode, cx);
                                    }))
                            })),
                    ),
            )
            .child(material_glass_bar)
            .child(
                h_flex()
                    .w_full()
                    .flex_wrap()
                    .gap_2()
                    .children(brand::suite::APHRODY_PILLARS.iter().map(|pillar| {
                        let target_mode = match pillar.id {
                            "material3-kit" => StudioMode::Material3ComposeStudio,
                            "design-stitch" => StudioMode::StitchStudio,
                            "craft-suite" => StudioMode::GoogleFigCraftStudio,
                            "brand-design" => StudioMode::GoogleDesignMirror,
                            "pilot-mcp" => StudioMode::GoogleCraftAnythingStudio,
                            "cosmic-os" | "browser" | "store" | "installer" | "vpn" | "auto" => {
                                StudioMode::GpuiStarterPack
                            }
                            _ => StudioMode::Material3ComposeStudio,
                        };
                        let surface_list = pillar
                            .surfaces
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(" · ");
                        div()
                            .id(SharedString::from(format!("hub-pillar-{}", pillar.id)))
                            .w(px(300.0))
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(colors.border)
                            .bg(colors.panel_background)
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.switch_mode(target_mode, cx);
                            }))
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .gap_2()
                                            .child(Label::new(pillar.name))
                                            .child(
                                                Label::new(pillar.primary_crate)
                                                    .size(gpui_ui::ui::LabelSize::Small),
                                            ),
                                    )
                                    .child(
                                        Label::new(pillar.summary)
                                            .size(gpui_ui::ui::LabelSize::Small),
                                    )
                                    .child(
                                        Label::new(SharedString::from(format!(
                                            "{surface_list} -> {}",
                                            target_mode.short_label()
                                        )))
                                        .size(gpui_ui::ui::LabelSize::XSmall),
                                    ),
                            )
                    })),
            )
            .into_any_element()
    }

    fn render_figcraft_workspace(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors();
        let parity_mode = self
            .craft_session
            .as_ref()
            .map_or(ParityRenderMode::GpuiGoogleDesign, |session| session.parity_mode);

        v_flex()
            .id("google-figcraft-studio-workspace")
            .size_full()
            .bg(colors.editor_background)
            .child(
                h_flex()
                    .w_full()
                    .px_3()
                    .py_1p5()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .bg(colors.toolbar_background)
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Label::new("Google FigCraft + Stitch Studio"))
                            .child(
                                Label::new(self.status_message.clone())
                                    .size(gpui_ui::ui::LabelSize::Small),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .id("figcraft-workspace-toggle-parity")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(colors.element_background)
                                    .border_1()
                                    .border_color(colors.border)
                                    .cursor_pointer()
                                    .child(
                                        Label::new(format!("Parity: {}", parity_mode.label()))
                                            .size(gpui_ui::ui::LabelSize::Small),
                                    )
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cycle_parity_mode(cx);
                                    })),
                            )
                            .child(
                                div()
                                    .id("figcraft-workspace-capture-png")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(colors.element_background)
                                    .cursor_pointer()
                                    .child(
                                        Label::new("Capture PNG")
                                            .size(gpui_ui::ui::LabelSize::Small),
                                    )
                                    .on_click(cx.listener(|this, _, _, cx| this.capture_png(cx))),
                            ),
                    ),
            )
            .child(match parity_mode {
                ParityRenderMode::GpuiGoogleDesign => div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .when_some(self.figcraft_view.clone(), |pane, view| pane.child(view))
                    .into_any_element(),
                ParityRenderMode::SplitSideBySide => h_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .when_some(self.figcraft_view.clone(), |pane, view| pane.child(view)),
                    )
                    .child(
                        div()
                            .id("figcraft-split-parity-hud")
                            .w(px(380.0))
                            .h_full()
                            .overflow_y_scroll()
                            .p_3()
                            .border_l_1()
                            .border_color(colors.border)
                            .when_some(self.craft_session.as_ref(), |pane, session| {
                                pane.child(render_gpui_studio_panel(session))
                            }),
                    )
                    .into_any_element(),
                ParityRenderMode::EguiNative037 => div()
                    .id("figcraft-egui-parity-only")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scroll()
                    .p_4()
                    .when_some(self.craft_session.as_ref(), |pane, session| {
                        pane.child(render_gpui_studio_panel(session))
                    })
                    .into_any_element(),
            })
            .into_any_element()
    }
}

impl Focusable for DesignStitchStudioItem {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        if let Some(ref view) = self.figcraft_view {
            return view.read(cx).focus_handle(cx);
        }
        self.focus_handle.clone()
    }
}

impl EventEmitter<ItemEvent> for DesignStitchStudioItem {}

impl Render for DesignStitchStudioItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.mode == StudioMode::AphrodySuiteHub {
            return self.render_aphrody_suite_hub(cx);
        }
        if self.mode == StudioMode::Material3ComposeStudio {
            return self.render_material3_compose_workspace(cx);
        }
        if self.mode == StudioMode::GoogleFigCraftStudio {
            return self.render_figcraft_workspace(cx);
        }
        let mode_title = match self.mode {
            StudioMode::AphrodySuiteHub => "Aphrody Hub (12 Capabilities)",
            StudioMode::Material3ComposeStudio => {
                "Material 3 Compose 1.5.0 & Parametric Prototyping Studio"
            }
            StudioMode::GoogleDesignMirror => "Google Design Live Mirror (design.google)",
            StudioMode::StitchStudio => "Google Stitch & DESIGN.md Studio",
            StudioMode::GpuiStarterPack => {
                "GPUI Starter Pack (CWV + Lighthouse + W3C + Vercel + shadcn/Base UI/Tailwind)"
            }
            StudioMode::GoogleFigCraftStudio => {
                "Design Canvas (.fig + Sketch + Penpot + GPUI vs egui 0.37)"
            }
            StudioMode::GoogleCraftAnythingStudio => {
                "Binary & JS Inspector (X64/A64 Disasm + oxc JS AST + GPUI vs egui 0.37)"
            }
            StudioMode::UnrealBevyStudio => {
                "Unreal Engine 5.8 Studio (Bevy 0.20 + Nanite Meshlets + Solari GI + MaterialX->WGSL)"
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
                &aphrody_ui::google_design::GoogleDesignTokens {
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
                        shape: aphrody_ui::google_design::ShapeScale {
                            none_px: 0.0,
                            extra_small_px: 4.0,
                            small_px: 8.0,
                            medium_px: 12.0,
                            large_px: 16.0,
                            large_increased_px: 20.0,
                            extra_large_px: 28.0,
                            full_px: 9999.0,
                        },
                        state: aphrody_ui::google_design::StateLayerOpacities {
                            hover_opacity: 0.08,
                            focus_opacity: 0.1,
                            pressed_opacity: 0.1,
                            dragged_opacity: 0.16,
                            disabled_container_opacity: 0.12,
                            disabled_content_opacity: 0.38,
                            focus_ring_px: 2.0,
                        },
                    },
                    comp: Default::default(),
                },
                AppearanceMode::Dark,
            )
        });
        let framework_count = self
            .parity_matrix
            .as_ref()
            .map_or(0, |matrix| matrix.frameworks.len());
        let reference_gallery = if self.mode == StudioMode::GoogleDesignMirror {
            self.render_reference_gallery(&starter_theme, cx)
        } else {
            None
        };
        let switcher_bar = self.render_mode_switcher_bar(cx);
        let colors = cx.theme().colors();

        v_flex()
            .id("google-design-stitch-studio")
            .size_full()
            .overflow_y_scroll()
            .p_4()
            .gap_3()
            .bg(colors.editor_background)
            .child(switcher_bar)
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
            .children(reference_gallery)
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
                                "CSS tokens lines: {} | Tailwind v4 lines: {} | Stitch NSH TSX bytes: {} | Maquettes: {} (Web/Desktop/Mobile) | Sitemap routes: {} | Parity stacks: {} | Starter primitives: {}",
                                css_vars.lines().count(),
                                tailwind_css.lines().count(),
                                tsx_preview.len(),
                                self.maquette_catalog.len(),
                                self.sitemap.len(),
                                framework_count,
                                StarterPackCatalog::primitives().len()
                            ))
                            .size(gpui_ui::ui::LabelSize::Small),
                        ),
                )
            })
            .when_some(self.craft_session.as_ref(), |col, session| {
                col.child(render_gpui_studio_panel(session))
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
            .into_any_element()
    }
}

impl Item for DesignStitchStudioItem {
    type Event = ItemEvent;

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::BoltFilled))
    }

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        SharedString::from(studio_mode_tab_title(self.mode))
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("google design studio: open")
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(ItemEvent)) {
        f(*event);
    }
}

pub(crate) fn studio_mode_tab_title(mode: StudioMode) -> &'static str {
    match mode {
        StudioMode::AphrodySuiteHub => "Aphrody Suite Hub",
        StudioMode::Material3ComposeStudio => "Material 3 Compose 1.5 Studio",
        StudioMode::GoogleDesignMirror => "Google Design Mirror",
        StudioMode::StitchStudio => "Stitch DESIGN.md Studio",
        StudioMode::GpuiStarterPack => "GPUI Starter Pack",
        StudioMode::GoogleFigCraftStudio => "Google FigCraft Studio",
        StudioMode::GoogleCraftAnythingStudio => "Google Craft Anything",
        StudioMode::UnrealBevyStudio => "Unreal Bevy 5.8",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aphrody_craft::{
        CraftStudioSession, FigCraftDesignMode, FigCraftDocument, ParityRenderMode,
    };

    #[test]
    fn consolidated_aphrody_suite_modes_cover_all_eight_studios_and_m3_compose15() -> anyhow::Result<()> {
        assert_eq!(StudioMode::ALL.len(), 8);
        assert_eq!(
            studio_mode_tab_title(StudioMode::AphrodySuiteHub),
            "Aphrody Suite Hub"
        );
        assert_eq!(
            studio_mode_tab_title(StudioMode::Material3ComposeStudio),
            "Material 3 Compose 1.5 Studio"
        );
        assert_eq!(brand::suite::APHRODY_PILLARS.len(), 12);
        let catalog = M3Compose15Catalog::canonical();
        assert!(!catalog.parity_entries.is_empty());
        assert_eq!(M3MaterialShape::ALL_35.len(), 35);
        assert_eq!(M3AppearanceMode::ALL.len(), 4);
        Ok(())
    }

    #[test]
    fn google_figcraft_studio_mode_and_session_default_to_gpui_google_design() -> anyhow::Result<()> {
        assert_eq!(
            studio_mode_tab_title(StudioMode::GoogleFigCraftStudio),
            "Google FigCraft Studio"
        );
        assert_eq!(
            studio_mode_tab_title(StudioMode::StitchStudio),
            "Stitch DESIGN.md Studio"
        );

        let mut session = CraftStudioSession::bootstrap(AppearanceMode::Dark)?;
        assert_eq!(session.parity_mode, ParityRenderMode::GpuiGoogleDesign);
        assert_eq!(
            session.cycle_parity_mode()?,
            ParityRenderMode::SplitSideBySide
        );
        assert_eq!(session.cycle_parity_mode()?, ParityRenderMode::EguiNative037);
        assert_eq!(
            session.cycle_parity_mode()?,
            ParityRenderMode::GpuiGoogleDesign
        );

        let mut doc = FigCraftDocument::google_expressive_default()?;
        doc.set_design_mode(FigCraftDesignMode::NeuralObsidian);
        session.sync_figcraft_document(doc)?;
        assert_eq!(
            session.figcraft.active_mode,
            FigCraftDesignMode::NeuralObsidian
        );
        Ok(())
    }
}
