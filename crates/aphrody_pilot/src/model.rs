use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ENVELOPE_VERSION: &str = "2.6";
pub const MAX_PNG_INPUT_BYTES: usize = 32 * 1024 * 1024;
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;
pub const MAX_TIMEOUT_MS: u64 = 120_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    #[must_use]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[must_use]
    pub fn from_session_rect(rect: aphrody_session::Rect) -> Self {
        Self {
            x: f64::from(rect.x),
            y: f64::from(rect.y),
            width: f64::from(rect.w),
            height: f64::from(rect.h),
        }
    }

    #[must_use]
    pub fn to_session_rect(self) -> aphrody_session::Rect {
        aphrody_session::Rect::new(
            self.x as f32,
            self.y as f32,
            self.width.max(1.0) as f32,
            self.height.max(1.0) as f32,
        )
    }

    #[must_use]
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    #[must_use]
    pub fn contains_point(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotSurface {
    #[default]
    Window,
    Menu,
    Menubar,
    Sheet,
    Popover,
    Alert,
    Dock,
    Tray,
    Focused,
    Desktop,
    Taskbar,
    #[serde(alias = "system-tray")]
    SystemTray,
    #[serde(alias = "quick-settings")]
    QuickSettings,
    #[serde(alias = "notification-center")]
    NotificationCenter,
    Toolbar,
    Spotlight,
    #[serde(alias = "menu-bar-extras")]
    MenuBarExtras,
    #[serde(alias = "system-tray-overflow")]
    SystemTrayOverflow,
    #[serde(alias = "start-menu")]
    StartMenu,
    #[serde(alias = "action-center")]
    ActionCenter,
}

impl SnapshotSurface {
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "window" => Some(Self::Window),
            "menu" => Some(Self::Menu),
            "menubar" | "menu-bar" => Some(Self::Menubar),
            "sheet" => Some(Self::Sheet),
            "popover" => Some(Self::Popover),
            "alert" => Some(Self::Alert),
            "dock" => Some(Self::Dock),
            "tray" => Some(Self::Tray),
            "focused" => Some(Self::Focused),
            "desktop" => Some(Self::Desktop),
            "taskbar" => Some(Self::Taskbar),
            "system_tray" | "system-tray" => Some(Self::SystemTray),
            "quick_settings" | "quick-settings" => Some(Self::QuickSettings),
            "notification_center" | "notification-center" => Some(Self::NotificationCenter),
            "toolbar" => Some(Self::Toolbar),
            "spotlight" => Some(Self::Spotlight),
            "menu_bar_extras" | "menu-bar-extras" => Some(Self::MenuBarExtras),
            "system_tray_overflow" | "system-tray-overflow" => Some(Self::SystemTrayOverflow),
            "start_menu" | "start-menu" => Some(Self::StartMenu),
            "action_center" | "action-center" => Some(Self::ActionCenter),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Window => "window",
            Self::Menu => "menu",
            Self::Menubar => "menubar",
            Self::Sheet => "sheet",
            Self::Popover => "popover",
            Self::Alert => "alert",
            Self::Dock => "dock",
            Self::Tray => "tray",
            Self::Focused => "focused",
            Self::Desktop => "desktop",
            Self::Taskbar => "taskbar",
            Self::SystemTray => "system-tray",
            Self::QuickSettings => "quick-settings",
            Self::NotificationCenter => "notification-center",
            Self::Toolbar => "toolbar",
            Self::Spotlight => "spotlight",
            Self::MenuBarExtras => "menu-bar-extras",
            Self::SystemTrayOverflow => "system-tray-overflow",
            Self::StartMenu => "start-menu",
            Self::ActionCenter => "action-center",
        }
    }

    #[must_use]
    pub const fn is_window(self) -> bool {
        matches!(self, Self::Window)
    }

    #[must_use]
    pub fn unavailable_error(self, supported: &[Self]) -> PilotError {
        PilotError::new(
            if supported.is_empty() {
                ErrorCode::PlatformNotSupported
            } else {
                ErrorCode::InvalidArgs
            },
            format!(
                "Snapshot surface '{}' is not available on this platform",
                self.as_str()
            ),
        )
        .with_details(serde_json::json!({
            "requested_surface": self.as_str(),
            "supported_surfaces": supported
                .iter()
                .map(|surface| surface.as_str())
                .collect::<Vec<_>>(),
        }))
        .with_suggestion("Choose one of the supported snapshot surfaces.")
        .with_disposition(DeliverySemantics::not_delivered())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
}

impl ElementState {
    #[must_use]
    pub fn has_state(&self, needle: &str) -> bool {
        let lower = needle.trim().to_ascii_lowercase();
        match lower.as_str() {
            "enabled" => {
                self.enabled.unwrap_or(false) || self.states.iter().any(|s| s == "enabled")
            }
            "disabled" => {
                self.enabled == Some(false) || self.states.iter().any(|s| s == "disabled")
            }
            "focused" => {
                self.focused.unwrap_or(false) || self.states.iter().any(|s| s == "focused")
            }
            "checked" => {
                self.checked.unwrap_or(false) || self.states.iter().any(|s| s == "checked")
            }
            "selected" => {
                self.selected.unwrap_or(false) || self.states.iter().any(|s| s == "selected")
            }
            "expanded" => {
                self.expanded.unwrap_or(false) || self.states.iter().any(|s| s == "expanded")
            }
            _ => self.states.iter().any(|s| s.eq_ignore_ascii_case(&lower)),
        }
    }

    pub fn push_flag(&mut self, flag: &str) {
        if !self.states.iter().any(|s| s == flag) {
            self.states.push(flag.to_owned());
        }
    }

    pub fn remove_flag(&mut self, flag: &str) {
        self.states.retain(|s| s != flag);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AccessibilityNode {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<String>,
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Rect>,
    #[serde(default)]
    pub state: ElementState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    #[serde(default)]
    pub interactive: bool,
    #[serde(default)]
    pub children_count: usize,
    #[serde(default)]
    pub subtree_truncated: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<AccessibilityNode>,
}

impl AccessibilityNode {
    #[must_use]
    pub fn is_interactive_role(role: &str) -> bool {
        matches!(
            role,
            "button"
                | "menubutton"
                | "textfield"
                | "checkbox"
                | "switch"
                | "link"
                | "menuitem"
                | "radiobutton"
                | "tab"
                | "slider"
                | "combobox"
                | "treeitem"
                | "disclosure"
                | "incrementor"
                | "colorwell"
                | "dockitem"
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessIdentity {
    pub pid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app: String,
    pub pid: u32,
    pub bounds: Rect,
    pub focused: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub visible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_token: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub include_minimized: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub pid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub id: String,
    pub bounds: Rect,
    pub is_primary: bool,
    pub scale: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SurfaceInfo {
    pub id: String,
    pub surface: SnapshotSurface,
    pub app: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Rect>,
    pub available: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardFormat {
    #[default]
    Auto,
    Text,
    Html,
    Png,
    Files,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum ClipboardContent {
    Text {
        text: String,
    },
    Html {
        html: String,
        fallback_text: Option<String>,
    },
    Png {
        base64: String,
        byte_len: usize,
    },
    Files {
        paths: Vec<String>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationInfo {
    pub id: String,
    pub app: String,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    #[default]
    Png,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBuffer {
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionReport {
    pub platform: String,
    pub accessibility_granted: bool,
    pub screen_recording_granted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elevation_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_server: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accessibility_detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen_recording_detail: Option<String>,
}

impl PermissionReport {
    #[must_use]
    pub fn accessibility_denied(&self) -> bool {
        !self.accessibility_granted
    }

    #[must_use]
    pub fn screen_recording_denied(&self) -> bool {
        !self.screen_recording_granted
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusReport {
    pub backend: String,
    pub platform: String,
    pub arch: String,
    pub os_version: String,
    pub session_id: String,
    pub permissions: PermissionReport,
    pub supported_surfaces: Vec<SnapshotSurface>,
    pub active_window_count: usize,
    pub display_count: usize,
    pub gpui_compositor_linked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    PermDenied,
    AppNotFound,
    WindowNotFound,
    ElementNotFound,
    StaleRef,
    AmbiguousRef,
    ActionNotSupported,
    ActionFailed,
    ElementDisabled,
    Timeout,
    InvalidArgs,
    AppUnresponsive,
    ElevationRequired,
    PlatformNotSupported,
    Internal,
}

impl ErrorCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PermDenied => "PERM_DENIED",
            Self::AppNotFound => "APP_NOT_FOUND",
            Self::WindowNotFound => "WINDOW_NOT_FOUND",
            Self::ElementNotFound => "ELEMENT_NOT_FOUND",
            Self::StaleRef => "STALE_REF",
            Self::AmbiguousRef => "AMBIGUOUS_REF",
            Self::ActionNotSupported => "ACTION_NOT_SUPPORTED",
            Self::ActionFailed => "ACTION_FAILED",
            Self::ElementDisabled => "ELEMENT_DISABLED",
            Self::Timeout => "TIMEOUT",
            Self::InvalidArgs => "INVALID_ARGS",
            Self::AppUnresponsive => "APP_UNRESPONSIVE",
            Self::ElevationRequired => "ELEVATION_REQUIRED",
            Self::PlatformNotSupported => "PLATFORM_NOT_SUPPORTED",
            Self::Internal => "INTERNAL",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryDisposition {
    #[default]
    Unknown,
    NotDelivered,
    DeliveryUncertain,
    DeliveredUnverified,
    DeliveredVerified,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryPolicy {
    Safe,
    Reobserve,
    #[default]
    DoNotRetry,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryDisposition {
    #[default]
    Unknown,
    Safe,
    Unsafe,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliverySemantics {
    pub disposition: DeliveryDisposition,
    pub retry: RetryPolicy,
}

impl DeliverySemantics {
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            disposition: DeliveryDisposition::Unknown,
            retry: RetryPolicy::DoNotRetry,
        }
    }

    #[must_use]
    pub const fn not_delivered() -> Self {
        Self {
            disposition: DeliveryDisposition::NotDelivered,
            retry: RetryPolicy::Safe,
        }
    }

    #[must_use]
    pub const fn reobserve() -> Self {
        Self {
            disposition: DeliveryDisposition::NotDelivered,
            retry: RetryPolicy::Reobserve,
        }
    }

    #[must_use]
    pub const fn delivered_verified() -> Self {
        Self {
            disposition: DeliveryDisposition::DeliveredVerified,
            retry: RetryPolicy::DoNotRetry,
        }
    }

    #[must_use]
    pub const fn delivered_unverified() -> Self {
        Self {
            disposition: DeliveryDisposition::DeliveredUnverified,
            retry: RetryPolicy::DoNotRetry,
        }
    }

    #[must_use]
    pub const fn uncertain() -> Self {
        Self {
            disposition: DeliveryDisposition::DeliveryUncertain,
            retry: RetryPolicy::Reobserve,
        }
    }

    #[must_use]
    pub const fn delivery(self) -> DeliveryDisposition {
        self.disposition
    }

    #[must_use]
    pub const fn retry_disposition(self) -> RetryDisposition {
        match self.disposition {
            DeliveryDisposition::Unknown => RetryDisposition::Unknown,
            DeliveryDisposition::NotDelivered => RetryDisposition::Safe,
            DeliveryDisposition::DeliveryUncertain
            | DeliveryDisposition::DeliveredUnverified
            | DeliveryDisposition::DeliveredVerified => RetryDisposition::Unsafe,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PilotErrorData {
    pub code: ErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    pub disposition: DeliverySemantics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// Compact pointer-sized error wrapper keeping `Result<T, PilotError>` fast across FFI & stack frames.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "PilotErrorData", into = "PilotErrorData")]
pub struct PilotError(Box<PilotErrorData>);

impl From<PilotErrorData> for PilotError {
    fn from(data: PilotErrorData) -> Self {
        Self(Box::new(data))
    }
}

impl From<PilotError> for PilotErrorData {
    fn from(err: PilotError) -> Self {
        *err.0
    }
}

impl std::ops::Deref for PilotError {
    type Target = PilotErrorData;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PilotError {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PilotError {
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        let disposition = match code {
            ErrorCode::StaleRef | ErrorCode::ElementNotFound | ErrorCode::AmbiguousRef => {
                DeliverySemantics::reobserve()
            }
            ErrorCode::PermDenied
            | ErrorCode::AppNotFound
            | ErrorCode::WindowNotFound
            | ErrorCode::InvalidArgs
            | ErrorCode::ActionNotSupported
            | ErrorCode::ElementDisabled
            | ErrorCode::ElevationRequired
            | ErrorCode::PlatformNotSupported
            | ErrorCode::Internal => DeliverySemantics::not_delivered(),
            ErrorCode::Timeout | ErrorCode::ActionFailed | ErrorCode::AppUnresponsive => {
                DeliverySemantics::uncertain()
            }
        };
        Self(Box::new(PilotErrorData {
            code,
            message: message.into(),
            suggestion: None,
            disposition,
            details: None,
        }))
    }

    #[must_use]
    pub fn stale_ref(ref_id: &str) -> Self {
        Self::new(
            ErrorCode::StaleRef,
            format!("ref '{ref_id}' is stale or no longer present in the UI tree"),
        )
        .with_suggestion("Run `snapshot` again and retry with a fresh ref ID.")
    }

    #[must_use]
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidArgs, msg)
    }

    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.0.suggestion = Some(suggestion.into());
        self
    }

    #[must_use]
    pub fn with_disposition(mut self, disposition: DeliverySemantics) -> Self {
        self.0.disposition = disposition;
        self
    }

    #[must_use]
    pub fn with_details(mut self, details: Value) -> Self {
        self.0.details = Some(details);
        self
    }

    #[must_use]
    pub const fn retry_disposition(&self) -> RetryDisposition {
        self.0.disposition.retry_disposition()
    }
}

impl fmt::Display for PilotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.0.code.as_str(), self.0.message)
    }
}

impl std::error::Error for PilotError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub version: String,
    pub ok: bool,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<PilotError>,
}

impl CommandEnvelope {
    #[must_use]
    pub fn ok(command: impl Into<String>, data: Value) -> Self {
        Self {
            version: ENVELOPE_VERSION.to_owned(),
            ok: true,
            command: command.into(),
            data: Some(data),
            error: None,
        }
    }

    #[must_use]
    pub fn err(command: impl Into<String>, error: PilotError) -> Self {
        Self {
            version: ENVELOPE_VERSION.to_owned(),
            ok: false,
            command: command.into(),
            data: None,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_disposition_maps_every_delivery_semantics() {
        assert_eq!(
            DeliverySemantics::unknown().retry_disposition(),
            RetryDisposition::Unknown
        );
        assert_eq!(
            DeliverySemantics::not_delivered().retry_disposition(),
            RetryDisposition::Safe
        );
        assert_eq!(
            DeliverySemantics::reobserve().retry_disposition(),
            RetryDisposition::Safe
        );
        assert_eq!(
            DeliverySemantics::uncertain().retry_disposition(),
            RetryDisposition::Unsafe
        );
        assert_eq!(
            DeliverySemantics::delivered_unverified().retry_disposition(),
            RetryDisposition::Unsafe
        );
        assert_eq!(
            DeliverySemantics::delivered_verified().retry_disposition(),
            RetryDisposition::Unsafe
        );

        let err = PilotError::new(ErrorCode::PlatformNotSupported, "unsupported surface");
        assert_eq!(err.code.as_str(), "PLATFORM_NOT_SUPPORTED");
        assert_eq!(err.retry_disposition(), RetryDisposition::Safe);
    }

    #[test]
    fn snapshot_surface_parses_kebab_and_snake_aliases() {
        for (raw, expected) in [
            ("taskbar", SnapshotSurface::Taskbar),
            ("system_tray", SnapshotSurface::SystemTray),
            ("system-tray", SnapshotSurface::SystemTray),
            ("quick-settings", SnapshotSurface::QuickSettings),
            ("notification-center", SnapshotSurface::NotificationCenter),
            ("toolbar", SnapshotSurface::Toolbar),
            ("spotlight", SnapshotSurface::Spotlight),
            ("menu-bar-extras", SnapshotSurface::MenuBarExtras),
            ("system-tray-overflow", SnapshotSurface::SystemTrayOverflow),
            ("start-menu", SnapshotSurface::StartMenu),
            ("action-center", SnapshotSurface::ActionCenter),
        ] {
            let parsed = SnapshotSurface::parse(raw);
            assert_eq!(parsed, Some(expected), "failed to parse surface '{raw}'");
            assert_eq!(
                SnapshotSurface::parse(expected.as_str()),
                Some(expected),
                "roundtrip failed for '{raw}'"
            );
        }

        let unsupported_none = SnapshotSurface::Spotlight.unavailable_error(&[]);
        assert_eq!(unsupported_none.code, ErrorCode::PlatformNotSupported);

        let unsupported_other =
            SnapshotSurface::Spotlight.unavailable_error(&[SnapshotSurface::Window]);
        assert_eq!(unsupported_other.code, ErrorCode::InvalidArgs);
    }
}
