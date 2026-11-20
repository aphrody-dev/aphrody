//! Native WireGuard VPN control surface for `aphrody_desktop`, synchronized with
//! `ChromiumBrowserView`'s per-tab Mullvad WireGuard exit router and `aphrody_vpn` Allow-LAN policy.

use aphrody_ui::{KeyValueRow, OsTheme, StatusTone, status_badge};
use gpui::{AnyElement, App, FontWeight, SharedString, div, prelude::*, px};
use ui::{Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::apps::section_heading;

/// Curated WireGuard exit relays shared between the OS VPN app and the native Browser VPN shield.
pub const VPN_EXIT_RELAYS: &[(&str, &str, &str)] = &[
    ("fr", "France · Paris", "10.64.0.12 · 11 ms"),
    ("de", "Germany · Frankfurt", "10.64.0.28 · 16 ms"),
    ("ch", "Switzerland · Zurich", "10.64.0.41 · 19 ms"),
    ("se", "Sweden · Stockholm", "10.64.0.64 · 27 ms"),
    ("us", "USA · New York", "10.64.0.88 · 74 ms"),
    ("jp", "Japan · Tokyo", "10.64.0.95 · 168 ms"),
];

pub struct VpnPanelState<'a> {
    pub connected: bool,
    pub active_exit: &'a str,
    pub allow_lan: bool,
    pub kill_switch: bool,
    pub post_quantum: bool,
    pub daita_enabled: bool,
}

pub struct VpnPanelCallbacks {
    pub on_toggle_connect: Box<dyn Fn(&mut App) + 'static>,
    pub on_select_exit: Box<dyn Fn(&str, &mut App) + 'static>,
    pub on_toggle_allow_lan: Box<dyn Fn(&mut App) + 'static>,
    pub on_toggle_kill_switch: Box<dyn Fn(&mut App) + 'static>,
    pub on_open_browser: Box<dyn Fn(&mut App) + 'static>,
}

pub fn render_vpn_window(
    theme: &OsTheme,
    state: &VpnPanelState<'_>,
    cb: VpnPanelCallbacks,
) -> AnyElement {
    let on_connect = std::rc::Rc::new(cb.on_toggle_connect);
    let on_exit = std::rc::Rc::new(cb.on_select_exit);
    let on_lan = std::rc::Rc::new(cb.on_toggle_allow_lan);
    let on_kill = std::rc::Rc::new(cb.on_toggle_kill_switch);
    let on_browser = std::rc::Rc::new(cb.on_open_browser);

    let (status_label, status_tone) = if state.connected {
        ("WireGuard Connected", StatusTone::Success)
    } else {
        ("Disconnected", StatusTone::Warning)
    };

    let active_relay_meta = VPN_EXIT_RELAYS
        .iter()
        .find(|(code, _, _)| *code == state.active_exit)
        .copied()
        .unwrap_or(VPN_EXIT_RELAYS[0]);

    let mut relay_grid = h_flex().w_full().flex_wrap().gap_2();
    for (idx, (code, city, endpoint)) in VPN_EXIT_RELAYS.iter().enumerate() {
        let selected = *code == state.active_exit;
        let code_owned = (*code).to_owned();
        let cb_exit = std::rc::Rc::clone(&on_exit);
        relay_grid = relay_grid.child(
            div()
                .id(("vpn-exit-relay", idx))
                .cursor_pointer()
                .w(px(210.0))
                .p_2p5()
                .rounded(px(12.0))
                .border_1()
                .border_color(if selected {
                    theme.colors.primary
                } else {
                    theme.colors.outline_variant.opacity(0.45)
                })
                .bg(if selected {
                    theme.colors.primary_container
                } else {
                    theme.colors.surface_container
                })
                .on_click(move |_ev, _w, cx| cb_exit(&code_owned, cx))
                .child(
                    v_flex()
                        .gap_0p5()
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    Label::new(*city)
                                        .size(LabelSize::Small)
                                        .weight(FontWeight::SEMIBOLD),
                                )
                                .child(
                                    Label::new(code.to_ascii_uppercase())
                                        .size(LabelSize::XSmall)
                                        .color(ui::Color::Accent),
                                ),
                        )
                        .child(
                            Label::new(*endpoint)
                                .size(LabelSize::XSmall)
                                .color(ui::Color::Muted),
                        ),
                ),
        );
    }

    v_flex()
        .id("vpn-window-scroll")
        .size_full()
        .p_4()
        .gap_3()
        .overflow_y_scroll()
        .bg(theme.colors.surface)
        .child(
            h_flex()
                .w_full()
                .justify_between()
                .items_center()
                .flex_wrap()
                .gap_2()
                .p_3()
                .rounded(px(14.0))
                .border_1()
                .border_color(theme.colors.outline_variant.opacity(0.5))
                .bg(theme.colors.surface_container_high)
                .child(
                    h_flex()
                        .gap_2p5()
                        .items_center()
                        .child(
                            Icon::new(IconName::VpnShield)
                                .size(IconSize::Medium)
                                .color(if state.connected {
                                    ui::Color::Success
                                } else {
                                    ui::Color::Muted
                                }),
                        )
                        .child(
                            v_flex()
                                .gap_0p5()
                                .child(
                                    Label::new(format!("{} WireGuard Tunnel", brand::APP_NAME))
                                        .size(LabelSize::Default)
                                        .weight(FontWeight::SEMIBOLD),
                                )
                                .child(
                                    Label::new(format!(
                                        "{} · {}",
                                        active_relay_meta.1, active_relay_meta.2
                                    ))
                                    .size(LabelSize::XSmall)
                                    .color(ui::Color::Muted),
                                ),
                        ),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(status_badge(
                            IconName::VpnShield,
                            status_label,
                            status_tone,
                            theme,
                        ))
                        .child(
                            div()
                                .id("vpn-connect-toggle")
                                .cursor_pointer()
                                .px_3()
                                .py_1p5()
                                .rounded_full()
                                .bg(if state.connected {
                                    theme.colors.error_container
                                } else {
                                    theme.colors.primary
                                })
                                .on_click(move |_ev, _w, cx| on_connect(cx))
                                .child(
                                    Label::new(if state.connected {
                                        "Disconnect"
                                    } else {
                                        "Connect Tunnel"
                                    })
                                    .size(LabelSize::Small)
                                    .color(ui::Color::Custom(if state.connected {
                                        theme.colors.on_error_container
                                    } else {
                                        theme.colors.on_primary
                                    })),
                                ),
                        ),
                ),
        )
        .child(section_heading(theme, "Mullvad WireGuard Exit Relays (Synced with Browser Per-Tab Shield)"))
        .child(relay_grid)
        .child(section_heading(theme, "Tunnel Security & LAN Routing"))
        .child(
            h_flex()
                .w_full()
                .flex_wrap()
                .gap_2()
                .child(
                    div()
                        .id("vpn-toggle-allow-lan")
                        .cursor_pointer()
                        .px_3()
                        .py_1p5()
                        .rounded_full()
                        .border_1()
                        .border_color(if state.allow_lan {
                            theme.colors.primary
                        } else {
                            theme.colors.outline_variant
                        })
                        .bg(if state.allow_lan {
                            theme.colors.primary_container
                        } else {
                            theme.colors.surface_container
                        })
                        .on_click(move |_ev, _w, cx| on_lan(cx))
                        .child(
                            Label::new(format!(
                                "Allow LAN (10.200.77.0/24): {}",
                                if state.allow_lan { "ON" } else { "OFF" }
                            ))
                            .size(LabelSize::XSmall),
                        ),
                )
                .child(
                    div()
                        .id("vpn-toggle-kill-switch")
                        .cursor_pointer()
                        .px_3()
                        .py_1p5()
                        .rounded_full()
                        .border_1()
                        .border_color(if state.kill_switch {
                            theme.colors.primary
                        } else {
                            theme.colors.outline_variant
                        })
                        .bg(if state.kill_switch {
                            theme.colors.secondary_container
                        } else {
                            theme.colors.surface_container
                        })
                        .on_click(move |_ev, _w, cx| on_kill(cx))
                        .child(
                            Label::new(format!(
                                "Lockdown Kill Switch: {}",
                                if state.kill_switch { "ON" } else { "OFF" }
                            ))
                            .size(LabelSize::XSmall),
                        ),
                )
                .child(
                    div()
                        .id("vpn-open-browser-shield")
                        .cursor_pointer()
                        .px_3()
                        .py_1p5()
                        .rounded_full()
                        .bg(theme.colors.surface_container_high)
                        .border_1()
                        .border_color(theme.colors.outline_variant)
                        .on_click(move |_ev, _w, cx| on_browser(cx))
                        .child(
                            Label::new(SharedString::from("Open Protected Browser"))
                                .size(LabelSize::XSmall),
                        ),
                ),
        )
        .child(
            v_flex()
                .gap_1()
                .p_3()
                .rounded(px(12.0))
                .bg(theme.colors.surface_container_low)
                .border_1()
                .border_color(theme.colors.outline_variant.opacity(0.4))
                .child(KeyValueRow::new(
                    "Post-Quantum Key Exchange (ML-KEM)",
                    if state.post_quantum { "Active" } else { "Disabled" },
                ))
                .child(KeyValueRow::new(
                    "DAITA Traffic Analysis Defense",
                    if state.daita_enabled { "Active" } else { "Standby" },
                ))
                .child(KeyValueRow::new(
                    "Backend Crate",
                    "aphrody_vpn + mullvad-daemon (v2026.11.20-aphrody.3)",
                )),
        )
        .into_any_element()
}
