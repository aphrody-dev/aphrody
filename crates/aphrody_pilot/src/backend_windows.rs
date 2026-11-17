#![cfg(all(target_os = "windows", not(target_family = "wasm")))]

use std::ffi::c_void;
use std::io::Cursor;
use std::time::Duration;

use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder as _};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, EnumDisplayMonitors, GetDC, GetDIBits, GetMonitorInfoW,
    HDC, HMONITOR, MONITORINFO, ReleaseDC, SRCCOPY, SelectObject,
};
use windows::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL,
    TOKEN_QUERY, TokenIntegrityLevel,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Threading::{
    OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, ExpandCollapseState_Expanded, IUIAutomation, IUIAutomationElement,
    IUIAutomationExpandCollapsePattern, IUIAutomationInvokePattern, IUIAutomationScrollItemPattern,
    IUIAutomationScrollPattern, IUIAutomationSelectionItemPattern, IUIAutomationTogglePattern,
    IUIAutomationValuePattern, ScrollAmount_NoAmount, ScrollAmount_SmallDecrement,
    ScrollAmount_SmallIncrement, ToggleState_On, UIA_ButtonControlTypeId, UIA_CONTROLTYPE_ID,
    UIA_CheckBoxControlTypeId, UIA_ComboBoxControlTypeId, UIA_DataGridControlTypeId,
    UIA_DataItemControlTypeId, UIA_DocumentControlTypeId, UIA_EditControlTypeId,
    UIA_ExpandCollapsePatternId, UIA_GroupControlTypeId, UIA_HyperlinkControlTypeId,
    UIA_ImageControlTypeId, UIA_InvokePatternId, UIA_ListControlTypeId, UIA_ListItemControlTypeId,
    UIA_MenuBarControlTypeId, UIA_MenuControlTypeId, UIA_MenuItemControlTypeId,
    UIA_PaneControlTypeId, UIA_ProgressBarControlTypeId, UIA_RadioButtonControlTypeId,
    UIA_ScrollBarControlTypeId, UIA_ScrollItemPatternId, UIA_ScrollPatternId,
    UIA_SelectionItemPatternId, UIA_SliderControlTypeId, UIA_SplitButtonControlTypeId,
    UIA_StatusBarControlTypeId, UIA_TabControlTypeId, UIA_TabItemControlTypeId,
    UIA_TableControlTypeId, UIA_TextControlTypeId, UIA_TogglePatternId, UIA_ToolBarControlTypeId,
    UIA_ToolTipControlTypeId, UIA_TreeControlTypeId, UIA_TreeItemControlTypeId, UIA_ValuePatternId,
    UIA_WindowControlTypeId,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput,
    VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT,
    VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GWL_EXSTYLE, GetClassNameW, GetDesktopWindow, GetForegroundWindow,
    GetSystemMetrics, GetWindowLongW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, IsZoomed, OBJID_CLIENT,
    SM_CMONITORS, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SMTO_ABORTIFHUNG, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SendMessageTimeoutW, SetForegroundWindow,
    SetWindowPos, ShowWindow, WM_GETOBJECT, WM_NULL, WS_EX_TOOLWINDOW,
};
use windows::core::{AgileReference, BSTR};
use windows_core::BOOL;

use crate::action::{
    Action, ActionRequest, ActionResult, ActionStepOutcome, Direction, DragParams, KeyCombo,
    Modifier, MouseButton, MouseEvent, MouseEventKind, sleep_brief,
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

const CF_UNICODETEXT_FMT: u32 = 13;
const CHROMIUM_WINDOW_CLASS: &str = "Chrome_WidgetWin_1";

#[derive(Clone)]
pub struct WinElementHandle {
    agile: AgileReference<IUIAutomationElement>,
    hwnd: isize,
    pid: u32,
}

/// Real Windows backend using Microsoft UI Automation COM (`CUIAutomation`), Win32 window &
/// monitor enumeration, UIPI token integrity checks, Chromium `WM_GETOBJECT` wake, `SendInput`,
/// Win32 clipboard, and GDI framebuffer capture.
pub struct WindowsAdapter;

impl Default for WindowsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsAdapter {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn ensure_com_mta() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
    }

    fn automation() -> Result<IUIAutomation, PilotError> {
        Self::ensure_com_mta();
        unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }.map_err(|err| {
            PilotError::new(
                ErrorCode::PermDenied,
                format!("UI Automation COM initialization failed: {err}"),
            )
        })
    }
}

fn find_target_window(filter: &WindowFilter) -> Result<WindowInfo, PilotError> {
    let windows = enumerate_windows_sync()?;
    if let Some(wid) = &filter.window_id
        && let Some(found) = windows.iter().find(|w| w.id == *wid)
    {
        return Ok(found.clone());
    }
    if let Some(app) = &filter.app {
        let needle = app.to_ascii_lowercase();
        if let Some(found) = windows.iter().find(|w| {
            w.app.to_ascii_lowercase().contains(&needle)
                || w.title.to_ascii_lowercase().contains(&needle)
        }) {
            return Ok(found.clone());
        }
    }
    if let Some(title) = &filter.title {
        let needle = title.to_ascii_lowercase();
        if let Some(found) = windows
            .iter()
            .find(|w| w.title.to_ascii_lowercase().contains(&needle))
        {
            return Ok(found.clone());
        }
    }
    if filter.app.is_none() && filter.window_id.is_none() && filter.title.is_none() {
        if let Some(fg) = windows.iter().find(|w| w.focused) {
            return Ok(fg.clone());
        }
        if let Some(first) = windows.into_iter().next() {
            return Ok(first);
        }
    }
    Err(PilotError::new(
        ErrorCode::WindowNotFound,
        "no matching Windows HWND found",
    ))
}

fn enumerate_windows_sync() -> Result<Vec<WindowInfo>, PilotError> {
    struct EnumCtx {
        fg: HWND,
        out: Vec<WindowInfo>,
    }
    let fg = unsafe { GetForegroundWindow() };
    let mut ctx = EnumCtx {
        fg,
        out: Vec::new(),
    };
    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = unsafe { &mut *(lparam.0 as *mut EnumCtx) };
        if unsafe { !IsWindowVisible(hwnd).as_bool() } {
            return BOOL(1);
        }
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
        if (ex_style & WS_EX_TOOLWINDOW.0) != 0 {
            return BOOL(1);
        }
        let title_len = unsafe { GetWindowTextLengthW(hwnd) };
        if title_len <= 0 {
            return BOOL(1);
        }
        let mut buf = vec![0_u16; (title_len + 1) as usize];
        let copied = unsafe { GetWindowTextW(hwnd, &mut buf) };
        let title = String::from_utf16_lossy(&buf[..copied.max(0) as usize]);
        if title.trim().is_empty() {
            return BOOL(1);
        }
        let mut rect = RECT::default();
        let _ = unsafe { GetWindowRect(hwnd, &mut rect) };
        let mut pid = 0_u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        let app_name = process_exe_basename(pid).unwrap_or_else(|| format!("pid-{pid}"));
        ctx.out.push(WindowInfo {
            id: format!("{}", hwnd.0 as isize),
            title,
            app: app_name,
            pid,
            bounds: Rect::new(
                f64::from(rect.left),
                f64::from(rect.top),
                f64::from((rect.right - rect.left).max(0)),
                f64::from((rect.bottom - rect.top).max(0)),
            ),
            focused: hwnd == ctx.fg,
            minimized: unsafe { IsIconic(hwnd).as_bool() },
            maximized: unsafe { IsZoomed(hwnd).as_bool() },
            visible: true,
            workspace: None,
            process_token: read_process_integrity_label(pid),
        });
        BOOL(1)
    }

    let _ = unsafe { EnumWindows(Some(cb), LPARAM(&mut ctx as *mut EnumCtx as isize)) };
    Ok(ctx.out)
}

fn process_exe_basename(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0_u16; 512];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
        .is_ok();
        let _ = windows::Win32::Foundation::CloseHandle(handle);
        if !ok {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..size as usize]);
        std::path::Path::new(&full)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_owned)
    }
}

fn read_process_integrity_rid(pid: u32) -> Option<u32> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let proc_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut token = windows::Win32::Foundation::HANDLE::default();
        let tok_ok = OpenProcessToken(proc_handle, TOKEN_QUERY, &mut token).is_ok();
        let _ = windows::Win32::Foundation::CloseHandle(proc_handle);
        if !tok_ok {
            return None;
        }
        let mut needed = 0_u32;
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut needed);
        if needed == 0 {
            let _ = windows::Win32::Foundation::CloseHandle(token);
            return None;
        }
        let mut buf = vec![0_u8; needed as usize];
        let info_ok = GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buf.as_mut_ptr().cast()),
            needed,
            &mut needed,
        )
        .is_ok();
        let _ = windows::Win32::Foundation::CloseHandle(token);
        if !info_ok {
            return None;
        }
        let label = &*(buf.as_ptr() as *const TOKEN_MANDATORY_LABEL);
        let sid = label.Label.Sid;
        if sid.is_invalid() {
            return None;
        }
        let sub_count = *GetSidSubAuthorityCount(sid);
        if sub_count == 0 {
            return None;
        }
        Some(*GetSidSubAuthority(sid, u32::from(sub_count - 1)))
    }
}

fn read_process_integrity_label(pid: u32) -> Option<String> {
    read_process_integrity_rid(pid).map(|rid| match rid {
        0x0000..=0x0fff => "untrusted".to_owned(),
        0x1000..=0x1fff => "low".to_owned(),
        0x2000..=0x2fff => "medium".to_owned(),
        0x3000..=0x3fff => "high".to_owned(),
        _ => "system".to_owned(),
    })
}

fn ensure_window_responsive(hwnd: HWND) -> Result<(), PilotError> {
    let mut res = 0_usize;
    let status = unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_NULL,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            600,
            Some(&mut res),
        )
    };
    if status == LRESULT(0) {
        return Err(PilotError::new(
            ErrorCode::AppUnresponsive,
            "target window did not answer Win32 message-pump ping within 600ms",
        ));
    }
    Ok(())
}

fn wake_chromium_if_needed(hwnd: HWND) {
    let mut class_buf = [0_u16; 128];
    let len = unsafe { GetClassNameW(hwnd, &mut class_buf) };
    if len > 0 {
        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
        if class_name.trim() == CHROMIUM_WINDOW_CLASS {
            let mut res = 0_usize;
            let _ = unsafe {
                SendMessageTimeoutW(
                    hwnd,
                    WM_GETOBJECT,
                    WPARAM(0),
                    LPARAM(OBJID_CLIENT.0 as isize),
                    SMTO_ABORTIFHUNG,
                    300,
                    Some(&mut res),
                )
            };
        }
    }
}

fn uia_control_type_role(ct: UIA_CONTROLTYPE_ID) -> &'static str {
    if ct == UIA_ButtonControlTypeId || ct == UIA_SplitButtonControlTypeId {
        "button"
    } else if ct == UIA_EditControlTypeId {
        "textfield"
    } else if ct == UIA_CheckBoxControlTypeId {
        "checkbox"
    } else if ct == UIA_RadioButtonControlTypeId {
        "radiobutton"
    } else if ct == UIA_ComboBoxControlTypeId {
        "combobox"
    } else if ct == UIA_MenuItemControlTypeId {
        "menuitem"
    } else if ct == UIA_TabItemControlTypeId {
        "tab"
    } else if ct == UIA_TabControlTypeId {
        "tablist"
    } else if ct == UIA_TreeItemControlTypeId {
        "treeitem"
    } else if ct == UIA_HyperlinkControlTypeId {
        "link"
    } else if ct == UIA_WindowControlTypeId {
        "window"
    } else if ct == UIA_TextControlTypeId {
        "statictext"
    } else if ct == UIA_DocumentControlTypeId {
        "document"
    } else if ct == UIA_ListControlTypeId {
        "list"
    } else if ct == UIA_ListItemControlTypeId {
        "listitem"
    } else if ct == UIA_ScrollBarControlTypeId {
        "scrollbar"
    } else if ct == UIA_SliderControlTypeId {
        "slider"
    } else if ct == UIA_ToolBarControlTypeId {
        "toolbar"
    } else if ct == UIA_MenuControlTypeId || ct == UIA_MenuBarControlTypeId {
        "menu"
    } else if ct == UIA_TableControlTypeId || ct == UIA_DataGridControlTypeId {
        "table"
    } else if ct == UIA_DataItemControlTypeId {
        "row"
    } else if ct == UIA_ImageControlTypeId {
        "image"
    } else if ct == UIA_ProgressBarControlTypeId {
        "progressbar"
    } else if ct == UIA_StatusBarControlTypeId {
        "status"
    } else if ct == UIA_ToolTipControlTypeId {
        "tooltip"
    } else if ct == UIA_TreeControlTypeId {
        "outline"
    } else if ct == UIA_GroupControlTypeId || ct == UIA_PaneControlTypeId {
        "group"
    } else {
        "group"
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_uia_node(
    walker: &windows::Win32::UI::Accessibility::IUIAutomationTreeWalker,
    el: &IUIAutomationElement,
    hwnd: isize,
    depth: usize,
    max_depth: usize,
    visited: &mut usize,
    max_nodes: usize,
    deadline: Deadline,
) -> AccessibilityNode {
    *visited += 1;
    let role = unsafe { el.CurrentControlType() }
        .map(uia_control_type_role)
        .unwrap_or("group")
        .to_owned();
    let name = unsafe { el.CurrentName() }
        .ok()
        .map(|b| b.to_string())
        .filter(|s| !s.trim().is_empty());
    let identifier = unsafe { el.CurrentAutomationId() }
        .ok()
        .map(|b| b.to_string())
        .filter(|s| !s.trim().is_empty());
    let enabled = unsafe { el.CurrentIsEnabled() }
        .map(|b| b.as_bool())
        .unwrap_or(true);
    let focused = unsafe { el.CurrentHasKeyboardFocus() }
        .map(|b| b.as_bool())
        .unwrap_or(false);
    let bounds = unsafe { el.CurrentBoundingRectangle() }.ok().map(|r| {
        Rect::new(
            f64::from(r.left),
            f64::from(r.top),
            f64::from((r.right - r.left).max(0)),
            f64::from((r.bottom - r.top).max(0)),
        )
    });

    let value = unsafe { el.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) }
        .ok()
        .and_then(|vp| unsafe { vp.CurrentValue() }.ok().map(|b| b.to_string()));
    let checked =
        unsafe { el.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId) }
            .ok()
            .and_then(|tp| {
                unsafe { tp.CurrentToggleState() }
                    .ok()
                    .map(|ts| ts == ToggleState_On)
            });
    let expanded = unsafe {
        el.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(UIA_ExpandCollapsePatternId)
    }
    .ok()
    .and_then(|ep| {
        unsafe { ep.CurrentExpandCollapseState() }
            .ok()
            .map(|es| es == ExpandCollapseState_Expanded)
    });

    let mut actions = Vec::new();
    if unsafe { el.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId) }.is_ok()
    {
        actions.push("click".to_owned());
    }
    if value.is_some() {
        actions.push("set_value".to_owned());
    }
    if checked.is_some() {
        actions.push("toggle".to_owned());
    }
    if expanded.is_some() {
        actions.push("expand".to_owned());
    }

    let mut state = ElementState {
        value: value.clone(),
        enabled: Some(enabled),
        focused: Some(focused),
        checked,
        selected: None,
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

    let rt_id = unsafe { el.GetRuntimeId() }
        .ok()
        .and_then(|sa| safearray_i32_to_string(sa))
        .unwrap_or_else(|| format!("uia-{hwnd}-{}", *visited));

    let mut children = Vec::new();
    let mut truncated = false;
    if depth < max_depth && *visited < max_nodes && !deadline.is_expired() {
        let mut child_opt = unsafe { walker.GetFirstChildElement(el) }.ok();
        while let Some(child) = child_opt {
            if *visited >= max_nodes || deadline.is_expired() {
                truncated = true;
                break;
            }
            children.push(walk_uia_node(
                walker,
                &child,
                hwnd,
                depth + 1,
                max_depth,
                visited,
                max_nodes,
                deadline,
            ));
            child_opt = unsafe { walker.GetNextSiblingElement(&child) }.ok();
        }
    }

    let interactive = !actions.is_empty() || AccessibilityNode::is_interactive_role(&role);
    AccessibilityNode {
        id: format!("uia:{hwnd}:{rt_id}"),
        ref_id: None,
        role,
        name,
        value,
        description: None,
        identifier,
        hint: None,
        bounds,
        state,
        actions,
        interactive,
        children_count: children.len(),
        subtree_truncated: truncated,
        children,
    }
}

fn safearray_i32_to_string(sa: *mut windows::Win32::System::Com::SAFEARRAY) -> Option<String> {
    if sa.is_null() {
        return None;
    }
    unsafe {
        use windows::Win32::System::Ole::{
            SafeArrayDestroy, SafeArrayGetElement, SafeArrayGetLBound, SafeArrayGetUBound,
        };
        let lb = SafeArrayGetLBound(sa, 1).ok()?;
        let ub = SafeArrayGetUBound(sa, 1).ok()?;
        let mut parts = Vec::new();
        for idx in lb..=ub {
            let mut val: i32 = 0;
            if SafeArrayGetElement(sa, &idx, (&mut val as *mut i32).cast()).is_ok() {
                parts.push(val.to_string());
            }
        }
        let _ = SafeArrayDestroy(sa);
        (!parts.is_empty()).then(|| parts.join("."))
    }
}

fn find_uia_element_by_entry(
    automation: &IUIAutomation,
    hwnd: HWND,
    entry: &RefEntry,
    deadline: Deadline,
) -> Result<IUIAutomationElement, PilotError> {
    let root = unsafe { automation.ElementFromHandle(hwnd) }.map_err(|err| {
        PilotError::new(
            ErrorCode::WindowNotFound,
            format!("ElementFromHandle failed: {err}"),
        )
    })?;
    let walker = unsafe { automation.ControlViewWalker() }.map_err(|err| {
        PilotError::new(
            ErrorCode::Internal,
            format!("ControlViewWalker failed: {err}"),
        )
    })?;
    find_matching_uia_recursive(&walker, &root, entry, 0, 18, deadline)
        .ok_or_else(|| PilotError::stale_ref(&entry.ref_id))
}

fn find_matching_uia_recursive(
    walker: &windows::Win32::UI::Accessibility::IUIAutomationTreeWalker,
    el: &IUIAutomationElement,
    entry: &RefEntry,
    depth: usize,
    max_depth: usize,
    deadline: Deadline,
) -> Option<IUIAutomationElement> {
    if deadline.is_expired() {
        return None;
    }
    let role = unsafe { el.CurrentControlType() }
        .map(uia_control_type_role)
        .unwrap_or("group");
    if role.eq_ignore_ascii_case(&entry.identity.role) {
        if let Some(exp_id) = &entry.identity.identifier
            && unsafe { el.CurrentAutomationId() }
                .ok()
                .is_some_and(|b| b.iter().copied().eq(exp_id.encode_utf16()))
        {
            return Some(el.clone());
        }
        if let Some(exp_name) = &entry.identity.name
            && unsafe { el.CurrentName() }
                .ok()
                .is_some_and(|b| b.iter().copied().eq(exp_name.encode_utf16()))
        {
            return Some(el.clone());
        }
    }
    if depth >= max_depth {
        return None;
    }
    let mut child_opt = unsafe { walker.GetFirstChildElement(el) }.ok();
    while let Some(child) = child_opt {
        if let Some(hit) =
            find_matching_uia_recursive(walker, &child, entry, depth + 1, max_depth, deadline)
        {
            return Some(hit);
        }
        child_opt = unsafe { walker.GetNextSiblingElement(&child) }.ok();
    }
    None
}

impl ObservationOps for WindowsAdapter {
    fn observe_tree(
        &self,
        filter: &WindowFilter,
        request: &ObservationRequest,
        deadline: Deadline,
    ) -> Result<(WindowInfo, AccessibilityNode, bool), PilotError> {
        deadline.ensure_remaining()?;
        let win = find_target_window(filter)?;
        let hwnd_val = win.id.parse::<isize>().unwrap_or(0);
        let hwnd = HWND(hwnd_val as *mut c_void);
        if unsafe { !IsWindow(Some(hwnd)).as_bool() } {
            return Err(PilotError::new(
                ErrorCode::WindowNotFound,
                "HWND destroyed before observation",
            ));
        }
        ensure_window_responsive(hwnd)?;
        wake_chromium_if_needed(hwnd);

        let automation = Self::automation()?;
        let root_el = unsafe { automation.ElementFromHandle(hwnd) }.map_err(|err| {
            PilotError::new(
                ErrorCode::WindowNotFound,
                format!("UIA ElementFromHandle failed: {err}"),
            )
        })?;
        let walker = unsafe { automation.ControlViewWalker() }.map_err(|err| {
            PilotError::new(
                ErrorCode::Internal,
                format!("UIA ControlViewWalker failed: {err}"),
            )
        })?;
        let mut visited = 0_usize;
        let tree = walk_uia_node(
            &walker,
            &root_el,
            hwnd_val,
            0,
            request.max_depth.min(20),
            &mut visited,
            request.max_nodes,
            deadline,
        );
        Ok((
            win,
            tree,
            visited < request.max_nodes && !deadline.is_expired(),
        ))
    }

    fn resolve_element(
        &self,
        entry: &RefEntry,
        deadline: Deadline,
    ) -> Result<(NativeHandle, AccessibilityNode), PilotError> {
        deadline.ensure_remaining()?;
        let hwnd_val = entry
            .scope
            .as_ref()
            .and_then(|s| s.window_id.parse::<isize>().ok())
            .ok_or_else(|| PilotError::stale_ref(&entry.ref_id))?;
        let pid = entry.scope.as_ref().map(|s| s.pid).unwrap_or(0);
        let hwnd = HWND(hwnd_val as *mut c_void);
        let automation = Self::automation()?;
        let el = find_uia_element_by_entry(&automation, hwnd, entry, deadline)?;
        let agile = AgileReference::new(&el).map_err(|err| {
            PilotError::new(
                ErrorCode::Internal,
                format!("AgileReference wrap failed: {err}"),
            )
        })?;
        let live_node = AccessibilityNode {
            id: entry.node_id.clone(),
            role: unsafe { el.CurrentControlType() }
                .map(uia_control_type_role)
                .unwrap_or("group")
                .to_owned(),
            name: unsafe { el.CurrentName() }.ok().map(|b| b.to_string()),
            identifier: unsafe { el.CurrentAutomationId() }
                .ok()
                .map(|b| b.to_string()),
            bounds: unsafe { el.CurrentBoundingRectangle() }.ok().map(|r| {
                Rect::new(
                    f64::from(r.left),
                    f64::from(r.top),
                    f64::from((r.right - r.left).max(0)),
                    f64::from((r.bottom - r.top).max(0)),
                )
            }),
            state: ElementState::default(),
            actions: entry.capabilities.actions.clone(),
            interactive: entry.capabilities.interactive,
            ..Default::default()
        };
        Ok((
            NativeHandle::new(WinElementHandle {
                agile,
                hwnd: hwnd_val,
                pid,
            }),
            live_node,
        ))
    }

    fn get_live_element(
        &self,
        handle: &NativeHandle,
        deadline: Deadline,
    ) -> Result<LiveElement, PilotError> {
        deadline.ensure_remaining()?;
        Self::ensure_com_mta();
        let h = handle
            .downcast_ref::<WinElementHandle>()
            .ok_or_else(|| PilotError::invalid_input("expected WinElementHandle"))?;
        let el: IUIAutomationElement = h.agile.resolve().map_err(|err| {
            PilotError::new(ErrorCode::StaleRef, format!("UIA resolve failed: {err}"))
        })?;
        let role = unsafe { el.CurrentControlType() }
            .map(uia_control_type_role)
            .unwrap_or("group")
            .to_owned();
        let name = unsafe { el.CurrentName() }.ok().map(|b| b.to_string());
        let enabled = unsafe { el.CurrentIsEnabled() }
            .map(|b| b.as_bool())
            .unwrap_or(true);
        let focused = unsafe { el.CurrentHasKeyboardFocus() }
            .map(|b| b.as_bool())
            .unwrap_or(false);
        let value =
            unsafe { el.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) }
                .ok()
                .and_then(|vp| unsafe { vp.CurrentValue() }.ok().map(|b| b.to_string()));
        let checked =
            unsafe { el.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId) }
                .ok()
                .and_then(|tp| {
                    unsafe { tp.CurrentToggleState() }
                        .ok()
                        .map(|ts| ts == ToggleState_On)
                });
        let expanded = unsafe {
            el.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(
                UIA_ExpandCollapsePatternId,
            )
        }
        .ok()
        .and_then(|ep| {
            unsafe { ep.CurrentExpandCollapseState() }
                .ok()
                .map(|es| es == ExpandCollapseState_Expanded)
        });
        let mut state = ElementState {
            value,
            enabled: Some(enabled),
            focused: Some(focused),
            checked,
            selected: None,
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
        Ok(LiveElement {
            id: format!("uia:{}", h.hwnd),
            role,
            name,
            bounds: None,
            state,
            actions: Vec::new(),
        })
    }

    fn list_surfaces(&self, deadline: Deadline) -> Result<Vec<SurfaceInfo>, PilotError> {
        deadline.ensure_remaining()?;
        Ok(vec![SurfaceInfo {
            id: "win32-desktop".to_owned(),
            surface: SnapshotSurface::Desktop,
            app: "windows".to_owned(),
            title: "Windows UI Automation Desktop".to_owned(),
            bounds: None,
            available: true,
        }])
    }
}

impl ActionOps for WindowsAdapter {
    fn execute_action(
        &self,
        handle: &NativeHandle,
        request: &ActionRequest,
        lease: &InteractionLease,
    ) -> Result<ActionResult, PilotError> {
        lease.deadline().ensure_remaining()?;
        Self::ensure_com_mta();
        let h = handle
            .downcast_ref::<WinElementHandle>()
            .ok_or_else(|| PilotError::invalid_input("expected WinElementHandle"))?;
        let el: IUIAutomationElement = h
            .agile
            .resolve()
            .map_err(|err| PilotError::new(ErrorCode::StaleRef, format!("UIA resolve: {err}")))?;

        if let (Some(self_rid), Some(target_rid)) = (
            read_process_integrity_rid(std::process::id()),
            read_process_integrity_rid(h.pid),
        ) && target_rid > self_rid
        {
            return Err(PilotError::new(
                ErrorCode::ElevationRequired,
                "target process runs at higher UIPI integrity level than aphrody-os",
            ));
        }

        let mechanism = match &request.action {
            Action::Click | Action::DoubleClick | Action::TripleClick => {
                if let Ok(inv) = unsafe {
                    el.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
                } {
                    unsafe { inv.Invoke() }.map_err(|err| {
                        PilotError::new(
                            ErrorCode::ActionFailed,
                            format!("UIA Invoke failed: {err}"),
                        )
                    })?;
                    "uia_invoke"
                } else {
                    let r = unsafe { el.CurrentBoundingRectangle() }.map_err(|err| {
                        PilotError::new(
                            ErrorCode::ActionFailed,
                            format!("UIA bounds failed: {err}"),
                        )
                    })?;
                    let cx = f64::from(r.left + r.right) / 2.0;
                    let cy = f64::from(r.top + r.bottom) / 2.0;
                    send_mouse_click(cx, cy, MouseButton::Left, 1)?;
                    "send_input_click"
                }
            }
            Action::RightClick => {
                let r = unsafe { el.CurrentBoundingRectangle() }.map_err(|err| {
                    PilotError::new(ErrorCode::ActionFailed, format!("UIA bounds failed: {err}"))
                })?;
                send_mouse_click(
                    f64::from(r.left + r.right) / 2.0,
                    f64::from(r.top + r.bottom) / 2.0,
                    MouseButton::Right,
                    1,
                )?;
                "send_input_right_click"
            }
            Action::Focus => {
                unsafe { el.SetFocus() }.map_err(|err| {
                    PilotError::new(ErrorCode::ActionFailed, format!("UIA SetFocus: {err}"))
                })?;
                "uia_set_focus"
            }
            Action::SetValue(_) | Action::Clear => {
                let target_val = if matches!(request.action, Action::Clear) {
                    ""
                } else {
                    text_or_empty(&request.action)
                };
                let vp = unsafe {
                    el.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
                }
                .map_err(|err| {
                    PilotError::new(
                        ErrorCode::ActionNotSupported,
                        format!("no ValuePattern: {err}"),
                    )
                })?;
                unsafe { vp.SetValue(&BSTR::from(target_val)) }.map_err(|err| {
                    PilotError::new(ErrorCode::ActionFailed, format!("UIA SetValue: {err}"))
                })?;
                "uia_value_pattern"
            }
            Action::TypeText(text) => {
                let _ = unsafe { el.SetFocus() };
                send_unicode_text(text)?;
                "send_input_unicode"
            }
            Action::Toggle | Action::Check | Action::Uncheck => {
                let tp = unsafe {
                    el.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
                }
                .map_err(|err| {
                    PilotError::new(
                        ErrorCode::ActionNotSupported,
                        format!("no TogglePattern: {err}"),
                    )
                })?;
                unsafe { tp.Toggle() }.map_err(|err| {
                    PilotError::new(ErrorCode::ActionFailed, format!("UIA Toggle: {err}"))
                })?;
                "uia_toggle_pattern"
            }
            Action::Expand => {
                let ep = unsafe {
                    el.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(
                        UIA_ExpandCollapsePatternId,
                    )
                }
                .map_err(|err| {
                    PilotError::new(
                        ErrorCode::ActionNotSupported,
                        format!("no ExpandCollapsePattern: {err}"),
                    )
                })?;
                unsafe { ep.Expand() }.map_err(|err| {
                    PilotError::new(ErrorCode::ActionFailed, format!("UIA Expand: {err}"))
                })?;
                "uia_expand_pattern"
            }
            Action::Collapse => {
                let ep = unsafe {
                    el.GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(
                        UIA_ExpandCollapsePatternId,
                    )
                }
                .map_err(|err| {
                    PilotError::new(
                        ErrorCode::ActionNotSupported,
                        format!("no ExpandCollapsePattern: {err}"),
                    )
                })?;
                unsafe { ep.Collapse() }.map_err(|err| {
                    PilotError::new(ErrorCode::ActionFailed, format!("UIA Collapse: {err}"))
                })?;
                "uia_collapse_pattern"
            }
            Action::Select(_) => {
                let sp = unsafe {
                    el.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                        UIA_SelectionItemPatternId,
                    )
                }
                .map_err(|err| {
                    PilotError::new(
                        ErrorCode::ActionNotSupported,
                        format!("no SelectionItemPattern: {err}"),
                    )
                })?;
                unsafe { sp.Select() }.map_err(|err| {
                    PilotError::new(ErrorCode::ActionFailed, format!("UIA Select: {err}"))
                })?;
                "uia_selection_item"
            }
            Action::Scroll {
                direction,
                amount: _,
            } => {
                let sp = unsafe {
                    el.GetCurrentPatternAs::<IUIAutomationScrollPattern>(UIA_ScrollPatternId)
                }
                .map_err(|err| {
                    PilotError::new(
                        ErrorCode::ActionNotSupported,
                        format!("no ScrollPattern: {err}"),
                    )
                })?;
                let (horiz, vert) = match direction {
                    Direction::Up => (ScrollAmount_NoAmount, ScrollAmount_SmallDecrement),
                    Direction::Down => (ScrollAmount_NoAmount, ScrollAmount_SmallIncrement),
                    Direction::Left => (ScrollAmount_SmallDecrement, ScrollAmount_NoAmount),
                    Direction::Right => (ScrollAmount_SmallIncrement, ScrollAmount_NoAmount),
                };
                unsafe { sp.Scroll(horiz, vert) }.map_err(|err| {
                    PilotError::new(ErrorCode::ActionFailed, format!("UIA Scroll: {err}"))
                })?;
                "uia_scroll_pattern"
            }
            Action::ScrollTo => {
                let sip = unsafe {
                    el.GetCurrentPatternAs::<IUIAutomationScrollItemPattern>(
                        UIA_ScrollItemPatternId,
                    )
                }
                .map_err(|err| {
                    PilotError::new(
                        ErrorCode::ActionNotSupported,
                        format!("no ScrollItemPattern: {err}"),
                    )
                })?;
                unsafe { sip.ScrollIntoView() }.map_err(|err| {
                    PilotError::new(
                        ErrorCode::ActionFailed,
                        format!("UIA ScrollIntoView: {err}"),
                    )
                })?;
                "uia_scroll_item"
            }
        };

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
                mechanism: mechanism.to_owned(),
                ok: true,
                duration_ms: 0,
                detail: None,
            }],
        })
    }
}

fn text_or_empty(act: &Action) -> &str {
    match act {
        Action::SetValue(s) | Action::TypeText(s) => s.as_str(),
        _ => "",
    }
}

fn send_unicode_text(text: &str) -> Result<(), PilotError> {
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let mut inputs = Vec::with_capacity(utf16.len() * 2);
    for code in utf16 {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: code,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: code,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(PilotError::new(
            ErrorCode::ActionFailed,
            "SendInput unicode returned 0",
        ));
    }
    Ok(())
}

fn map_vk(key: &str) -> Option<VIRTUAL_KEY> {
    let lower = key.trim().to_ascii_lowercase();
    let vk = match lower.as_str() {
        "enter" | "return" => VK_RETURN,
        "esc" | "escape" => VK_ESCAPE,
        "tab" => VK_TAB,
        "space" => VK_SPACE,
        "backspace" => VK_BACK,
        "delete" | "del" => VK_DELETE,
        "home" => VK_HOME,
        "end" => VK_END,
        "pageup" => VK_PRIOR,
        "pagedown" => VK_NEXT,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "up" => VK_UP,
        "down" => VK_DOWN,
        _ => {
            if lower.len() == 1 {
                let b = lower.as_bytes()[0];
                if b.is_ascii_alphanumeric() {
                    return Some(VIRTUAL_KEY(u16::from(b.to_ascii_uppercase())));
                }
            }
            return None;
        }
    };
    Some(vk)
}

fn mod_vk(m: Modifier) -> VIRTUAL_KEY {
    match m {
        Modifier::Shift => VK_SHIFT,
        Modifier::Control => VK_CONTROL,
        Modifier::Alt => VK_MENU,
        Modifier::Meta => VK_LWIN,
    }
}

fn send_key_vk(combo: &KeyCombo, down: bool) -> Result<(), PilotError> {
    let vk = map_vk(&combo.key).ok_or_else(|| {
        PilotError::invalid_input(format!("unsupported Windows VK key '{}'", combo.key))
    })?;
    let flags = if down {
        Default::default()
    } else {
        KEYEVENTF_KEYUP
    };
    let mut inputs = Vec::new();
    for m in &combo.modifiers {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: mod_vk(*m),
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }
    inputs.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    });
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(PilotError::new(
            ErrorCode::ActionFailed,
            "SendInput keyboard returned 0",
        ));
    }
    Ok(())
}

fn normalize_virtual_screen_xy(x: f64, y: f64) -> (i32, i32) {
    let vx = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) } as f64;
    let vy = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) } as f64;
    let vw = (unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) } as f64).max(1.0);
    let vh = (unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) } as f64).max(1.0);
    let nx = (((x - vx) * 65535.0) / vw).round() as i32;
    let ny = (((y - vy) * 65535.0) / vh).round() as i32;
    (nx.clamp(0, 65535), ny.clamp(0, 65535))
}

fn send_mouse_click(x: f64, y: f64, btn: MouseButton, count: u8) -> Result<(), PilotError> {
    let (nx, ny) = normalize_virtual_screen_xy(x, y);
    let (down_f, up_f) = match btn {
        MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
    };
    let mut inputs = vec![INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: nx,
                dy: ny,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }];
    for _ in 0..count.max(1) {
        inputs.push(INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: nx,
                    dy: ny,
                    mouseData: 0,
                    dwFlags: down_f | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
        inputs.push(INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: nx,
                    dy: ny,
                    mouseData: 0,
                    dwFlags: up_f | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(PilotError::new(
            ErrorCode::ActionFailed,
            "SendInput mouse returned 0",
        ));
    }
    Ok(())
}

impl InputOps for WindowsAdapter {
    fn press_key(&self, combo: &KeyCombo, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        send_key_vk(combo, true)?;
        send_key_vk(combo, false)
    }

    fn key_state(
        &self,
        combo: &KeyCombo,
        down: bool,
        deadline: Deadline,
    ) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        send_key_vk(combo, down)
    }

    fn mouse_event(&self, event: &MouseEvent, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let (nx, ny) = normalize_virtual_screen_xy(event.x, event.y);
        match event.kind {
            MouseEventKind::Click => {
                send_mouse_click(event.x, event.y, event.button, event.click_count)
            }
            MouseEventKind::Move => {
                let inp = [INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: nx,
                            dy: ny,
                            mouseData: 0,
                            dwFlags: MOUSEEVENTF_MOVE
                                | MOUSEEVENTF_ABSOLUTE
                                | MOUSEEVENTF_VIRTUALDESK,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                }];
                let _ = unsafe { SendInput(&inp, std::mem::size_of::<INPUT>() as i32) };
                Ok(())
            }
            MouseEventKind::Down | MouseEventKind::Up => {
                let flag = match (event.kind, event.button) {
                    (MouseEventKind::Down, MouseButton::Left) => MOUSEEVENTF_LEFTDOWN,
                    (MouseEventKind::Up, MouseButton::Left) => MOUSEEVENTF_LEFTUP,
                    (MouseEventKind::Down, MouseButton::Right) => MOUSEEVENTF_RIGHTDOWN,
                    (MouseEventKind::Up, MouseButton::Right) => MOUSEEVENTF_RIGHTUP,
                    (MouseEventKind::Down, MouseButton::Middle) => MOUSEEVENTF_MIDDLEDOWN,
                    (MouseEventKind::Up, MouseButton::Middle) => MOUSEEVENTF_MIDDLEUP,
                    _ => MOUSEEVENTF_LEFTDOWN,
                };
                let inp = [INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: nx,
                            dy: ny,
                            mouseData: 0,
                            dwFlags: flag | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                }];
                let _ = unsafe { SendInput(&inp, std::mem::size_of::<INPUT>() as i32) };
                Ok(())
            }
            MouseEventKind::Wheel => {
                let mut inputs = Vec::new();
                if event.wheel_dy != 0 {
                    inputs.push(INPUT {
                        r#type: INPUT_MOUSE,
                        Anonymous: INPUT_0 {
                            mi: MOUSEINPUT {
                                dx: nx,
                                dy: ny,
                                mouseData: event.wheel_dy as u32,
                                dwFlags: MOUSEEVENTF_WHEEL,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    });
                }
                if event.wheel_dx != 0 {
                    inputs.push(INPUT {
                        r#type: INPUT_MOUSE,
                        Anonymous: INPUT_0 {
                            mi: MOUSEINPUT {
                                dx: nx,
                                dy: ny,
                                mouseData: event.wheel_dx as u32,
                                dwFlags: MOUSEEVENTF_HWHEEL,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    });
                }
                if !inputs.is_empty() {
                    let _ = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
                }
                Ok(())
            }
        }
    }

    fn drag(&self, params: &DragParams, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        self.mouse_event(
            &MouseEvent {
                kind: MouseEventKind::Down,
                x: params.from_x,
                y: params.from_y,
                button: params.button,
                click_count: 1,
                modifiers: Vec::new(),
                wheel_dx: 0,
                wheel_dy: 0,
            },
            deadline,
        )?;
        let steps = params.steps.max(1);
        let delay = Duration::from_millis(params.duration_ms / u64::from(steps));
        for i in 1..=steps {
            let t = f64::from(i) / f64::from(steps);
            self.mouse_event(
                &MouseEvent {
                    kind: MouseEventKind::Move,
                    x: params.from_x + (params.to_x - params.from_x) * t,
                    y: params.from_y + (params.to_y - params.from_y) * t,
                    button: params.button,
                    click_count: 0,
                    modifiers: Vec::new(),
                    wheel_dx: 0,
                    wheel_dy: 0,
                },
                deadline,
            )?;
            if !delay.is_zero() {
                sleep_brief(delay);
            }
        }
        self.mouse_event(
            &MouseEvent {
                kind: MouseEventKind::Up,
                x: params.to_x,
                y: params.to_y,
                button: params.button,
                click_count: 1,
                modifiers: Vec::new(),
                wheel_dx: 0,
                wheel_dy: 0,
            },
            deadline,
        )
    }

    fn clipboard_get(
        &self,
        _format: ClipboardFormat,
        deadline: Deadline,
    ) -> Result<ClipboardContent, PilotError> {
        deadline.ensure_remaining()?;
        unsafe {
            OpenClipboard(None).map_err(|err| {
                PilotError::new(
                    ErrorCode::ActionFailed,
                    format!("OpenClipboard failed: {err}"),
                )
            })?;
            let result = (|| {
                let handle = GetClipboardData(CF_UNICODETEXT_FMT).map_err(|err| {
                    PilotError::new(
                        ErrorCode::ElementNotFound,
                        format!("clipboard text empty: {err}"),
                    )
                })?;
                let hmem = windows::Win32::Foundation::HGLOBAL(handle.0);
                let ptr = GlobalLock(hmem) as *const u16;
                if ptr.is_null() {
                    return Err(PilotError::new(
                        ErrorCode::ActionFailed,
                        "GlobalLock failed",
                    ));
                }
                let mut len = 0_usize;
                while *ptr.add(len) != 0 {
                    len += 1;
                }
                let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
                let _ = GlobalUnlock(hmem);
                Ok(ClipboardContent::Text { text })
            })();
            let _ = CloseClipboard();
            result
        }
    }

    fn clipboard_set(
        &self,
        content: &ClipboardContent,
        deadline: Deadline,
    ) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let text = match content {
            ClipboardContent::Text { text } => text.as_str(),
            ClipboardContent::Html { html, .. } => html.as_str(),
            _ => {
                return Err(PilotError::new(
                    ErrorCode::ActionNotSupported,
                    "only UTF-16 text clipboard writes are enabled",
                ));
            }
        };
        let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = utf16.len() * std::mem::size_of::<u16>();
        unsafe {
            OpenClipboard(None).map_err(|err| {
                PilotError::new(
                    ErrorCode::ActionFailed,
                    format!("OpenClipboard failed: {err}"),
                )
            })?;
            let _ = EmptyClipboard();
            if let Ok(hmem) = GlobalAlloc(GMEM_MOVEABLE, bytes) {
                let ptr = GlobalLock(hmem) as *mut u16;
                if !ptr.is_null() {
                    std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
                    let _ = GlobalUnlock(hmem);
                    let _ = SetClipboardData(
                        CF_UNICODETEXT_FMT,
                        Some(windows::Win32::Foundation::HANDLE(hmem.0)),
                    );
                }
            }
            let _ = CloseClipboard();
        }
        Ok(())
    }

    fn clipboard_clear(&self, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        unsafe {
            if OpenClipboard(None).is_ok() {
                let _ = EmptyClipboard();
                let _ = CloseClipboard();
            }
        }
        Ok(())
    }
}

impl SystemOps for WindowsAdapter {
    fn backend_name(&self) -> &'static str {
        "windows-uia"
    }

    fn permissions(&self, deadline: Deadline) -> Result<PermissionReport, PilotError> {
        deadline.ensure_remaining()?;
        let uia_ok = Self::automation().is_ok();
        let monitors = unsafe { GetSystemMetrics(SM_CMONITORS) };
        let self_integrity = read_process_integrity_label(std::process::id());
        Ok(PermissionReport {
            platform: "windows".to_owned(),
            accessibility_granted: uia_ok,
            screen_recording_granted: monitors > 0,
            elevation_status: self_integrity,
            display_server: Some("dwm-win32".to_owned()),
            accessibility_detail: Some(if uia_ok {
                "CUIAutomation COM ready".to_owned()
            } else {
                "CUIAutomation unavailable".to_owned()
            }),
            screen_recording_detail: Some(format!("{monitors} monitor(s) attached")),
        })
    }

    fn list_apps(&self, deadline: Deadline) -> Result<Vec<AppInfo>, PilotError> {
        deadline.ensure_remaining()?;
        let windows = enumerate_windows_sync()?;
        let mut seen = std::collections::BTreeSet::new();
        let mut out = Vec::new();
        for w in windows {
            if seen.insert((w.pid, w.app.clone())) {
                out.push(AppInfo {
                    id: format!("{}:{}", w.pid, w.app),
                    name: w.app,
                    pid: w.pid,
                    bundle_id: None,
                    executable: None,
                    active: w.focused,
                });
            }
        }
        Ok(out)
    }

    fn list_windows(&self, deadline: Deadline) -> Result<Vec<WindowInfo>, PilotError> {
        deadline.ensure_remaining()?;
        enumerate_windows_sync()
    }

    fn list_displays(&self, deadline: Deadline) -> Result<Vec<DisplayInfo>, PilotError> {
        deadline.ensure_remaining()?;
        struct DisplayState {
            displays: Vec<DisplayInfo>,
        }
        unsafe extern "system" fn cb(
            hmon: HMONITOR,
            _hdc: HDC,
            _rc: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let st = unsafe { &mut *(lparam.0 as *mut DisplayState) };
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if unsafe { GetMonitorInfoW(hmon, &mut info).as_bool() } {
                let mut dpi_x = 96_u32;
                let mut dpi_y = 96_u32;
                let _ =
                    unsafe { GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
                let r = info.rcMonitor;
                st.displays.push(DisplayInfo {
                    id: format!("monitor-{}", hmon.0 as usize),
                    bounds: Rect::new(
                        f64::from(r.left),
                        f64::from(r.top),
                        f64::from((r.right - r.left).max(0)),
                        f64::from((r.bottom - r.top).max(0)),
                    ),
                    is_primary: (info.dwFlags & 1) != 0,
                    scale: f64::from(dpi_x) / 96.0,
                });
            }
            BOOL(1)
        }
        let mut st = DisplayState {
            displays: Vec::new(),
        };
        let _ = unsafe {
            EnumDisplayMonitors(
                None,
                None,
                Some(cb),
                LPARAM(&mut st as *mut DisplayState as isize),
            )
        };
        Ok(st.displays)
    }

    fn focus_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        let win = find_target_window(filter)?;
        let hwnd = HWND(win.id.parse::<isize>().unwrap_or(0) as *mut c_void);
        if unsafe { IsIconic(hwnd).as_bool() } {
            let _ = unsafe { ShowWindow(hwnd, SW_RESTORE) };
        }
        let _ = unsafe { SetForegroundWindow(hwnd) };
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
        let mut win = find_target_window(filter)?;
        let hwnd = HWND(win.id.parse::<isize>().unwrap_or(0) as *mut c_void);
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                None,
                x as i32,
                y as i32,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
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
        let mut win = find_target_window(filter)?;
        let hwnd = HWND(win.id.parse::<isize>().unwrap_or(0) as *mut c_void);
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                width as i32,
                height as i32,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
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
        let mut win = find_target_window(filter)?;
        let hwnd = HWND(win.id.parse::<isize>().unwrap_or(0) as *mut c_void);
        let _ = unsafe { ShowWindow(hwnd, SW_MINIMIZE) };
        win.minimized = true;
        Ok(win)
    }

    fn maximize_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        let mut win = find_target_window(filter)?;
        let hwnd = HWND(win.id.parse::<isize>().unwrap_or(0) as *mut c_void);
        let _ = unsafe { ShowWindow(hwnd, SW_MAXIMIZE) };
        win.maximized = true;
        Ok(win)
    }

    fn restore_window(
        &self,
        filter: &WindowFilter,
        deadline: Deadline,
    ) -> Result<WindowInfo, PilotError> {
        deadline.ensure_remaining()?;
        let mut win = find_target_window(filter)?;
        let hwnd = HWND(win.id.parse::<isize>().unwrap_or(0) as *mut c_void);
        let _ = unsafe { ShowWindow(hwnd, SW_RESTORE) };
        win.minimized = false;
        win.maximized = false;
        Ok(win)
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
            id: format!("{pid}:{app}"),
            name: app.to_owned(),
            pid,
            bundle_id: None,
            executable: Some(app.to_owned()),
            active: true,
        })
    }

    fn close_app(&self, app: &str, deadline: Deadline) -> Result<(), PilotError> {
        deadline.ensure_remaining()?;
        let windows = enumerate_windows_sync()?;
        let needle = app.to_ascii_lowercase();
        let pid = windows
            .into_iter()
            .find(|w| w.app.to_ascii_lowercase() == needle)
            .map(|w| w.pid)
            .ok_or_else(|| {
                PilotError::new(ErrorCode::AppNotFound, format!("app '{app}' not running"))
            })?;
        let _ = smol::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T"])
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
            id: format!("windows-{}", surface.as_str()),
            surface,
            app: "windows-shell".to_owned(),
            title: format!("Windows {}", surface.as_str()),
            bounds: None,
            available: true,
        })
    }

    fn screenshot(
        &self,
        filter: &WindowFilter,
        _display_index: Option<usize>,
        deadline: Deadline,
    ) -> Result<ImageBuffer, PilotError> {
        deadline.ensure_remaining()?;
        let (x, y, w, h) =
            if filter.app.is_some() || filter.window_id.is_some() || filter.title.is_some() {
                let win = find_target_window(filter)?;
                (
                    win.bounds.x as i32,
                    win.bounds.y as i32,
                    win.bounds.width.max(1.0) as i32,
                    win.bounds.height.max(1.0) as i32,
                )
            } else {
                (
                    0,
                    0,
                    unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(1),
                    unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(1),
                )
            };

        let rgba = capture_gdi_rect_rgba(x, y, w, h)?;
        let mut png = Vec::new();
        PngEncoder::new(Cursor::new(&mut png))
            .write_image(&rgba, w as u32, h as u32, ColorType::Rgba8.into())
            .map_err(|err| PilotError::new(ErrorCode::Internal, format!("encode png: {err}")))?;
        Ok(ImageBuffer {
            format: ImageFormat::Png,
            width: w as u32,
            height: h as u32,
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

fn capture_gdi_rect_rgba(x: i32, y: i32, w: i32, h: i32) -> Result<Vec<u8>, PilotError> {
    unsafe {
        let hwnd_desktop = GetDesktopWindow();
        let hdc_screen = GetDC(Some(hwnd_desktop));
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let hbm = CreateCompatibleBitmap(hdc_screen, w, h);
        let old = SelectObject(hdc_mem, hbm.into());
        let ok = BitBlt(hdc_mem, 0, 0, w, h, Some(hdc_screen), x, y, SRCCOPY).is_ok();
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0_u8; (w as usize) * (h as usize) * 4];
        if ok {
            let _ = GetDIBits(
                hdc_mem,
                hbm,
                0,
                h as u32,
                Some(pixels.as_mut_ptr().cast()),
                &mut bi,
                DIB_RGB_COLORS,
            );
        }
        let _ = SelectObject(hdc_mem, old);
        let _ = DeleteObject(hbm.into());
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(Some(hwnd_desktop), hdc_screen);
        if !ok {
            return Err(PilotError::new(
                ErrorCode::ActionFailed,
                "GDI BitBlt failed",
            ));
        }
        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2);
            px[3] = 255;
        }
        Ok(pixels)
    }
}
