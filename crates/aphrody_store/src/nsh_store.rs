//! Cross-platform Awesome GPUI Store UI spanning:
//! 1. Embeddable NSH TSX source (`STORE_NSH_TSX`) compilable via `nsh-bundler` / QuickJS (`nsh-js`)
//! 2. Native NSH Rust reactive root (`StoreNshRoot` via `nsh_core` + `nsh_base`) + GPUI `AppView` host
//! 3. Adaptive `gpui` + `aphrody_ui` Store view (`StoreState`, `StoreCallbacks`, `render_store_window`)
//!    scaling cleanly across Desktop, Web (`wasm32` WebGPU), and Mobile (`iOS`/`Android`).

use std::sync::Arc;

use gpui::{AnyElement, App, ElementId, Role, SharedString, Window, div, prelude::*, px};
use aphrody_ui::{
    AdaptiveViewport, FormFactor, ImmediateUi, NavItem, OsTheme, StatusTone, focusable_action_chip,
    render_nav_rail, status_badge,
};
use ui::{Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::packagers::{GeneratedArtifact, generate_target};
use crate::{
    AwesomeGpuiCatalog, EntryStatus, PlatformTarget, StoreCategory, StoreEntry, StoreTarget,
    UptAction, detect_host_vendor, resolve_store_entry_command,
};

/// Cross-platform NSH TSX entry source (`nsh` + `nsh/base` + `nsh/host` channel) for Awesome GPUI Store.
pub const STORE_NSH_TSX: &str = r#"import { computed, host, mount, Scroll, signal, Text, View } from "nsh";
import { Button, Input } from "nsh/base";

export interface StoreCardItem {
  id: string;
  name: string;
  tagline: string;
  category: string;
  stars: number;
  targets: string[];
}

const TARGETS = [
  "winget", "msstore", "flatpak", "playstore", "appstore", "snap",
  "arch", "nix", "cargo", "npm", "chrome", "apk", "rpm", "deb",
  "xbps", "gentoo", "freebsd", "brew", "scoop", "choco", "appimage", "oci",
];

const CATEGORIES = ["all", "app", "library", "showcase", "tool", "resource"];

const DEFAULT_ENTRIES: StoreCardItem[] = [
  {
    id: "aphrody",
    name: "Aphrody",
    tagline: "OS and desktop (Zed fork, GPUI)",
    category: "app",
    stars: 0,
    targets: TARGETS,
  },
  {
    id: "aphrody-os",
    name: "Aphrody OS",
    tagline: "Cross-platform COSMIC + GPUI + NSH Desktop & Store",
    category: "app",
    stars: 0,
    targets: TARGETS,
  },
  {
    id: "gpui-store",
    name: "Awesome GPUI Store",
    tagline: "NSH + GPUI cross-platform app store & 22-target universal packager",
    category: "tool",
    stars: 0,
    targets: TARGETS,
  },
];

function StoreApp() {
  const query = signal("");
  const activeCategory = signal("all");
  const selectedId = signal("aphrody");
  const selectedTarget = signal("winget");
  const manifestPreview = signal("Select a target to preview generated store manifest.");

  const initial = Array.isArray(host.params?.entries)
    ? (host.params.entries as unknown as StoreCardItem[])
    : DEFAULT_ENTRIES;
  const entries = signal<StoreCardItem[]>(initial);

  const filtered = computed(() => {
    const q = query().trim().toLowerCase();
    const cat = activeCategory();
    return entries().filter((item) => {
      if (cat !== "all" && item.category !== cat) return false;
      if (!q) return true;
      return (
        item.name.toLowerCase().includes(q) ||
        item.tagline.toLowerCase().includes(q) ||
        item.id.toLowerCase().includes(q)
      );
    });
  });

  const activeEntry = computed(() => {
    return entries().find((e) => e.id === selectedId()) ?? entries()[0];
  });

  async function refreshPreview(id: string, target: string) {
    selectedId.set(id);
    selectedTarget.set(target);
    if (!host.available()) {
      manifestPreview.set(`// ${id} -> ${target} (host bridge inactive)`);
      return;
    }
    try {
      const res = await host.call<{ path?: string; content?: string }>("package_preview", { id, target });
      manifestPreview.set(res?.content ?? `// ${res?.path ?? target}`);
    } catch (err) {
      manifestPreview.set(String(err));
    }
  }

  return (
    <View
      class="flex-1 bg-background"
      style={{
        flexDirection: "column",
        paddingTop: "env(safe-area-inset-top)",
        paddingRight: "env(safe-area-inset-right)",
        paddingBottom: "env(safe-area-inset-bottom)",
        paddingLeft: "env(safe-area-inset-left)",
        padding: 12,
        gap: 10,
      }}
    >
      <View style={{ flexDirection: "row", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
        <Text class="text-foreground" style={{ fontSize: 18, fontWeight: "700" }}>
          Awesome GPUI Store
        </Text>
        <Text class="text-muted-foreground" style={{ fontSize: 12 }}>
          {`${filtered().length} packages · 22 targets`}
        </Text>
      </View>

      <Input
        value={query()}
        placeholder="Search GPUI apps, libraries, UI kits, crates..."
        onChange={(next) => query.set(next)}
        style={{ padding: 8, borderRadius: 8 }}
      />

      <View style={{ flexDirection: "row", flexWrap: "wrap", gap: 6 }}>
        <Each of={CATEGORIES} key={(c) => c}>
          {(cat) => (
            <Button
              selected={activeCategory() === cat()}
              onPress={() => activeCategory.set(cat())}
              style={{ paddingHorizontal: 10, paddingVertical: 6, borderRadius: 6 }}
            >
              <Text style={{ fontSize: 12 }}>{cat().toUpperCase()}</Text>
            </Button>
          )}
        </Each>
      </View>

      <Scroll style={{ flex: 1 }}>
        <View style={{ gap: 8 }}>
          <Each of={filtered()} key={(item) => item.id}>
            {(item) => (
              <Button
                selected={selectedId() === item().id}
                onPress={() => refreshPreview(item().id, selectedTarget())}
                style={{ padding: 10, borderRadius: 8, alignItems: "flex-start" }}
              >
                <View style={{ flexDirection: "column", gap: 2 }}>
                  <Text style={{ fontSize: 14, fontWeight: "600" }}>
                    {`${item().name} · ${item().category}`}
                  </Text>
                  <Text style={{ fontSize: 12 }}>{item().tagline}</Text>
                </View>
              </Button>
            )}
          </Each>
        </View>
      </Scroll>

      <If when={Boolean(activeEntry())}>
        <View style={{ gap: 6, paddingTop: 6 }}>
          <Text style={{ fontSize: 13, fontWeight: "600" }}>
            {`Export Matrix — ${activeEntry()?.name ?? ""} (${selectedTarget()})`}
          </Text>
          <View style={{ flexDirection: "row", flexWrap: "wrap", gap: 4 }}>
            <Each of={TARGETS} key={(t) => t}>
              {(tgt) => (
                <Button
                  selected={selectedTarget() === tgt()}
                  onPress={() => refreshPreview(selectedId(), tgt())}
                  style={{ paddingHorizontal: 8, paddingVertical: 4, borderRadius: 6 }}
                >
                  <Text style={{ fontSize: 11 }}>{tgt()}</Text>
                </Button>
              )}
            </Each>
          </View>
          <Text style={{ fontSize: 11 }}>{manifestPreview()}</Text>
        </View>
      </If>
    </View>
  );
}

mount(StoreApp);
"#;

/// Compiles [`STORE_NSH_TSX`] to ES module JS via `nsh-bundler` when `nsh-dev` is enabled.
#[cfg(feature = "nsh-dev")]
pub fn compile_store_tsx() -> Result<String, String> {
    nsh_bundler::compile_module(
        "StoreApp.tsx",
        STORE_NSH_TSX,
        &[nsh_base::registry().into()],
    )
}

#[cfg(feature = "nsh-ui")]
#[derive(Clone, PartialEq, Eq)]
struct NshCatalogRow {
    id: String,
    name: String,
    tagline: String,
    category: StoreCategory,
    stars: u64,
    license: String,
    target_count: usize,
}

/// Native cross-platform NSH Rust UI root (`nsh_core` + `nsh_base`) runnable on desktop, web, mobile, and TUI.
#[cfg(feature = "nsh-ui")]
#[nsh_core::prelude::component]
pub fn StoreNshRoot() -> impl nsh_core::prelude::IntoElement {
    use nsh_base::{button, theme};
    use nsh_core::element::input;
    use nsh_core::prelude::*;

    let catalog = AwesomeGpuiCatalog::bundled().unwrap_or(AwesomeGpuiCatalog {
        schema_version: 1,
        source_repo: "zed-industries/awesome-gpui".to_owned(),
        source_url: "https://github.com/zed-industries/awesome-gpui".to_owned(),
        synced_at: String::new(),
        upstream_stars: 0,
        total_entries: 0,
        entries: Vec::new(),
    });
    let catalog_arc = Arc::new(catalog);

    let query = signal(String::new());
    let category_filter = signal(Option::<StoreCategory>::None);
    let selected_id = signal(
        catalog_arc
            .entries
            .first()
            .map_or_else(|| "aphrody".to_owned(), |e| e.id.clone()),
    );
    let selected_target = signal(StoreTarget::WinGet);

    let cat_for_rows = Arc::clone(&catalog_arc);
    let rows = computed(move || {
        let q = query.get();
        let cat = category_filter.get();
        cat_for_rows
            .search(&q, cat, None, None)
            .into_iter()
            .take(64)
            .map(|e| NshCatalogRow {
                id: e.id.clone(),
                name: e.name.clone(),
                tagline: e.tagline.clone(),
                category: e.category,
                stars: e.stars,
                license: e.license.clone(),
                target_count: e.targets.len(),
            })
            .collect::<Vec<_>>()
    });

    let cat_for_preview = Arc::clone(&catalog_arc);
    let preview_text = computed(move || {
        let id = selected_id.get();
        let tgt = selected_target.get();
        if let Some(entry) = cat_for_preview.by_id(&id) {
            let artifacts = generate_target(entry, tgt);
            if let Some(first) = artifacts.first() {
                let snippet: String = first
                    .content
                    .lines()
                    .take(12)
                    .collect::<Vec<_>>()
                    .join("\n");
                return format!("// {} [{}]\n{}", first.relative_path, tgt.slug(), snippet);
            }
        }
        "Select an entry and target to inspect generated manifest.".to_owned()
    });

    let categories: Vec<Option<StoreCategory>> = std::iter::once(None)
        .chain(StoreCategory::ALL.into_iter().map(Some))
        .collect();
    let targets_vec: Vec<StoreTarget> = StoreTarget::ALL.to_vec();

    view()
        .flex_1()
        .flex_col()
        .p_safe_area()
        .p(px(12.0))
        .gap(px(10.0))
        .bg(move || theme().background)
        .child(
            view()
                .flex_row()
                .justify_between()
                .items_center()
                .child(
                    text("Awesome GPUI Store")
                        .text_size(px(18.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(move || theme().foreground),
                )
                .child(
                    text(move || format!("{} shown · 22 targets", rows.get().len()))
                        .text_size(px(12.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(
            input()
                .value(move || query.get())
                .placeholder("Search awesome-gpui apps, UI kits, crates...")
                .on_change(move |val: &str| query.set(val.to_owned()))
                .px(px(10.0))
                .py(px(8.0))
                .rounded(px(8.0))
                .border_1()
                .border_color(move || theme().input),
        )
        .child(view().flex_row().flex_wrap().gap(px(6.0)).child(each(
            move || categories.clone(),
            |c| c.map_or("all", StoreCategory::slug),
            move |cat_sig, _| {
                let c = cat_sig.get();
                let label = c.map_or("All", StoreCategory::label);
                button()
                    .selected(move || category_filter.get() == c)
                    .on_press(move || category_filter.set(c))
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(move || {
                        if category_filter.get() == c {
                            theme().primary
                        } else {
                            theme().secondary
                        }
                    })
                    .child(text(label).text_size(px(12.0)).text_color(move || {
                        if category_filter.get() == c {
                            theme().primary_foreground
                        } else {
                            theme().secondary_foreground
                        }
                    }))
            },
        )))
        .child(
            scroll()
                .flex_1()
                .child(view().flex_col().gap(px(6.0)).child(each(
                    rows,
                    |row| row.id.clone(),
                    move |row_sig, _| {
                        let row = row_sig.get();
                        let row_id = row.id.clone();
                        let row_id_cmp = row.id.clone();
                        let row_id_fg = row.id.clone();
                        button()
                            .selected(move || selected_id.get() == row_id_cmp)
                            .on_press(move || selected_id.set(row_id.clone()))
                            .p(px(10.0))
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(move || theme().border)
                            .bg(move || {
                                if selected_id.get() == row_id_fg {
                                    theme().accent
                                } else {
                                    theme().card
                                }
                            })
                            .child(
                                view()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        text(format!(
                                            "{} · {} · {}★ · {} ({} targets)",
                                            row.name,
                                            row.category.slug(),
                                            row.stars,
                                            row.license,
                                            row.target_count
                                        ))
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(move || theme().card_foreground),
                                    )
                                    .child(
                                        text(row.tagline)
                                            .text_size(px(11.0))
                                            .text_color(move || theme().muted_foreground),
                                    ),
                            )
                    },
                ))),
        )
        .child(
            view()
                .flex_col()
                .gap(px(6.0))
                .p(px(10.0))
                .rounded(px(8.0))
                .border_1()
                .border_color(move || theme().border)
                .bg(move || theme().card)
                .child(view().flex_row().flex_wrap().gap(px(4.0)).child(each(
                    move || targets_vec.clone(),
                    |t| *t,
                    move |tgt_sig, _| {
                        let tgt = tgt_sig.get();
                        button()
                            .selected(move || selected_target.get() == tgt)
                            .on_press(move || selected_target.set(tgt))
                            .px(px(6.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(move || {
                                if selected_target.get() == tgt {
                                    theme().primary
                                } else {
                                    theme().secondary
                                }
                            })
                            .child(text(tgt.slug()).text_size(px(11.0)).text_color(move || {
                                if selected_target.get() == tgt {
                                    theme().primary_foreground
                                } else {
                                    theme().secondary_foreground
                                }
                            }))
                    },
                )))
                .child(
                    text(move || preview_text.get())
                        .text_size(px(11.0))
                        .text_color(move || theme().foreground),
                ),
        )
}

/// Returns the NSH [`nsh_core::element::Root`] for mounting into any NSH backend (GPUI, Web, iOS, Android, TUI).
#[cfg(feature = "nsh-ui")]
#[must_use]
pub fn build_store_nsh_root() -> nsh_core::element::Root {
    nsh_core::element::Root::of::<StoreNshRoot>()
}

/// Mounts the live NSH Store UI inside a GPUI `AppView` on desktop platforms and bridges
/// `package_preview` & `upt_command` calls back to Rust.
#[cfg(all(
    feature = "desktop",
    any(target_os = "linux", target_os = "macos", windows)
))]
pub fn mount_store_nsh_surface(
    window: &mut Window,
    cx: &mut gpui::Context<gpui_ui::nsh::AppView>,
) -> gpui_ui::nsh::AppView {
    use gpui_ui::nsh::{
        AppOptions, AppView, ensure_nsh_ui_sets_registered, nsh_engine,
        sync_app_view_with_gpui_theme,
    };

    ensure_nsh_ui_sets_registered();
    let options = AppOptions {
        app_id: format!("{}.Store", brand::IDENTIFIER_STABLE),
        title: "Awesome GPUI Store".to_owned(),
        ..Default::default()
    };

    let view = AppView::new(options, window, cx);
    let catalog = Arc::new(AwesomeGpuiCatalog::bundled().unwrap_or(AwesomeGpuiCatalog {
        schema_version: 1,
        source_repo: "zed-industries/awesome-gpui".to_owned(),
        source_url: "https://github.com/zed-industries/awesome-gpui".to_owned(),
        synced_at: String::new(),
        upstream_stars: 0,
        total_entries: 0,
        entries: Vec::new(),
    }));

    let host = view.host();
    let preview_catalog = Arc::clone(&catalog);
    host.handle("package_preview", move |call| {
        let id = call
            .data()
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("aphrody");
        let target_slug = call
            .data()
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("winget");
        if let (Some(entry), Some(target)) = (
            preview_catalog.by_id(id),
            StoreTarget::from_slug(target_slug),
        ) {
            let artifacts = generate_target(entry, target);
            if let Some(first) = artifacts.first() {
                call.resolve(serde_json::json!({
                    "path": first.relative_path,
                    "content": first.content,
                    "fileCount": artifacts.len(),
                }));
                return;
            }
        }
        call.resolve(serde_json::json!({ "error": "unknown entry or target" }));
    });

    let upt_catalog = Arc::clone(&catalog);
    host.handle("upt_command", move |call| {
        let id = call
            .data()
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("aphrody");
        let vendor = call.data().get("vendor").and_then(|v| v.as_str());
        if let Some(entry) = upt_catalog.by_id(id) {
            match resolve_store_entry_command(entry, vendor, UptAction::Install, false) {
                Ok(plan) => {
                    call.resolve(serde_json::to_value(plan).unwrap_or_default());
                    return;
                }
                Err(err) => {
                    call.resolve(serde_json::json!({ "error": err.to_string() }));
                    return;
                }
            }
        }
        call.resolve(serde_json::json!({ "error": "entry not found" }));
    });

    sync_app_view_with_gpui_theme(&view, cx);
    view.load(nsh_engine::App::Rust(build_store_nsh_root()));
    view
}

/// Interactive state backing the cross-platform `gpui` + `aphrody_ui` Awesome GPUI Store view.
#[derive(Debug, Clone)]
pub struct StoreState {
    pub query: String,
    pub category_filter: Option<StoreCategory>,
    pub target_filter: Option<StoreTarget>,
    pub platform_filter: Option<PlatformTarget>,
    pub selected_entry_id: String,
    pub active_export_target: StoreTarget,
    pub active_artifact_idx: usize,
    pub host_os: Option<String>,
    pub host_vendor: Option<String>,
    pub last_export_notice: Option<String>,
}

impl StoreState {
    #[must_use]
    pub fn new(catalog: &AwesomeGpuiCatalog) -> Self {
        let selected_entry_id = catalog
            .by_id("aphrody")
            .or_else(|| catalog.entries.first())
            .map_or_else(|| "aphrody".to_owned(), |e| e.id.clone());
        let (host_os, host_vendor) = detect_host_vendor();
        Self {
            query: String::new(),
            category_filter: None,
            target_filter: None,
            platform_filter: None,
            selected_entry_id,
            active_export_target: StoreTarget::WinGet,
            active_artifact_idx: 0,
            host_os,
            host_vendor,
            last_export_notice: None,
        }
    }

    pub fn select_entry(&mut self, id: &str) {
        self.selected_entry_id = id.to_owned();
        self.active_artifact_idx = 0;
    }

    pub fn select_export_target(&mut self, target: StoreTarget) {
        self.active_export_target = target;
        self.active_artifact_idx = 0;
    }
}

/// Callback table decoupling `render_store_window` from parent GPUI entity state.
pub struct StoreCallbacks {
    pub on_select_category: Arc<dyn Fn(Option<StoreCategory>, &mut Window, &mut App) + Send + Sync>,
    pub on_select_target_filter:
        Arc<dyn Fn(Option<StoreTarget>, &mut Window, &mut App) + Send + Sync>,
    pub on_select_entry: Arc<dyn Fn(String, &mut Window, &mut App) + Send + Sync>,
    pub on_select_export_target: Arc<dyn Fn(StoreTarget, &mut Window, &mut App) + Send + Sync>,
    pub on_select_artifact: Arc<dyn Fn(usize, &mut Window, &mut App) + Send + Sync>,
    pub on_export_active: Arc<dyn Fn(String, StoreTarget, &mut Window, &mut App) + Send + Sync>,
}

fn category_icon(cat: StoreCategory) -> IconName {
    match cat {
        StoreCategory::App => IconName::Blocks,
        StoreCategory::Library => IconName::Code,
        StoreCategory::Showcase => IconName::Sparkle,
        StoreCategory::Tool => IconName::ToolHammer,
        StoreCategory::Resource => IconName::Book,
    }
}

fn status_label_and_tone(status: EntryStatus) -> (&'static str, StatusTone) {
    match status {
        EntryStatus::Active => ("Active", StatusTone::Success),
        EntryStatus::Quiet => ("Quiet", StatusTone::Accent),
        EntryStatus::Dormant => ("Dormant", StatusTone::Neutral),
        EntryStatus::Archived => ("Archived", StatusTone::Warning),
        EntryStatus::Unavailable => ("External", StatusTone::Neutral),
    }
}

/// Renders the adaptive cross-platform GPUI + `aphrody_ui` Awesome GPUI Store view.
pub fn render_store_window(
    state: &StoreState,
    catalog: &AwesomeGpuiCatalog,
    window_width: f32,
    theme: &OsTheme,
    cb: StoreCallbacks,
) -> AnyElement {
    let vp = AdaptiveViewport::from_size(window_width, 900.0);
    let mut nav_items = vec![NavItem {
        id: SharedString::from("all"),
        label: SharedString::from("All Packages"),
        icon: IconName::Public,
        badge: Some(SharedString::from(catalog.entries.len().to_string())),
    }];
    for (cat, count) in catalog.category_counts() {
        nav_items.push(NavItem {
            id: SharedString::from(cat.slug()),
            label: SharedString::from(cat.label()),
            icon: category_icon(cat),
            badge: Some(SharedString::from(count.to_string())),
        });
    }

    let active_cat_id = state.category_filter.map_or("all", StoreCategory::slug);
    let on_cat = Arc::clone(&cb.on_select_category);
    let rail = render_nav_rail(
        "gpui-store-category-rail",
        &nav_items,
        active_cat_id,
        vp.compact_nav_rail,
        theme,
        move |id, window, cx| {
            let parsed = StoreCategory::ALL
                .into_iter()
                .find(|c| c.slug() == id.as_ref());
            on_cat(parsed, window, cx);
        },
    );

    let filtered = catalog.search(
        &state.query,
        state.category_filter,
        state.target_filter,
        state.platform_filter,
    );
    let selected: Option<&StoreEntry> = catalog
        .by_id(&state.selected_entry_id)
        .or_else(|| filtered.first().copied())
        .or_else(|| catalog.entries.first());

    let center = render_store_catalog_pane(state, catalog, &filtered, vp.form_factor, theme, &cb);
    let inspector = render_store_inspector_pane(state, selected, vp, theme, &cb);

    h_flex()
        .w_full()
        .flex_1()
        .min_h(px(0.0))
        .overflow_hidden()
        .child(rail)
        .child(
            div()
                .flex_1()
                .h_full()
                .min_w(px(0.0))
                .flex()
                .when(vp.stacked_inspector, |el| el.flex_col())
                .when(!vp.stacked_inspector, |el| el.flex_row())
                .overflow_hidden()
                .child(center)
                .child(inspector),
        )
        .into_any_element()
}

fn render_store_catalog_pane(
    state: &StoreState,
    catalog: &AwesomeGpuiCatalog,
    filtered: &[&StoreEntry],
    form_factor: FormFactor,
    theme: &OsTheme,
    cb: &StoreCallbacks,
) -> AnyElement {
    let host_badge = match (&state.host_os, &state.host_vendor) {
        (Some(os), Some(vendor)) => format!("upt: {os} ({vendor})"),
        (Some(os), None) => format!("host: {os}"),
        _ => "host: portable".to_owned(),
    };

    let mut target_bar = h_flex().w_full().gap_1().flex_wrap().items_center();
    let on_clear_tgt = Arc::clone(&cb.on_select_target_filter);
    target_bar = target_bar.child(focusable_action_chip(
        "store-target-all",
        IconName::Public,
        SharedString::from("All 22"),
        None,
        SharedString::from("Show all store targets"),
        state.target_filter.is_none(),
        200,
        theme,
        move |_, window, cx| on_clear_tgt(None, window, cx),
    ));

    for (idx, tgt) in StoreTarget::ALL.iter().copied().enumerate() {
        let selected = state.target_filter == Some(tgt);
        let on_tgt = Arc::clone(&cb.on_select_target_filter);
        target_bar = target_bar.child(focusable_action_chip(
            ElementId::Name(format!("store-filter-{}", tgt.slug()).into()),
            IconName::Box,
            SharedString::from(tgt.slug()),
            None,
            SharedString::from(format!("Filter by {}", tgt.label())),
            selected,
            (201 + idx) as isize,
            theme,
            move |_, window, cx| on_tgt(Some(tgt), window, cx),
        ));
    }

    let mut grid = div().w_full().flex().flex_wrap().gap_2();

    let card_min_w = match form_factor {
        FormFactor::Compact => px(240.0),
        FormFactor::Medium => px(260.0),
        FormFactor::Expanded => px(280.0),
    };

    for (idx, entry) in filtered.iter().take(60).enumerate() {
        let is_selected = entry.id == state.selected_entry_id;
        let entry_id = entry.id.clone();
        let on_pick = Arc::clone(&cb.on_select_entry);
        let (status_text, tone) = status_label_and_tone(entry.status);
        let bg = if is_selected {
            theme.colors.primary_container
        } else {
            theme.colors.surface_container_low
        };
        let border = if is_selected {
            theme.colors.primary
        } else {
            theme.colors.outline_variant
        };

        grid = grid.child(
            v_flex()
                .id(ElementId::Name(format!("store-card-{}", entry.id).into()))
                .flex_1()
                .min_w(card_min_w)
                .p_2p5()
                .gap_1p5()
                .rounded(px(10.0))
                .bg(bg)
                .border_1()
                .border_color(border)
                .cursor_pointer()
                .focusable()
                .tab_index((300 + idx) as isize)
                .role(Role::Button)
                .aria_label(SharedString::from(format!("Select {}", entry.name)))
                .on_click(move |_, window, cx| on_pick(entry_id.clone(), window, cx))
                .child(
                    h_flex()
                        .w_full()
                        .justify_between()
                        .items_center()
                        .gap_1()
                        .child(
                            h_flex()
                                .gap_1p5()
                                .items_center()
                                .child(
                                    Icon::new(category_icon(entry.category)).size(IconSize::Small),
                                )
                                .child(Label::new(entry.name.clone()).size(LabelSize::Default)),
                        )
                        .child(status_badge(IconName::Check, status_text, tone, theme)),
                )
                .child(
                    Label::new(entry.tagline.clone())
                        .size(LabelSize::XSmall)
                        .color(ui::Color::Muted)
                        .line_clamp(2),
                )
                .child(
                    h_flex()
                        .w_full()
                        .justify_between()
                        .items_center()
                        .child(
                            Label::new(format!("{} · {}", entry.subcategory, entry.license))
                                .size(LabelSize::XSmall)
                                .color(ui::Color::Muted),
                        )
                        .child(
                            Label::new(format!("{}★ · {} pkg", entry.stars, entry.targets.len()))
                                .size(LabelSize::XSmall),
                        ),
                ),
        );
    }

    v_flex()
        .flex_1()
        .h_full()
        .min_w(px(0.0))
        .p_3()
        .gap_2p5()
        .bg(theme.colors.surface)
        .child(
            h_flex()
                .w_full()
                .flex_wrap()
                .justify_between()
                .items_center()
                .gap_2()
                .child(
                    v_flex()
                        .gap_0p5()
                        .child(Label::new("Awesome GPUI Store").size(LabelSize::Large))
                        .child(
                            Label::new(format!(
                                "{} · {} / {} entries · 22 packaging ecosystems",
                                catalog.source_repo,
                                filtered.len(),
                                catalog.entries.len()
                            ))
                            .size(LabelSize::XSmall)
                            .color(ui::Color::Muted),
                        ),
                )
                .child(status_badge(
                    IconName::Terminal,
                    host_badge,
                    StatusTone::Accent,
                    theme,
                )),
        )
        .child(target_bar)
        .child(
            div()
                .id("store-grid-scroll")
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .child(grid),
        )
        .into_any_element()
}

fn render_store_inspector_pane(
    state: &StoreState,
    selected: Option<&StoreEntry>,
    vp: AdaptiveViewport,
    theme: &OsTheme,
    cb: &StoreCallbacks,
) -> AnyElement {
    let mut ui = ImmediateUi::new(theme);

    if let Some(entry) = selected {
        let matrix = entry.compatibility_matrix();
        ui.heading(entry.name.clone());
        ui.label(entry.description.clone());
        ui.kv_row("App ID", entry.app_id.clone());
        ui.kv_row("Crate", entry.crate_name.clone());
        ui.kv_row("npm", entry.npm_package.clone());
        ui.kv_row("Version", entry.version.clone());
        ui.kv_row("License", entry.license.clone());
        ui.kv_row("Stars", entry.stars.to_string());

        ui.section("Cross-Platform Matrix", |sec| {
            sec.kv_row(
                "Desktop (Win/Mac/Linux)",
                if matrix.desktop { "Ready" } else { "No" },
            );
            sec.kv_row(
                "WebGPU / Wasm32",
                if matrix.web_wasm { "Ready" } else { "Optional" },
            );
            sec.kv_row(
                "iOS / iPadOS",
                if matrix.mobile_ios {
                    "Ready"
                } else {
                    "Optional"
                },
            );
            sec.kv_row(
                "Android NDK / AAB",
                if matrix.mobile_android {
                    "Ready"
                } else {
                    "Optional"
                },
            );
            sec.kv_row(
                "FreeBSD pkg",
                if matrix.freebsd { "Ready" } else { "Optional" },
            );
            sec.kv_row(
                "Chrome MV3 Extension",
                if matrix.browser_extension {
                    "Ready"
                } else {
                    "Optional"
                },
            );
        });

        let preferred = state
            .active_export_target
            .upt_vendor_name()
            .or(state.host_vendor.as_deref());
        if let Ok(plan) = resolve_store_entry_command(entry, preferred, UptAction::Install, false) {
            ui.section("Unified `upt` Command", |sec| {
                sec.kv_row("Vendor", plan.vendor_name);
                sec.kv_row("Package Ref", plan.package_ref);
                sec.label(plan.shell_line);
            });
        }

        ui.section("22-Target Export Matrix", |sec| {
            for tgt in StoreTarget::ALL {
                let on_tgt = Arc::clone(&cb.on_select_export_target);
                let active = state.active_export_target == tgt;
                let supported = entry.supports_target(tgt);
                let marker = if supported { "*" } else { "gen" };
                sec.button(
                    ElementId::Name(format!("export-tgt-{}", tgt.slug()).into()),
                    IconName::FileCode,
                    format!("{} [{marker}]", tgt.label()),
                    active,
                    move |_, window, cx| on_tgt(tgt, window, cx),
                );
            }
        });

        let artifacts: Vec<GeneratedArtifact> = generate_target(entry, state.active_export_target);
        let active_art = artifacts
            .get(state.active_artifact_idx)
            .or_else(|| artifacts.first());

        let export_entry_id = entry.id.clone();
        let export_target = state.active_export_target;
        let on_export = Arc::clone(&cb.on_export_active);
        ui.section(
            format!("Preview: {}", state.active_export_target.label()),
            |sec| {
                for (idx, art) in artifacts.iter().enumerate() {
                    let on_art = Arc::clone(&cb.on_select_artifact);
                    sec.button(
                        ElementId::Name(format!("art-tab-{idx}").into()),
                        IconName::FileToml,
                        art.relative_path.clone(),
                        idx == state.active_artifact_idx,
                        move |_, window, cx| on_art(idx, window, cx),
                    );
                }
                if let Some(art) = active_art {
                    let preview: String =
                        art.content.lines().take(18).collect::<Vec<_>>().join("\n");
                    sec.label(preview);
                }
                sec.button(
                    "export-active-manifest",
                    IconName::Download,
                    format!("Export {}", export_target.slug()),
                    false,
                    move |_, window, cx| {
                        on_export(export_entry_id.clone(), export_target, window, cx)
                    },
                );
            },
        );

        if let Some(notice) = &state.last_export_notice {
            ui.label(notice.clone());
        }
    } else {
        ui.heading("No Matching Packages");
        ui.label("Clear search filters or select All Packages.");
    }

    let container = v_flex()
        .id("store-inspector-pane")
        .p_3()
        .gap_2()
        .overflow_y_scroll()
        .bg(theme.colors.surface_container)
        .border_color(theme.colors.outline_variant);

    if vp.stacked_inspector {
        container
            .w_full()
            .h(px(300.0))
            .border_t_1()
            .child(ui.finish())
            .into_any_element()
    } else {
        container
            .w(px(vp.studio_inspector_width + 48.0))
            .h_full()
            .border_l_1()
            .child(ui.finish())
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "nsh-dev")]
    #[test]
    fn store_nsh_tsx_compiles_with_nsh_bundler() {
        let compiled = compile_store_tsx().expect("STORE_NSH_TSX compiles cleanly");
        assert!(compiled.contains("from \"nsh:rt\""));
        assert!(compiled.contains("from \"nsh/base\""));
        assert!(!compiled.contains("<View"));
    }

    #[cfg(feature = "nsh-ui")]
    #[test]
    fn store_nsh_root_instantiates_cleanly() {
        let root = build_store_nsh_root();
        let debug = format!("{root:?}");
        assert!(debug.contains("Root"));
    }
}
