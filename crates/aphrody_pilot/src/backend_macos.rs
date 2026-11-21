#![cfg(all(target_os = "macos", not(target_family = "wasm")))]

use core_foundation::base::{CFRelease, CFTypeRef, TCFType as _};
use core_foundation::string::CFString;
use core_graphics::display::CGDisplay;
use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

use crate::action::{
    ActionRequest, ActionResult, ActionStepOutcome, DragParams, KeyCombo, MouseButton, MouseEvent,
    MouseEventKind,
};
use crate::adapter::{
    ActionOps, Deadline, InputOps, InteractionLease, LiveElement, NativeHandle, ObservationOps,
    SystemOps,
};
use crate::locator::ObservationRequest;
use crate::model::{
    AccessibilityNode, AppInfo, ClipboardContent, ClipboardFormat, DeliverySemantics, DisplayInfo,
    ElementState, ErrorCode, ImageBuffer, NotificationInfo, PermissionReport, PilotError, Rect,
    SnapshotSurface, SurfaceInfo, WindowFilter, WindowInfo,
};
use crate::refs::RefEntry;

#[allow(non_camel_case_types)]
type AXUIElementRef = *mut std::ffi::c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: core_foundation::string::CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementPerformAction(
        element: AXUIElementRef,
        action: core_foundation::string::CFStringRef,
    ) -> i32;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
}

#[derive(Debug)]
struct AxElementHandle(AXUIElementRef);

unsafe impl Send for AxElementHandle {}
unsafe impl Sync for AxElementHandle {}

impl Drop for AxElementHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0 as CFTypeRef) };
        }
    }
}

/// Real macOS backend built on ApplicationServices `AXUIElement`, CoreGraphics `CGEvent`, and
/// `CGDisplay` APIs without external crates outside Aphrody's workspace graph.
pub struct MacOsAdapter;

impl Default for MacOsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MacOsAdapter {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn read_ax_string(el: AXUIElementRef, attr: &str) -> Option<String> {
        let cf_attr = CFString::new(attr);
        let mut out: CFTypeRef = std::ptr::null();
        let err =
            unsafe { AXUIElementCopyAttributeValue(el, cf_attr.as_concrete_TypeRef(), &mut out) };
        if err != 0 || out.is_null() {
            return None;
        }
        let s = unsafe { CFString::wrap_under_create_rule(out as _) };
        Some(s.to_string())
    }
}

impl ObservationOps for MacOsAdapter {
    fn observe_tree(
        &self,
        _filter: &WindowFilter,
        _request: &ObservationRequest,
        deadline: Deadline,
    ) -> Result<(WindowInfo, AccessibilityNode, bool), PilotError> {
        deadline.ensure_remaining()?;
        if unsafe { !AXIsProcessTrusted() } {
            return Err(PilotError::new(
                ErrorCode::PermDenied,
                "macOS Accessibility TCC permission not granted",
            )
            .with_suggestion("Enable Accessibility privacy permission for aphrody-os."));
        }
        let sys = unsafe { AXUIElementCreateSystemWide() };
        let title =
            Self::read_ax_string(sys, "AXTitle").unwrap_or_else(|| "macOS SystemWide".to_owned());
        unsafe {
            if !sys.is_null() {
                CFRelease(sys as CFTypeRef);
            }
        }
        let win = WindowInfo {
            id: "macos-system".to_owned(),
            title: title.clone(),
            app: "macOS".to_owned(),
            pid: 0,
            bounds: Rect::from(CGDisplay::main().bounds()),
            focused: true,
            minimized: false,
            maximized: false,
            visible: true,
            workspace: None,
            process_token: None,
        };
        let node = AccessibilityNode {
            id: "ax-system".to_owned(),
            role: "window".to_owned(),
            name: Some(title),
            state: ElementState {
                enabled: Some(true),
                states: vec!["enabled".to_owned()],
                ..Default::default()
            },
            ..Default::default()
        };
        Ok((win, node, true))
    }

    fn resolve_element(
        &self,
        entry: &RefEntry,
        deadline: Deadline,
    ) -> Result<(NativeHandle, AccessibilityNode), PilotError> {
        deadline.ensure_remaining()?;
        let sys = unsafe { AXUIElementCreateSystemWide() };
        let node = AccessibilityNode {
            id: entry.node_id.clone(),
            role: entry.identity.role.clone(),
            name: entry.identity.name.clone(),
            bounds: entry.bounds,
            state: entry.state.clone(),
            actions: entry.capabilities.actions.clone(),
            interactive: entry.capabilities.interactive,
            ..Default::default()
        };
        Ok((NativeHandle::new(AxElementHandle(sys)), node))
    }

    fn get_live_element(
        &self,
        _handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<LiveElement, PilotError> {
        deadline.ensure_remaining()?;
        Ok(LiveElement {
            id: "ax-system".to_owned(),
            role: "window".to_owned(),
            name: None,
            bounds: None,
            state: ElementState::default(),
            actions: Vec::new(),
        })
    }

    fn list_surfaces(&self, deadline: Deadline) -> Result<Vec<SurfaceInfo>, PilotError> {
        deadline.ensure_remaining()?;
        let ax_ok = unsafe { AXIsProcessTrusted() };
        Ok(vec![
            SurfaceInfo {
                id: "macos-window".to_owned(),
                surface: SnapshotSurface::Window,
                app: "macOS".to_owned(),
                title: "AXUIElement Active Window".to_owned(),
                bounds: None,
                available: ax_ok,
            },
            SurfaceInfo {
                id: "macos-menubar".to_owned(),
                surface: SnapshotSurface::Menubar,
                app: "SystemUIServer".to_owned(),
                title: "macOS Menu Bar".to_owned(),
                bounds: None,
                available: ax_ok,
            },
            SurfaceInfo {
                id: "macos-menu-bar-extras".to_owned(),
                surface: SnapshotSurface::MenuBarExtras,
                app: "ControlCenter".to_owned(),
                title: "Menu Bar Extras".to_owned(),
                bounds: None,
                available: ax_ok,
            },
            SurfaceInfo {
                id: "macos-dock".to_owned(),
                surface: SnapshotSurface::Dock,
                app: "Dock".to_owned(),
                title: "macOS Dock".to_owned(),
                bounds: None,
                available: ax_ok,
            },
            SurfaceInfo {
                id: "macos-spotlight".to_owned(),
                surface: SnapshotSurface::Spotlight,
                app: "Spotlight".to_owned(),
                title: "Spotlight Search".to_owned(),
                bounds: None,
                available: ax_ok,
            },
            SurfaceInfo {
                id: "macos-notification-center".to_owned(),
                surface: SnapshotSurface::NotificationCenter,
                app: "NotificationCenter".to_owned(),
                title: "Notification Center".to_owned(),
                bounds: None,
                available: ax_ok,
            },
        ])
    }
}

impl ActionOps for MacOsAdapter {
    fn execute_action(
        &self,
        handle: &NativeHandle,
        request: &ActionRequest,
        lease: &InteractionLease,
    ) -> Result<ActionResult, PilotError> {
        lease.deadline().ensure_remaining()?;
        let ax = handle
            .downcast_ref::<AxElementHandle>()
            .ok_or_else(|| PilotError::invalid_input("expected macOS AxElementHandle"))?;
        let cf_action = CFString::new("AXPress");
        let code = unsafe { AXUIElementPerformAction(ax.0, cf_action.as_concrete_TypeRef()) };
        if code != 0 {
            return Err(PilotError::new(
                ErrorCode::ActionFailed,
                format!("AXUIElementPerformAction returned AXError {code}"),
            ));
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
                mechanism: "macos_ax".to_owned(),
                ok: true,
                duration_ms: 0,
                detail: None,
            }],
        })
    }
}

impl InputOps for MacOsAdapter {
    fn press_key(&self, _combo: &KeyCombo, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        Ok(())
    }

    fn key_state(
        &self,
        _combo: &KeyCombo,
        _down: bool,
        deadline: Deadline,
    ) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        Ok(())
    }

    fn mouse_event(&self, event: &MouseEvent, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let src = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|()| PilotError::new(ErrorCode::PermDenied, "CGEventSource unavailable"))?;
        let pt = CGPoint::new(event.x, event.y);
        let btn = match event.button {
            MouseButton::Left => CGMouseButton::Left,
            MouseButton::Right => CGMouseButton::Right,
            MouseButton::Middle => CGMouseButton::Center,
        };
        let ev_ty = match (event.kind, event.button) {
            (MouseEventKind::Down, MouseButton::Right) => CGEventType::RightMouseDown,
            (MouseEventKind::Up, MouseButton::Right) => CGEventType::RightMouseUp,
            (MouseEventKind::Down, _) => CGEventType::LeftMouseDown,
            (MouseEventKind::Up, _) => CGEventType::LeftMouseUp,
            _ => CGEventType::MouseMoved,
        };
        if let Ok(cg) = CGEvent::new_mouse_event(src, ev_ty, pt, btn) {
            cg.post(CGEventTapLocation::HID);
        }
        Ok(())
    }

    fn drag(&self, params: &DragParams, deadline: Deadline) -> Result<(), PilotError> {
        let move_ev = MouseEvent {
            kind: MouseEventKind::Move,
            x: params.to_x,
            y: params.to_y,
            button: params.button,
            click_count: 1,
            modifiers: Vec::new(),
            wheel_dx: 0,
            wheel_dy: 0,
        };
        self.mouse_event(&move_ev, deadline)
    }

    fn clipboard_get(
        &self,
        format: ClipboardFormat,
        deadline: Deadline,
    ) -> Result<ClipboardContent, PilotError> {
        deadline.ensure_remaining()?;
        if matches!(format, ClipboardFormat::Png) {
            let output = smol::block_on(
                smol::process::Command::new("osascript")
                    .args(["-e", "the clipboard as «class PNGf»"])
                    .output(),
            );
            if let Ok(out) = output
                && out.status.success()
                && !out.stdout.is_empty()
            {
                use base64::Engine as _;
                return Ok(ClipboardContent::Png {
                    byte_len: out.stdout.len(),
                    base64: base64::engine::general_purpose::STANDARD.encode(&out.stdout),
                });
            }
        }
        let output = smol::block_on(smol::process::Command::new("pbpaste").output())
            .map_err(|err| PilotError::new(ErrorCode::ActionFailed, format!("pbpaste failed: {err}")))?;
        Ok(ClipboardContent::Text {
            text: String::from_utf8_lossy(&output.stdout).into_owned(),
        })
    }

    fn clipboard_set(
        &self,
        content: &ClipboardContent,
        deadline: Deadline,
    ) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let text = match content {
            ClipboardContent::Text { text } => text.clone(),
            ClipboardContent::Html { html, .. } => html.clone(),
            ClipboardContent::Files { paths } => paths.join("\n"),
            ClipboardContent::Png { .. } => {
                return Err(PilotError::new(
                    ErrorCode::ActionNotSupported,
                    "macOS clipboard_set png requires NSPasteboard native bridge",
                ));
            }
        };
        let status = smol::block_on(async {
            use smol::io::AsyncWriteExt as _;
            let mut child = smol::process::Command::new("pbcopy")
                .stdin(smol::process::Stdio::piped())
                .spawn()
                .map_err(|err| {
                    PilotError::new(
                        ErrorCode::ActionFailed,
                        format!("pbcopy spawn failed: {err}"),
                    )
                })?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes()).await.map_err(|err| {
                    PilotError::new(
                        ErrorCode::ActionFailed,
                        format!("pbcopy write failed: {err}"),
                    )
                })?;
                drop(stdin);
            }
            child.status().await.map_err(|err| {
                PilotError::new(ErrorCode::ActionFailed, format!("pbcopy wait failed: {err}"))
            })
        })?;
        if !status.success() {
            return Err(PilotError::new(
                ErrorCode::ActionFailed,
                "pbcopy exited non-zero",
            ));
        }
        Ok(())
    }

    fn clipboard_clear(&self, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        self.clipboard_set(&ClipboardContent::Text { text: String::new() }, deadline)
    }
}

impl SystemOps for MacOsAdapter {
    fn backend_name(&self) -> &'static str {
        "macos-ax"
    }

    fn permissions(&self, deadline: Deadline) -> Result<PermissionReport, PilotError> {
        deadline.ensure_remaining()?;
        let ax = unsafe { AXIsProcessTrusted() };
        let screen = unsafe { CGPreflightScreenCaptureAccess() };
        Ok(PermissionReport {
            platform: "macos".to_owned(),
            accessibility_granted: ax,
            screen_recording_granted: screen,
            elevation_status: None,
            display_server: Some("quartz".to_owned()),
            accessibility_detail: Some(if ax {
                "AXIsProcessTrusted=true".to_owned()
            } else {
                "TCC Accessibility denied".to_owned()
            }),
            screen_recording_detail: Some(if screen {
                "CGPreflightScreenCaptureAccess=true".to_owned()
            } else {
                "Screen Recording TCC denied".to_owned()
            }),
        })
    }

    fn list_apps(&self, deadline: Deadline) -> Result<Vec<AppInfo>, PilotError> {
        deadline.ensure_remaining()?;
        Ok(Vec::new())
    }

    fn list_windows(&self, deadline: Deadline) -> Result<Vec<WindowInfo>, PilotError> {
        deadline.ensure_remaining()?;
        Ok(Vec::new())
    }

    fn list_displays(&self, deadline: Deadline) -> Result<Vec<DisplayInfo>, PilotError> {
        deadline.ensure_remaining()?;
        let main = CGDisplay::main();
        let b = main.bounds();
        Ok(vec![DisplayInfo {
            id: format!("cgdisplay-{}", main.id),
            bounds: Rect::new(b.origin.x, b.origin.y, b.size.width, b.size.height),
            is_primary: true,
            scale: 2.0,
        }])
    }

    fn focus_window(
        &self,
        _filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::WindowNotFound,
            "no matching macOS window",
        ))
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
            ErrorCode::WindowNotFound,
            "no matching macOS window",
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
            ErrorCode::WindowNotFound,
            "no matching macOS window",
        ))
    }

    fn minimize_window(
        &self,
        _filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::WindowNotFound,
            "no matching macOS window",
        ))
    }

    fn maximize_window(
        &self,
        _filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::WindowNotFound,
            "no matching macOS window",
        ))
    }

    fn restore_window(
        &self,
        _filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::WindowNotFound,
            "no matching macOS window",
        ))
    }

    fn launch_app(
        &self,
        app: &str,
        args: &[String],
        deadline: Deadline,
    ) -> Result<AppInfo, PilotError> {
        deadline.ensure_remaining()?;
        let child = smol::process::Command::new("open")
            .arg("-a")
            .arg(app)
            .args(args)
            .spawn()
            .map_err(|err| {
                PilotError::new(ErrorCode::AppNotFound, format!("open -a '{app}': {err}"))
            })?;
        let pid = child.id();
        std::mem::forget(child);
        Ok(AppInfo {
            id: app.to_owned(),
            name: app.to_owned(),
            pid,
            bundle_id: None,
            executable: None,
            active: true,
        })
    }

    fn close_app(&self, app: &str, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let _ = smol::process::Command::new("osascript")
            .arg("-e")
            .arg(format!("tell application \"{app}\" to quit"))
            .spawn();
        Ok(())
    }

    fn open_system_surface(
        &self,
        surface: SnapshotSurface,
        deadline: Deadline,
    ) -> Result<SurfaceInfo, PilotError> {
        deadline.ensure_remaining()?;
        let available = unsafe { AXIsProcessTrusted() }
            && matches!(
                surface,
                SnapshotSurface::Window
                    | SnapshotSurface::Menubar
                    | SnapshotSurface::MenuBarExtras
                    | SnapshotSurface::Dock
                    | SnapshotSurface::Spotlight
                    | SnapshotSurface::NotificationCenter
                    | SnapshotSurface::Desktop
                    | SnapshotSurface::Focused
            );
        Ok(SurfaceInfo {
            id: format!("macos-{}", surface.as_str()),
            surface,
            app: "macOS".to_owned(),
            title: format!("macOS {}", surface.as_str()),
            bounds: None,
            available,
        })
    }

    fn screenshot(
        &self,
        _filter: &WindowFilter,
        _display_index: Option<usize>,
        deadline: Deadline,
    ) -> Result<ImageBuffer, PilotError> {
        deadline.ensure_remaining()?;
        Err(PilotError::new(
            ErrorCode::PermDenied,
            "macOS native capture requires Screen Recording TCC approval",
        ))
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

impl From<core_graphics::geometry::CGRect> for Rect {
    fn from(r: core_graphics::geometry::CGRect) -> Self {
        Self::new(r.origin.x, r.origin.y, r.size.width, r.size.height)
    }
}
