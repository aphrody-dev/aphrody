use gpui::{AnyElement, SharedString, prelude::*, px};
use aphrody_pilot::{HudActivityItem, RetryDisposition};
use aphrody_ui::{ImmediateUi, OsTheme, StatusTone, status_badge};
use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex};

/// Summarizes verification status and retry safety across recent HUD action records.
#[must_use]
pub(crate) fn pilot_hud_summary(recent: &[HudActivityItem]) -> (usize, usize, RetryDisposition) {
    let verified_count = recent
        .iter()
        .filter(|item| item.disposition.contains("DeliveredVerified") || item.disposition == "delivered_verified")
        .count();
    let last_retry = recent.last().map_or(RetryDisposition::Unknown, |item| {
        if item.disposition.contains("NotDelivered") || item.disposition == "not_delivered" {
            RetryDisposition::Safe
        } else if item.disposition.contains("Delivered") || item.disposition.contains("Uncertain") {
            RetryDisposition::Unsafe
        } else {
            RetryDisposition::Unknown
        }
    });
    (verified_count, recent.len(), last_retry)
}

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
    let (verified_count, total_count, last_retry) = pilot_hud_summary(recent);
    let tone = if recent.last().is_some_and(|item| !item.ok) {
        StatusTone::Warning
    } else {
        StatusTone::Accent
    };
    let detail_text = recent
        .last()
        .map(|item| {
            let ref_str = item.target_ref.as_deref().unwrap_or("sys");
            if compact {
                format!("{} {}", item.command, ref_str)
            } else {
                format!(
                    "{} {} ({}ms · {verified_count}/{total_count})",
                    item.command, ref_str, item.duration_ms
                )
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
        .id("cosmic-pilot-status-pill")
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
            tone,
            theme,
        ))
        .child(
            Label::new(SharedString::from(detail_text))
                .size(LabelSize::XSmall)
                .color(ui::Color::Muted)
                .single_line(),
        )
        .when(!compact && last_retry != RetryDisposition::Unknown, |row| {
            let retry_label = match last_retry {
                RetryDisposition::Safe => "retry:safe",
                RetryDisposition::Unsafe => "retry:unsafe",
                RetryDisposition::Unknown => "retry:?",
            };
            row.child(
                Label::new(SharedString::from(retry_label))
                    .size(LabelSize::XSmall)
                    .color(ui::Color::Muted)
                    .single_line(),
            )
        })
        .into_any_element()
}

/// Appends AI Pilot telemetry, recent verified actions, and target backend details into the
/// COSMIC Inspector Context Drawer (`aphrody_desktop/src/overlays.rs`).
pub(crate) fn append_pilot_drawer_section(
    ui_builder: &mut ImmediateUi,
    backend_name: &str,
    recent: &[HudActivityItem],
) {
    let (verified_count, total_count, last_retry) = pilot_hud_summary(recent);
    ui_builder.heading("AI OS Pilot (aphrody_pilot + agent-browser 0.43)");
    ui_builder.kv_row("Backend", backend_name.to_owned());
    ui_builder.kv_row(
        "Actions Verified",
        format!("{verified_count} / {total_count}"),
    );
    ui_builder.kv_row("Last Retry Safety", format!("{last_retry:?}"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pilot_hud_summary_counts_verified_and_derives_retry_safety() {
        let items = vec![
            HudActivityItem {
                command: "click".to_owned(),
                target_ref: Some("@s1:e1".to_owned()),
                ok: true,
                duration_ms: 8,
                disposition: "DeliveredVerified".to_owned(),
            },
            HudActivityItem {
                command: "type".to_owned(),
                target_ref: Some("@s1:e2".to_owned()),
                ok: false,
                duration_ms: 4,
                disposition: "NotDelivered".to_owned(),
            },
        ];
        let (verified, total, last_retry) = pilot_hud_summary(&items);
        assert_eq!(verified, 1);
        assert_eq!(total, 2);
        assert_eq!(last_retry, RetryDisposition::Safe);
    }
}
