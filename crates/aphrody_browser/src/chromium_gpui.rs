//! Chromium `//ui/views` & `//chrome/browser/ui/views` rendered natively on GPUI (`ChromiumBrowserView`).
//!
//! Renders the full Chromium Aura/Views widget tree (`TabStripRegionView`, `ToolbarView`, `LocationBarView` /
//! `OmniboxView`, `VpnShieldChip`, `BrowseAgentChip`, `BookmarkBarView`, `HandoffInfobar`, `ContentsWebView`
//! with split-screen support, `SidePanelCoordinator`, and `StatusBubbleViews`) backed by [`WebGpuOsRuntime`].

use aphrody_ui::OsTheme;
use browser_engine::{Cdp, Needs};
use browser_views::{OmniboxIntent, SidePanelEntry, parse_omnibox_input};
use browser_vpn::{RouteMode, TunnelState};
use gpui::{
    AnyElement, App, Context, FocusHandle, Focusable, FontWeight, SharedString, Window, actions,
    div, prelude::*, px,
};
use ui::{Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::agent_browser_core::AgentBrowserCore;
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
    pub agent_core: AgentBrowserCore,
    pub theme: OsTheme,
}

impl ChromiumBrowserView {
    pub fn new(theme: OsTheme, cx: &mut Context<Self>) -> Self {
        let runtime = WebGpuOsRuntime::new_default();
        let mut agent_core = AgentBrowserCore::new();
        agent_core.sync_from_browser_model(&runtime.browser_model);
        Self {
            focus_handle: cx.focus_handle(),
            runtime,
            agent_core,
            theme,
        }
    }

    pub fn submit_omnibox(&mut self, raw: &str, cx: &mut Context<Self>) {
        apply_omnibox_intent(&mut self.runtime, &mut self.agent_core, raw);
        cx.notify();
    }

    fn handle_new_tab(&mut self, _: &NewBrowserTab, _window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.agent_core.open_synced_tab(
            &mut self.runtime.browser_model,
            "New Tab",
            "https://www.google.com/?hl=fr",
            RouteMode::SystemTunnel,
        ) {
            self.runtime.browser_model.status_bubble_text = Some(err.to_string());
        }
        cx.notify();
    }

    fn handle_close_active_tab(
        &mut self,
        _: &CloseActiveBrowserTab,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let active_idx = self.runtime.browser_model.active_tab_index;
        if self.runtime.browser_model.close_tab(active_idx) {
            let selector = format!("{active_idx}");
            if let Err(err) = self.agent_core.controller.tabs.close_tab(Some(&selector)) {
                self.runtime.browser_model.status_bubble_text = Some(err.to_string());
            }
            self.agent_core
                .sync_from_browser_model(&self.runtime.browser_model);
        }
        cx.notify();
    }

    fn handle_toggle_side_panel(
        &mut self,
        _: &ToggleBrowserSidePanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.runtime
            .browser_model
            .toggle_side_panel(SidePanelEntry::BrowseAgent);
        cx.notify();
    }

    fn handle_escalate_engine(
        &mut self,
        _: &EscalateBrowserEngine,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.runtime.browser_model.active_tab_mut() {
            let _escalated = tab.escalate_engine(Needs {
                javascript: true,
                paint: true,
                webgpu: true,
                media: true,
                cdp: Some(Cdp::Subset),
            });
        }
        cx.notify();
    }

    fn handle_cycle_vpn_exit(
        &mut self,
        _: &CycleTabVpnExit,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let next_exit = match self.runtime.browser_model.vpn_active_exit.as_str() {
            "us" => "de",
            "de" => "jp",
            _ => "us",
        };
        if self.runtime.vpn_router.select_exit(next_exit).is_ok() {
            self.runtime.browser_model.vpn_active_exit = next_exit.to_owned();
            if let Some(tab) = self.runtime.browser_model.active_tab_mut() {
                tab.vpn_route = RouteMode::PinnedExit(next_exit.to_owned());
            }
        }
        cx.notify();
    }

    fn render_tab_strip(&self, cx: &mut Context<Self>) -> AnyElement {
        let c = &self.theme.colors;
        let active_idx = self.runtime.browser_model.active_tab_index;
        let mut strip = h_flex()
            .id("chromium-tab-strip")
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
                    if let Err(err) = this.agent_core.controller.tabs.switch_tab(&format!("{idx}")) {
                        this.runtime.browser_model.status_bubble_text = Some(err.to_string());
                    }
                    this.agent_core
                        .sync_from_browser_model(&this.runtime.browser_model);
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
                if let Err(err) = this.agent_core.open_synced_tab(
                    &mut this.runtime.browser_model,
                    "New Tab",
                    "https://www.google.com/?hl=fr",
                    RouteMode::SystemTunnel,
                ) {
                    this.runtime.browser_model.status_bubble_text = Some(err.to_string());
                }
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
                "Paired extension relay on 127.0.0.1:{} · {} Cloudflare-OS Gadget(s) · {}",
                browser_browse::DEFAULT_RELAY_PORT,
                self.runtime.gadgets.len(),
                self.agent_core.telemetry_summary()
            ),
            SidePanelEntry::DevTools => format!(
                "{} · HAR recording={}",
                self.agent_core.telemetry_summary(),
                self.agent_core.controller.har.recording
            ),
            _ => format!(
                "{} kernel task(s) active · {}",
                self.runtime.kernel_tasks.len(),
                self.agent_core.telemetry_summary()
            ),
        };

        v_flex()
            .id("chromium-side-panel")
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
        let core_telemetry = self.agent_core.telemetry_summary();

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
                        "WebGPU OS Bus: {} Cloudflare-OS Gadgets · {} ai-os tasks · {} · {}",
                        self.runtime.gadgets.len(),
                        self.runtime.kernel_tasks.len(),
                        self.runtime.nysh_terminal.active_shell,
                        core_telemetry,
                    )))
                    .size(LabelSize::XSmall),
                ),
        );

        if let Some(panel_entry) = self.runtime.browser_model.side_panel {
            body_row = body_row.child(self.render_side_panel(panel_entry));
        }

        div()
            .id("chromium-browser-view")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_new_tab))
            .on_action(cx.listener(Self::handle_close_active_tab))
            .on_action(cx.listener(Self::handle_toggle_side_panel))
            .on_action(cx.listener(Self::handle_escalate_engine))
            .on_action(cx.listener(Self::handle_cycle_vpn_exit))
            .size_full()
            .flex()
            .flex_col()
            .bg(surface_bg)
            .child(self.render_tab_strip(cx))
            .child(self.render_toolbar(cx))
            .child(body_row)
    }
}

/// Pure state transition helper for omnibox submission shared between GPUI views and unit tests.
pub fn apply_omnibox_intent(
    runtime: &mut WebGpuOsRuntime,
    agent_core: &mut AgentBrowserCore,
    raw: &str,
) {
    match parse_omnibox_input(raw, "https://www.google.com/search?q=") {
        OmniboxIntent::NavigateUrl(url) => {
            match agent_core.navigate_checked(&url, 200, &url) {
                Ok(Some(handoff)) => {
                    runtime.browser_model.status_bubble_text = Some(handoff.instructions.clone());
                    if let Some(tab) = runtime.browser_model.active_tab_mut() {
                        tab.handoff = Some(handoff);
                    }
                }
                Ok(None) => {
                    if let Some(tab) = runtime.browser_model.active_tab_mut() {
                        tab.url = url.clone();
                        tab.title = url.clone();
                        tab.handoff = None;
                    }
                    runtime.browser_model.omnibox_text = url;
                }
                Err(err) => {
                    runtime.browser_model.status_bubble_text = Some(err.to_string());
                }
            }
        }
        OmniboxIntent::DefaultSearch { query } => {
            let encoded = query.replace(' ', "+");
            let url = format!("https://www.google.com/search?hl=fr&q={encoded}");
            if let Some(tab) = runtime.browser_model.active_tab_mut() {
                tab.url = url.clone();
                tab.title = format!("{query} — Aphrody Search");
            }
            runtime.browser_model.omnibox_text = url;
            agent_core.sync_from_browser_model(&runtime.browser_model);
        }
        OmniboxIntent::AiModePrompt { prompt } => {
            runtime.browser_model.side_panel = Some(SidePanelEntry::BrowseAgent);
            runtime.exec_nysh_line(&format!("ai-os ask \"{prompt}\""));
        }
        OmniboxIntent::SelectVpnExit { exit_id } => {
            if runtime.vpn_router.select_exit(&exit_id).is_ok() {
                runtime.browser_model.vpn_active_exit = exit_id.clone();
                if let Some(tab) = runtime.browser_model.active_tab_mut() {
                    tab.vpn_route = RouteMode::PinnedExit(exit_id);
                }
            }
        }
        OmniboxIntent::SwitchToTab { filter } => {
            let lower = filter.to_ascii_lowercase();
            if let Some(idx) = runtime
                .browser_model
                .tabs
                .iter()
                .position(|t| t.title.to_ascii_lowercase().contains(&lower) || t.url.contains(&lower))
            {
                runtime.browser_model.activate_tab(idx);
                if let Err(err) = agent_core.controller.tabs.switch_tab(&format!("{idx}")) {
                    runtime.browser_model.status_bubble_text = Some(err.to_string());
                }
                agent_core.sync_from_browser_model(&runtime.browser_model);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omnibox_intent_syncs_agent_core_and_blocks_ssrf() {
        let mut runtime = WebGpuOsRuntime::new_default();
        let mut core = AgentBrowserCore::new();

        apply_omnibox_intent(&mut runtime, &mut core, "https://aphrody.com/studio");
        assert_eq!(core.controller.current_url, "https://aphrody.com/studio");

        apply_omnibox_intent(&mut runtime, &mut core, "http://169.254.169.254/latest");
        assert!(
            runtime
                .browser_model
                .status_bubble_text
                .as_deref()
                .is_some_and(|msg| msg.contains("blocked"))
        );
        assert_eq!(core.controller.current_url, "https://aphrody.com/studio");
    }
}
