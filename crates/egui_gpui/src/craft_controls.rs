use egui::{CornerRadius, RichText, Stroke, Ui};
use os_ui::ShadcnGpuiTheme;

use crate::theme_bridge::hsla_to_color32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftDockTab {
    pub id: String,
    pub label: String,
    pub badge: Option<String>,
}

impl CraftDockTab {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            badge: None,
        }
    }

    pub fn with_badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

pub struct CraftDockTabs<'a> {
    tabs: &'a [CraftDockTab],
    active_id: &'a mut String,
}

impl<'a> CraftDockTabs<'a> {
    pub fn new(tabs: &'a [CraftDockTab], active_id: &'a mut String) -> Self {
        Self { tabs, active_id }
    }

    pub fn show(self, ui: &mut Ui, theme: &ShadcnGpuiTheme) -> Option<String> {
        let mut changed_to = None;
        let primary = hsla_to_color32(theme.primary);
        let on_primary = hsla_to_color32(theme.primary_foreground);
        let surface = hsla_to_color32(theme.popover);
        let on_surface = hsla_to_color32(theme.foreground);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for tab in self.tabs {
                let is_active = *self.active_id == tab.id;
                let text = match &tab.badge {
                    Some(badge) => format!("{} ({})", tab.label, badge),
                    None => tab.label.clone(),
                };
                let fill = if is_active { primary } else { surface };
                let fg = if is_active { on_primary } else { on_surface };
                let button = egui::Button::new(RichText::new(text).color(fg).strong())
                    .fill(fill)
                    .corner_radius(CornerRadius::same(99))
                    .stroke(Stroke::new(1.0, hsla_to_color32(theme.border)));
                if ui.add(button).clicked() && !is_active {
                    *self.active_id = tab.id.clone();
                    changed_to = Some(tab.id.clone());
                }
            }
        });
        changed_to
    }
}

pub struct CraftComboMenu<'a> {
    id_salt: &'a str,
    label: &'a str,
    options: &'a [String],
    selected_index: &'a mut usize,
}

impl<'a> CraftComboMenu<'a> {
    pub fn new(
        id_salt: &'a str,
        label: &'a str,
        options: &'a [String],
        selected_index: &'a mut usize,
    ) -> Self {
        Self {
            id_salt,
            label,
            options,
            selected_index,
        }
    }

    pub fn show(self, ui: &mut Ui) -> bool {
        let mut changed = false;
        let current_text = self
            .options
            .get(*self.selected_index)
            .map(String::as_str)
            .unwrap_or("—");
        egui::ComboBox::from_id_salt(self.id_salt)
            .selected_text(format!("{}: {}", self.label, current_text))
            .show_ui(ui, |ui| {
                for (index, option) in self.options.iter().enumerate() {
                    if ui
                        .selectable_value(self.selected_index, index, option)
                        .changed()
                    {
                        changed = true;
                    }
                }
            });
        changed
    }
}

pub struct CraftToolbarChip<'a> {
    label: &'a str,
    selected: bool,
}

impl<'a> CraftToolbarChip<'a> {
    pub fn new(label: &'a str, selected: bool) -> Self {
        Self { label, selected }
    }

    pub fn show(self, ui: &mut Ui, theme: &ShadcnGpuiTheme) -> bool {
        let fill = if self.selected {
            hsla_to_color32(theme.secondary)
        } else {
            hsla_to_color32(theme.muted)
        };
        let fg = if self.selected {
            hsla_to_color32(theme.secondary_foreground)
        } else {
            hsla_to_color32(theme.muted_foreground)
        };
        ui.add(
            egui::Button::new(RichText::new(self.label).color(fg).size(12.0))
                .fill(fill)
                .corner_radius(CornerRadius::same(99)),
        )
        .clicked()
    }
}

pub struct CraftPropertyRow<'a> {
    label: &'a str,
    value: &'a mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &'a str,
}

impl<'a> CraftPropertyRow<'a> {
    pub fn new(
        label: &'a str,
        value: &'a mut f32,
        range: std::ops::RangeInclusive<f32>,
        suffix: &'a str,
    ) -> Self {
        Self {
            label,
            value,
            range,
            suffix,
        }
    }

    pub fn show(self, ui: &mut Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(self.label);
            let drag = egui::DragValue::new(self.value)
                .range(self.range)
                .suffix(self.suffix)
                .speed(0.5);
            if ui.add(drag).changed() {
                changed = true;
            }
        });
        changed
    }
}

pub struct CraftImeBanner<'a> {
    preedit: &'a str,
}

impl<'a> CraftImeBanner<'a> {
    pub fn new(preedit: &'a str) -> Self {
        Self { preedit }
    }

    pub fn show(self, ui: &mut Ui, theme: &ShadcnGpuiTheme) {
        if self.preedit.is_empty() {
            return;
        }
        egui::Frame::new()
            .fill(hsla_to_color32(theme.accent))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(format!("IME: {}", self.preedit))
                        .color(hsla_to_color32(theme.accent_foreground))
                        .monospace(),
                );
            });
    }
}

#[cfg(test)]
mod tests {
    use os_ui::AppearanceMode;

    use super::*;
    use crate::host::EguiGpuiHost;

    #[test]
    fn renders_craft_controls_on_egui_037_with_google_design_tokens() -> anyhow::Result<()> {
        let mut host = EguiGpuiHost::new(AppearanceMode::Dark)?;
        let theme = ShadcnGpuiTheme::from_bundled(AppearanceMode::Dark)?;
        let tabs = vec![
            CraftDockTab::new("figcraft", "FigCraft").with_badge("Google"),
            CraftDockTab::new("unreal", "Unreal Bevy 5.8").with_badge("Nanite"),
        ];
        let mut active_tab = "figcraft".to_string();
        let options = vec![
            "Vulkan 1.3".to_string(),
            "DirectX 12".to_string(),
            "WebGPU".to_string(),
        ];
        let mut selected_backend = 0usize;
        let mut roughness = 0.35f32;

        let artifact = host.run_frame(640, 240, 1.0, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                CraftDockTabs::new(&tabs, &mut active_tab).show(ui, &theme);
                CraftComboMenu::new("gpu_backend", "Backend", &options, &mut selected_backend)
                    .show(ui);
                let _clicked = CraftToolbarChip::new("Split GPUI + egui Parity", true)
                    .show(ui, &theme);
                CraftPropertyRow::new("Roughness", &mut roughness, 0.0..=1.0, "").show(ui);
                CraftImeBanner::new("東京").show(ui, &theme);
            });
        });

        assert_eq!(artifact.stats.width_px, 640);
        assert_eq!(artifact.stats.height_px, 240);
        assert!(artifact.stats.draw_calls > 0);
        assert!(artifact.stats.vertices > 0);
        Ok(())
    }
}
