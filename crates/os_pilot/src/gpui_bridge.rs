use std::collections::BTreeMap;
use std::io::Cursor;

use accesskit::{Action as AkAction, Node as AkNode, NodeId, Role as AkRole, TreeUpdate};
use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder as _, Rgba, RgbaImage};
use os_session::{
    CompositorState, Rect as SessionRect, SessionManifest, SessionSupervisor, TilingMode,
    ToplevelId, discover_system_apps,
};
use parking_lot::Mutex;

use crate::action::{
    Action, ActionRequest, ActionResult, ActionStepOutcome, DragParams, KeyCombo, MouseEvent,
    MouseEventKind, sleep_brief,
};
use crate::adapter::{
    ActionOps, Deadline, InputOps, InteractionLease, LiveElement, NativeHandle, ObservationOps,
    SystemOps,
};
use crate::locator::{ObservationRequest, find_node_by_ref};
use crate::model::{
    AccessibilityNode, AppInfo, ClipboardContent, ClipboardFormat, DeliverySemantics, DisplayInfo,
    ElementState, ErrorCode, ImageBuffer, ImageFormat, NotificationInfo, PermissionReport,
    PilotError, Rect, SnapshotSurface, SurfaceInfo, WindowFilter, WindowInfo,
};
use crate::refs::RefEntry;

#[must_use]
fn wasm_safe_pid() -> u32 {
    #[cfg(not(target_family = "wasm"))]
    {
        std::process::id()
    }
    #[cfg(target_family = "wasm")]
    {
        1
    }
}

#[derive(Debug, Clone)]
pub struct GpuiElementHandle {
    pub node_id: String,
    pub window_id: String,
    pub is_compositor_window: bool,
}

#[derive(Debug)]
struct GpuiVirtualOsState {
    compositor: CompositorState,
    #[allow(dead_code)]
    supervisor: Option<SessionSupervisor>,
    accesskit_trees: BTreeMap<String, TreeUpdate>,
    pending_actions: Vec<(String, accesskit::ActionRequest)>,
    synthetic_values: BTreeMap<String, String>,
    synthetic_checks: BTreeMap<String, bool>,
    synthetic_expanded: BTreeMap<String, bool>,
    focused_node: Option<String>,
    clipboard: ClipboardContent,
    notifications: Vec<NotificationInfo>,
    display: DisplayInfo,
    cursor_xy: (f64, f64),
    active_theme: String,
    dark_mode: bool,
    launched_apps: Vec<AppInfo>,
    next_notif_id: u64,
}

impl Default for GpuiVirtualOsState {
    fn default() -> Self {
        let mut compositor = CompositorState::new(
            SessionRect::new(0.0, 0.0, 1440.0, 900.0),
            &["Studio", "Browser", "Code", "Monitor"],
            TilingMode::Dwindle,
        );
        compositor.open_window("ArtCraft Studio", "photocraft");
        compositor.open_window("System Monitor", "system-monitor");

        let supervisor = SessionManifest::load_with_user_override()
            .ok()
            .and_then(|m| SessionSupervisor::new(m).ok());

        Self {
            compositor,
            supervisor,
            accesskit_trees: BTreeMap::new(),
            pending_actions: Vec::new(),
            synthetic_values: BTreeMap::new(),
            synthetic_checks: BTreeMap::new(),
            synthetic_expanded: BTreeMap::new(),
            focused_node: None,
            clipboard: ClipboardContent::Text {
                text: String::new(),
            },
            notifications: vec![NotificationInfo {
                id: "notif-1".to_owned(),
                app: "aphrody-os".to_owned(),
                title: "AI Pilot Ready".to_owned(),
                body: "GPUI AccessKit + COSMIC compositor bridge linked.".to_owned(),
                actions: vec!["inspect".to_owned(), "dismiss".to_owned()],
                timestamp_ms: 1,
            }],
            display: DisplayInfo {
                id: "gpui-virtual-0".to_owned(),
                bounds: Rect::new(0.0, 0.0, 1440.0, 900.0),
                is_primary: true,
                scale: 1.0,
            },
            cursor_xy: (720.0, 450.0),
            active_theme: "aphrody".to_owned(),
            dark_mode: true,
            launched_apps: Vec::new(),
            next_notif_id: 2,
        }
    }
}

/// Cross-platform (Native + Wasm) bridge between Aphrody's GPUI `accesskit::TreeUpdate`,
/// COSMIC `os_session::CompositorState`, and the unified `PlatformAdapter` trait.
pub struct GpuiCompositorAdapter {
    state: Mutex<GpuiVirtualOsState>,
}

impl Default for GpuiCompositorAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuiCompositorAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(GpuiVirtualOsState::default()),
        }
    }

    pub fn update_accesskit_tree(&self, window_id: impl Into<String>, update: TreeUpdate) {
        self.state
            .lock()
            .accesskit_trees
            .insert(window_id.into(), update);
    }

    pub fn sync_compositor(&self, compositor: CompositorState, theme: &str, dark: bool) {
        let mut st = self.state.lock();
        st.compositor = compositor;
        st.active_theme = theme.to_owned();
        st.dark_mode = dark;
    }

    #[must_use]
    pub fn export_compositor(&self) -> CompositorState {
        self.state.lock().compositor.clone()
    }

    pub fn drain_accesskit_actions(&self) -> Vec<(String, accesskit::ActionRequest)> {
        std::mem::take(&mut self.state.lock().pending_actions)
    }

    pub fn push_notification(
        &self,
        app: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> String {
        let mut st = self.state.lock();
        let next_id = st.next_notif_id;
        let id = format!("notif-{next_id}");
        st.next_notif_id += 1;
        st.notifications.push(NotificationInfo {
            id: id.clone(),
            app: app.into(),
            title: title.into(),
            body: body.into(),
            actions: vec!["open".to_owned(), "dismiss".to_owned()],
            timestamp_ms: next_id,
        });
        id
    }

    fn build_tree_for_state(
        &self,
        st: &GpuiVirtualOsState,
        filter: &WindowFilter,
    ) -> (WindowInfo, AccessibilityNode) {
        let placed = st.compositor.arrange_active_workspace();
        let target_win = find_matching_compositor_window(&placed, &st.compositor, filter)
            .unwrap_or_else(|| WindowInfo {
                id: "gpui-shell".to_owned(),
                title: format!("{} OS Desktop", brand::APP_NAME),
                app: brand::APP_SLUG.to_owned(),
                pid: wasm_safe_pid(),
                bounds: st.display.bounds,
                focused: true,
                minimized: false,
                maximized: false,
                visible: true,
                workspace: Some(st.compositor.active_workspace),
                process_token: Some("gpui-inproc".to_owned()),
            });

        if let Some(update) = st
            .accesskit_trees
            .get(&target_win.id)
            .or_else(|| st.accesskit_trees.values().next())
        {
            let root = convert_accesskit_update(update, st);
            return (target_win, root);
        }

        let root = build_synthetic_cosmic_tree(st, &placed, &target_win);
        (target_win, root)
    }
}

fn find_matching_compositor_window(
    placed: &[os_session::PlacedWindow],
    compositor: &CompositorState,
    filter: &WindowFilter,
) -> Option<WindowInfo> {
    let first = placed
        .iter()
        .filter(|pw| {
            if let Some(wid) = &filter.window_id
                && pw.id.to_string() != *wid
            {
                return false;
            }
            if let Some(app) = &filter.app
                && !pw.app_id.eq_ignore_ascii_case(app)
                && !pw
                    .title
                    .to_ascii_lowercase()
                    .contains(&app.to_ascii_lowercase())
            {
                return false;
            }
            if let Some(title) = &filter.title
                && !pw
                    .title
                    .to_ascii_lowercase()
                    .contains(&title.to_ascii_lowercase())
            {
                return false;
            }
            true
        })
        .min_by_key(|pw| !pw.focused)?;

    let minimized = compositor
        .toplevels
        .get(&first.id)
        .is_some_and(|t| t.minimized);
    Some(WindowInfo {
        id: first.id.to_string(),
        title: first.title.clone(),
        app: first.app_id.clone(),
        pid: wasm_safe_pid(),
        bounds: Rect::from_session_rect(first.rect),
        focused: first.focused,
        minimized,
        maximized: first.maximized,
        visible: !minimized,
        workspace: Some(compositor.active_workspace),
        process_token: Some("gpui-compositor".to_owned()),
    })
}

fn ak_role_to_str(role: AkRole) -> &'static str {
    match role {
        AkRole::Window => "window",
        AkRole::Button => "button",
        AkRole::TextInput | AkRole::MultilineTextInput | AkRole::Search => "textfield",
        AkRole::CheckBox => "checkbox",
        AkRole::Switch => "switch",
        AkRole::Link => "link",
        AkRole::MenuItem | AkRole::MenuItemCheckBox | AkRole::MenuItemRadio => "menuitem",
        AkRole::RadioButton => "radiobutton",
        AkRole::Tab => "tab",
        AkRole::TabList => "tablist",
        AkRole::Slider => "slider",
        AkRole::ComboBox => "combobox",
        AkRole::TreeItem => "treeitem",
        AkRole::Row => "row",
        AkRole::Cell => "cell",
        AkRole::Dialog => "dialog",
        AkRole::Toolbar => "toolbar",
        AkRole::Label => "statictext",
        AkRole::Image => "image",
        AkRole::Table => "table",
        AkRole::List => "list",
        AkRole::ListItem => "listitem",
        AkRole::Tree => "outline",
        AkRole::ScrollView => "scrollarea",
        AkRole::ScrollBar => "scrollbar",
        AkRole::Menu | AkRole::MenuBar => "menu",
        AkRole::ProgressIndicator => "progressbar",
        AkRole::Document => "document",
        AkRole::Heading => "heading",
        AkRole::Status => "status",
        AkRole::Tooltip => "tooltip",
        _ => "group",
    }
}

fn convert_accesskit_update(update: &TreeUpdate, st: &GpuiVirtualOsState) -> AccessibilityNode {
    let map: BTreeMap<NodeId, &AkNode> = update.nodes.iter().map(|(id, n)| (*id, n)).collect();
    let root_id = update
        .tree
        .as_ref()
        .map(|t| t.root)
        .or_else(|| update.nodes.first().map(|(id, _)| *id))
        .unwrap_or(NodeId(0));
    convert_ak_node(root_id, &map, update.focus, st, 0)
}

fn convert_ak_node(
    id: NodeId,
    map: &BTreeMap<NodeId, &AkNode>,
    focused_id: NodeId,
    st: &GpuiVirtualOsState,
    depth: usize,
) -> AccessibilityNode {
    let Some(node) = map.get(&id) else {
        return AccessibilityNode {
            id: format!("ak-{}", id.0),
            role: "group".to_owned(),
            ..Default::default()
        };
    };
    let id_str = format!("ak-{}", id.0);
    let role = ak_role_to_str(node.role()).to_owned();
    let name = node.label().map(str::to_owned);
    let override_val = st.synthetic_values.get(&id_str).cloned();
    let value = override_val.or_else(|| node.value().map(str::to_owned));
    let checked = st
        .synthetic_checks
        .get(&id_str)
        .copied()
        .or_else(|| node.toggled().map(|t| t == accesskit::Toggled::True));
    let expanded = st
        .synthetic_expanded
        .get(&id_str)
        .copied()
        .or_else(|| node.is_expanded());
    let focused = st.focused_node.as_deref() == Some(&id_str) || focused_id == id;
    let enabled = !node.is_disabled();

    let mut state = ElementState {
        value: value.clone(),
        enabled: Some(enabled),
        focused: Some(focused),
        checked,
        selected: node.is_selected(),
        expanded,
        states: Vec::new(),
    };
    state.push_flag(if enabled { "enabled" } else { "disabled" });
    if focused {
        state.push_flag("focused");
    }
    if checked == Some(true) {
        state.push_flag("checked");
    }
    if expanded == Some(true) {
        state.push_flag("expanded");
    }
    if node.is_selected() == Some(true) {
        state.push_flag("selected");
    }

    let bounds = node
        .bounds()
        .map(|b| Rect::new(b.x0, b.y0, (b.x1 - b.x0).max(0.0), (b.y1 - b.y0).max(0.0)));
    let actions = collect_ak_actions(node);
    let interactive = !actions.is_empty() || AccessibilityNode::is_interactive_role(&role);

    let children = if depth < 24 {
        node.children()
            .iter()
            .map(|cid| convert_ak_node(*cid, map, focused_id, st, depth + 1))
            .collect()
    } else {
        Vec::new()
    };

    AccessibilityNode {
        id: id_str,
        ref_id: None,
        role,
        name,
        value,
        description: node.description().map(str::to_owned),
        identifier: node.author_id().map(str::to_owned),
        hint: None,
        bounds,
        state,
        actions,
        interactive,
        children_count: node.children().len(),
        subtree_truncated: depth >= 24 && !node.children().is_empty(),
        children,
    }
}

fn collect_ak_actions(node: &AkNode) -> Vec<String> {
    let mut actions = Vec::new();
    for (ak_act, name) in [
        (AkAction::Click, "click"),
        (AkAction::Focus, "focus"),
        (AkAction::SetValue, "set_value"),
        (AkAction::Expand, "expand"),
        (AkAction::Collapse, "collapse"),
        (AkAction::ScrollIntoView, "scroll_to"),
    ] {
        if node.supports_action(ak_act) {
            actions.push(name.to_owned());
        }
    }
    actions
}

fn build_synthetic_cosmic_tree(
    st: &GpuiVirtualOsState,
    placed: &[os_session::PlacedWindow],
    target_win: &WindowInfo,
) -> AccessibilityNode {
    let mut ws_buttons = Vec::new();
    for (&ws_id, ws) in &st.compositor.workspaces {
        let active = ws_id == st.compositor.active_workspace;
        let id = format!("ws-btn-{ws_id}");
        let mut s = ElementState {
            enabled: Some(true),
            selected: Some(active),
            states: vec!["enabled".to_owned()],
            ..Default::default()
        };
        if active {
            s.push_flag("selected");
        }
        ws_buttons.push(AccessibilityNode {
            id,
            role: "button".to_owned(),
            name: Some(format!("Workspace {} ({})", ws_id, ws.name)),
            identifier: Some(format!("workspace.{ws_id}")),
            bounds: Some(Rect::new(
                16.0 + f64::from(ws_id - 1) * 96.0,
                6.0,
                88.0,
                32.0,
            )),
            state: s,
            actions: vec!["click".to_owned(), "focus".to_owned()],
            interactive: true,
            ..Default::default()
        });
    }

    let dark_id = "shell-theme-toggle".to_owned();
    let dark_checked = st
        .synthetic_checks
        .get(&dark_id)
        .copied()
        .unwrap_or(st.dark_mode);
    let mut dark_state = ElementState {
        enabled: Some(true),
        checked: Some(dark_checked),
        states: vec!["enabled".to_owned()],
        ..Default::default()
    };
    if dark_checked {
        dark_state.push_flag("checked");
    }

    let search_id = "shell-launcher-input".to_owned();
    let search_val = st
        .synthetic_values
        .get(&search_id)
        .cloned()
        .unwrap_or_default();
    let search_state = ElementState {
        enabled: Some(true),
        value: Some(search_val.clone()),
        focused: Some(st.focused_node.as_deref() == Some(&search_id)),
        states: vec!["enabled".to_owned()],
        ..Default::default()
    };

    let layout_mode = st
        .compositor
        .workspaces
        .get(&st.compositor.active_workspace)
        .map(|w| w.mode.slug())
        .unwrap_or("dwindle");

    let top_panel = AccessibilityNode {
        id: "cosmic-top-panel".to_owned(),
        role: "toolbar".to_owned(),
        name: Some("COSMIC Top Panel".to_owned()),
        bounds: Some(Rect::new(0.0, 0.0, st.display.bounds.width, 44.0)),
        state: ElementState {
            enabled: Some(true),
            states: vec!["enabled".to_owned()],
            ..Default::default()
        },
        children_count: ws_buttons.len() + 3,
        children: {
            let mut kids = ws_buttons;
            kids.push(AccessibilityNode {
                id: "shell-layout-cycle".to_owned(),
                role: "button".to_owned(),
                name: Some(format!("Layout: {layout_mode}")),
                identifier: Some("shell.layout".to_owned()),
                bounds: Some(Rect::new(480.0, 6.0, 140.0, 32.0)),
                state: ElementState {
                    enabled: Some(true),
                    value: Some(layout_mode.to_owned()),
                    states: vec!["enabled".to_owned()],
                    ..Default::default()
                },
                actions: vec!["click".to_owned()],
                interactive: true,
                ..Default::default()
            });
            kids.push(AccessibilityNode {
                id: dark_id,
                role: "switch".to_owned(),
                name: Some("Dark Appearance".to_owned()),
                identifier: Some("shell.dark_mode".to_owned()),
                bounds: Some(Rect::new(1240.0, 6.0, 120.0, 32.0)),
                state: dark_state,
                actions: vec!["click".to_owned(), "toggle".to_owned()],
                interactive: true,
                ..Default::default()
            });
            kids.push(AccessibilityNode {
                id: search_id,
                role: "textfield".to_owned(),
                name: Some("Launcher Query".to_owned()),
                identifier: Some("shell.launcher_input".to_owned()),
                value: Some(search_val),
                bounds: Some(Rect::new(640.0, 6.0, 280.0, 32.0)),
                state: search_state,
                actions: vec!["set_value".to_owned(), "focus".to_owned()],
                interactive: true,
                ..Default::default()
            });
            kids
        },
        ..Default::default()
    };

    let win_nodes: Vec<AccessibilityNode> = placed
        .iter()
        .map(|pw| {
            let wid = format!("comp-win-{}", pw.id);
            let mut wstate = ElementState {
                enabled: Some(true),
                focused: Some(pw.focused),
                states: vec!["enabled".to_owned()],
                ..Default::default()
            };
            if pw.focused {
                wstate.push_flag("focused");
            }
            AccessibilityNode {
                id: wid,
                role: "window".to_owned(),
                name: Some(pw.title.clone()),
                identifier: Some(format!("window.{}", pw.id)),
                bounds: Some(Rect::from_session_rect(pw.rect)),
                state: wstate,
                actions: vec!["click".to_owned(), "focus".to_owned()],
                interactive: true,
                ..Default::default()
            }
        })
        .collect();

    AccessibilityNode {
        id: "gpui-root".to_owned(),
        role: "window".to_owned(),
        name: Some(target_win.title.clone()),
        identifier: Some(format!("root.{}", target_win.id)),
        bounds: Some(st.display.bounds),
        state: ElementState {
            enabled: Some(true),
            focused: Some(true),
            states: vec!["enabled".to_owned(), "focused".to_owned()],
            ..Default::default()
        },
        children_count: 1 + win_nodes.len(),
        children: {
            let mut all = vec![top_panel];
            all.extend(win_nodes);
            all
        },
        ..Default::default()
    }
}

impl ObservationOps for GpuiCompositorAdapter {
    fn observe_tree(
        &self,
        root_filter: &WindowFilter,
        request: &ObservationRequest,
        deadline: Deadline,
    ) -> Result<(WindowInfo, AccessibilityNode, bool), PilotError> {
        deadline.ensure_remaining()?;
        let st = self.state.lock();
        let (window, mut root) = self.build_tree_for_state(&st, root_filter);
        if let Some(root_ref) = &request.root_ref
            && let Some(sub) = find_node_by_ref(&root, root_ref)
        {
            root = sub.clone();
        }
        Ok((window, root, true))
    }

    fn resolve_element(
        &self,
        entry: &RefEntry,
        deadline: Deadline,
    ) -> Result<(NativeHandle, AccessibilityNode), PilotError> {
        deadline.ensure_remaining()?;
        let st = self.state.lock();
        let filter = WindowFilter {
            app: entry.scope.as_ref().map(|s| s.app.clone()),
            window_id: entry.scope.as_ref().map(|s| s.window_id.clone()),
            ..Default::default()
        };
        let (win, root) = self.build_tree_for_state(&st, &filter);
        let found = find_node_by_ref(&root, &entry.node_id)
            .or_else(|| find_node_by_name_and_role(&root, entry))
            .cloned()
            .ok_or_else(|| PilotError::stale_ref(&entry.ref_id))?;

        let handle = NativeHandle::new(GpuiElementHandle {
            node_id: found.id.clone(),
            window_id: win.id,
            is_compositor_window: found.id.starts_with("comp-win-"),
        });
        Ok((handle, found))
    }

    fn get_live_element(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<LiveElement, PilotError> {
        deadline.ensure_remaining()?;
        let h = handle.downcast_ref::<GpuiElementHandle>().ok_or_else(|| {
            PilotError::invalid_input("native handle is not a GPUI element handle")
        })?;
        let st = self.state.lock();
        let filter = WindowFilter {
            window_id: Some(h.window_id.clone()),
            ..Default::default()
        };
        let (_, root) = self.build_tree_for_state(&st, &filter);
        let node = find_node_by_ref(&root, &h.node_id)
            .ok_or_else(|| PilotError::new(ErrorCode::ElementNotFound, "GPUI node disappeared"))?;
        Ok(LiveElement {
            id: node.id.clone(),
            role: node.role.clone(),
            name: node.name.clone(),
            bounds: node.bounds,
            state: node.state.clone(),
            actions: node.actions.clone(),
        })
    }

    fn list_surfaces(&self, deadline: Deadline) -> Result<Vec<SurfaceInfo>, PilotError> {
        deadline.ensure_remaining()?;
        let st = self.state.lock();
        Ok(vec![
            SurfaceInfo {
                id: "gpui-window".to_owned(),
                surface: SnapshotSurface::Window,
                app: brand::APP_SLUG.to_owned(),
                title: "COSMIC GPUI Compositor".to_owned(),
                bounds: Some(st.display.bounds),
                available: true,
            },
            SurfaceInfo {
                id: "gpui-menubar".to_owned(),
                surface: SnapshotSurface::Menubar,
                app: brand::APP_SLUG.to_owned(),
                title: "Top Panel Applets".to_owned(),
                bounds: Some(Rect::new(0.0, 0.0, st.display.bounds.width, 44.0)),
                available: true,
            },
            SurfaceInfo {
                id: "gpui-dock".to_owned(),
                surface: SnapshotSurface::Dock,
                app: brand::APP_SLUG.to_owned(),
                title: "Bottom Dock Bar".to_owned(),
                bounds: Some(Rect::new(
                    0.0,
                    st.display.bounds.height - 64.0,
                    st.display.bounds.width,
                    64.0,
                )),
                available: true,
            },
        ])
    }
}

fn find_node_by_name_and_role<'a>(
    node: &'a AccessibilityNode,
    entry: &RefEntry,
) -> Option<&'a AccessibilityNode> {
    if node.role.eq_ignore_ascii_case(&entry.identity.role) {
        if let Some(id) = &entry.identity.identifier
            && node.identifier.as_deref() == Some(id.as_str())
        {
            return Some(node);
        }
        if let Some(name) = &entry.identity.name
            && node.name.as_deref() == Some(name.as_str())
        {
            return Some(node);
        }
    }
    for child in &node.children {
        if let Some(found) = find_node_by_name_and_role(child, entry) {
            return Some(found);
        }
    }
    None
}

impl ActionOps for GpuiCompositorAdapter {
    fn execute_action(
        &self,
        handle: &NativeHandle,
        request: &ActionRequest,
        _lease: &InteractionLease,
    ) -> Result<ActionResult, PilotError> {
        let h = handle.downcast_ref::<GpuiElementHandle>().ok_or_else(|| {
            PilotError::invalid_input("native handle is not a GPUI element handle")
        })?;
        let mut st = self.state.lock();

        if let Some(ws_str) = h.node_id.strip_prefix("ws-btn-")
            && let Ok(ws_id) = ws_str.parse::<u32>()
        {
            st.compositor.switch_workspace(ws_id);
        } else if h.node_id == "shell-layout-cycle" {
            st.compositor.cycle_active_mode();
        } else if let Some(wid_str) = h.node_id.strip_prefix("comp-win-")
            && let Ok(wid) = wid_str.parse::<ToplevelId>()
        {
            st.compositor.focus_window(wid);
        }

        match &request.action {
            Action::Click | Action::DoubleClick | Action::TripleClick | Action::RightClick => {
                if h.node_id == "shell-theme-toggle" {
                    let next = !st.dark_mode;
                    st.dark_mode = next;
                    st.synthetic_checks.insert(h.node_id.clone(), next);
                }
            }
            Action::Focus => {
                st.focused_node = Some(h.node_id.clone());
            }
            Action::Check => {
                st.synthetic_checks.insert(h.node_id.clone(), true);
                if h.node_id == "shell-theme-toggle" {
                    st.dark_mode = true;
                }
            }
            Action::Uncheck => {
                st.synthetic_checks.insert(h.node_id.clone(), false);
                if h.node_id == "shell-theme-toggle" {
                    st.dark_mode = false;
                }
            }
            Action::Toggle => {
                let next = !st
                    .synthetic_checks
                    .get(&h.node_id)
                    .copied()
                    .unwrap_or(false);
                st.synthetic_checks.insert(h.node_id.clone(), next);
                if h.node_id == "shell-theme-toggle" {
                    st.dark_mode = next;
                }
            }
            Action::Expand => {
                st.synthetic_expanded.insert(h.node_id.clone(), true);
            }
            Action::Collapse => {
                st.synthetic_expanded.insert(h.node_id.clone(), false);
            }
            Action::SetValue(val) => {
                st.synthetic_values.insert(h.node_id.clone(), val.clone());
            }
            Action::TypeText(text) => {
                let entry = st.synthetic_values.entry(h.node_id.clone()).or_default();
                entry.push_str(text);
            }
            Action::Clear => {
                st.synthetic_values.insert(h.node_id.clone(), String::new());
            }
            _ => {}
        }

        if let Some(num) = h
            .node_id
            .strip_prefix("ak-")
            .and_then(|s| s.parse::<u64>().ok())
        {
            let ak_action = match request.action {
                Action::Focus => AkAction::Focus,
                Action::Expand => AkAction::Expand,
                Action::Collapse => AkAction::Collapse,
                Action::SetValue(_) | Action::TypeText(_) | Action::Clear => AkAction::SetValue,
                _ => AkAction::Click,
            };
            st.pending_actions.push((
                h.window_id.clone(),
                accesskit::ActionRequest {
                    action: ak_action,
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: NodeId(num),
                    data: match &request.action {
                        Action::SetValue(v) | Action::TypeText(v) => {
                            Some(accesskit::ActionData::Value(v.clone().into_boxed_str()))
                        }
                        Action::Clear => Some(accesskit::ActionData::Value(Box::from(""))),
                        _ => None,
                    },
                },
            ));
        }

        Ok(ActionResult {
            action: request.action.name().to_owned(),
            target_ref: None,
            disposition: DeliverySemantics::delivered_verified(),
            verified: true,
            duration_ms: 0,
            before_state: None,
            after_state: None,
            steps: vec![ActionStepOutcome {
                step: request.action.name().to_owned(),
                mechanism: "gpui_accesskit_compositor".to_owned(),
                ok: true,
                duration_ms: 0,
                detail: Some(h.node_id.clone()),
            }],
        })
    }
}

impl InputOps for GpuiCompositorAdapter {
    fn press_key(&self, _combo: &KeyCombo, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()
    }

    fn key_state(
        &self,
        _combo: &KeyCombo,
        _down: bool,
        deadline: Deadline,
    ) -> Result<(), PilotError> {
        deadline.ensure_remaining()
    }

    fn mouse_event(&self, event: &MouseEvent, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        st.cursor_xy = (event.x, event.y);
        if event.kind == MouseEventKind::Click {
            let placed = st.compositor.arrange_active_workspace();
            if let Some(pw) = placed.iter().find(|p| {
                let r = Rect::from_session_rect(p.rect);
                r.contains_point(event.x, event.y)
            }) {
                let wid = pw.id;
                st.compositor.focus_window(wid);
            }
        }
        Ok(())
    }

    fn drag(&self, params: &DragParams, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let steps = params.steps.max(1);
        let step_delay = std::time::Duration::from_millis(params.duration_ms / u64::from(steps));
        for i in 0..=steps {
            let t = f64::from(i) / f64::from(steps);
            self.state.lock().cursor_xy = (
                params.from_x + (params.to_x - params.from_x) * t,
                params.from_y + (params.to_y - params.from_y) * t,
            );
            if !step_delay.is_zero() {
                sleep_brief(step_delay);
            }
        }
        Ok(())
    }

    fn clipboard_get(
        &self,
        _format: ClipboardFormat,
        deadline: Deadline,
    ) -> Result<ClipboardContent, PilotError> {
        deadline.ensure_remaining()?;
        Ok(self.state.lock().clipboard.clone())
    }

    fn clipboard_set(
        &self,
        content: &ClipboardContent,
        deadline: Deadline,
    ) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        self.state.lock().clipboard = content.clone();
        Ok(())
    }

    fn clipboard_clear(&self, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        self.state.lock().clipboard = ClipboardContent::Text {
            text: String::new(),
        };
        Ok(())
    }
}

impl SystemOps for GpuiCompositorAdapter {
    fn backend_name(&self) -> &'static str {
        "gpui-compositor"
    }

    fn permissions(&self, deadline: Deadline) -> Result<PermissionReport, PilotError> {
        deadline.ensure_remaining()?;
        Ok(PermissionReport {
            platform: std::env::consts::OS.to_owned(),
            accessibility_granted: true,
            screen_recording_granted: true,
            elevation_status: Some("gpui-inproc".to_owned()),
            display_server: Some("gpui-compositor".to_owned()),
            accessibility_detail: Some(
                "GPUI AccessKit + COSMIC compositor linked in-process".to_owned(),
            ),
            screen_recording_detail: Some("Compositor rasterizer active".to_owned()),
        })
    }

    fn list_apps(&self, deadline: Deadline) -> Result<Vec<AppInfo>, PilotError> {
        deadline.ensure_remaining()?;
        let st = self.state.lock();
        let mut apps: Vec<AppInfo> = discover_system_apps(40)
            .into_iter()
            .enumerate()
            .map(|(i, e)| AppInfo {
                id: e.id,
                name: e.name,
                pid: wasm_safe_pid(),
                bundle_id: None,
                executable: Some(e.exec),
                active: i == 0,
            })
            .collect();
        apps.extend(st.launched_apps.clone());
        if apps.is_empty() {
            apps.push(AppInfo {
                id: brand::APP_SLUG.to_owned(),
                name: brand::APP_NAME.to_owned(),
                pid: wasm_safe_pid(),
                bundle_id: None,
                executable: None,
                active: true,
            });
        }
        Ok(apps)
    }

    fn list_windows(&self, deadline: Deadline) -> Result<Vec<WindowInfo>, PilotError> {
        deadline.ensure_remaining()?;
        let st = self.state.lock();
        let placed = st.compositor.arrange_active_workspace();
        Ok(placed
            .into_iter()
            .map(|pw| {
                let minimized = st
                    .compositor
                    .toplevels
                    .get(&pw.id)
                    .is_some_and(|t| t.minimized);
                WindowInfo {
                    id: pw.id.to_string(),
                    title: pw.title,
                    app: pw.app_id,
                    pid: wasm_safe_pid(),
                    bounds: Rect::from_session_rect(pw.rect),
                    focused: pw.focused,
                    minimized,
                    maximized: pw.maximized,
                    visible: !minimized,
                    workspace: Some(st.compositor.active_workspace),
                    process_token: Some("gpui-compositor".to_owned()),
                }
            })
            .collect())
    }

    fn list_displays(&self, deadline: Deadline) -> Result<Vec<DisplayInfo>, PilotError> {
        deadline.ensure_remaining()?;
        Ok(vec![self.state.lock().display.clone()])
    }

    fn focus_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        let placed = st.compositor.arrange_active_workspace();
        let win =
            find_matching_compositor_window(&placed, &st.compositor, filter).ok_or_else(|| {
                PilotError::new(ErrorCode::WindowNotFound, "no matching compositor window")
            })?;
        if let Ok(id) = win.id.parse::<ToplevelId>() {
            st.compositor.focus_window(id);
        }
        Ok(win)
    }

    fn move_window(
        &self,
        filter: &WindowFilter,
        x: f64,
        y: f64,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        let placed = st.compositor.arrange_active_workspace();
        let mut win =
            find_matching_compositor_window(&placed, &st.compositor, filter).ok_or_else(|| {
                PilotError::new(ErrorCode::WindowNotFound, "no matching compositor window")
            })?;
        if let Ok(id) = win.id.parse::<ToplevelId>()
            && let Some(t) = st.compositor.toplevels.get_mut(&id)
        {
            t.float_rect.x = x as f32;
            t.float_rect.y = y as f32;
            t.floating = true;
        }
        win.bounds.x = x;
        win.bounds.y = y;
        Ok(win)
    }

    fn resize_window(
        &self,
        filter: &WindowFilter,
        width: f64,
        height: f64,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        let placed = st.compositor.arrange_active_workspace();
        let mut win =
            find_matching_compositor_window(&placed, &st.compositor, filter).ok_or_else(|| {
                PilotError::new(ErrorCode::WindowNotFound, "no matching compositor window")
            })?;
        if let Ok(id) = win.id.parse::<ToplevelId>()
            && let Some(t) = st.compositor.toplevels.get_mut(&id)
        {
            t.float_rect.w = width.max(100.0) as f32;
            t.float_rect.h = height.max(80.0) as f32;
            t.floating = true;
        }
        win.bounds.width = width;
        win.bounds.height = height;
        Ok(win)
    }

    fn minimize_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        let placed = st.compositor.arrange_active_workspace();
        let mut win =
            find_matching_compositor_window(&placed, &st.compositor, filter).ok_or_else(|| {
                PilotError::new(ErrorCode::WindowNotFound, "no matching compositor window")
            })?;
        if let Ok(id) = win.id.parse::<ToplevelId>()
            && let Some(t) = st.compositor.toplevels.get_mut(&id)
        {
            t.minimized = true;
        }
        win.minimized = true;
        win.visible = false;
        Ok(win)
    }

    fn maximize_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        let placed = st.compositor.arrange_active_workspace();
        let mut win =
            find_matching_compositor_window(&placed, &st.compositor, filter).ok_or_else(|| {
                PilotError::new(ErrorCode::WindowNotFound, "no matching compositor window")
            })?;
        if let Ok(id) = win.id.parse::<ToplevelId>() {
            st.compositor.toggle_maximize(id);
        }
        win.maximized = true;
        Ok(win)
    }

    fn restore_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        let placed = st.compositor.arrange_active_workspace();
        let mut win =
            find_matching_compositor_window(&placed, &st.compositor, filter).ok_or_else(|| {
                PilotError::new(ErrorCode::WindowNotFound, "no matching compositor window")
            })?;
        if let Ok(id) = win.id.parse::<ToplevelId>()
            && let Some(t) = st.compositor.toplevels.get_mut(&id)
        {
            t.minimized = false;
            t.maximized = false;
        }
        win.minimized = false;
        win.maximized = false;
        win.visible = true;
        Ok(win)
    }

    fn launch_app(
        &self,
        app: &str,
        _args: &[String],
        deadline: Deadline,
    ) -> Result<AppInfo, PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        let wid = st.compositor.open_window(app, app);
        let info = AppInfo {
            id: format!("app-{wid}"),
            name: app.to_owned(),
            pid: wasm_safe_pid(),
            bundle_id: None,
            executable: None,
            active: true,
        };
        st.launched_apps.push(info.clone());
        Ok(info)
    }

    fn close_app(&self, app: &str, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        st.launched_apps
            .retain(|a| !a.name.eq_ignore_ascii_case(app) && !a.id.eq_ignore_ascii_case(app));
        let matching_ids: Vec<ToplevelId> = st
            .compositor
            .toplevels
            .values()
            .filter(|t| t.app_id.eq_ignore_ascii_case(app) || t.title.eq_ignore_ascii_case(app))
            .map(|t| t.id)
            .collect();
        for id in matching_ids {
            st.compositor.close_window(id);
        }
        Ok(())
    }

    fn open_system_surface(
        &self,
        surface: SnapshotSurface,
        deadline: Deadline,
    ) -> Result<SurfaceInfo, PilotError> {
        deadline.ensure_remaining()?;
        Ok(SurfaceInfo {
            id: format!("surface-{}", surface.as_str()),
            surface,
            app: brand::APP_SLUG.to_owned(),
            title: format!("{} {}", brand::APP_NAME, surface.as_str()),
            bounds: Some(self.state.lock().display.bounds),
            available: true,
        })
    }

    fn screenshot(
        &self,
        _filter: &WindowFilter,
        _display_index: Option<usize>,
        deadline: Deadline,
    ) -> Result<ImageBuffer, PilotError> {
        deadline.ensure_remaining()?;
        let st = self.state.lock();
        let scale = 2_u32;
        let width = (st.display.bounds.width as u32 / scale).max(320);
        let height = (st.display.bounds.height as u32 / scale).max(200);
        let bg = if st.dark_mode {
            Rgba([14, 17, 26, 255])
        } else {
            Rgba([244, 246, 250, 255])
        };
        let mut img = RgbaImage::from_pixel(width, height, bg);

        let panel_color = Rgba([30, 36, 52, 255]);
        fill_rect(&mut img, 0, 0, width, 44 / scale, panel_color);

        for pw in st.compositor.arrange_active_workspace() {
            let wx = (pw.rect.x.max(0.0) as u32) / scale;
            let wy = (pw.rect.y.max(0.0) as u32) / scale;
            let ww = (pw.rect.w.max(20.0) as u32) / scale;
            let wh = (pw.rect.h.max(20.0) as u32) / scale;
            let fill = if pw.focused {
                Rgba([38, 50, 76, 255])
            } else {
                Rgba([24, 30, 44, 255])
            };
            fill_rect(&mut img, wx, wy, ww, wh, fill);
        }

        let mut png_bytes = Vec::new();
        let encoder = PngEncoder::new(Cursor::new(&mut png_bytes));
        encoder
            .write_image(img.as_raw(), width, height, ColorType::Rgba8.into())
            .map_err(|err| PilotError::new(ErrorCode::Internal, format!("encode png: {err}")))?;

        Ok(ImageBuffer {
            format: ImageFormat::Png,
            width,
            height,
            scale: 1.0 / f64::from(scale),
            data: png_bytes,
        })
    }

    fn list_notifications(&self, deadline: Deadline) -> Result<Vec<NotificationInfo>, PilotError> {
        deadline.ensure_remaining()?;
        Ok(self.state.lock().notifications.clone())
    }

    fn dismiss_notification(&self, id: &str, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        let before = st.notifications.len();
        st.notifications.retain(|n| n.id != id);
        if st.notifications.len() == before {
            return Err(PilotError::new(
                ErrorCode::ElementNotFound,
                format!("notification '{id}' not found"),
            ));
        }
        Ok(())
    }

    fn dismiss_all_notifications(&self, deadline: Deadline) -> Result<usize, PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        let count = st.notifications.len();
        st.notifications.clear();
        Ok(count)
    }

    fn notification_action(
        &self,
        id: &str,
        action_name: &str,
        deadline: Deadline,
    ) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let mut st = self.state.lock();
        let notif = st
            .notifications
            .iter()
            .find(|n| n.id == id)
            .ok_or_else(|| {
                PilotError::new(
                    ErrorCode::ElementNotFound,
                    format!("notification '{id}' not found"),
                )
            })?
            .clone();
        if action_name.eq_ignore_ascii_case("dismiss") {
            st.notifications.retain(|n| n.id != id);
        } else {
            let app = notif.app;
            st.compositor
                .open_window(format!("{app} ({action_name})"), app);
        }
        Ok(())
    }
}

fn fill_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    let max_x = (x + w).min(img.width());
    let max_y = (y + h).min(img.height());
    for py in y..max_y {
        for px in x..max_x {
            img.put_pixel(px, py, color);
        }
    }
}
