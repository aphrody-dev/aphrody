//! Unified Native Shell (`nsh`) <-> `aphrody` Workspace integration.
//!
//! Shared by both [`gpui_app_shell`] and `zed` (`--features nsh`), providing:
//! 1. **`GpuiCodeEditorComponent`** (`nativeComponent("GpuiCodeEditor", ...)`): embeds a full
//!    GPU-accelerated Tree-sitter multi-buffer [`Editor`] inside any NSH `.tsx` or Rust layout!
//! 2. **`NshSurfaceItem`** (`impl workspace::item::Item`): mounts any [`AppView`] inside a
//!    workspace pane/tab with automatic live theme, appearance, and font-scale synchronization.
//! 3. **Live TSX Hot-Reload Preview** (`OpenLiveTsxPreview`): compiles any `.tsx`/`.jsx`/`.ts`
//!    editor buffer in-memory via `nsh_bundler::compile_module` and live-updates the adjacent
//!    native GPUI surface on every buffer edit — with zero Node.js and zero `rolldown` overhead.
//! 4. **Dev Server & Bundle Surfaces** (`OpenShowcaseSurface`, `ConnectDevServer`).

#[cfg(feature = "studio")]
pub mod design_stitch_studio;
pub mod material3_kit;

pub use gpui_ui::nsh::{
    M3_ACCENTS, M3_SHAPES, M3_SPRINGS, M3_STATE_LAYERS, M3Mode, M3RolePalette, M3ShapeTokens,
    M3SpringPreset, M3SpringSpec, m3_theme,
};
pub use material3_kit::{
    M3AuraBadge, M3CardVariant, M3NavItem, M3NshTsxKit, M3SearchOmnibar, M3SegmentItem,
    M3SurfaceLevel, M3TopAppBar, export_m3_nsh_tsx_kit, m3_aura_badge, m3_card, m3_chip, m3_fab,
    m3_filled_button, m3_navigation_rail, m3_search_omnibar, m3_segmented_tabs, m3_surface,
    m3_switch, m3_tonal_button, m3_top_app_bar,
};

use std::rc::Rc;
use std::sync::Arc;

use editor::Editor;
use file_icons::FileIcons;
use gpui_ui::ActiveTheme as _;
use gpui_ui::GlobalTheme;
use gpui_ui::gpui::{
    self, AnyElement, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Window, actions, div,
};
use gpui_ui::nsh::{
    AppOptions, AppView, ComponentContext, NativeComponent, ensure_nsh_ui_sets_registered,
    nsh_base, nsh_bundler, nsh_engine, sync_app_view_with_gpui_theme,
};
use gpui_ui::ui::{
    FluentBuilder as _, Icon, IconName, InteractiveElement as _, Label, LabelCommon as _, h_flex,
    v_flex,
};
use language::{Buffer, BufferEvent, LanguageRegistry};
use serde_json::Value as JsonValue;
use util::ResultExt as _;
use workspace::item::{Item, ItemEvent};
use workspace::{SplitDirection, Workspace};

actions!(
    aphrody_nsh,
    [
        /// Opens a live hot-reloading NSH surface preview to the side of the active `.tsx`/`.jsx` editor.
        OpenLiveTsxPreview,
        /// Opens the interactive GPUI + NSH + embedded `GpuiCodeEditor` showcase surface in a new tab.
        OpenShowcaseSurface,
        /// Connects a new workspace pane surface to a configured `nsh dev` server.
        ConnectDevServer,
    ]
);

fn resolve_dev_server_url() -> String {
    if let Ok(url) = std::env::var("NSH_DEV_SERVER_URL")
        && !url.trim().is_empty()
    {
        return url;
    }
    if let Ok(url) = brand::env_var("DEV_SERVER_URL")
        && !url.trim().is_empty()
    {
        return url;
    }
    let host = std::env::var("NSH_DEV_HOST")
        .ok()
        .filter(|h| !h.trim().is_empty())
        .unwrap_or_else(|| std::net::Ipv4Addr::LOCALHOST.to_string());
    let port = std::env::var("NSH_DEV_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8790);
    format!("ws://{host}:{port}/ws")
}

const SHOWCASE_TSX_SOURCE: &str = r#"import { mount, signal, Text, View, ScrollView } from "nsh";
import { nativeComponent } from "nsh/native";

const GpuiCodeEditor = nativeComponent<{
  text?: string;
  language?: string;
  onChange?: (event: { text: string }) => void;
}>("GpuiCodeEditor");

const INITIAL_RUST_CODE = `// Embedded gpui_code::Editor running inside an NSH TSX surface!
pub fn compute_frame_budget_ms(target_hz: u32) -> f64 {
    1000.0 / Math.max(target_hz, 1) as f64
}
`;

function ShowcaseApp() {
  const taps = signal(0);
  const lineCount = signal(INITIAL_RUST_CODE.split("\n").length);

  return (
    <ScrollView class="flex-1 bg-background">
      <View class="p-6 gap-5">
        <View class="gap-1">
          <Text class="text-2xl font-bold text-foreground">
            APHRODY × Native Shell (NSH) Unified Surface
          </Text>
          <Text class="text-sm text-muted-foreground">
            Rendered natively by gpui 0.2.2 with live GlobalTheme tokens, Solid-style signals, and an embedded Tree-sitter GpuiCodeEditor.
          </Text>
        </View>

        <View class="flex-row items-center gap-3">
          <View
            role="button"
            onPress={() => taps.update((n) => n + 1)}
            class="rounded-(--radius) bg-primary px-4 py-2 active:bg-primary/90"
          >
            <Text class="text-sm font-medium text-primary-foreground">
              Trigger Signal ({taps()})
            </Text>
          </View>
          <View class="rounded-(--radius) border border-border bg-card px-3 py-2">
            <Text class="text-xs text-card-foreground">
              Editor Lines: {lineCount()}
            </Text>
          </View>
        </View>

        <View class="gap-2 rounded-(--radius) border border-border bg-card p-3">
          <Text class="text-xs font-semibold text-muted-foreground">
            NATIVE COMPONENT: GpuiCodeEditor (Tree-sitter + MultiBuffer + Vim)
          </Text>
          <GpuiCodeEditor
            language="Rust"
            text={INITIAL_RUST_CODE}
            onChange={(ev) => lineCount.set(ev.text.split("\n").length)}
            class="h-56 w-full rounded-(--radius) border border-input"
          />
        </View>
      </View>
    </ScrollView>
  );
}

mount(ShowcaseApp);
"#;

/// Native GPUI code editor (`gpui_code::Editor`) embeddable inside any NSH layout
/// via `nativeComponent("GpuiCodeEditor", { text, language, onChange })`.
pub struct GpuiCodeEditorComponent {
    component_cx: ComponentContext,
    languages: Arc<LanguageRegistry>,
    editor: Option<Entity<Editor>>,
    buffer: Option<Entity<Buffer>>,
    pending_text: String,
    pending_language: Option<String>,
    suppress_next_emit: bool,
    _subscription: Option<Subscription>,
}

impl GpuiCodeEditorComponent {
    /// Create a new factory closure that registers `"GpuiCodeEditor"` on an [`AppView`].
    pub fn register_on(view: &AppView, languages: Arc<LanguageRegistry>) {
        view.register_component(
            "GpuiCodeEditor",
            Rc::new(move |component_cx| {
                Box::new(Self {
                    component_cx,
                    languages: languages.clone(),
                    editor: None,
                    buffer: None,
                    pending_text: String::new(),
                    pending_language: None,
                    suppress_next_emit: false,
                    _subscription: None,
                })
            }),
        );
    }

    fn ensure_editor(&mut self, window: &mut Window, cx: &mut App) -> Entity<Editor> {
        if let Some(editor) = &self.editor {
            return editor.clone();
        }

        let initial_text = self.pending_text.clone();
        let buffer = cx.new(|cx| Buffer::local(initial_text, cx));
        if let Some(language_name) = self.pending_language.clone() {
            Self::apply_language(&buffer, &self.languages, language_name, cx);
        }

        let component_cx = self.component_cx.clone();
        let subscription = cx.subscribe(&buffer, move |buffer_entity, event, cx| {
            if matches!(event, BufferEvent::Edited { .. }) {
                let text = buffer_entity.read(cx).text();
                component_cx.emit("change", serde_json::json!({ "text": text }));
            }
        });

        let editor = cx.new(|cx| Editor::for_buffer(buffer.clone(), None, window, cx));
        self.buffer = Some(buffer);
        self.editor = Some(editor.clone());
        self._subscription = Some(subscription);
        editor
    }

    fn apply_language(
        buffer: &Entity<Buffer>,
        languages: &Arc<LanguageRegistry>,
        language_name: String,
        cx: &mut App,
    ) {
        let languages = languages.clone();
        let buffer = buffer.clone();
        cx.spawn(async move |cx| {
            if let Some(language) = languages.language_for_name(&language_name).await.log_err() {
                buffer.update(cx, |buffer, cx| buffer.set_language(Some(language), cx));
            }
        })
        .detach();
    }
}

impl NativeComponent for GpuiCodeEditorComponent {
    fn render(&mut self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let editor = self.ensure_editor(window, cx);
        Some(div().size_full().child(editor).into_any_element())
    }

    fn focus_handle(&self) -> Option<FocusHandle> {
        None
    }

    fn set_prop(
        &mut self,
        name: &str,
        value: Option<&JsonValue>,
        window: &mut Window,
        cx: &mut App,
    ) {
        match name {
            "text" => {
                let next_text = value.and_then(JsonValue::as_str).unwrap_or_default();
                self.pending_text = next_text.to_owned();
                if let Some(buffer) = &self.buffer {
                    let unchanged = buffer.read(cx).text() == next_text;
                    if !unchanged {
                        self.suppress_next_emit = true;
                        buffer.update(cx, |buffer, cx| {
                            buffer.set_text(next_text.to_owned(), cx);
                        });
                        self.suppress_next_emit = false;
                    }
                } else {
                    let _editor = self.ensure_editor(window, cx);
                }
            }
            "language" => {
                let lang = value.and_then(JsonValue::as_str).map(str::to_owned);
                self.pending_language = lang.clone();
                if let (Some(buffer), Some(language_name)) = (&self.buffer, lang) {
                    Self::apply_language(buffer, &self.languages, language_name, cx);
                }
            }
            _ => {}
        }
    }
}

/// First-class Workspace [`Item`] hosting a Native Shell [`AppView`] inside any pane or dock.
pub struct NshSurfaceItem {
    title: SharedString,
    app_view: Entity<AppView>,
    compile_error: Option<SharedString>,
    watched_buffer: Option<Entity<Buffer>>,
    _theme_subscription: Subscription,
    _buffer_subscription: Option<Subscription>,
}

impl NshSurfaceItem {
    /// Create a new [`NshSurfaceItem`] initialized with `aphrody` theme sync and `GpuiCodeEditor`.
    pub fn new(
        title: impl Into<SharedString>,
        options: AppOptions,
        languages: Arc<LanguageRegistry>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        ensure_nsh_ui_sets_registered();
        let title = title.into();
        let app_view = cx.new(|cx| AppView::new(options, window, cx));
        app_view.update(cx, |view, cx| {
            GpuiCodeEditorComponent::register_on(view, languages);
            sync_app_view_with_gpui_theme(view, cx);
        });

        let view_for_theme = app_view.clone();
        let theme_subscription = cx.observe_global::<GlobalTheme>(move |_this, cx| {
            view_for_theme.update(cx, |view, cx| {
                sync_app_view_with_gpui_theme(view, cx);
            });
            cx.notify();
        });

        Self {
            title,
            app_view,
            compile_error: None,
            watched_buffer: None,
            _theme_subscription: theme_subscription,
            _buffer_subscription: None,
        }
    }

    /// Create an interactive showcase surface demonstrating GPUI + NSH + `GpuiCodeEditor`.
    pub fn showcase(
        languages: Arc<LanguageRegistry>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut options = AppOptions::default();
        options.title = "NSH Showcase".to_owned();
        let mut item = Self::new("NSH Showcase", options, languages, window, cx);
        item.compile_and_load_tsx("Showcase.tsx", SHOWCASE_TSX_SOURCE, cx);
        item
    }

    /// Create a surface connected to a live `nsh dev` WebSocket server (`ws://127.0.0.1:8790/ws`).
    pub fn dev_server(
        url: impl Into<String>,
        languages: Arc<LanguageRegistry>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let url = url.into();
        let mut options = AppOptions::default();
        options.title = format!("NSH Dev ({url})");
        let item = Self::new(format!("NSH Dev ({url})"), options, languages, window, cx);
        item.app_view.update(cx, |view, cx| {
            view.connect_dev_server(url, cx);
        });
        item
    }

    /// Create a live-reloading preview surface bound to an open `.tsx`/`.jsx`/`.ts` [`Buffer`].
    pub fn live_tsx_preview(
        buffer: Entity<Buffer>,
        languages: Arc<LanguageRegistry>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let file_name = buffer
            .read(cx)
            .file()
            .map(|file| file.file_name(cx).to_owned())
            .unwrap_or_else(|| "App.tsx".to_owned());
        let tab_title = format!("NSH Preview: {file_name}");

        let mut options = AppOptions::default();
        options.title = tab_title.clone();
        let mut item = Self::new(tab_title, options, languages, window, cx);

        let initial_source = buffer.read(cx).text();
        item.compile_and_load_tsx(&file_name, &initial_source, cx);

        let module_name = file_name;
        let subscription = cx.subscribe(&buffer, move |this, buffer_entity, event, cx| {
            if matches!(event, BufferEvent::Edited { .. }) {
                let updated_source = buffer_entity.read(cx).text();
                this.compile_and_load_tsx(&module_name, &updated_source, cx);
            }
        });

        item.watched_buffer = Some(buffer);
        item._buffer_subscription = Some(subscription);
        item
    }

    /// Compile a single `.tsx`/`.jsx`/`.ts` module in-memory and load it into the underlying [`AppView`].
    pub fn compile_and_load_tsx(
        &mut self,
        module_name: &str,
        source: &str,
        cx: &mut Context<Self>,
    ) {
        let ui_sets = [nsh_base::registry().into()];
        match nsh_bundler::compile_module(module_name, source, &ui_sets) {
            Ok(es_module_code) => {
                self.compile_error = None;
                self.app_view.update(cx, |view, _cx| {
                    view.load(nsh_engine::App::Js(nsh_engine::Bundle::source(
                        es_module_code,
                    )));
                });
            }
            Err(diagnostic) => {
                self.compile_error = Some(SharedString::from(diagnostic));
            }
        }
        cx.notify();
    }

    /// Access the underlying `nsh_platform_gpui::AppView` entity.
    pub fn app_view(&self) -> &Entity<AppView> {
        &self.app_view
    }

    /// Register all NSH actions (`OpenLiveTsxPreview`, `OpenShowcaseSurface`, `ConnectDevServer`)
    /// on a [`Workspace`].
    pub fn register(workspace: &mut Workspace, _window: &mut Window, _cx: &mut Context<Workspace>) {
        workspace.register_action(|workspace, _: &OpenShowcaseSurface, window, cx| {
            let languages = workspace.app_state().languages.clone();
            let item = cx.new(|cx| Self::showcase(languages, window, cx));
            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
        });

        workspace.register_action(|workspace, _: &ConnectDevServer, window, cx| {
            let languages = workspace.app_state().languages.clone();
            let url = resolve_dev_server_url();
            let item = cx.new(|cx| Self::dev_server(url, languages, window, cx));
            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
        });

        workspace.register_action(|workspace, _: &OpenLiveTsxPreview, window, cx| {
            let languages = workspace.app_state().languages.clone();
            let active_buffer = workspace
                .active_item(cx)
                .and_then(|item| item.act_as::<Editor>(cx))
                .and_then(|editor| editor.read(cx).buffer().read(cx).as_singleton());

            if let Some(buffer) = active_buffer {
                let preview = cx.new(|cx| Self::live_tsx_preview(buffer, languages, window, cx));
                workspace.split_item(SplitDirection::Right, Box::new(preview), window, cx);
            } else {
                let showcase = cx.new(|cx| Self::showcase(languages, window, cx));
                workspace.add_item_to_active_pane(Box::new(showcase), None, true, window, cx);
            }
        });
    }
}

impl Focusable for NshSurfaceItem {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.app_view.read(cx).focus_handle(cx)
    }
}

impl EventEmitter<ItemEvent> for NshSurfaceItem {}

impl Render for NshSurfaceItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("nsh-surface-item")
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .child(div().flex_1().size_full().child(self.app_view.clone()))
            .when_some(self.compile_error.clone(), |container, err| {
                container.child(
                    h_flex()
                        .w_full()
                        .p_3()
                        .gap_2()
                        .border_t_1()
                        .border_color(cx.theme().status().error_border)
                        .bg(cx.theme().status().error_background)
                        .child(Icon::new(IconName::Warning))
                        .child(Label::new(err).size(gpui_ui::ui::LabelSize::Small)),
                )
            })
    }
}

impl Item for NshSurfaceItem {
    type Event = ItemEvent;

    fn tab_icon(&self, _window: &Window, cx: &App) -> Option<Icon> {
        self.watched_buffer
            .as_ref()
            .and_then(|buffer| buffer.read(cx).file())
            .and_then(|file| FileIcons::get_icon(file.path().as_std_path(), cx))
            .map(Icon::from_path)
            .or_else(|| Some(Icon::new(IconName::BoltFilled)))
    }

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        self.title.clone()
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("nsh surface: open")
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(ItemEvent)) {
        f(*event);
    }
}

/// Initialize global NSH Workspace integration so every new [`Workspace`] automatically
/// supports `OpenLiveTsxPreview`, `OpenShowcaseSurface`, and `ConnectDevServer`.
pub fn init(cx: &mut App) {
    ensure_nsh_ui_sets_registered();
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        NshSurfaceItem::register(workspace, window, cx);
        #[cfg(feature = "studio")]
        design_stitch_studio::DesignStitchStudioItem::register(workspace, window, cx);
    })
    .detach();
}
