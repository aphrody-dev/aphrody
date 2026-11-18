#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

//! Unified cross-platform AI OS pilotage engine for Aphrody GPUI OS.
//!
//! Migrates and consolidates the operational computer-use core and native platform adapters
//! (`UIA` on Windows, `AT-SPI2` + `X11`/`Wayland` on Linux, `AXUIElement` on macOS) into clean
//! Zed/GPUI modules without `mod.rs` boilerplate, while fusing live `accesskit::TreeUpdate` and
//! COSMIC `aphrody_session::CompositorState` control for in-process headless and WebGPU/Wasm pilotage.

pub mod action;
pub mod adapter;
#[cfg(all(target_os = "linux", not(target_family = "wasm")))]
pub mod backend_linux;
#[cfg(all(target_os = "macos", not(target_family = "wasm")))]
pub mod backend_macos;
#[cfg(all(target_os = "windows", not(target_family = "wasm")))]
pub mod backend_windows;
pub mod browser;
pub mod gpui_bridge;
pub mod locator;
pub mod model;
pub mod policy;
pub mod refs;
pub mod zed_headless;

pub use browser::{
    AnnotatedElementBox, AnnotatedScreenshot, BrowserActionPolicy, BrowserBatchReport,
    BrowserBatchStep, BrowserBatchStepResult, BrowserControllerState, BrowserCookie,
    BrowserEventRingBuffer, BrowserLaunchOptions, BrowserPolicyDecision, BrowserStorageState,
    BrowserTabSession, BrowserUrlCheck, ConsoleLogEntry, DiffLine, DiffLineKind, DomainFilter,
    HarContentMode, HarEntry, HarRingBuffer, NetworkRoute, NetworkRouteTable, OriginStorage,
    PageErrorEntry, RouteDisposition, RouteResponse, SemanticLocator, SemanticLocatorKind,
    SnapshotDiff, StorageEntry, TabRef, TabSessionManager, build_annotation_overlay_js,
    build_chrome_launch_args, cleanup_annotation_overlay_js, diff_snapshot_text,
    filter_non_overlapping_annotations, find_ax_nodes_by_locator, full_page_clip_size,
    matches_status_filter, normalize_ax_role, project_annotation_boxes, redact_header_map,
    route_url_matches, snapshot_comparison_text, validate_navigation_url,
};
#[cfg(not(target_family = "wasm"))]
pub use zed_headless::run_headless_mcp_stdio;
pub use zed_headless::{
    HeadlessBuffer, HeadlessZedController, SyntaxLanguage, TreeSitterReport, TreeSitterSymbol,
    analyze_with_tree_sitter, builtin_headless_and_browser_tools, dispatch_headless_mcp_line,
};

pub use action::{
    Action, ActionRequest, ActionResult, ActionStepOutcome, Direction, DragParams,
    InteractionPolicy, KeyCombo, Modifier, MouseButton, MouseEvent, MouseEventKind,
    PostconditionVerdict, evaluate_postcondition, execute_verified_action, postcondition_satisfied,
};
pub use adapter::{
    CommandContext, Deadline, HeldInputState, HudActivityItem, InteractionLease, LiveElement,
    NativeHandle, PilotEngine, PlatformAdapter, TargetBackend,
};
pub use gpui_bridge::GpuiCompositorAdapter;
pub use locator::{
    ContainmentPredicate, IdentityPredicate, LocatorMatch, LocatorQuery, LocatorResolution,
    ObservationBudgetStats, ObservationRequest, ObservedTree, StatePredicate,
    evaluate_locator_tree, render_snapshot_text, resolve_query, resolve_unique,
};
pub use model::{
    AccessibilityNode, AppInfo, ClipboardContent, ClipboardFormat, CommandEnvelope,
    DeliveryDisposition, DeliverySemantics, DisplayInfo, ENVELOPE_VERSION, ElementState, ErrorCode,
    ImageBuffer, ImageFormat, NotificationInfo, PermissionReport, PilotError, ProcessIdentity,
    Rect, RetryDisposition, RetryPolicy, SnapshotSurface, StatusReport, SurfaceInfo, WindowFilter,
    WindowInfo,
};
pub use policy::{
    AiActOutcome, AiActPlan, BatchExecutionState, BatchReport, BatchStep, BatchStepResult,
    MAX_BATCH_ENTRIES, MAX_BATCH_OUTPUT_BYTES, PermissionNeed, TraceEvent, TraceRecorder,
    WaitOutcome, WaitSelector, act_verify, ai_act_verify, execute_batch, execute_batch_with_limits,
    wait_for_selector,
};
pub use refs::{
    IdentityMatch, RefCapabilities, RefEntry, RefEntryIdentity, RefMap, RefScope, RefStore,
    new_snapshot_id, stamp_tree_refs, validate_ref_id, validate_session_id, validate_snapshot_id,
    verify_entry_identity,
};

use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize)]
pub struct PilotToolDescriptor {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Complete list of AI OS pilotage tools merged into the unified `aphrody-os` / `aphrody_craft` MCP
/// surface, matching CLI `aphrody-os pilot <op>` 1:1.
#[must_use]
pub fn builtin_pilot_tools() -> Vec<PilotToolDescriptor> {
    let empty_obj = || json!({"type": "object", "properties": {}, "additionalProperties": true});
    let ref_obj = |desc: &'static str| {
        let _ = desc;
        json!({
            "type": "object",
            "properties": {
                "ref": { "type": "string", "description": "Snapshot ref ID e.g. @s1a2b3c4:e5" },
                "target": { "type": "string", "enum": ["auto", "gpui", "native"] },
                "timeout_ms": { "type": "integer", "minimum": 50 }
            },
            "required": ["ref"],
            "additionalProperties": true
        })
    };
    let win_filter_obj = || {
        json!({
            "type": "object",
            "properties": {
                "app": { "type": "string" },
                "window_id": { "type": "string" },
                "title": { "type": "string" },
                "target": { "type": "string", "enum": ["auto", "gpui", "native"] }
            },
            "additionalProperties": true
        })
    };

    vec![
        PilotToolDescriptor {
            name: "desktop_status",
            description: "Inspect active AI OS pilot backend, session, displays, and accessibility readiness.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_permissions",
            description: "Check OS accessibility, screen-recording, display-server, and UIPI elevation status.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_list_apps",
            description: "List discoverable/running desktop applications across GPUI compositor or native OS.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_list_windows",
            description: "Enumerate open desktop/compositor windows with geometry, focus, min/max, and process token.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_list_displays",
            description: "List attached displays with bounds, primary flag, and effective DPI scale.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_list_surfaces",
            description: "List inspectable OS/GPUI surfaces (window, menubar, dock, tray, desktop).",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_snapshot",
            description: "Capture a live Accessibility Tree snapshot (UIA/AT-SPI2/AX or in-process GPUI AccessKit) with @ref IDs.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "app": { "type": "string" },
                    "window_id": { "type": "string" },
                    "surface": { "type": "string" },
                    "interactive": { "type": "boolean" },
                    "compact": { "type": "boolean" },
                    "max_depth": { "type": "integer", "minimum": 1, "maximum": 32 },
                    "target": { "type": "string", "enum": ["auto", "gpui", "native"] }
                },
                "additionalProperties": true
            }),
        },
        PilotToolDescriptor {
            name: "desktop_find",
            description: "Locate accessibility elements by role, name, name_contains, identifier, state, or within-ref.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_get",
            description: "Read fresh live properties and state for an existing @ref element.",
            input_schema: ref_obj("Get element"),
        },
        PilotToolDescriptor {
            name: "desktop_is",
            description: "Check a boolean state (enabled, focused, checked, selected, expanded) on a @ref element.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ref": { "type": "string" },
                    "state": { "type": "string" }
                },
                "required": ["ref", "state"],
                "additionalProperties": true
            }),
        },
        PilotToolDescriptor {
            name: "desktop_click",
            description: "Invoke/click element by @ref.",
            input_schema: ref_obj("Click"),
        },
        PilotToolDescriptor {
            name: "desktop_double_click",
            description: "Double-click element by @ref.",
            input_schema: ref_obj("Double click"),
        },
        PilotToolDescriptor {
            name: "desktop_triple_click",
            description: "Triple-click element by @ref.",
            input_schema: ref_obj("Triple click"),
        },
        PilotToolDescriptor {
            name: "desktop_right_click",
            description: "Context/right-click element by @ref.",
            input_schema: ref_obj("Right click"),
        },
        PilotToolDescriptor {
            name: "desktop_type",
            description: "Type text into a target @ref element with post-action value verification.",
            input_schema: json!({
                "type": "object",
                "properties": { "ref": { "type": "string" }, "text": { "type": "string" } },
                "required": ["ref", "text"],
                "additionalProperties": true
            }),
        },
        PilotToolDescriptor {
            name: "desktop_set_value",
            description: "Set control value directly on a @ref element via Accessibility ValuePattern.",
            input_schema: json!({
                "type": "object",
                "properties": { "ref": { "type": "string" }, "value": { "type": "string" } },
                "required": ["ref", "value"],
                "additionalProperties": true
            }),
        },
        PilotToolDescriptor {
            name: "desktop_clear",
            description: "Clear value of an editable @ref element.",
            input_schema: ref_obj("Clear"),
        },
        PilotToolDescriptor {
            name: "desktop_focus",
            description: "Move keyboard focus onto a @ref element.",
            input_schema: ref_obj("Focus"),
        },
        PilotToolDescriptor {
            name: "desktop_select",
            description: "Select a list/tab/menu item by @ref or option name.",
            input_schema: json!({
                "type": "object",
                "properties": { "ref": { "type": "string" }, "item": { "type": "string" } },
                "required": ["ref"],
                "additionalProperties": true
            }),
        },
        PilotToolDescriptor {
            name: "desktop_toggle",
            description: "Toggle a switch/checkbox by @ref and verify post-toggle state.",
            input_schema: ref_obj("Toggle"),
        },
        PilotToolDescriptor {
            name: "desktop_check",
            description: "Set checkbox/switch to checked by @ref.",
            input_schema: ref_obj("Check"),
        },
        PilotToolDescriptor {
            name: "desktop_uncheck",
            description: "Set checkbox/switch to unchecked by @ref.",
            input_schema: ref_obj("Uncheck"),
        },
        PilotToolDescriptor {
            name: "desktop_expand",
            description: "Expand tree/disclosure/combobox element by @ref.",
            input_schema: ref_obj("Expand"),
        },
        PilotToolDescriptor {
            name: "desktop_collapse",
            description: "Collapse tree/disclosure/combobox element by @ref.",
            input_schema: ref_obj("Collapse"),
        },
        PilotToolDescriptor {
            name: "desktop_scroll",
            description: "Scroll container @ref up/down/left/right.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ref": { "type": "string" },
                    "direction": { "type": "string", "enum": ["up", "down", "left", "right"] },
                    "amount": { "type": "integer" }
                },
                "required": ["ref", "direction"],
                "additionalProperties": true
            }),
        },
        PilotToolDescriptor {
            name: "desktop_scroll_to",
            description: "Scroll @ref element into view.",
            input_schema: ref_obj("Scroll into view"),
        },
        PilotToolDescriptor {
            name: "desktop_press",
            description: "Synthesize a keyboard key or modifier combo (e.g. 'ctrl+k', 'enter').",
            input_schema: json!({
                "type": "object",
                "properties": { "key": { "type": "string" } },
                "required": ["key"],
                "additionalProperties": true
            }),
        },
        PilotToolDescriptor {
            name: "desktop_key_down",
            description: "Hold a key or modifier down inside stateful session daemon.",
            input_schema: json!({ "type": "object", "properties": { "key": { "type": "string" } }, "required": ["key"], "additionalProperties": true }),
        },
        PilotToolDescriptor {
            name: "desktop_key_up",
            description: "Release a held key or modifier.",
            input_schema: json!({ "type": "object", "properties": { "key": { "type": "string" } }, "required": ["key"], "additionalProperties": true }),
        },
        PilotToolDescriptor {
            name: "desktop_hover",
            description: "Move pointer cursor to absolute (x, y) coordinates.",
            input_schema: json!({ "type": "object", "properties": { "x": { "type": "number" }, "y": { "type": "number" } }, "required": ["x", "y"], "additionalProperties": true }),
        },
        PilotToolDescriptor {
            name: "desktop_drag",
            description: "Smoothly drag from (from_x, from_y) to (to_x, to_y).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "from_x": { "type": "number" }, "from_y": { "type": "number" },
                    "to_x": { "type": "number" }, "to_y": { "type": "number" }
                },
                "required": ["from_x", "from_y", "to_x", "to_y"],
                "additionalProperties": true
            }),
        },
        PilotToolDescriptor {
            name: "desktop_mouse_move",
            description: "Move mouse cursor to (x, y).",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_mouse_click",
            description: "Click mouse at (x, y).",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_mouse_down",
            description: "Hold mouse button down at (x, y).",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_mouse_up",
            description: "Release mouse button at (x, y).",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_mouse_wheel",
            description: "Emit mouse wheel scroll delta (dx, dy).",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_launch",
            description: "Launch an application in compositor or native OS.",
            input_schema: json!({ "type": "object", "properties": { "app": { "type": "string" } }, "required": ["app"], "additionalProperties": true }),
        },
        PilotToolDescriptor {
            name: "desktop_close_app",
            description: "Close/quit a running application cleanly.",
            input_schema: json!({ "type": "object", "properties": { "app": { "type": "string" } }, "required": ["app"], "additionalProperties": true }),
        },
        PilotToolDescriptor {
            name: "desktop_focus_window",
            description: "Bring a matching window to foreground focus.",
            input_schema: win_filter_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_move_window",
            description: "Move a window to (x, y).",
            input_schema: win_filter_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_resize_window",
            description: "Resize a window to (width, height).",
            input_schema: win_filter_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_minimize",
            description: "Minimize a matching window.",
            input_schema: win_filter_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_maximize",
            description: "Maximize a matching window.",
            input_schema: win_filter_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_restore",
            description: "Restore a minimized/maximized window.",
            input_schema: win_filter_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_open_system_surface",
            description: "Open or inspect an OS system surface (dock, menubar, tray, menu, alert).",
            input_schema: json!({ "type": "object", "properties": { "surface": { "type": "string" } }, "required": ["surface"], "additionalProperties": true }),
        },
        PilotToolDescriptor {
            name: "desktop_clipboard_get",
            description: "Read clipboard content.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_clipboard_set",
            description: "Write text to clipboard.",
            input_schema: json!({ "type": "object", "properties": { "text": { "type": "string" } }, "required": ["text"], "additionalProperties": true }),
        },
        PilotToolDescriptor {
            name: "desktop_clipboard_clear",
            description: "Clear clipboard content.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_list_notifications",
            description: "List pending OS/compositor notifications.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_dismiss_notification",
            description: "Dismiss notification by id.",
            input_schema: json!({ "type": "object", "properties": { "id": { "type": "string" } }, "required": ["id"], "additionalProperties": true }),
        },
        PilotToolDescriptor {
            name: "desktop_dismiss_all_notifications",
            description: "Dismiss all pending notifications.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_notification_action",
            description: "Trigger an action button on a notification.",
            input_schema: json!({ "type": "object", "properties": { "id": { "type": "string" }, "action": { "type": "string" } }, "required": ["id", "action"], "additionalProperties": true }),
        },
        PilotToolDescriptor {
            name: "desktop_screenshot",
            description: "Capture PNG screenshot of window or display.",
            input_schema: win_filter_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_wait",
            description: "Wait with bounded deadline for element/state/text/window predicate.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_batch",
            description: "Execute multiple AI OS steps sequentially with timing & stop-on-error.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_session",
            description: "Inspect or switch stateful AI pilot session & held input state.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_trace",
            description: "Start, stop, or inspect JSONL execution trace telemetry.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_act_verify",
            description: "One-shot observe -> locate -> act -> readback-verify -> diff-snapshot transaction with retry disposition.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "desktop_ai_act_verify",
            description: "One-shot observe -> locate -> act -> post-verify -> diff-snapshot transaction for AI computer use.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_status",
            description: "Inspect absorbed agent-browser controller state and discovered Chrome/Chromium binary.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_validate_url",
            description: "Check a URL against agent-browser SSRF / private IP / IMDS / scheme policy.",
            input_schema: json!({"type": "object", "properties": {"url": {"type": "string"}}, "required": ["url"], "additionalProperties": true}),
        },
        PilotToolDescriptor {
            name: "browser_connect_cdp",
            description: "Attach to Chrome DevTools Protocol /json/version WebSocket.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_open",
            description: "Validate and navigate active CDP tab to a URL.",
            input_schema: json!({"type": "object", "properties": {"url": {"type": "string"}}, "required": ["url"], "additionalProperties": true}),
        },
        PilotToolDescriptor {
            name: "browser_snapshot",
            description: "Capture CDP Accessibility.getFullAXTree snapshot from active browser page.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_snapshot_diff",
            description: "Compute line-level SnapshotDiff against the previous browser snapshot baseline.",
            input_schema: json!({"type": "object", "properties": {"after": {"type": "string"}, "before": {"type": "string"}}, "required": ["after"], "additionalProperties": true}),
        },
        PilotToolDescriptor {
            name: "browser_find",
            description: "Resolve semantic locators (role, text, label, placeholder, testid, xpath, nth) against CDP AX/DOM trees.",
            input_schema: json!({"type": "object", "properties": {"by": {"type": "string"}, "query": {"type": "string"}, "name": {"type": "string"}, "exact": {"type": "boolean"}, "nth": {"type": "integer"}}, "required": ["query"], "additionalProperties": true}),
        },
        PilotToolDescriptor {
            name: "browser_eval",
            description: "Evaluate JavaScript inside active CDP page context.",
            input_schema: json!({"type": "object", "properties": {"expression": {"type": "string"}}, "required": ["expression"], "additionalProperties": true}),
        },
        PilotToolDescriptor {
            name: "browser_screenshot",
            description: "Capture PNG screenshot via CDP Page.captureScreenshot.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_screenshot_annotated",
            description: "Filter/generate non-overlapping AnnotatedElementBox overlay metadata and script.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_route",
            description: "Register a URL glob route intercept rule (fulfill, abort, continue).",
            input_schema: json!({"type": "object", "properties": {"pattern": {"type": "string"}, "disposition": {"type": "string"}}, "required": ["pattern"], "additionalProperties": true}),
        },
        PilotToolDescriptor {
            name: "browser_unroute",
            description: "Remove matching browser network route intercepts.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_har_start",
            description: "Start bounded HAR 1.2 network capture ringbuffer.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_har_stop",
            description: "Stop HAR capture and export redacted HAR 1.2 JSON.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_console",
            description: "Inspect buffered browser console log entries.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_errors",
            description: "Inspect buffered browser page error entries.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_state_save",
            description: "Serialize browser cookies and origin storage state to JSON or path.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_state_load",
            description: "Load serialized browser cookies and origin storage state.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_cookies",
            description: "List, set, or clear browser cookies in controller storage.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_storage",
            description: "Read or update origin-scoped browser storage entries.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_tab_list",
            description: "List managed browser tabs with t<N> refs and active index.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_tab_new",
            description: "Open and focus a managed browser tab after URL policy validation.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_tab_switch",
            description: "Switch active managed browser tab by t<N> ref, label, target_id, or index.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_tab_close",
            description: "Close a managed browser tab.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_policy_check",
            description: "Load and evaluate fail-closed browser action policy rules and confirmation tokens.",
            input_schema: empty_obj(),
        },
        PilotToolDescriptor {
            name: "browser_batch",
            description: "Execute sequential browser tool steps with stop-on-error and policy gating.",
            input_schema: empty_obj(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::ObservationOps as _;
    use accesskit::{
        Action as AkAction, Node as AkNode, Role as AkRole, TreeId, TreeInfo, TreeUpdate,
    };

    #[test]
    fn gpui_compositor_full_ai_pilot_loop_verifies_actions_and_refs() {
        let mut engine = PilotEngine::new();
        engine.set_target(TargetBackend::Gpui);
        let ctx = CommandContext::default();

        let status = engine.status().expect("status succeeds");
        assert_eq!(status.backend, "gpui-compositor");
        assert!(status.gpui_compositor_linked);
        assert!(status.permissions.accessibility_granted);

        let snap = engine
            .execute_tool("desktop_snapshot", &json!({ "interactive": true }), &ctx)
            .expect("snapshot succeeds");
        let snap_id = snap["snapshot_id"].as_str().expect("snapshot_id present");
        assert!(snap_id.starts_with('s'));
        assert!(snap["ref_count"].as_u64().unwrap_or(0) >= 6);

        let found = engine
            .execute_tool(
                "desktop_find",
                &json!({ "role": "switch", "name": "Dark Appearance" }),
                &ctx,
            )
            .expect("find succeeds");
        let dark_ref = found["matches"][0]["ref_id"]
            .as_str()
            .expect("dark toggle ref present")
            .to_owned();

        let toggled = engine
            .execute_tool("desktop_toggle", &json!({ "ref": dark_ref }), &ctx)
            .expect("toggle dark succeeds");
        assert_eq!(toggled["verified"], true);
        assert_eq!(toggled["after_state"]["checked"], false);

        let input_found = engine
            .execute_tool("desktop_find", &json!({ "role": "textfield" }), &ctx)
            .expect("find textfield");
        let input_ref = input_found["matches"][0]["ref_id"]
            .as_str()
            .expect("input ref present")
            .to_owned();
        let typed = engine
            .execute_tool(
                "desktop_set_value",
                &json!({ "ref": input_ref, "value": "photocraft" }),
                &ctx,
            )
            .expect("set value succeeds");
        assert_eq!(typed["verified"], true);
        assert_eq!(typed["after_state"]["value"], "photocraft");

        let shot = engine
            .execute_tool("desktop_screenshot", &json!({}), &ctx)
            .expect("screenshot encodes PNG");
        assert_eq!(shot["format"], "png");
        assert!(shot["byte_len"].as_u64().unwrap_or(0) > 100);
    }

    #[test]
    fn accesskit_tree_update_feeds_live_gpui_pilot_and_queues_actions() {
        let mut engine = PilotEngine::new();
        let mut root = AkNode::new(AkRole::Window);
        root.set_label("ArtCraft Inspector");
        root.set_children([accesskit::NodeId(2)]);

        let mut btn = AkNode::new(AkRole::Button);
        btn.set_label("Export SVG");
        btn.add_action(AkAction::Click);
        btn.set_bounds(accesskit::Rect::new(10.0, 20.0, 120.0, 48.0));

        engine.gpui.update_accesskit_tree(
            "1",
            TreeUpdate {
                nodes: vec![(accesskit::NodeId(1), root), (accesskit::NodeId(2), btn)],
                tree: Some(TreeInfo::new(accesskit::NodeId(1))),
                tree_id: TreeId::ROOT,
                focus: accesskit::NodeId(2),
            },
        );

        let ctx = CommandContext {
            target: TargetBackend::Gpui,
            window_id: Some("1".to_owned()),
            ..Default::default()
        };
        let ai_outcome = engine
            .execute_tool(
                "desktop_ai_act_verify",
                &json!({
                    "locator": { "role": "button", "name": "Export SVG" },
                    "action": { "kind": "click" },
                    "include_post_snapshot": true
                }),
                &ctx,
            )
            .expect("ai_act_verify succeeds");
        assert_eq!(ai_outcome["action_result"]["verified"], true);
        let queued = engine.gpui.drain_accesskit_actions();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].1.target_node, accesskit::NodeId(2));
        assert_eq!(queued[0].1.action, AkAction::Click);
    }

    #[test]
    fn ref_store_roundtrip_and_identity_verification_detects_staleness() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RefStore::at_dir(dir.path());
        let adapter = GpuiCompositorAdapter::new();
        let (_, raw, _) = adapter
            .observe_tree(
                &WindowFilter::default(),
                &ObservationRequest::default(),
                Deadline::from_timeout_ms(None),
            )
            .expect("observe ok");
        let (stamped, map) = stamp_tree_refs("sdeadbeef", None, raw, 1);
        assert!(stamped.children_count > 0);
        store.save_snapshot(&map).expect("save snapshot ok");

        let (loaded, first_entry) = store.resolve_ref("@sdeadbeef:e1").expect("resolve ref");
        assert_eq!(loaded.snapshot_id, "sdeadbeef");
        assert_eq!(
            verify_entry_identity(&first_entry, &stamped),
            IdentityMatch::Match
        );

        let wrong_node = AccessibilityNode {
            role: "slider".to_owned(),
            ..Default::default()
        };
        assert_eq!(
            verify_entry_identity(&first_entry, &wrong_node),
            IdentityMatch::NoMatch
        );
    }

    #[test]
    fn builtin_pilot_tool_catalog_covers_full_operational_surface() {
        let tools = builtin_pilot_tools();
        assert!(tools.len() >= 55);
        for required in [
            "desktop_status",
            "desktop_snapshot",
            "desktop_click",
            "desktop_type",
            "desktop_key_down",
            "desktop_key_up",
            "desktop_mouse_down",
            "desktop_mouse_up",
            "desktop_wait",
            "desktop_batch",
            "desktop_screenshot",
            "desktop_ai_act_verify",
        ] {
            assert!(
                tools.iter().any(|t| t.name == required),
                "missing pilot tool {required}"
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn native_windows_uia_permissions_and_displays_respond_live() {
        let mut engine = PilotEngine::new();
        let ctx = CommandContext {
            target: TargetBackend::Native,
            ..Default::default()
        };
        let perms = engine
            .execute_tool("desktop_permissions", &json!({}), &ctx)
            .expect("native permissions ok");
        assert_eq!(perms["platform"], "windows");
        assert_eq!(perms["accessibility_granted"], true);
        let displays = engine
            .execute_tool("desktop_list_displays", &json!({}), &ctx)
            .expect("native displays ok");
        assert!(!displays["displays"].as_array().expect("array").is_empty());
    }

    #[test]
    fn act_verify_and_fused_browser_tools_dispatch_through_engine() {
        let mut engine = PilotEngine::new();
        engine.set_target(TargetBackend::Gpui);
        let ctx = CommandContext::default();

        let act = engine
            .execute_tool(
                "desktop_act_verify",
                &json!({
                    "locator": { "role": "switch", "label": "Dark Appearance" },
                    "action": { "kind": "toggle" },
                    "include_post_snapshot": true
                }),
                &ctx,
            )
            .expect("desktop_act_verify succeeds");
        assert_eq!(act["action_result"]["verified"], true);
        assert_eq!(act["retry_disposition"], "unsafe");
        assert!(act["after_snapshot_diff"].is_object());

        let diff = engine
            .execute_tool(
                "browser_snapshot_diff",
                &json!({ "before": "a\nb", "after": "a\nc" }),
                &ctx,
            )
            .expect("browser_snapshot_diff succeeds");
        assert_eq!(diff["changed"], true);
        assert_eq!(diff["additions"], 1);
        assert_eq!(diff["removals"], 1);
    }
}
