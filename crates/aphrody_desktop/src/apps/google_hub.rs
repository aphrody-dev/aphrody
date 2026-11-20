//! Human GPUI desktop surfaces for Google Workspace + Official Google Cloud AI (`gcloud`/OAuth 2.0),
//! YouTube + Media Player (`aphrody_media`), and YouTube Music (`WEB_REMIX` + dynamic Material 3
//! Expressive HCT artwork palettes).

use aphrody_google::{GoogleAuthManager, sdk_parity_matrix, workspace::WORKSPACE_SERVICES};
use aphrody_media::{MediaPlayerState, capabilities};
use aphrody_ui::{KeyValueRow, OsTheme, StatusTone, status_badge};
use gpui::{AnyElement, App, FontWeight, SharedString, div, prelude::*, px};
use ui::{Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::apps::section_heading;

pub struct GoogleHubCallbacks {
    pub on_open_youtube: Box<dyn Fn(&mut App) + 'static>,
    pub on_open_ytmusic: Box<dyn Fn(&mut App) + 'static>,
    pub on_open_browser: Box<dyn Fn(&mut App) + 'static>,
}

pub struct MediaSurfaceCallbacks {
    pub on_toggle_play: Box<dyn Fn(&mut App) + 'static>,
    pub on_toggle_sponsorblock: Box<dyn Fn(&mut App) + 'static>,
    pub on_seek_relative_ms: Box<dyn Fn(i64, &mut App) + 'static>,
    pub on_apply_artwork_seed: Box<dyn Fn([u8; 3], &mut App) + 'static>,
    pub on_switch_app: Box<dyn Fn(&mut App) + 'static>,
}

/// Renders the native GPUI Google Workspace & Official Google Cloud AI control surface.
pub fn render_google_workspace_window(
    theme: &OsTheme,
    cb: GoogleHubCallbacks,
) -> AnyElement {
    let on_yt = std::rc::Rc::new(cb.on_open_youtube);
    let on_ytm = std::rc::Rc::new(cb.on_open_ytmusic);
    let on_browser = std::rc::Rc::new(cb.on_open_browser);

    let auth_status = GoogleAuthManager::default().status();
    let (auth_label, auth_tone) = if auth_status.authenticated {
        ("gcloud / OAuth 2.0 Active", StatusTone::Success)
    } else {
        ("Sign In via OAuth 2.0 / gcloud", StatusTone::Warning)
    };

    let mut services_grid = h_flex().w_full().flex_wrap().gap_2();
    for (idx, spec) in WORKSPACE_SERVICES.iter().enumerate() {
        services_grid = services_grid.child(
            div()
                .id(("gws-service", idx))
                .w(px(195.0))
                .p_2p5()
                .rounded(px(14.0))
                .border_1()
                .border_color(theme.colors.outline_variant.opacity(0.45))
                .bg(theme.colors.surface_container)
                .child(
                    v_flex()
                        .gap_0p5()
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    Label::new(spec.id.to_ascii_uppercase())
                                        .size(LabelSize::Small)
                                        .weight(FontWeight::SEMIBOLD),
                                )
                                .child(
                                    Label::new(spec.discovery_version)
                                        .size(LabelSize::XSmall)
                                        .color(ui::Color::Accent),
                                ),
                        )
                        .child(
                            Label::new(spec.base_url)
                                .size(LabelSize::XSmall)
                                .color(ui::Color::Muted),
                        ),
                ),
        );
    }

    let mut parity_rows = v_flex().w_full().gap_1p5();
    for (idx, entry) in sdk_parity_matrix().into_iter().enumerate() {
        parity_rows = parity_rows.child(
            div()
                .id(("google-sdk-parity", idx))
                .w_full()
                .p_2p5()
                .rounded(px(12.0))
                .bg(theme.colors.surface_container_low)
                .border_1()
                .border_color(theme.colors.outline_variant.opacity(0.35))
                .child(
                    v_flex()
                        .gap_0p5()
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    Label::new(entry.domain)
                                        .size(LabelSize::Small)
                                        .weight(FontWeight::SEMIBOLD),
                                )
                                .child(
                                    Label::new(entry.rust_module)
                                        .size(LabelSize::XSmall)
                                        .color(ui::Color::Accent),
                                ),
                        )
                        .child(
                            Label::new(format!(
                                "SDK Parity: {}",
                                entry.official_sdks.join(" · ")
                            ))
                            .size(LabelSize::XSmall)
                            .color(ui::Color::Muted),
                        ),
                ),
        );
    }

    v_flex()
        .id("google-workspace-scroll")
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
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            Icon::new(IconName::Sparkle)
                                .size(IconSize::Medium)
                                .color(ui::Color::Accent),
                        )
                        .child(
                            v_flex()
                                .child(
                                    Label::new("Google Workspace & Cloud AI (Pure Rust)")
                                        .size(LabelSize::Default)
                                        .weight(FontWeight::BOLD),
                                )
                                .child(
                                    Label::new(
                                        "OAuth 2.0 PKCE · gcloud ADC · Gemini / Vertex / Imagen / Veo / Lyria · Discovery APIs",
                                    )
                                    .size(LabelSize::XSmall)
                                    .color(ui::Color::Muted),
                                ),
                        ),
                )
                .child(status_badge(
                    IconName::Sparkle,
                    auth_label,
                    auth_tone,
                    theme,
                )),
        )
        .child(
            h_flex()
                .gap_2()
                .flex_wrap()
                .child({
                    let cb = std::rc::Rc::clone(&on_yt);
                    action_pill(theme, "gws-open-yt", "Open YouTube GPUI", true, move |cx| {
                        cb(cx)
                    })
                })
                .child({
                    let cb = std::rc::Rc::clone(&on_ytm);
                    action_pill(
                        theme,
                        "gws-open-ytm",
                        "Open YouTube Music",
                        false,
                        move |cx| cb(cx),
                    )
                })
                .child({
                    let cb = std::rc::Rc::clone(&on_browser);
                    action_pill(
                        theme,
                        "gws-open-browser",
                        "Open Chromium WebGPU",
                        false,
                        move |cx| cb(cx),
                    )
                }),
        )
        .child(section_heading(theme, "Authentication & GCP Context"))
        .child(
            v_flex()
                .w_full()
                .p_3()
                .gap_1p5()
                .rounded(px(14.0))
                .bg(theme.colors.surface_container_low)
                .border_1()
                .border_color(theme.colors.outline_variant.opacity(0.4))
                .child(KeyValueRow::new(
                    "Project",
                    auth_status
                        .project_id
                        .unwrap_or_else(|| "Dynamic (gcloud config)".to_string()),
                ))
                .child(KeyValueRow::new("Region", auth_status.region))
                .child(KeyValueRow::new(
                    "Scopes",
                    format!("{} configured (Cloud, AI, YouTube, Workspace)", auth_status.scopes_count),
                ))
                .child(KeyValueRow::new(
                    "Headless CLI",
                    "aphrody-cli google | ws | yt | ytm | media",
                )),
        )
        .child(section_heading(theme, "Google Workspace Discovery Services"))
        .child(services_grid)
        .child(section_heading(
            theme,
            "Pure-Rust Parity Across Google Kotlin / Go / Python / TS / Rust SDKs",
        ))
        .child(parity_rows)
        .into_any_element()
}

/// Renders the human GPUI YouTube & Universal Media Player window (`aphrody_media` + `aphrody_google`).
pub fn render_youtube_window(
    theme: &OsTheme,
    player: &MediaPlayerState,
    cb: MediaSurfaceCallbacks,
) -> AnyElement {
    let on_play = std::rc::Rc::new(cb.on_toggle_play);
    let on_sb = std::rc::Rc::new(cb.on_toggle_sponsorblock);
    let on_seek = std::rc::Rc::new(cb.on_seek_relative_ms);
    let on_switch = std::rc::Rc::new(cb.on_switch_app);
    let caps = capabilities();

    let title = if player.title.is_empty() {
        "Aphrody Pure-Rust YouTube & Media Player".to_string()
    } else {
        player.title.clone()
    };

    v_flex()
        .id("youtube-player-scroll")
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
                .child(
                    v_flex()
                        .child(
                            Label::new(title)
                                .size(LabelSize::Default)
                                .weight(FontWeight::BOLD),
                        )
                        .child(
                            Label::new("InnerTube WEB + Data API v3 · WGPU 30 YUV->RGB · Zero FFmpeg")
                                .size(LabelSize::XSmall)
                                .color(ui::Color::Muted),
                        ),
                )
                .child(status_badge(
                    IconName::PlayFilled,
                    if player.playing { "Playing" } else { "Paused" },
                    if player.playing {
                        StatusTone::Success
                    } else {
                        StatusTone::Accent
                    },
                    theme,
                )),
        )
        .child(
            div()
                .w_full()
                .h(px(210.0))
                .rounded(px(20.0))
                .border_1()
                .border_color(theme.colors.outline_variant.opacity(0.45))
                .bg(theme.colors.surface_container_lowest)
                .p_4()
                .child(
                    v_flex()
                        .size_full()
                        .justify_between()
                        .child(
                            h_flex()
                                .justify_between()
                                .child(status_badge(
                                    IconName::Sparkle,
                                    "WGSL YUV->RGB + AgX HDR",
                                    StatusTone::Accent,
                                    theme,
                                ))
                                .child(status_badge(
                                    IconName::VpnShield,
                                    if player.sponsorblock_enabled {
                                        "SponsorBlock ON"
                                    } else {
                                        "SponsorBlock OFF"
                                    },
                                    if player.sponsorblock_enabled {
                                        StatusTone::Success
                                    } else {
                                        StatusTone::Warning
                                    },
                                    theme,
                                )),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    Label::new(format!(
                                        "Position: {}s / {}s · Speed: {:.2}x · Volume: {}%",
                                        player.position_ms / 1000,
                                        player.duration_ms.max(180_000) / 1000,
                                        player.speed.0,
                                        (player.volume * 100.0).round() as u32
                                    ))
                                    .size(LabelSize::Small)
                                    .weight(FontWeight::MEDIUM),
                                )
                                .child(
                                    Label::new(format!(
                                        "Pure-Rust Codecs: {} · WGSL Shaders: {}",
                                        caps.supported_video_codecs.join(", "),
                                        caps.wgsl_shaders_count
                                    ))
                                    .size(LabelSize::XSmall)
                                    .color(ui::Color::Muted),
                                ),
                        ),
                ),
        )
        .child(
            h_flex()
                .gap_2()
                .flex_wrap()
                .child({
                    let cb = std::rc::Rc::clone(&on_play);
                    action_pill(
                        theme,
                        "yt-play-toggle",
                        if player.playing { "Pause" } else { "Play" },
                        true,
                        move |cx| cb(cx),
                    )
                })
                .child({
                    let cb = std::rc::Rc::clone(&on_seek);
                    action_pill(theme, "yt-seek-back", "-10s", false, move |cx| {
                        cb(-10_000, cx)
                    })
                })
                .child({
                    let cb = std::rc::Rc::clone(&on_seek);
                    action_pill(theme, "yt-seek-fwd", "+10s", false, move |cx| {
                        cb(10_000, cx)
                    })
                })
                .child({
                    let cb = std::rc::Rc::clone(&on_sb);
                    action_pill(
                        theme,
                        "yt-sponsorblock",
                        "Toggle SponsorBlock",
                        false,
                        move |cx| cb(cx),
                    )
                })
                .child({
                    let cb = std::rc::Rc::clone(&on_switch);
                    action_pill(
                        theme,
                        "yt-to-music",
                        "Switch to YouTube Music",
                        false,
                        move |cx| cb(cx),
                    )
                }),
        )
        .child(section_heading(theme, "Headless Agent & Watch FTS5 Parity"))
        .child(
            v_flex()
                .w_full()
                .p_3()
                .gap_1p5()
                .rounded(px(14.0))
                .bg(theme.colors.surface_container_low)
                .child(KeyValueRow::new("Search CLI", "aphrody-cli yt search \"<query>\""))
                .child(KeyValueRow::new("Player Streams", "aphrody-cli yt info <video_id>"))
                .child(KeyValueRow::new("Gemini Multimodal", "aphrody-cli google ai --youtube-url <url> \"Summarize\"")),
        )
        .into_any_element()
}

/// Renders the human GPUI YouTube Music surface (`WEB_REMIX`) with live Material 3 Expressive
/// artwork seed presets that dynamically re-theme the desktop shell via `material-colors 0.5`.
pub fn render_youtube_music_window(
    theme: &OsTheme,
    player: &MediaPlayerState,
    cb: MediaSurfaceCallbacks,
) -> AnyElement {
    let on_play = std::rc::Rc::new(cb.on_toggle_play);
    let on_seed = std::rc::Rc::new(cb.on_apply_artwork_seed);
    let on_switch = std::rc::Rc::new(cb.on_switch_app);

    let artwork_presets: &[(&str, &str, [u8; 3])] = &[
        ("Crimson Vinyl", "YouTube Red Seed", [0xFF, 0x00, 0x33]),
        ("Midnight Synth", "Gemini Violet Seed", [0x9B, 0x72, 0xCB]),
        ("Ocean Lo-Fi", "Material Cyan Seed", [0x13, 0x5B, 0x82]),
        ("Emerald Acoustic", "Forest Green Seed", [0x2E, 0x7D, 0x32]),
    ];

    let mut palette_row = h_flex().w_full().flex_wrap().gap_2();
    for (idx, (title, subtitle, rgb)) in artwork_presets.iter().enumerate() {
        let cb_seed = std::rc::Rc::clone(&on_seed);
        let seed_rgb = *rgb;
        palette_row = palette_row.child(
            div()
                .id(("ytm-artwork-seed", idx))
                .cursor_pointer()
                .w(px(185.0))
                .p_2p5()
                .rounded(px(14.0))
                .border_1()
                .border_color(theme.colors.outline_variant.opacity(0.45))
                .bg(theme.colors.surface_container)
                .on_click(move |_ev, _w, cx| cb_seed(seed_rgb, cx))
                .child(
                    v_flex()
                        .gap_0p5()
                        .child(
                            Label::new(*title)
                                .size(LabelSize::Small)
                                .weight(FontWeight::SEMIBOLD),
                        )
                        .child(
                            Label::new(*subtitle)
                                .size(LabelSize::XSmall)
                                .color(ui::Color::Muted),
                        ),
                ),
        );
    }

    v_flex()
        .id("ytmusic-scroll")
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
                .child(
                    v_flex()
                        .child(
                            Label::new("YouTube Music (Pure-Rust GPUI)")
                                .size(LabelSize::Default)
                                .weight(FontWeight::BOLD),
                        )
                        .child(
                            Label::new(
                                "InnerTube WEB_REMIX · Gapless Opus/AAC · Synced Lyrics · Dynamic HCT Palette",
                            )
                            .size(LabelSize::XSmall)
                            .color(ui::Color::Muted),
                        ),
                )
                .child(status_badge(
                    IconName::AudioOn,
                    if player.playing {
                        "Playing Opus 251"
                    } else {
                        "Ready"
                    },
                    StatusTone::Success,
                    theme,
                )),
        )
        .child(
            h_flex()
                .gap_2()
                .flex_wrap()
                .child({
                    let cb = std::rc::Rc::clone(&on_play);
                    action_pill(
                        theme,
                        "ytm-play-toggle",
                        if player.playing { "Pause Track" } else { "Play Queue" },
                        true,
                        move |cx| cb(cx),
                    )
                })
                .child({
                    let cb = std::rc::Rc::clone(&on_switch);
                    action_pill(
                        theme,
                        "ytm-to-yt",
                        "Open YouTube Video",
                        false,
                        move |cx| cb(cx),
                    )
                }),
        )
        .child(section_heading(
            theme,
            "Dynamic Artwork Material 3 Expressive Theme Seeds",
        ))
        .child(palette_row)
        .child(section_heading(theme, "Headless Agent Control"))
        .child(
            v_flex()
                .w_full()
                .p_3()
                .gap_1p5()
                .rounded(px(14.0))
                .bg(theme.colors.surface_container_low)
                .child(KeyValueRow::new(
                    "Search Songs",
                    "aphrody-cli ytm search \"<query>\" --filter songs",
                ))
                .child(KeyValueRow::new("Home Feed", "aphrody-cli ytm home"))
                .child(KeyValueRow::new("Synced Lyrics", "aphrody-cli ytm lyrics <video_id>")),
        )
        .into_any_element()
}

fn action_pill(
    theme: &OsTheme,
    id: &'static str,
    label: impl Into<SharedString>,
    primary: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> AnyElement {
    let label = label.into();
    div()
        .id(id)
        .cursor_pointer()
        .px_3()
        .py_1p5()
        .rounded(px(999.0))
        .border_1()
        .border_color(if primary {
            theme.colors.primary
        } else {
            theme.colors.outline_variant.opacity(0.55)
        })
        .bg(if primary {
            theme.colors.primary_container
        } else {
            theme.colors.surface_container_high
        })
        .on_click(move |_ev, _w, cx| on_click(cx))
        .child(
            Label::new(label)
                .size(LabelSize::Small)
                .weight(FontWeight::MEDIUM),
        )
        .into_any_element()
}
