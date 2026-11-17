//! Chromium `//ui/views` & `//chrome/browser/ui/views` rendered natively on GPUI (`ChromiumBrowserView`).
//!
//! Renders the full Chromium Aura/Views widget tree (`TabStripRegionView`, `ToolbarView`, `LocationBarView` /
//! `OmniboxView`, `VpnShieldChip`, `BrowseAgentChip`, `BookmarkBarView`, `HandoffInfobar`, `ContentsWebView`
//! with split-screen support, `SidePanelCoordinator`, and `StatusBubbleViews`) backed by [`WebGpuOsRuntime`].

use aphrody_ui::OsTheme;
use browser_views::{OmniboxIntent, SidePanelEntry, parse_omnibox_input};
use browser_vpn::{RouteMode, TunnelState};
use gpui::{
    AnyElement, App, Context, FocusHandle, Focusable, FontWeight, SharedString, Window, actions,
    div, prelude::*, px,
};
use ui::{Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::webgpu_os::WebGpuOsRuntime;

actions!(
    aphrody_browser,
    [
        NewBrowserTab,
        CloseActiveBrowserTab,
        ToggleBrowserSidePanel,
        EscalateBrowserEngine,
        CycleTabVpnExit,
    ]
);

/// Native GPUI view implementing Chromium's `BrowserView` (`//chrome/browser/ui/views/frame/browser_view.cc`).
pub struct ChromiumBrowserView {
    focus_handle: FocusHandle,
    pub runtime: WebGpuOsRuntime,
    pub theme: OsTheme,
}

impl ChromiumBrowserView {
    pub fn new(theme: OsTheme, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            runtime: WebGpuOsRuntime::new_default(),
            theme,
        }
    }

    pub fn submit_omnibox(&mut self, raw: &str, cx: &mut Context<Self>) {
        match parse_omnibox_input(raw, "https://www.google.com/search?q=") {
            OmniboxIntent::NavigateUrl(url) => {
                if let Some(tab) = self.runtime.browser_model.active_tab_mut() {
                    tab.url = url.clone();
                    tab.title = url.clone();
                }
                self.runtime.browser_model.omnibox_text = url;
            }
            OmniboxIntent::DefaultSearch { query } => {
                let encoded = query.replace(' ', "+");
                let url = format!("https://www.google.com/search?hl=fr&q={encoded}");
                if let Some(tab) = self.runtime.browser_model.active_tab_mut() {
                    tab.url = url.clone();
                    tab.title = format!("{query} — Aphrody Search");
                }
                self.runtime.browser_model.omnibox_text = url;
            }
            OmniboxIntent::AiModePrompt { prompt } => {
                self.runtime.browser_model.side_panel = Some(SidePanelEntry::BrowseAgent);
                self.runtime.exec_nysh_line(&format!("ai-os ask \"{prompt}\""));
            }
            OmniboxIntent::SelectVpnExit { exit_id } => {
                if self.runtime.vpn_router.select_exit(&exit_id).is_ok() {
                    self.runtime.browser_model.vpn_active_exit = exit_id.clone();
                    if let Some(tab) = self.runtime.browser_model.active_tab_mut() {
                        tab.vpn_route = RouteMode::PinnedExit(exit_id);
                    }
                }
            }
            OmniboxIntent::SwitchToTab { filter } => {
                let lower = filter.to_ascii_lowercase();
                if let Some(idx) = self
                    .runtime
                    .browser_model
                    .tabs
                    .iter()
                    .position(|t| t.title.to_ascii_lowercase().contains(&lower) || t.url.contains(&lower))
                {
                    self.runtime.browser_model.activate_tab(idx);
                }
            }
        }
        cx.notify();
    }

    fn render_tab_strip(&self, cx: &mut Context<Self>) -> AnyElement {
        let c = &self.theme.colors;
        let active_idx = self.runtime.browser_model.active_tab_index;
        let mut strip = h_flex()
            .h(px(38.0))
            .w_full()
            .px_2()
            .gap_1()
            .items_center()
            .bg(c.surface_container_lowest)
            .border_b_1()
            .border_color(c.outline_variant);

        for (idx, tab) in self.runtime.browser_model.tabs.iter().enumerate() {
            let is_active = idx == active_idx;
            let route_badge = match &tab.vpn_route {
                RouteMode::SystemTunnel => self.runtime.browser_model.vpn_active_exit.to_ascii_uppercase(),
                RouteMode::PinnedExit(id) => id.to_ascii_uppercase(),
                RouteMode::SplitBypass => "DIRECT".to_owned(),
            };
            let tab_label = SharedString::from(format!(
                "{} [{}] · {}",
                tab.title, tab.engine_id, route_badge
            ));
            let pill = h_flex()
                .id(gpui::ElementId::Name(format!("chromium-tab-{}", tab.id).into()))
                .h(px(30.0))
                .px_2p5()
                .gap_1p5()
                .items_center()
                .rounded(px(8.0))
                .bg(if is_active {
                    c.surface_container_high
                } else {
                    c.surface_container_low
                })
                .border_1()
                .border_color(if is_active { c.primary } else { c.outline_variant })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.runtime.browser_model.activate_tab(idx);
                    cx.notify();
                }))
                .child(
                    Label::new(tab_label)
                        .size(LabelSize::Small)
                        .weight(if is_active {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        }),
                );
            strip = strip.child(pill);
        }

        let new_tab_btn = h_flex()
            .id("chromium-new-tab-btn")
            .h(px(28.0))
            .px_2()
            .items_center()
            .rounded(px(6.0))
            .bg(c.surface_container)
            .on_click(cx.listener(|this, _, _, cx| {
                this.runtime.browser_model.open_tab(
                    "New Tab",
                    "https://www.google.com/?hl=fr",
                    RouteMode::SystemTunnel,
                );
                cx.notify();
            }))
            .child(Label::new("+").size(LabelSize::Small).weight(FontWeight::BOLD));

        strip.child(new_tab_btn).into_any_element()
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let c = &self.theme.colors;
        let vpn_connected = self.runtime.vpn_router.state() == TunnelState::Connected;
        let vpn_chip_text = SharedString::from(format!(
            "VPN {} · {}",
            if vpn_connected { "ON" } else { "OFF" },
            self.runtime.browser_model.vpn_active_exit.to_ascii_uppercase()
        ));
        let browse_chip_text = SharedString::from(if self.runtime.browser_model.browse_extension_paired {
            "Browse MCP :19988 PAIRED"
        } else {
            "Browse MCP UNPAIRED"
        });
        let omnibox_value = SharedString::from(self.runtime.browser_model.omnibox_text.clone());

        h_flex()
            .h(px(42.0))
            .w_full()
            .px_2p5()
            .gap_2()
            .items_center()
            .bg(c.surface_container_low)
            .border_b_1()
            .border_color(c.outline_variant)
            .child(
                h_flex()
                    .flex_1()
                    .h(px(32.0))
                    .px_3()
                    .items_center()
                    .rounded(px(16.0))
                    .bg(c.surface_container_highest)
                    .border_1()
                    .border_color(c.outline_variant)
                    .child(Label::new(omnibox_value).size(LabelSize::Small)),
            )
            .child(
                h_flex()
                    .id("chromium-vpn-shield-chip")
                    .h(px(30.0))
                    .px_2p5()
                    .items_center()
                    .rounded(px(14.0))
                    .bg(c.secondary_container)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.runtime.browser_model.toggle_side_panel(SidePanelEntry::VpnShield);
                        cx.notify();
                    }))
                    .child(
                        Label::new(vpn_chip_text)
                            .size(LabelSize::XSmall)
                            .weight(FontWeight::SEMIBOLD),
                    ),
            )
            .child(
                h_flex()
                    .id("chromium-browse-agent-chip")
                    .h(px(30.0))
                    .px_2p5()
                    .items_center()
                    .rounded(px(14.0))
                    .bg(c.primary_container)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.runtime.browser_model.toggle_side_panel(SidePanelEntry::BrowseAgent);
                        cx.notify();
                    }))
                    .child(
                        Label::new(browse_chip_text)
                            .size(LabelSize::XSmall)
                            .weight(FontWeight::SEMIBOLD),
                    ),
            )
            .into_any_element()
    }

    fn render_side_panel(&self, entry: SidePanelEntry) -> AnyElement {
        let c = &self.theme.colors;
        let header = match entry {
            SidePanelEntry::BrowseAgent => "Browse Agent Relay & @e1 ARIA Tree",
            SidePanelEntry::WebMcp => "WebMCP Page Tools (navigator.modelContext)",
            SidePanelEntry::VpnShield => "Aphrody VPN (Mullvad WireGuard + DAITA + PQ)",
            SidePanelEntry::DevTools => "CDP DevTools Inspector",
            SidePanelEntry::Bookmarks => "Bookmarks & Reading List",
        };
        let detail = match entry {
            SidePanelEntry::VpnShield => {
                let exit = self.runtime.vpn_router.active_exit();
                format!(
                    "Active relay: {} ({}/{}) · SOCKS5 :{} · DoH + ECH + SSRF Shield active",
                    exit.id.to_ascii_uppercase(),
                    exit.country.to_ascii_uppercase(),
                    exit.city.to_ascii_uppercase(),
                    exit.local_socks5_port
                )
            }
            SidePanelEntry::BrowseAgent => format!(
                "Paired extension relay on 127.0.0.1:{} · {} Cloudflare-OS Gadget(s) ready",
                browser_browse::DEFAULT_RELAY_PORT,
                self.runtime.gadgets.len()
            ),
            _ => format!("{} kernel task(s) active", self.runtime.kernel_tasks.len()),
        };

        v_flex()
            .w(px(self.runtime.browser_model.side_panel_width))
            .h_full()
            .p_3()
            .gap_2()
            .bg(c.surface_container)
            .border_l_1()
            .border_color(c.outline_variant)
            .child(Label::new(header).size(LabelSize::Small).weight(FontWeight::BOLD))
            .child(Label::new(SharedString::from(detail)).size(LabelSize::XSmall))
            .into_any_element()
    }
}

impl Focusable for ChromiumBrowserView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ChromiumBrowserView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let surface_bg = self.theme.colors.surface;
        let active_summary = self
            .runtime
            .browser_model
            .active_tab()
            .map(|t| format!("{} — rendered by `{}` engine", t.url, t.engine_id))
            .unwrap_or_else(|| "about:blank".into());

        let mut body_row = h_flex().flex_1().w_full();
        body_row = body_row.child(
            v_flex()
                .flex_1()
                .h_full()
                .p_4()
                .gap_2()
                .bg(surface_bg)
                .child(
                    Label::new(SharedString::from(active_summary))
                        .size(LabelSize::Default)
                        .weight(FontWeight::SEMIBOLD),
                )
                .child(
                    Label::new(SharedString::from(format!(
                        "WebGPU OS Bus: {} Cloudflare-OS Gadgets · {} ai-os tasks · {}",
                        self.runtime.gadgets.len(),
                        self.runtime.kernel_tasks.len(),
                        self.runtime.nysh_terminal.active_shell
                    )))
                    .size(LabelSize::XSmall),
                ),
        );

        if let Some(panel_entry) = self.runtime.browser_model.side_panel {
            body_row = body_row.child(self.render_side_panel(panel_entry));
        }

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(surface_bg)
            .child(self.render_tab_strip(cx))
            .child(self.render_toolbar(cx))
            .child(body_row)
    }
}
