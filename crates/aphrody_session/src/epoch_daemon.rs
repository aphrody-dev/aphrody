//! COSMIC Epoch & Pop!_OS system daemon state (`cosmic-settings-daemon`, `cosmic-randr`,
//! `cosmic-osd`, `cosmic-notifications`, `system76-power`, and `pop-launcher` provider).
//!
//! Pure-Rust and toolkit-agnostic so both native GPUI (`aphrody_desktop`), AI pilot (`aphrody_pilot`),
//! and Wasm32 builds share one deterministic state model.

use serde::{Deserialize, Serialize};

use crate::discovery::AppEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerProfile {
    Battery,
    #[default]
    Balanced,
    Performance,
}

impl PowerProfile {
    pub const ALL: [Self; 3] = [Self::Battery, Self::Balanced, Self::Performance];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Battery => "Battery Life",
            Self::Balanced => "Balanced",
            Self::Performance => "High Performance",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Battery => Self::Balanced,
            Self::Balanced => Self::Performance,
            Self::Performance => Self::Battery,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayOutputState {
    pub connector: String,
    pub make_model: String,
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
    pub scale: f32,
    pub primary: bool,
    pub vrr_enabled: bool,
}

impl Default for DisplayOutputState {
    fn default() -> Self {
        Self {
            connector: "DP-1".to_owned(),
            make_model: "Primary Studio Display".to_owned(),
            width: 1440,
            height: 900,
            refresh_hz: 120,
            scale: 1.0,
            primary: true,
            vrr_enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CosmicSettingsDaemon {
    pub volume_percent: u8,
    pub muted: bool,
    pub brightness_percent: u8,
    pub night_light_enabled: bool,
    pub night_light_kelvin: u16,
    pub power_profile: PowerProfile,
    pub system76_scheduler: bool,
    pub idle_lock_minutes: u16,
    pub panel_top_enabled: bool,
    pub dock_enabled: bool,
    pub dock_extend: bool,
    pub active_hint: bool,
    pub active_de_env: String,
    pub outputs: Vec<DisplayOutputState>,
}

impl Default for CosmicSettingsDaemon {
    fn default() -> Self {
        Self {
            volume_percent: 68,
            muted: false,
            brightness_percent: 85,
            night_light_enabled: false,
            night_light_kelvin: 4200,
            power_profile: PowerProfile::Balanced,
            system76_scheduler: true,
            idle_lock_minutes: 10,
            panel_top_enabled: true,
            dock_enabled: true,
            dock_extend: false,
            active_hint: true,
            active_de_env: "aphrody-gpui".to_owned(),
            outputs: vec![DisplayOutputState::default()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OsdKind {
    Volume,
    Brightness,
    PowerProfile,
    TilingLayout,
    Workspace,
    Screenshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsdBanner {
    pub kind: OsdKind,
    pub title: String,
    pub subtitle: String,
    pub level_percent: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationUrgency {
    Low,
    Normal,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopNotification {
    pub id: u64,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub urgency: NotificationUrgency,
}

#[derive(Debug, Clone, Default)]
pub struct NotificationCenter {
    pub do_not_disturb: bool,
    pub items: Vec<DesktopNotification>,
    next_id: u64,
}

impl NotificationCenter {
    #[must_use]
    pub fn with_welcome() -> Self {
        let mut center = Self::default();
        center.push(
            "COSMIC Epoch",
            "Aphrody Desktop Ready",
            "Pop Shell tiling, stack tabs, and DE Switcher active.",
            NotificationUrgency::Low,
        );
        center
    }

    pub fn push(
        &mut self,
        app_name: impl Into<String>,
        summary: impl Into<String>,
        body: impl Into<String>,
        urgency: NotificationUrgency,
    ) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.items.insert(
            0,
            DesktopNotification {
                id,
                app_name: app_name.into(),
                summary: summary.into(),
                body: body.into(),
                urgency,
            },
        );
        if self.items.len() > 16 {
            self.items.truncate(16);
        }
        id
    }

    pub fn dismiss(&mut self, id: u64) {
        self.items.retain(|n| n.id != id);
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LauncherActionKind {
    CalculatorResult(String),
    CompositorCommand(String),
    ShellCommand(String),
    CraftApp(String),
    DesktopApp(String),
    FilePath(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LauncherResultItem {
    pub title: String,
    pub subtitle: String,
    pub badge: String,
    pub action: LauncherActionKind,
}

/// Evaluates `pop-launcher` prefixes (`= math`, `: cmd`, `sh cmd`, `/ path`) plus fuzzy search.
#[must_use]
pub fn query_pop_launcher(
    raw_query: &str,
    craft_apps: &[(String, String, String)],
    system_apps: &[AppEntry],
) -> Vec<LauncherResultItem> {
    let trimmed = raw_query.trim();
    if let Some(expr) = trimmed.strip_prefix('=') {
        let mut out = Vec::new();
        if let Some(val) = eval_math_expr(expr) {
            let formatted = format_math_number(val);
            out.push(LauncherResultItem {
                title: formatted.clone(),
                subtitle: format!("Pop Calculator: {}", expr.trim()),
                badge: "= CALC".to_owned(),
                action: LauncherActionKind::CalculatorResult(formatted),
            });
        } else {
            out.push(LauncherResultItem {
                title: "Type expression, e.g. = 1440 / 3 + 48".to_owned(),
                subtitle: "Supports +, -, *, /, parentheses, decimals".to_owned(),
                badge: "= CALC".to_owned(),
                action: LauncherActionKind::CalculatorResult(String::new()),
            });
        }
        return out;
    }

    if let Some(cmd_raw) = trimmed.strip_prefix("sh ") {
        let cmd = cmd_raw.trim();
        return vec![LauncherResultItem {
            title: if cmd.is_empty() {
                "Run in COSMIC Term".to_owned()
            } else {
                cmd.to_owned()
            },
            subtitle: "Execute command inside integrated COSMIC Terminal".to_owned(),
            badge: "SHELL".to_owned(),
            action: LauncherActionKind::ShellCommand(cmd.to_owned()),
        }];
    }

    if let Some(path_raw) = trimmed.strip_prefix('/') {
        let path_val = format!("/{path_raw}");
        return vec![LauncherResultItem {
            title: path_val.clone(),
            subtitle: "Open path inside COSMIC Files".to_owned(),
            badge: "FILES".to_owned(),
            action: LauncherActionKind::FilePath(path_val),
        }];
    }

    let built_in_cmds = [
        (":overview", "Toggle COSMIC Workspaces Overview", "WM"),
        (":stack", "Toggle Pop Shell Window Stacking", "WM"),
        (":tile", "Cycle Auto-Tiling Layout", "WM"),
        (":float", "Toggle Focused Window Floating", "WM"),
        (":dark", "Toggle Light / Dark Polarity", "THEME"),
        (":shot", "Capture Desktop Screenshot", "PORTAL"),
        (":lock", "Show COSMIC Greeter / Lock Screen", "SESSION"),
        (":term", "Open COSMIC Terminal", "APP"),
        (":edit", "Open COSMIC Text Editor", "APP"),
        (":store", "Open COSMIC / Awesome GPUI Store", "APP"),
        (":installer", "Open OS Installer & DE Switcher", "APP"),
        (":settings", "Open COSMIC Settings", "APP"),
        (":monitor", "Open COSMIC System Monitor", "APP"),
        (":files", "Open COSMIC Files", "APP"),
    ];

    if let Some(cmd_filter) = trimmed.strip_prefix(':') {
        let q = cmd_filter.trim().to_ascii_lowercase();
        return built_in_cmds
            .iter()
            .filter(|(cmd, desc, _)| {
                q.is_empty()
                    || cmd.to_ascii_lowercase().contains(&q)
                    || desc.to_ascii_lowercase().contains(&q)
            })
            .map(|(cmd, desc, badge)| LauncherResultItem {
                title: (*cmd).to_owned(),
                subtitle: (*desc).to_owned(),
                badge: (*badge).to_owned(),
                action: LauncherActionKind::CompositorCommand((*cmd).to_owned()),
            })
            .collect();
    }

    let q = trimmed.to_ascii_lowercase();
    let mut results = Vec::new();

    if q.is_empty() {
        for (cmd, desc, badge) in built_in_cmds.iter().take(4) {
            results.push(LauncherResultItem {
                title: (*cmd).to_owned(),
                subtitle: (*desc).to_owned(),
                badge: (*badge).to_owned(),
                action: LauncherActionKind::CompositorCommand((*cmd).to_owned()),
            });
        }
    }

    for (id, name, category) in craft_apps {
        if !q.is_empty()
            && !name.to_ascii_lowercase().contains(&q)
            && !id.to_ascii_lowercase().contains(&q)
            && !category.to_ascii_lowercase().contains(&q)
        {
            continue;
        }
        results.push(LauncherResultItem {
            title: name.clone(),
            subtitle: format!("ArtCraft · {category}"),
            badge: "CRAFT".to_owned(),
            action: LauncherActionKind::CraftApp(id.clone()),
        });
        if results.len() >= 14 {
            return results;
        }
    }

    for entry in system_apps {
        if !q.is_empty() && !entry.name.to_ascii_lowercase().contains(&q) {
            continue;
        }
        results.push(LauncherResultItem {
            title: entry.name.clone(),
            subtitle: format!("System entry ({:?})", entry.origin),
            badge: "SYSTEM".to_owned(),
            action: LauncherActionKind::DesktopApp(entry.name.clone()),
        });
        if results.len() >= 18 {
            break;
        }
    }

    results
}

fn format_math_number(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 && value.abs() < 1e15 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

/// Recursive-descent arithmetic evaluator (`pop-launcher` `= <expr>` plugin).
#[must_use]
pub fn eval_math_expr(input: &str) -> Option<f64> {
    let bytes = input.as_bytes();
    let mut pos = 0_usize;
    let val = parse_expr(bytes, &mut pos)?;
    skip_ws(bytes, &mut pos);
    if pos == bytes.len() && val.is_finite() {
        Some(val)
    } else {
        None
    }
}

fn skip_ws(bytes: &[u8], pos: &mut usize) {
    while *pos < bytes.len() && bytes[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
}

fn parse_expr(bytes: &[u8], pos: &mut usize) -> Option<f64> {
    let mut acc = parse_term(bytes, pos)?;
    loop {
        skip_ws(bytes, pos);
        if *pos >= bytes.len() {
            break;
        }
        match bytes[*pos] {
            b'+' => {
                *pos += 1;
                acc += parse_term(bytes, pos)?;
            }
            b'-' => {
                *pos += 1;
                acc -= parse_term(bytes, pos)?;
            }
            _ => break,
        }
    }
    Some(acc)
}

fn parse_term(bytes: &[u8], pos: &mut usize) -> Option<f64> {
    let mut acc = parse_factor(bytes, pos)?;
    loop {
        skip_ws(bytes, pos);
        if *pos >= bytes.len() {
            break;
        }
        match bytes[*pos] {
            b'*' => {
                *pos += 1;
                acc *= parse_factor(bytes, pos)?;
            }
            b'/' => {
                *pos += 1;
                let rhs = parse_factor(bytes, pos)?;
                if rhs.abs() < f64::EPSILON {
                    return None;
                }
                acc /= rhs;
            }
            _ => break,
        }
    }
    Some(acc)
}

fn parse_factor(bytes: &[u8], pos: &mut usize) -> Option<f64> {
    skip_ws(bytes, pos);
    if *pos >= bytes.len() {
        return None;
    }
    if bytes[*pos] == b'-' {
        *pos += 1;
        return parse_factor(bytes, pos).map(|v| -v);
    }
    if bytes[*pos] == b'(' {
        *pos += 1;
        let val = parse_expr(bytes, pos)?;
        skip_ws(bytes, pos);
        if *pos < bytes.len() && bytes[*pos] == b')' {
            *pos += 1;
            return Some(val);
        }
        return None;
    }
    let start = *pos;
    let mut seen_dot = false;
    while *pos < bytes.len() {
        let ch = bytes[*pos];
        if ch.is_ascii_digit() {
            *pos += 1;
        } else if ch == b'.' && !seen_dot {
            seen_dot = true;
            *pos += 1;
        } else {
            break;
        }
    }
    if *pos == start {
        return None;
    }
    let slice = std::str::from_utf8(&bytes[start..*pos]).ok()?;
    slice.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pop_launcher_evaluates_math_and_compositor_commands() {
        let res = query_pop_launcher("= (1440 - 48) / 3", &[], &[]);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "464");

        let cmds = query_pop_launcher(":stack", &[], &[]);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].title, ":stack");
    }

    #[test]
    fn notifications_center_pushes_and_caps_cleanly() {
        let mut center = NotificationCenter::with_welcome();
        for idx in 0..20 {
            center.push("test", format!("Item {idx}"), "Body", NotificationUrgency::Normal);
        }
        assert_eq!(center.items.len(), 16);
    }
}
