#![cfg(all(target_os = "linux", not(target_family = "wasm")))]

use std::io::Cursor;

use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder as _};
use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::ConnectionExt as _;
use zbus::blocking::Connection;

use crate::action::{
    Action, ActionRequest, ActionResult, ActionStepOutcome, DragParams, KeyCombo, Modifier,
    MouseButton, MouseEvent, MouseEventKind,
};
use crate::adapter::{
    ActionOps, Deadline, InputOps, InteractionLease, LiveElement, NativeHandle, ObservationOps,
    SystemOps,
};
use crate::locator::ObservationRequest;
use crate::model::{
    AccessibilityNode, AppInfo, ClipboardContent, ClipboardFormat, DeliverySemantics, DisplayInfo,
    ElementState, ErrorCode, ImageBuffer, ImageFormat, NotificationInfo, PermissionReport,
    PilotError, Rect, SnapshotSurface, SurfaceInfo, WindowFilter, WindowInfo,
};
use crate::refs::RefEntry;

#[derive(Debug, Clone)]
pub struct AtspiHandle {
    pub bus_name: String,
    pub object_path: String,
    pub pid: u32,
}

/// Real Linux backend combining AT-SPI2 D-Bus (`zbus`), X11 (`x11rb` + `xtest` + `randr`), and
/// Wayland/FreeDesktop portal diagnostics cleanly under Aphrody's shared crate versions.
pub struct LinuxAdapter;

impl Default for LinuxAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxAdapter {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn connect_a11y_bus() -> Result<Connection, PilotError> {
        if let Ok(addr) = std::env::var("AT_SPI_BUS_ADDRESS")
            && !addr.trim().is_empty()
        {
            return Connection::session().map_err(|err| {
                PilotError::new(ErrorCode::PermDenied, format!("AT-SPI bus error: {err}"))
            });
        }
        let session = Connection::session().map_err(|err| {
            PilotError::new(
                ErrorCode::PermDenied,
                format!("D-Bus session unavailable: {err}"),
            )
        })?;
        let reply = session.call_method(
            Some("org.a11y.Bus"),
            "/org/a11y/bus",
            Some("org.a11y.Bus"),
            "GetAddress",
            &(),
        );
        match reply {
            Ok(msg) => {
                let addr: String = msg.body().deserialize().map_err(|err| {
                    PilotError::new(
                        ErrorCode::PermDenied,
                        format!("invalid a11y bus address: {err}"),
                    )
                })?;
                zbus::blocking::connection::Builder::address(addr.as_str())
                    .map_err(|err| {
                        PilotError::new(ErrorCode::PermDenied, format!("parse a11y addr: {err}"))
                    })?
                    .build()
                    .map_err(|err| {
                        PilotError::new(ErrorCode::PermDenied, format!("connect a11y bus: {err}"))
                    })
            }
            Err(err) => Err(PilotError::new(
                ErrorCode::PermDenied,
                format!("AT-SPI2 org.a11y.Bus unreachable: {err}"),
            )),
        }
    }

    fn is_wayland() -> bool {
        std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
    }
}

fn atspi_walk(
    conn: &Connection,
    bus_name: &str,
    path: &str,
    depth: usize,
    max_depth: usize,
    visited: &mut usize,
    max_nodes: usize,
) -> AccessibilityNode {
    *visited += 1;
    let name = read_atspi_string_prop(conn, bus_name, path, "Name").unwrap_or_default();
    let role_raw = call_atspi_u32(conn, bus_name, path, "GetRole").unwrap_or(0);
    let role = atspi_role_name(role_raw).to_owned();
    let desc = read_atspi_string_prop(conn, bus_name, path, "Description");

    let mut actions = Vec::new();
    if let Ok(msg) = conn.call_method(
        Some(bus_name),
        path,
        Some("org.a11y.atspi.Action"),
        "GetActions",
        &(),
    ) && let Ok(list) = msg.body().deserialize::<Vec<(String, String, String)>>()
    {
        for (act_name, _, _) in list {
            if !act_name.is_empty() {
                actions.push(act_name.to_ascii_lowercase());
            }
        }
    }

    let bounds = conn
        .call_method(
            Some(bus_name),
            path,
            Some("org.a11y.atspi.Component"),
            "GetExtents",
            &(0_u32,),
        )
        .ok()
        .and_then(|msg| msg.body().deserialize::<(i32, i32, i32, i32)>().ok())
        .map(|(x, y, w, h)| Rect::new(f64::from(x), f64::from(y), f64::from(w), f64::from(h)));

    let interactive = !actions.is_empty() || AccessibilityNode::is_interactive_role(&role);
    let mut children = Vec::new();
    let mut children_count = 0_usize;
    let mut truncated = false;

    if depth < max_depth && *visited < max_nodes {
        if let Ok(msg) = conn.call_method(
            Some(bus_name),
            path,
            Some("org.a11y.atspi.Accessible"),
            "GetChildren",
            &(),
        ) && let Ok(raw_children) = msg
            .body()
            .deserialize::<Vec<(String, zbus::zvariant::OwnedObjectPath)>>()
        {
            children_count = raw_children.len();
            for (cbus, cpath) in raw_children {
                if *visited >= max_nodes {
                    truncated = true;
                    break;
                }
                let child = atspi_walk(
                    conn,
                    &cbus,
                    cpath.as_str(),
                    depth + 1,
                    max_depth,
                    visited,
                    max_nodes,
                );
                children.push(child);
            }
        }
    }

    AccessibilityNode {
        id: format!("{bus_name}::{path}"),
        ref_id: None,
        role,
        name: (!name.is_empty()).then_some(name),
        value: None,
        description: desc.filter(|d| !d.is_empty()),
        identifier: Some(path.to_owned()),
        hint: None,
        bounds,
        state: ElementState {
            enabled: Some(true),
            states: vec!["enabled".to_owned()],
            ..Default::default()
        },
        actions,
        interactive,
        children_count,
        subtree_truncated: truncated,
        children,
    }
}

fn read_atspi_string_prop(conn: &Connection, dest: &str, path: &str, prop: &str) -> Option<String> {
    let msg = conn
        .call_method(
            Some(dest),
            path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.a11y.atspi.Accessible", prop),
        )
        .ok()?;
    let val: zbus::zvariant::OwnedValue = msg.body().deserialize().ok()?;
    String::try_from(val).ok()
}

fn call_atspi_u32(conn: &Connection, dest: &str, path: &str, method: &str) -> Option<u32> {
    let msg = conn
        .call_method(
            Some(dest),
            path,
            Some("org.a11y.atspi.Accessible"),
            method,
            &(),
        )
        .ok()?;
    msg.body().deserialize::<u32>().ok()
}

fn atspi_role_name(role_id: u32) -> &'static str {
    match role_id {
        7 | 80 => "checkbox",
        9 => "combobox",
        16 => "dialog",
        22 | 68 => "window",
        26 => "image",
        28 => "statictext",
        30 => "list",
        31 => "listitem",
        32 | 33 => "menu",
        34 => "menuitem",
        41 => "progressbar",
        42 | 43 => "button",
        44 => "radiobutton",
        47 => "scrollbar",
        50 => "slider",
        56 | 57 => "tablist",
        55 => "table",
        60 | 82 | 109 => "textfield",
        63 => "toolbar",
        64 => "tooltip",
        65 | 66 => "treeitem",
        104 => "switch",
        89 => "link",
        105 => "heading",
        85 => "document",
        _ => "group",
    }
}

impl ObservationOps for LinuxAdapter {
    fn observe_tree(
        &self,
        filter: &WindowFilter,
        request: &ObservationRequest,
        deadline: Deadline,
    ) -> Result<(WindowInfo, AccessibilityNode, bool), PilotError> {
        deadline.ensure_remaining()?;
        let a11y = Self::connect_a11y_bus()?;
        let mut visited = 0_usize;
        let registry = atspi_walk(
            &a11y,
            "org.a11y.atspi.Registry",
            "/org/a11y/atspi/accessible/root",
            0,
            request.max_depth.min(8),
            &mut visited,
            request.max_nodes,
        );
        let selected = if let Some(app_name) = &filter.app {
            registry
                .children
                .iter()
                .find(|c| {
                    c.name.as_deref().is_some_and(|n| {
                        n.to_ascii_lowercase()
                            .contains(&app_name.to_ascii_lowercase())
                    })
                })
                .cloned()
                .unwrap_or(registry)
        } else {
            registry
        };
        let win = WindowInfo {
            id: selected.id.clone(),
            title: selected
                .name
                .clone()
                .unwrap_or_else(|| "Linux AT-SPI Root".to_owned()),
            app: filter
                .app
                .clone()
                .unwrap_or_else(|| "linux-atspi".to_owned()),
            pid: 0,
            bounds: selected.bounds.unwrap_or_default(),
            focused: true,
            minimized: false,
            maximized: false,
            visible: true,
            workspace: None,
            process_token: None,
        };
        Ok((win, selected, visited < request.max_nodes))
    }

    fn resolve_element(
        &self,
        entry: &RefEntry,
        deadline: Deadline,
    ) -> Result<(NativeHandle, AccessibilityNode), PilotError> {
        deadline.ensure_remaining()?;
        let Some((bus, path)) = entry.node_id.split_once("::") else {
            return Err(PilotError::stale_ref(&entry.ref_id));
        };
        let handle = NativeHandle::new(AtspiHandle {
            bus_name: bus.to_owned(),
            object_path: path.to_owned(),
            pid: entry.scope.as_ref().map(|s| s.pid).unwrap_or(0),
        });
        let node = AccessibilityNode {
            id: entry.node_id.clone(),
            role: entry.identity.role.clone(),
            name: entry.identity.name.clone(),
            identifier: entry.identity.identifier.clone(),
            bounds: entry.bounds,
            state: entry.state.clone(),
            actions: entry.capabilities.actions.clone(),
            interactive: entry.capabilities.interactive,
            ..Default::default()
        };
        Ok((handle, node))
    }

    fn get_live_element(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<LiveElement, PilotError> {
        deadline.ensure_remaining()?;
        let h = handle
            .downcast_ref::<AtspiHandle>()
            .ok_or_else(|| PilotError::invalid_input("expected Linux AtspiHandle"))?;
        let conn = Self::connect_a11y_bus()?;
        let name = read_atspi_string_prop(&conn, &h.bus_name, &h.object_path, "Name");
        let role_id = call_atspi_u32(&conn, &h.bus_name, &h.object_path, "GetRole").unwrap_or(0);
        Ok(LiveElement {
            id: format!("{}::{}", h.bus_name, h.object_path),
            role: atspi_role_name(role_id).to_owned(),
            name,
            bounds: None,
            state: ElementState::default(),
            actions: Vec::new(),
        })
    }

    fn list_surfaces(&self, deadline: Deadline) -> Result<Vec<SurfaceInfo>, PilotError> {
        deadline.ensure_remaining()?;
        Ok(vec![SurfaceInfo {
            id: "linux-atspi-root".to_owned(),
            surface: SnapshotSurface::Desktop,
            app: "at-spi2".to_owned(),
            title: "AT-SPI2 Accessibility Tree".to_owned(),
            bounds: None,
            available: Self::connect_a11y_bus().is_ok(),
        }])
    }
}

impl ActionOps for LinuxAdapter {
    fn execute_action(
        &self,
        handle: &NativeHandle,
        request: &ActionRequest,
        lease: &InteractionLease,
    ) -> Result<ActionResult, PilotError> {
        lease.deadline().ensure_remaining()?;
        let h = handle
            .downcast_ref::<AtspiHandle>()
            .ok_or_else(|| PilotError::invalid_input("expected Linux AtspiHandle"))?;
        let conn = Self::connect_a11y_bus()?;

        let (iface, method) = match &request.action {
            Action::Focus => ("org.a11y.atspi.Component", "GrabFocus"),
            _ => ("org.a11y.atspi.Action", "DoAction"),
        };
        if iface == "org.a11y.atspi.Component" {
            conn.call_method(
                Some(h.bus_name.as_str()),
                h.object_path.as_str(),
                Some(iface),
                method,
                &(),
            )
            .map_err(|err| {
                PilotError::new(ErrorCode::ActionFailed, format!("AT-SPI GrabFocus: {err}"))
            })?;
        } else if let Action::SetValue(val) | Action::TypeText(val) = &request.action {
            conn.call_method(
                Some(h.bus_name.as_str()),
                h.object_path.as_str(),
                Some("org.a11y.atspi.EditableText"),
                "SetTextContents",
                &(val.as_str(),),
            )
            .map_err(|err| {
                PilotError::new(
                    ErrorCode::ActionFailed,
                    format!("AT-SPI SetTextContents: {err}"),
                )
            })?;
        } else {
            conn.call_method(
                Some(h.bus_name.as_str()),
                h.object_path.as_str(),
                Some("org.a11y.atspi.Action"),
                "DoAction",
                &(0_i32,),
            )
            .map_err(|err| {
                PilotError::new(ErrorCode::ActionFailed, format!("AT-SPI DoAction: {err}"))
            })?;
        }

        Ok(ActionResult {
            action: request.action.name().to_owned(),
            target_ref: None,
            disposition: DeliverySemantics::delivered_unverified(),
            verified: false,
            duration_ms: 0,
            before_state: None,
            after_state: None,
            steps: vec![ActionStepOutcome {
                step: request.action.name().to_owned(),
                mechanism: "linux_atspi2".to_owned(),
                ok: true,
                duration_ms: 0,
                detail: Some(h.object_path.clone()),
            }],
        })
    }
}

impl InputOps for LinuxAdapter {
    fn press_key(&self, combo: &KeyCombo, deadline: Deadline) -> Result<(), PilotError> {
        self.key_state(combo, true, deadline)?;
        self.key_state(combo, false, deadline)
    }

    fn key_state(
        &self,
        combo: &KeyCombo,
        down: bool,
        deadline: Deadline,
    ) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let (conn, screen_num) = x11rb::connect(None).map_err(|err| {
            PilotError::new(
                ErrorCode::ActionNotSupported,
                format!("X11 XTest unavailable (Wayland portal required?): {err}"),
            )
        })?;
        let root = conn.setup().roots[screen_num].root;
        let keycode = linux_x11_keycode(&combo.key).unwrap_or(36);
        let ty = if down { 2_u8 } else { 3_u8 };
        for m in &combo.modifiers {
            let mcode = match m {
                Modifier::Shift => 50_u8,
                Modifier::Control => 37_u8,
                Modifier::Alt => 64_u8,
                Modifier::Meta => 133_u8,
            };
            let _ = x11rb::protocol::xtest::fake_input(&conn, ty, mcode, 0, root, 0, 0, 0);
        }
        x11rb::protocol::xtest::fake_input(&conn, ty, keycode, 0, root, 0, 0, 0).map_err(
            |err| PilotError::new(ErrorCode::ActionFailed, format!("XTest key error: {err}")),
        )?;
        conn.flush().map_err(|err| {
            PilotError::new(ErrorCode::ActionFailed, format!("XTest flush: {err}"))
        })?;
        Ok(())
    }

    fn mouse_event(&self, event: &MouseEvent, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let (conn, screen_num) = x11rb::connect(None).map_err(|err| {
            PilotError::new(
                ErrorCode::ActionNotSupported,
                format!("X11 connect failed: {err}"),
            )
        })?;
        let root = conn.setup().roots[screen_num].root;
        let _ = x11rb::protocol::xtest::fake_input(
            &conn,
            6,
            0,
            0,
            root,
            event.x as i16,
            event.y as i16,
            0,
        );
        let btn = match event.button {
            MouseButton::Left => 1_u8,
            MouseButton::Middle => 2_u8,
            MouseButton::Right => 3_u8,
        };
        match event.kind {
            MouseEventKind::Down => {
                let _ = x11rb::protocol::xtest::fake_input(&conn, 4, btn, 0, root, 0, 0, 0);
            }
            MouseEventKind::Up => {
                let _ = x11rb::protocol::xtest::fake_input(&conn, 5, btn, 0, root, 0, 0, 0);
            }
            MouseEventKind::Click => {
                for _ in 0..event.click_count.max(1) {
                    let _ = x11rb::protocol::xtest::fake_input(&conn, 4, btn, 0, root, 0, 0, 0);
                    let _ = x11rb::protocol::xtest::fake_input(&conn, 5, btn, 0, root, 0, 0, 0);
                }
            }
            MouseEventKind::Wheel => {
                let wheel_btn = if event.wheel_dy >= 0 { 4_u8 } else { 5_u8 };
                let _ = x11rb::protocol::xtest::fake_input(&conn, 4, wheel_btn, 0, root, 0, 0, 0);
                let _ = x11rb::protocol::xtest::fake_input(&conn, 5, wheel_btn, 0, root, 0, 0, 0);
            }
            MouseEventKind::Move => {}
        }
        conn.flush()
            .map_err(|err| PilotError::new(ErrorCode::ActionFailed, format!("X11 flush: {err}")))?;
        Ok(())
    }

    fn drag(&self, params: &DragParams, deadline: Deadline) -> Result<(), PilotError> {
        let down = MouseEvent {
            kind: MouseEventKind::Down,
            x: params.from_x,
            y: params.from_y,
            button: params.button,
            click_count: 1,
            modifiers: Vec::new(),
            wheel_dx: 0,
            wheel_dy: 0,
        };
        self.mouse_event(&down, deadline)?;
        let mv = MouseEvent {
            kind: MouseEventKind::Move,
            x: params.to_x,
            y: params.to_y,
            ..down
        };
        self.mouse_event(&mv, deadline)?;
        let up = MouseEvent {
            kind: MouseEventKind::Up,
            ..mv
        };
        self.mouse_event(&up, deadline)
    }

    fn clipboard_get(
        &self,
        _format: ClipboardFormat,
        deadline: Deadline,
    ) -> Result<ClipboardContent, PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::ActionNotSupported,
            "Linux native clipboard requires active X11/Wayland clipboard session",
        ))
    }

    fn clipboard_set(
        &self,
        _content: &ClipboardContent,
        deadline: Deadline,
    ) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::ActionNotSupported,
            "Linux native clipboard set requires persistent selection owner",
        ))
    }

    fn clipboard_clear(&self, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        Ok(())
    }
}

impl SystemOps for LinuxAdapter {
    fn backend_name(&self) -> &'static str {
        if Self::is_wayland() {
            "linux-atspi-wayland"
        } else {
            "linux-atspi-x11"
        }
    }

    fn permissions(&self, deadline: Deadline) -> Result<PermissionReport, PilotError> {
        deadline.ensure_remaining()?;
        let a11y_ok = Self::connect_a11y_bus().is_ok();
        let display_server = if Self::is_wayland() {
            "wayland"
        } else if std::env::var_os("DISPLAY").is_some() {
            "x11"
        } else {
            "headless"
        };
        Ok(PermissionReport {
            platform: "linux".to_owned(),
            accessibility_granted: a11y_ok,
            screen_recording_granted: display_server != "headless",
            elevation_status: None,
            display_server: Some(display_server.to_owned()),
            accessibility_detail: Some(if a11y_ok {
                "org.a11y.Bus reachable".to_owned()
            } else {
                "AT-SPI2 bus unreachable".to_owned()
            }),
            screen_recording_detail: Some(display_server.to_owned()),
        })
    }

    fn list_apps(&self, deadline: Deadline) -> Result<Vec<AppInfo>, PilotError> {
        deadline.ensure_remaining()?;
        Ok(aphrody_session::discover_system_apps(50)
            .into_iter()
            .map(|e| AppInfo {
                id: e.id,
                name: e.name,
                pid: 0,
                bundle_id: None,
                executable: Some(e.exec),
                active: false,
            })
            .collect())
    }

    fn list_windows(&self, deadline: Deadline) -> Result<Vec<WindowInfo>, PilotError> {
        deadline.ensure_remaining()?;
        let Ok((conn, screen_num)) = x11rb::connect(None) else {
            return Ok(Vec::new());
        };
        let screen = &conn.setup().roots[screen_num];
        let tree = conn
            .query_tree(screen.root)
            .ok()
            .and_then(|c| c.reply().ok());
        let Some(reply) = tree else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for win in reply.children {
            if let Ok(geom) = conn.get_geometry(win).map(|c| c.reply())
                && let Ok(g) = geom
                && g.width > 16
                && g.height > 16
            {
                out.push(WindowInfo {
                    id: win.to_string(),
                    title: format!("X11 Window {win}"),
                    app: "x11".to_owned(),
                    pid: 0,
                    bounds: Rect::new(
                        f64::from(g.x),
                        f64::from(g.y),
                        f64::from(g.width),
                        f64::from(g.height),
                    ),
                    focused: false,
                    minimized: false,
                    maximized: false,
                    visible: true,
                    workspace: None,
                    process_token: None,
                });
            }
        }
        Ok(out)
    }

    fn list_displays(&self, deadline: Deadline) -> Result<Vec<DisplayInfo>, PilotError> {
        deadline.ensure_remaining()?;
        let Ok((conn, screen_num)) = x11rb::connect(None) else {
            return Ok(Vec::new());
        };
        let screen = &conn.setup().roots[screen_num];
        Ok(vec![DisplayInfo {
            id: format!("x11-screen-{screen_num}"),
            bounds: Rect::new(
                0.0,
                0.0,
                f64::from(screen.width_in_pixels),
                f64::from(screen.height_in_pixels),
            ),
            is_primary: true,
            scale: 1.0,
        }])
    }

    fn focus_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        let windows = self.list_windows(deadline)?;
        let win = windows
            .into_iter()
            .find(|w| filter.window_id.as_deref() == Some(w.id.as_str()))
            .ok_or_else(|| PilotError::new(ErrorCode::WindowNotFound, "window not found"))?;
        if let Ok((conn, _)) = x11rb::connect(None)
            && let Ok(xid) = win.id.parse::<u32>()
        {
            let _ = conn.set_input_focus(
                x11rb::protocol::xproto::InputFocus::POINTER_ROOT,
                xid,
                0_u32,
            );
            let _ = conn.flush();
        }
        Ok(win)
    }

    fn move_window(
        &self,
        _filter: &WindowFilter,
        _x: f64,
        _y: f64,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::ActionNotSupported,
            "move_window requires X11 EWMH WM",
        ))
    }

    fn resize_window(
        &self,
        _filter: &WindowFilter,
        _w: f64,
        _h: f64,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::ActionNotSupported,
            "resize_window requires X11 EWMH WM",
        ))
    }

    fn minimize_window(
        &self,
        _filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::ActionNotSupported,
            "minimize requires EWMH client",
        ))
    }

    fn maximize_window(
        &self,
        _filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::ActionNotSupported,
            "maximize requires EWMH client",
        ))
    }

    fn restore_window(
        &self,
        _filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::ActionNotSupported,
            "restore requires EWMH client",
        ))
    }

    fn launch_app(
        &self,
        app: &str,
        args: &[String],
        deadline: Deadline,
    ) -> Result<AppInfo, PilotError> {
        deadline.ensure_remaining()?;
        let child = smol::process::Command::new(app)
            .args(args)
            .spawn()
            .map_err(|err| {
                PilotError::new(
                    ErrorCode::AppNotFound,
                    format!("failed launching '{app}': {err}"),
                )
            })?;
        let pid = child.id();
        std::mem::forget(child);
        Ok(AppInfo {
            id: app.to_owned(),
            name: app.to_owned(),
            pid,
            bundle_id: None,
            executable: Some(app.to_owned()),
            active: true,
        })
    }

    fn close_app(&self, app: &str, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let _ = smol::process::Command::new("pkill")
            .arg("-x")
            .arg(app)
            .spawn();
        Ok(())
    }

    fn open_system_surface(
        &self,
        surface: SnapshotSurface,
        deadline: Deadline,
    ) -> Result<SurfaceInfo, PilotError> {
        deadline.ensure_remaining()?;
        Ok(SurfaceInfo {
            id: format!("linux-{}", surface.as_str()),
            surface,
            app: "linux".to_owned(),
            title: surface.as_str().to_owned(),
            bounds: None,
            available: false,
        })
    }

    fn screenshot(
        &self,
        _filter: &WindowFilter,
        _display_index: Option<usize>,
        deadline: Deadline,
    ) -> Result<ImageBuffer, PilotError> {
        deadline.ensure_remaining()?;
        let (conn, screen_num) = x11rb::connect(None).map_err(|err| {
            PilotError::new(
                ErrorCode::PermDenied,
                format!("X11 display capture unavailable: {err}"),
            )
        })?;
        let screen = &conn.setup().roots[screen_num];
        let w = screen.width_in_pixels;
        let h = screen.height_in_pixels;
        let img = conn
            .get_image(
                x11rb::protocol::xproto::ImageFormat::Z_PIXMAP,
                screen.root,
                0,
                0,
                w,
                h,
                u32::MAX,
            )
            .map_err(|err| {
                PilotError::new(ErrorCode::ActionFailed, format!("X11 get_image: {err}"))
            })?
            .reply()
            .map_err(|err| {
                PilotError::new(
                    ErrorCode::ActionFailed,
                    format!("X11 get_image reply: {err}"),
                )
            })?;
        let mut rgba = img.data;
        for px in rgba.chunks_exact_mut(4) {
            px.swap(0, 2);
            px[3] = 255;
        }
        let mut png = Vec::new();
        PngEncoder::new(Cursor::new(&mut png))
            .write_image(&rgba, u32::from(w), u32::from(h), ColorType::Rgba8.into())
            .map_err(|err| PilotError::new(ErrorCode::Internal, format!("encode png: {err}")))?;
        Ok(ImageBuffer {
            format: ImageFormat::Png,
            width: u32::from(w),
            height: u32::from(h),
            scale: 1.0,
            data: png,
        })
    }

    fn list_notifications(&self, deadline: Deadline) -> Result<Vec<NotificationInfo>, PilotError> {
        deadline.ensure_remaining()?;
        Ok(Vec::new())
    }

    fn dismiss_notification(&self, _id: &str, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()
    }

    fn dismiss_all_notifications(&self, deadline: Deadline) -> Result<usize, PilotError> {
        deadline.ensure_remaining()?;
        Ok(0)
    }

    fn notification_action(
        &self,
        _id: &str,
        _action_name: &str,
        deadline: Deadline,
    ) -> Result<(), PilotError> {
        deadline.ensure_remaining()
    }
}

fn linux_x11_keycode(key: &str) -> Option<u8> {
    match key.to_ascii_lowercase().as_str() {
        "return" | "enter" => Some(36),
        "escape" | "esc" => Some(9),
        "tab" => Some(23),
        "space" => Some(65),
        "backspace" => Some(22),
        _ => None,
    }
}
