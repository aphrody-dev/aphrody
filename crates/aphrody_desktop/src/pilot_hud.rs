use gpui::{AnyElement, SharedString, prelude::*, px};
use aphrody_pilot::HudActivityItem;
use aphrody_ui::{ImmediateUi, OsTheme, StatusTone, status_badge};
use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex};

/// Renders the compact GPUI-native AI Pilot status pill for the COSMIC Top Panel,
/// replacing legacy multi-platform GDI/X11/Cocoa overlay hacks with GPU compositor chrome.
pub(crate) fn render_pilot_status_pill(
    backend_name: &str,
    snapshot_count: usize,
    recent: &[HudActivityItem],
    compact: bool,
    theme: &OsTheme,
) -> AnyElement {
    let badge_text = if compact { "AI" } else { "AI Pilot" };
    let detail_text = recent
        .last()
        .map(|item| {
            let ref_str = item.target_ref.as_deref().unwrap_or("sys");
            if compact {
                format!("{} {}", item.command, ref_str)
            } else {
                format!("{} {} ({}ms)", item.command, ref_str, item.duration_ms)
            }
        })
        .unwrap_or_else(|| {
            if compact {
                format!("{snapshot_count}")
            } else {
                let short_be = backend_name.split('-').next().unwrap_or(backend_name);
                format!("{short_be} ({snapshot_count})")
            }
        });

    h_flex()
        .flex_shrink_0()
        .gap_1()
        .items_center()
        .px_1p5()
        .py_0p5()
        .rounded(px(8.0))
        .bg(theme.colors.surface_container)
        .border_1()
        .border_color(theme.colors.outline_variant)
        .child(status_badge(
            IconName::Sparkle,
            badge_text,
            StatusTone::Accent,
            theme,
        ))
        .child(
            Label::new(SharedString::from(detail_text))
                .size(LabelSize::XSmall)
                .color(ui::Color::Muted)
                .single_line(),
        )
        .into_any_element()
}

/// Appends AI Pilot telemetry, recent verified actions, and target backend details into the
/// COSMIC Inspector Context Drawer (`aphrody_desktop/src/overlays.rs`).
pub(crate) fn append_pilot_drawer_section(
    ui_builder: &mut ImmediateUi,
    backend_name: &str,
    recent: &[HudActivityItem],
) {
    ui_builder.heading("AI OS Pilot (aphrody_pilot + AccessKit)");
    ui_builder.kv_row("Backend", backend_name.to_owned());
    ui_builder.kv_row("Actions Executed", recent.len().to_string());
    if recent.is_empty() {
        ui_builder.kv_row("Last Step", "idle — awaiting MCP or CLI step");
    } else {
        for item in recent.iter().rev().take(4) {
            let key = item
                .target_ref
                .clone()
                .unwrap_or_else(|| item.command.clone());
            let val = format!(
                "{} · {} ({}ms)",
                item.command, item.disposition, item.duration_ms
            );
            ui_builder.kv_row(key, val);
        }
    }
}
