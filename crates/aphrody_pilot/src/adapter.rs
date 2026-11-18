use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use web_time::Instant;

use crate::action::{
    Action, ActionRequest, ActionResult, Direction, DragParams, InteractionPolicy, KeyCombo,
    MouseButton, MouseEvent, MouseEventKind, execute_verified_action,
};
use crate::gpui_bridge::GpuiCompositorAdapter;
use crate::locator::{
    LocatorQuery, ObservationBudgetStats, ObservationRequest, ObservedTree, evaluate_locator_tree,
    render_snapshot_text,
};
use crate::model::{
    AccessibilityNode, AppInfo, ClipboardContent, ClipboardFormat, DEFAULT_TIMEOUT_MS, DisplayInfo,
    ElementState, ErrorCode, ImageBuffer, MAX_PNG_INPUT_BYTES, MAX_TIMEOUT_MS, NotificationInfo,
    PermissionReport, PilotError, Rect, SnapshotSurface, StatusReport, SurfaceInfo, WindowFilter,
    WindowInfo,
};
use crate::policy::{
    AiActPlan, BatchStep, TraceEvent, TraceRecorder, WaitSelector, act_verify, execute_batch,
    wait_for_selector,
};
use crate::refs::{
    IdentityMatch, RefEntry, RefMap, RefScope, RefStore, new_snapshot_id, stamp_tree_refs,
    validate_ref_id, validate_session_id, verify_entry_identity,
};

#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    started: Instant,
    timeout: Duration,
}

impl Deadline {
    #[must_use]
    pub fn from_timeout_ms(timeout_ms: Option<u64>) -> Self {
        let clamped = timeout_ms
            .unwrap_or(DEFAULT_TIMEOUT_MS)
            .clamp(50, MAX_TIMEOUT_MS);
        Self {
            started: Instant::now(),
            timeout: Duration::from_millis(clamped),
        }
    }

    #[must_use]
    pub fn remaining(&self) -> Duration {
        self.timeout.saturating_sub(self.started.elapsed())
    }

    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.remaining().is_zero()
    }

    pub fn ensure_remaining(&self) -> Result<(), PilotError> {
        if self.is_expired() {
            Err(PilotError::new(
                ErrorCode::Timeout,
                "operation deadline expired before host call completed",
            ))
        } else {
            Ok(())
        }
    }

    #[must_use]
    pub fn capped(&self, cap: Duration) -> Self {
        let rem = self.remaining().min(cap);
        Self {
            started: Instant::now(),
            timeout: rem,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InteractionLease {
    deadline: Deadline,
    policy: InteractionPolicy,
}

impl InteractionLease {
    #[must_use]
    pub const fn new(deadline: Deadline, policy: InteractionPolicy) -> Self {
        Self { deadline, policy }
    }

    #[must_use]
    pub const fn deadline(&self) -> Deadline {
        self.deadline
    }

    #[must_use]
    pub const fn policy(&self) -> InteractionPolicy {
        self.policy
    }
}

#[derive(Clone, Default)]
pub struct NativeHandle(Option<Arc<dyn Any + Send + Sync>>);

impl std::fmt::Debug for NativeHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NativeHandle({})",
            if self.0.is_some() { "some" } else { "null" }
        )
    }
}

impl NativeHandle {
    #[must_use]
    pub const fn null() -> Self {
        Self(None)
    }

    #[must_use]
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self(Some(Arc::new(value)))
    }

    #[must_use]
    pub fn is_null(&self) -> bool {
        self.0.is_none()
    }

    #[must_use]
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.as_ref().and_then(|arc| arc.downcast_ref::<T>())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveElement {
    pub id: String,
    pub role: String,
    pub name: Option<String>,
    pub bounds: Option<Rect>,
    pub state: ElementState,
    pub actions: Vec<String>,
}

pub trait ObservationOps: Send + Sync {
    fn observe_tree(
        &self,
        root: &WindowFilter,
        request: &ObservationRequest,
        deadline: Deadline,
    ) -> Result<(WindowInfo, AccessibilityNode, bool), PilotError>;

    fn resolve_element(
        &self,
        entry: &RefEntry,
        deadline: Deadline,
    ) -> Result<(NativeHandle, AccessibilityNode), PilotError>;

    fn get_live_element(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<LiveElement, PilotError>;

    fn list_surfaces(&self, deadline: Deadline) -> Result<Vec<SurfaceInfo>, PilotError>;
}

pub trait ActionOps: Send + Sync {
    fn execute_action(
        &self,
        handle: &NativeHandle,
        request: &ActionRequest,
        lease: &InteractionLease,
    ) -> Result<ActionResult, PilotError>;
}

pub trait InputOps: Send + Sync {
    fn press_key(&self, combo: &KeyCombo, deadline: Deadline) -> Result<(), PilotError>;
    fn key_state(&self, combo: &KeyCombo, down: bool, deadline: Deadline)
    -> Result<(), PilotError>;
    fn mouse_event(&self, event: &MouseEvent, deadline: Deadline) -> Result<(), PilotError>;
    fn drag(&self, params: &DragParams, deadline: Deadline) -> Result<(), PilotError>;
    fn clipboard_get(
        &self,
        format: ClipboardFormat,
        deadline: Deadline,
    ) -> Result<ClipboardContent, PilotError>;
    fn clipboard_set(
        &self,
        content: &ClipboardContent,
        deadline: Deadline,
    ) -> Result<(), PilotError>;
    fn clipboard_clear(&self, deadline: Deadline) -> Result<(), PilotError>;
}

pub trait SystemOps: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn permissions(&self, deadline: Deadline) -> Result<PermissionReport, PilotError>;
    fn list_apps(&self, deadline: Deadline) -> Result<Vec<AppInfo>, PilotError>;
    fn list_windows(&self, deadline: Deadline) -> Result<Vec<WindowInfo>, PilotError>;
    fn list_displays(&self, deadline: Deadline) -> Result<Vec<DisplayInfo>, PilotError>;
    fn focus_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError>;
    fn move_window(
        &self,
        filter: &WindowFilter,
        x: f64,
        y: f64,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError>;
    fn resize_window(
        &self,
        filter: &WindowFilter,
        width: f64,
        height: f64,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError>;
    fn minimize_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError>;
    fn maximize_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError>;
    fn restore_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError>;
    fn launch_app(
        &self,
        app: &str,
        args: &[String],
        deadline: Deadline,
    ) -> Result<AppInfo, PilotError>;
    fn close_app(&self, app: &str, deadline: Deadline) -> Result<(), PilotError>;
    fn open_system_surface(
        &self,
        surface: SnapshotSurface,
        deadline: Deadline,
    ) -> Result<SurfaceInfo, PilotError>;
    fn screenshot(
        &self,
        filter: &WindowFilter,
        display_index: Option<usize>,
        deadline: Deadline,
    ) -> Result<ImageBuffer, PilotError>;
    fn list_notifications(&self, deadline: Deadline) -> Result<Vec<NotificationInfo>, PilotError>;
    fn dismiss_notification(&self, id: &str, deadline: Deadline) -> Result<(), PilotError>;
    fn dismiss_all_notifications(&self, deadline: Deadline) -> Result<usize, PilotError>;
    fn notification_action(
        &self,
        id: &str,
        action_name: &str,
        deadline: Deadline,
    ) -> Result<(), PilotError>;
}

pub trait PlatformAdapter: ObservationOps + ActionOps + InputOps + SystemOps {}

impl<T: ObservationOps + ActionOps + InputOps + SystemOps> PlatformAdapter for T {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetBackend {
    #[default]
    Auto,
    Gpui,
    Native,
}

impl TargetBackend {
    #[must_use]
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.unwrap_or("auto").trim().to_ascii_lowercase().as_str() {
            "gpui" | "compositor" => Self::Gpui,
            "native" | "os" | "host" => Self::Native,
            _ => Self::Auto,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub target: TargetBackend,
    #[serde(default)]
    pub policy: InteractionPolicy,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HeldInputState {
    pub held_keys: Vec<String>,
    pub held_buttons: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudActivityItem {
    pub command: String,
    pub target_ref: Option<String>,
    pub ok: bool,
    pub duration_ms: u64,
    pub disposition: String,
}

/// Stateful unified AI OS Pilot controller coordinating GPUI AccessKit/Compositor state AND live
/// host OS accessibility backends (Windows UIA, Linux AT-SPI2/X11, macOS AXUIElement).
pub struct PilotEngine {
    pub gpui: GpuiCompositorAdapter,
    pub browser: crate::browser::BrowserControllerState,
    native: Box<dyn PlatformAdapter>,
    session_id: String,
    target: TargetBackend,
    ref_store: Option<RefStore>,
    memory_snapshots: BTreeMap<String, RefMap>,
    latest_snapshot_id: Option<String>,
    snapshot_counter: u64,
    pub trace: TraceRecorder,
    pub held_input: HeldInputState,
    pub recent_hud_activity: Vec<HudActivityItem>,
}

impl Default for PilotEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PilotEngine {
    #[must_use]
    pub fn new() -> Self {
        #[cfg(not(target_family = "wasm"))]
        let ref_store = RefStore::for_session(Some("default")).ok();
        #[cfg(target_family = "wasm")]
        let ref_store = None;
        Self {
            gpui: GpuiCompositorAdapter::new(),
            browser: crate::browser::BrowserControllerState::default(),
            native: create_native_adapter(),
            session_id: "default".to_owned(),
            target: TargetBackend::Auto,
            ref_store,
            memory_snapshots: BTreeMap::new(),
            latest_snapshot_id: None,
            snapshot_counter: 0,
            trace: TraceRecorder::default(),
            held_input: HeldInputState::default(),
            recent_hud_activity: Vec::new(),
        }
    }

    pub fn set_session(&mut self, session_id: &str) -> Result<(), PilotError> {
        validate_session_id(session_id)?;
        self.session_id = session_id.to_owned();
        #[cfg(not(target_family = "wasm"))]
        {
            self.ref_store = RefStore::for_session(Some(session_id)).ok();
        }
        Ok(())
    }

    pub fn set_target(&mut self, target: TargetBackend) {
        self.target = target;
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn active_adapter(&self) -> &dyn PlatformAdapter {
        match self.target {
            TargetBackend::Gpui => &self.gpui,
            TargetBackend::Native => self.native.as_ref(),
            TargetBackend::Auto => &self.gpui,
        }
    }

    #[must_use]
    pub fn adapter_for(&self, ctx: &CommandContext) -> &dyn PlatformAdapter {
        match ctx.target {
            TargetBackend::Gpui => &self.gpui,
            TargetBackend::Native => self.native.as_ref(),
            TargetBackend::Auto => match self.target {
                TargetBackend::Native => self.native.as_ref(),
                _ => &self.gpui,
            },
        }
    }

    pub fn status(&self) -> Result<StatusReport, PilotError> {
        let deadline = Deadline::from_timeout_ms(Some(DEFAULT_TIMEOUT_MS));
        let adapter = self.active_adapter();
        let permissions = adapter.permissions(deadline)?;
        let windows = adapter.list_windows(deadline).unwrap_or_default();
        let displays = adapter.list_displays(deadline).unwrap_or_default();
        Ok(StatusReport {
            backend: adapter.backend_name().to_owned(),
            platform: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            os_version: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            session_id: self.session_id.clone(),
            permissions,
            supported_surfaces: vec![
                SnapshotSurface::Window,
                SnapshotSurface::Menu,
                SnapshotSurface::Menubar,
                SnapshotSurface::Sheet,
                SnapshotSurface::Popover,
                SnapshotSurface::Alert,
                SnapshotSurface::Dock,
                SnapshotSurface::Tray,
                SnapshotSurface::Focused,
                SnapshotSurface::Desktop,
                SnapshotSurface::Taskbar,
                SnapshotSurface::SystemTray,
                SnapshotSurface::QuickSettings,
                SnapshotSurface::NotificationCenter,
                SnapshotSurface::Toolbar,
                SnapshotSurface::Spotlight,
                SnapshotSurface::MenuBarExtras,
                SnapshotSurface::SystemTrayOverflow,
                SnapshotSurface::StartMenu,
                SnapshotSurface::ActionCenter,
            ],
            active_window_count: windows.len(),
            display_count: displays.len(),
            gpui_compositor_linked: true,
        })
    }

    pub fn snapshot(
        &mut self,
        request: &ObservationRequest,
        ctx: &CommandContext,
    ) -> Result<ObservedTree, PilotError> {
        let started = Instant::now();
        let deadline = Deadline::from_timeout_ms(ctx.timeout_ms);
        let filter = WindowFilter {
            app: request.app.clone().or_else(|| ctx.app.clone()),
            window_id: request.window_id.clone().or_else(|| ctx.window_id.clone()),
            ..Default::default()
        };
        let (window, raw_root, complete) = self
            .adapter_for(ctx)
            .observe_tree(&filter, request, deadline)?;

        self.snapshot_counter += 1;
        let sid = new_snapshot_id(self.snapshot_counter ^ (started.elapsed().as_nanos() as u64));
        let scope = Some(RefScope {
            app: window.app.clone(),
            window_id: window.id.clone(),
            surface: request.surface,
            pid: window.pid,
        });
        let (stamped_root, map) =
            stamp_tree_refs(&sid, scope, raw_root, started.elapsed().as_millis() as u64);
        let ref_count = map.len();
        let text = render_snapshot_text(&stamped_root, request);
        self.save_ref_map(map);

        let duration_ms = started.elapsed().as_millis() as u64;
        self.push_trace("snapshot", true, duration_ms, None, None);

        Ok(ObservedTree {
            snapshot_id: sid,
            window,
            surface: request.surface,
            root: stamped_root,
            complete,
            stats: ObservationBudgetStats {
                nodes_visited: ref_count,
                max_depth_reached: request.max_depth,
                truncated: !complete,
                duration_ms,
            },
            text,
            ref_count,
        })
    }

    pub fn resolve_saved_ref(
        &self,
        ref_id: &str,
        deadline: Deadline,
    ) -> Result<NativeHandle, PilotError> {
        let normalized = validate_ref_id(ref_id)?;
        let token = normalized.strip_prefix('@').unwrap_or(&normalized);
        let (snap_id, _) = token
            .split_once(':')
            .ok_or_else(|| PilotError::stale_ref(ref_id))?;

        let entry = self
            .memory_snapshots
            .get(snap_id)
            .and_then(|m| m.get(&normalized))
            .cloned()
            .or_else(|| {
                self.ref_store
                    .as_ref()
                    .and_then(|s| s.resolve_ref(&normalized).ok().map(|(_, e)| e))
            })
            .ok_or_else(|| PilotError::stale_ref(ref_id))?;

        let (handle, live_node) = self.active_adapter().resolve_element(&entry, deadline)?;
        if verify_entry_identity(&entry, &live_node) == IdentityMatch::NoMatch {
            return Err(PilotError::stale_ref(ref_id));
        }
        Ok(handle)
    }

    pub fn list_windows(&self, deadline: Deadline) -> Result<Vec<WindowInfo>, PilotError> {
        self.active_adapter().list_windows(deadline)
    }

    pub fn save_ref_map(&mut self, map: RefMap) {
        self.latest_snapshot_id = Some(map.snapshot_id.clone());
        if let Some(store) = &self.ref_store
            && let Err(err) = store.save_snapshot(&map)
        {
            log::warn!("failed to persist pilot snapshot: {err}");
        }
        if self.memory_snapshots.len() >= 64
            && let Some(oldest) = self.memory_snapshots.keys().next().cloned()
        {
            self.memory_snapshots.remove(&oldest);
        }
        self.memory_snapshots.insert(map.snapshot_id.clone(), map);
    }

    pub fn record_action_event(&mut self, res: &ActionResult) {
        self.push_trace(
            &res.action,
            res.disposition.disposition != crate::model::DeliveryDisposition::NotDelivered,
            res.duration_ms,
            res.target_ref.clone(),
            None,
        );
        if self.recent_hud_activity.len() >= 12 {
            self.recent_hud_activity.remove(0);
        }
        self.recent_hud_activity.push(HudActivityItem {
            command: res.action.clone(),
            target_ref: res.target_ref.clone(),
            ok: res.disposition.disposition != crate::model::DeliveryDisposition::NotDelivered,
            duration_ms: res.duration_ms,
            disposition: format!("{:?}", res.disposition.disposition),
        });
    }

    fn push_trace(
        &mut self,
        command: &str,
        ok: bool,
        duration_ms: u64,
        target_ref: Option<String>,
        error_code: Option<String>,
    ) {
        #[cfg(not(target_family = "wasm"))]
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        #[cfg(target_family = "wasm")]
        let now_ms = self.snapshot_counter;
        self.trace.record(TraceEvent {
            seq: 0,
            timestamp_ms: now_ms,
            session_id: self.session_id.clone(),
            backend: self.active_adapter().backend_name().to_owned(),
            command: command.to_owned(),
            ok,
            duration_ms,
            target_ref,
            error_code,
        });
    }

    /// Unified dispatcher for all 57+ desktop/AI-OS tools used by both `aphrody_craft` MCP and
    /// `aphrody-os pilot` CLI commands.
    pub fn execute_tool(
        &mut self,
        tool_name: &str,
        args: &Value,
        ctx: &CommandContext,
    ) -> Result<Value, PilotError> {
        let started = Instant::now();
        let deadline = Deadline::from_timeout_ms(ctx.timeout_ms);
        let op = tool_name
            .trim()
            .strip_prefix("desktop_")
            .unwrap_or(tool_name.trim())
            .replace('-', "_");

        if let Some(target_arg) = args.get("target").and_then(Value::as_str) {
            self.set_target(TargetBackend::parse(Some(target_arg)));
        }

        let normalized = tool_name.trim().replace('-', "_");
        let res = if normalized.starts_with("browser_") {
            self.browser
                .execute_tool(&normalized, args)
                .map_err(|err| PilotError::new(ErrorCode::Internal, err.to_string()))
        } else {
            self.dispatch_op(&op, args, ctx, deadline)
        };
        let dur = started.elapsed().as_millis() as u64;
        if let Err(ref err) = res {
            self.push_trace(&op, false, dur, None, Some(err.code.as_str().to_owned()));
        }
        res
    }

    fn dispatch_op(
        &mut self,
        op: &str,
        args: &Value,
        ctx: &CommandContext,
        deadline: Deadline,
    ) -> Result<Value, PilotError> {
        match op {
            "status" => Ok(json!(self.status()?)),
            "permissions" => Ok(json!(self.adapter_for(ctx).permissions(deadline)?)),
            "list_apps" => Ok(json!({ "apps": self.adapter_for(ctx).list_apps(deadline)? })),
            "list_windows" => {
                Ok(json!({ "windows": self.adapter_for(ctx).list_windows(deadline)? }))
            }
            "list_displays" => {
                Ok(json!({ "displays": self.adapter_for(ctx).list_displays(deadline)? }))
            }
            "list_surfaces" => {
                Ok(json!({ "surfaces": self.adapter_for(ctx).list_surfaces(deadline)? }))
            }
            "snapshot" => {
                let req = ObservationRequest {
                    surface: args
                        .get("surface")
                        .and_then(Value::as_str)
                        .and_then(SnapshotSurface::parse)
                        .unwrap_or_default(),
                    interactive_only: args
                        .get("interactive")
                        .or_else(|| args.get("interactive_only"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    include_bounds: args
                        .get("include_bounds")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    compact: args
                        .get("compact")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    max_depth: args.get("max_depth").and_then(Value::as_u64).unwrap_or(18) as usize,
                    max_nodes: args
                        .get("max_nodes")
                        .and_then(Value::as_u64)
                        .unwrap_or(2500) as usize,
                    root_ref: args.get("root").and_then(Value::as_str).map(str::to_owned),
                    app: args.get("app").and_then(Value::as_str).map(str::to_owned),
                    window_id: args
                        .get("window_id")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                };
                Ok(json!(self.snapshot(&req, ctx)?))
            }
            "find" => {
                let obs = self.snapshot(&ObservationRequest::default(), ctx)?;
                let query = parse_locator_from_args(args);
                Ok(json!(evaluate_locator_tree(&obs.root, &query)))
            }
            "get" => {
                let ref_id = req_str(args, "ref")?;
                let handle = self.resolve_saved_ref(ref_id, deadline)?;
                let el = self.adapter_for(ctx).get_live_element(&handle, deadline)?;
                Ok(json!(el))
            }
            "is" => {
                let ref_id = req_str(args, "ref")?;
                let state_name = req_str(args, "state")?;
                let handle = self.resolve_saved_ref(ref_id, deadline)?;
                let el = self.adapter_for(ctx).get_live_element(&handle, deadline)?;
                Ok(
                    json!({ "ref": ref_id, "state": state_name, "value": el.state.has_state(state_name) }),
                )
            }
            "click" => self.run_ref_action(args, Action::Click, ctx, deadline),
            "double_click" => self.run_ref_action(args, Action::DoubleClick, ctx, deadline),
            "triple_click" => self.run_ref_action(args, Action::TripleClick, ctx, deadline),
            "right_click" => self.run_ref_action(args, Action::RightClick, ctx, deadline),
            "type" => {
                let text = req_str(args, "text")?.to_owned();
                self.run_ref_action(args, Action::TypeText(text), ctx, deadline)
            }
            "set_value" => {
                let val = req_str(args, "value")?.to_owned();
                self.run_ref_action(args, Action::SetValue(val), ctx, deadline)
            }
            "clear" => self.run_ref_action(args, Action::Clear, ctx, deadline),
            "focus" => self.run_ref_action(args, Action::Focus, ctx, deadline),
            "select" => {
                let item = req_str(args, "item").unwrap_or_default().to_owned();
                self.run_ref_action(args, Action::Select(item), ctx, deadline)
            }
            "toggle" => self.run_ref_action(args, Action::Toggle, ctx, deadline),
            "check" => self.run_ref_action(args, Action::Check, ctx, deadline),
            "uncheck" => self.run_ref_action(args, Action::Uncheck, ctx, deadline),
            "expand" => self.run_ref_action(args, Action::Expand, ctx, deadline),
            "collapse" => self.run_ref_action(args, Action::Collapse, ctx, deadline),
            "scroll" => {
                let dir_raw = args
                    .get("direction")
                    .and_then(Value::as_str)
                    .unwrap_or("down");
                let direction = Direction::parse(dir_raw).ok_or_else(|| {
                    PilotError::invalid_input(format!("invalid scroll direction '{dir_raw}'"))
                })?;
                let amount = args.get("amount").and_then(Value::as_i64).unwrap_or(3) as i32;
                self.run_ref_action(args, Action::Scroll { direction, amount }, ctx, deadline)
            }
            "scroll_to" => self.run_ref_action(args, Action::ScrollTo, ctx, deadline),
            "press" => {
                let key = req_str(args, "key")?;
                let combo = KeyCombo::parse(key)?;
                self.adapter_for(ctx).press_key(&combo, deadline)?;
                Ok(json!({ "pressed": key }))
            }
            "key_down" | "key_up" => {
                let down = op == "key_down";
                let key = req_str(args, "key")?;
                let combo = KeyCombo::parse(key)?;
                self.adapter_for(ctx).key_state(&combo, down, deadline)?;
                if down {
                    if !self.held_input.held_keys.contains(&combo.key) {
                        self.held_input.held_keys.push(combo.key.clone());
                    }
                } else {
                    self.held_input.held_keys.retain(|k| k != &combo.key);
                }
                Ok(json!({ "key": key, "down": down, "held": self.held_input.held_keys }))
            }
            "hover" | "mouse_move" => {
                let (x, y) = parse_xy(args)?;
                let ev = MouseEvent {
                    kind: MouseEventKind::Move,
                    x,
                    y,
                    button: MouseButton::Left,
                    click_count: 0,
                    modifiers: Vec::new(),
                    wheel_dx: 0,
                    wheel_dy: 0,
                };
                self.adapter_for(ctx).mouse_event(&ev, deadline)?;
                Ok(json!({ "x": x, "y": y }))
            }
            "mouse_click" | "mouse_down" | "mouse_up" => {
                let (x, y) = parse_xy(args)?;
                let btn_str = args.get("button").and_then(Value::as_str).unwrap_or("left");
                let button = MouseButton::parse(btn_str).unwrap_or(MouseButton::Left);
                let kind = match op {
                    "mouse_down" => {
                        self.held_input.held_buttons.insert(btn_str.to_owned());
                        MouseEventKind::Down
                    }
                    "mouse_up" => {
                        self.held_input.held_buttons.remove(btn_str);
                        MouseEventKind::Up
                    }
                    _ => MouseEventKind::Click,
                };
                let ev = MouseEvent {
                    kind,
                    x,
                    y,
                    button,
                    click_count: args.get("count").and_then(Value::as_u64).unwrap_or(1) as u8,
                    modifiers: Vec::new(),
                    wheel_dx: 0,
                    wheel_dy: 0,
                };
                self.adapter_for(ctx).mouse_event(&ev, deadline)?;
                Ok(json!({ "kind": op, "x": x, "y": y, "button": btn_str }))
            }
            "mouse_wheel" => {
                let (x, y) = parse_xy(args).unwrap_or((0.0, 0.0));
                let dy = args.get("dy").and_then(Value::as_i64).unwrap_or(-120) as i32;
                let dx = args.get("dx").and_then(Value::as_i64).unwrap_or(0) as i32;
                let ev = MouseEvent {
                    kind: MouseEventKind::Wheel,
                    x,
                    y,
                    button: MouseButton::Left,
                    click_count: 0,
                    modifiers: Vec::new(),
                    wheel_dx: dx,
                    wheel_dy: dy,
                };
                self.adapter_for(ctx).mouse_event(&ev, deadline)?;
                Ok(json!({ "dx": dx, "dy": dy }))
            }
            "drag" => {
                let params = DragParams {
                    from_x: req_f64(args, "from_x")?,
                    from_y: req_f64(args, "from_y")?,
                    to_x: req_f64(args, "to_x")?,
                    to_y: req_f64(args, "to_y")?,
                    button: MouseButton::parse(
                        args.get("button").and_then(Value::as_str).unwrap_or("left"),
                    )
                    .unwrap_or(MouseButton::Left),
                    steps: args.get("steps").and_then(Value::as_u64).unwrap_or(12) as u16,
                    duration_ms: args
                        .get("duration_ms")
                        .and_then(Value::as_u64)
                        .unwrap_or(240),
                };
                self.adapter_for(ctx).drag(&params, deadline)?;
                Ok(json!({ "dragged": params }))
            }
            "launch" => {
                let app = req_str(args, "app")?;
                let launch_args: Vec<String> = args
                    .get("args")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(json!(self.adapter_for(ctx).launch_app(
                    app,
                    &launch_args,
                    deadline
                )?))
            }
            "close_app" => {
                let app = req_str(args, "app")?;
                self.adapter_for(ctx).close_app(app, deadline)?;
                Ok(json!({ "closed": app }))
            }
            "focus_window" => {
                let filter = parse_window_filter(args, ctx);
                Ok(json!(
                    self.adapter_for(ctx).focus_window(&filter, deadline)?
                ))
            }
            "move_window" => {
                let filter = parse_window_filter(args, ctx);
                let (x, y) = parse_xy(args)?;
                Ok(json!(
                    self.adapter_for(ctx).move_window(&filter, x, y, deadline)?
                ))
            }
            "resize_window" => {
                let filter = parse_window_filter(args, ctx);
                let w = req_f64(args, "width")?;
                let h = req_f64(args, "height")?;
                Ok(json!(
                    self.adapter_for(ctx)
                        .resize_window(&filter, w, h, deadline)?
                ))
            }
            "minimize" => {
                let filter = parse_window_filter(args, ctx);
                Ok(json!(
                    self.adapter_for(ctx).minimize_window(&filter, deadline)?
                ))
            }
            "maximize" => {
                let filter = parse_window_filter(args, ctx);
                Ok(json!(
                    self.adapter_for(ctx).maximize_window(&filter, deadline)?
                ))
            }
            "restore" => {
                let filter = parse_window_filter(args, ctx);
                Ok(json!(
                    self.adapter_for(ctx).restore_window(&filter, deadline)?
                ))
            }
            "open_system_surface" => {
                let s = SnapshotSurface::parse(req_str(args, "surface")?)
                    .ok_or_else(|| PilotError::invalid_input("unknown snapshot surface"))?;
                Ok(json!(
                    self.adapter_for(ctx).open_system_surface(s, deadline)?
                ))
            }
            "clipboard_get" => {
                let fmt = match args
                    .get("format")
                    .and_then(Value::as_str)
                    .map(str::to_ascii_lowercase)
                    .as_deref()
                {
                    Some("text") => ClipboardFormat::Text,
                    Some("html") => ClipboardFormat::Html,
                    Some("png") => ClipboardFormat::Png,
                    Some("files") => ClipboardFormat::Files,
                    _ => ClipboardFormat::Auto,
                };
                Ok(json!(self.adapter_for(ctx).clipboard_get(fmt, deadline)?))
            }
            "clipboard_set" => {
                let content = parse_clipboard_content_from_args(args)?;
                self.adapter_for(ctx).clipboard_set(&content, deadline)?;
                Ok(json!({ "ok": true }))
            }
            "clipboard_clear" => {
                self.adapter_for(ctx).clipboard_clear(deadline)?;
                Ok(json!({ "cleared": true }))
            }
            "list_notifications" => {
                Ok(json!({ "notifications": self.adapter_for(ctx).list_notifications(deadline)? }))
            }
            "dismiss_notification" => {
                let id = req_str(args, "id")?;
                self.adapter_for(ctx).dismiss_notification(id, deadline)?;
                Ok(json!({ "dismissed": id }))
            }
            "dismiss_all_notifications" => {
                let count = self.adapter_for(ctx).dismiss_all_notifications(deadline)?;
                Ok(json!({ "dismissed_count": count }))
            }
            "notification_action" => {
                let id = req_str(args, "id")?;
                let action_name = req_str(args, "action")?;
                self.adapter_for(ctx)
                    .notification_action(id, action_name, deadline)?;
                Ok(json!({ "id": id, "action": action_name }))
            }
            "screenshot" => {
                let filter = parse_window_filter(args, ctx);
                let display = args
                    .get("display")
                    .and_then(Value::as_u64)
                    .map(|d| d as usize);
                let img = self
                    .adapter_for(ctx)
                    .screenshot(&filter, display, deadline)?;
                use base64::Engine as _;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&img.data);
                if let Some(path) = args.get("out").and_then(Value::as_str) {
                    std::fs::write(path, &img.data).map_err(|err| {
                        PilotError::new(
                            ErrorCode::Internal,
                            format!("failed writing screenshot '{path}': {err}"),
                        )
                    })?;
                }
                Ok(json!({
                    "format": "png",
                    "width": img.width,
                    "height": img.height,
                    "scale": img.scale,
                    "byte_len": img.data.len(),
                    "base64": b64,
                }))
            }
            "wait" => {
                let selector: WaitSelector =
                    serde_json::from_value(args.clone()).map_err(|err| {
                        PilotError::invalid_input(format!("invalid wait selector: {err}"))
                    })?;
                Ok(json!(wait_for_selector(self, &selector, ctx)?))
            }
            "batch" => {
                let steps: Vec<BatchStep> = args
                    .get("steps")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .ok_or_else(|| PilotError::invalid_input("batch requires array `steps`"))?;
                let stop_on_error = args
                    .get("stop_on_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let mut batch_ctx = ctx.clone();
                if let Some(timeout_ms) = args.get("timeout_ms").and_then(Value::as_u64) {
                    batch_ctx.timeout_ms = Some(timeout_ms);
                }
                Ok(json!(execute_batch(self, &steps, stop_on_error, &batch_ctx)))
            }
            "session" => {
                if let Some(id) = args.get("id").and_then(Value::as_str) {
                    self.set_session(id)?;
                }
                Ok(json!({
                    "session_id": self.session_id,
                    "target": self.target,
                    "latest_snapshot_id": self.latest_snapshot_id,
                    "held_input": self.held_input,
                }))
            }
            "trace" => {
                let sub = args.get("op").and_then(Value::as_str).unwrap_or("status");
                match sub {
                    "start" => {
                        let out = args
                            .get("out")
                            .and_then(Value::as_str)
                            .map(std::path::PathBuf::from);
                        self.trace.start(out);
                    }
                    "stop" => self.trace.stop(),
                    _ => {}
                }
                let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(50) as usize;
                Ok(json!({
                    "enabled": self.trace.is_enabled(),
                    "output_path": self.trace.output_path(),
                    "events": self.trace.snapshot(limit),
                }))
            }
            "act_verify" | "ai_act_verify" => {
                let plan: AiActPlan = serde_json::from_value(args.clone()).map_err(|err| {
                    PilotError::invalid_input(format!("invalid act_verify plan: {err}"))
                })?;
                Ok(json!(act_verify(self, &plan, ctx)?))
            }
            other => Err(PilotError::invalid_input(format!(
                "unknown pilot operation '{other}'"
            ))),
        }
    }

    fn run_ref_action(
        &mut self,
        args: &Value,
        action: Action,
        ctx: &CommandContext,
        deadline: Deadline,
    ) -> Result<Value, PilotError> {
        let ref_id = req_str(args, "ref")?;
        let normalized = validate_ref_id(ref_id)?;
        let handle = self.resolve_saved_ref(&normalized, deadline)?;
        let lease = InteractionLease::new(deadline, ctx.policy);
        let req = ActionRequest {
            action,
            policy: ctx.policy,
        };
        let result = {
            let adapter = self.adapter_for(ctx);
            execute_verified_action(adapter, &handle, Some(&normalized), req, &lease)?
        };
        self.record_action_event(&result);
        Ok(json!(result))
    }
}

fn req_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, PilotError> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| PilotError::invalid_input(format!("missing required string '{key}'")))
}

fn req_f64(args: &Value, key: &str) -> Result<f64, PilotError> {
    args.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| PilotError::invalid_input(format!("missing numeric '{key}'")))
}

fn parse_xy(args: &Value) -> Result<(f64, f64), PilotError> {
    Ok((req_f64(args, "x")?, req_f64(args, "y")?))
}

fn parse_window_filter(args: &Value, ctx: &CommandContext) -> WindowFilter {
    WindowFilter {
        app: args
            .get("app")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| ctx.app.clone()),
        window_id: args
            .get("window_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| ctx.window_id.clone()),
        title: args.get("title").and_then(Value::as_str).map(str::to_owned),
        include_minimized: args
            .get("include_minimized")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn parse_clipboard_content_from_args(args: &Value) -> Result<ClipboardContent, PilotError> {
    let explicit_format = args
        .get("format")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    match explicit_format.as_deref() {
        Some("html") => {
            let html = req_str(args, "html")?.to_owned();
            let fallback_text = args
                .get("fallback_text")
                .or_else(|| args.get("text"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            Ok(ClipboardContent::Html {
                html,
                fallback_text,
            })
        }
        Some("png") => {
            let b64 = args
                .get("base64")
                .or_else(|| args.get("png_base64"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    PilotError::invalid_input("clipboard_set png format requires `base64`")
                })?;
            use base64::Engine as _;
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(b64.trim())
                .map_err(|err| PilotError::invalid_input(format!("invalid png base64: {err}")))?;
            if decoded.len() > MAX_PNG_INPUT_BYTES {
                return Err(PilotError::invalid_input(format!(
                    "clipboard png payload exceeds {} byte limit",
                    MAX_PNG_INPUT_BYTES
                )));
            }
            if !decoded.starts_with(b"\x89PNG\r\n\x1a\n") {
                return Err(PilotError::invalid_input(
                    "clipboard png payload missing PNG signature",
                ));
            }
            Ok(ClipboardContent::Png {
                base64: b64.trim().to_owned(),
                byte_len: decoded.len(),
            })
        }
        Some("files") => {
            let paths = args
                .get("paths")
                .or_else(|| args.get("files"))
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    PilotError::invalid_input("clipboard_set files format requires `paths` array")
                })?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if paths.is_empty() {
                return Err(PilotError::invalid_input(
                    "clipboard_set files requires at least one path",
                ));
            }
            Ok(ClipboardContent::Files { paths })
        }
        _ => {
            if let Some(html) = args.get("html").and_then(Value::as_str) {
                return Ok(ClipboardContent::Html {
                    html: html.to_owned(),
                    fallback_text: args.get("text").and_then(Value::as_str).map(str::to_owned),
                });
            }
            if let Some(paths_val) = args.get("paths").or_else(|| args.get("files")).and_then(Value::as_array) {
                let paths: Vec<String> = paths_val
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                if !paths.is_empty() {
                    return Ok(ClipboardContent::Files { paths });
                }
            }
            let text = req_str(args, "text")?.to_owned();
            Ok(ClipboardContent::Text { text })
        }
    }
}

fn parse_locator_from_args(args: &Value) -> LocatorQuery {
    let raw_nth = args.get("nth").and_then(Value::as_i64);
    let nth = raw_nth.and_then(|n| (n >= 0).then_some(n as usize));
    let nth_signed = raw_nth.map(|n| n as isize);
    LocatorQuery {
        identity: crate::locator::IdentityPredicate {
            role: args.get("role").and_then(Value::as_str).map(str::to_owned),
            name: args.get("name").and_then(Value::as_str).map(str::to_owned),
            name_contains: args
                .get("name_contains")
                .and_then(Value::as_str)
                .map(str::to_owned),
            identifier: args
                .get("identifier")
                .and_then(Value::as_str)
                .map(str::to_owned),
            value_contains: args
                .get("value_contains")
                .and_then(Value::as_str)
                .map(str::to_owned),
            label: args.get("label").and_then(Value::as_str).map(str::to_owned),
            placeholder: args
                .get("placeholder")
                .and_then(Value::as_str)
                .map(str::to_owned),
            testid: args.get("testid").and_then(Value::as_str).map(str::to_owned),
            xpath: args.get("xpath").and_then(Value::as_str).map(str::to_owned),
            exact: args.get("exact").and_then(Value::as_bool),
        },
        state: crate::locator::StatePredicate {
            enabled: args.get("enabled").and_then(Value::as_bool),
            focused: args.get("focused").and_then(Value::as_bool),
            checked: args.get("checked").and_then(Value::as_bool),
            selected: args.get("selected").and_then(Value::as_bool),
            expanded: args.get("expanded").and_then(Value::as_bool),
            interactive_only: args
                .get("interactive")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        },
        containment: crate::locator::ContainmentPredicate {
            within_ref: args.get("in").and_then(Value::as_str).map(str::to_owned),
        },
        nth,
        nth_signed,
        all: args.get("all").and_then(Value::as_bool).unwrap_or(false),
    }
}

fn create_native_adapter() -> Box<dyn PlatformAdapter> {
    #[cfg(all(target_os = "windows", not(target_family = "wasm")))]
    {
        return Box::new(crate::backend_windows::WindowsAdapter::new());
    }
    #[cfg(all(target_os = "linux", not(target_family = "wasm")))]
    {
        return Box::new(crate::backend_linux::LinuxAdapter::new());
    }
    #[cfg(all(target_os = "macos", not(target_family = "wasm")))]
    {
        return Box::new(crate::backend_macos::MacOsAdapter::new());
    }
    #[cfg(any(
        target_family = "wasm",
        not(any(target_os = "windows", target_os = "linux", target_os = "macos"))
    ))]
    {
        Box::new(GpuiCompositorAdapter::new())
    }
}
