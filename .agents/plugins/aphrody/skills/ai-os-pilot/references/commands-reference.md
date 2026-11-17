# AI OS Pilot Command & MCP Surface

Every `desktop_*` tool runs identically in-process via `aphrody_craft` MCP (`/mcp` HTTP or stdio JSON-RPC) and CLI (`aphrody-os pilot <op> --json`). All responses use envelope version `2.6` (`version`, `ok`, `command`, `data` | `error`).

| Group | Operations / MCP Tools |
|---|---|
| **Observe** | `desktop_status`, `desktop_permissions`, `desktop_snapshot`, `desktop_find`, `desktop_get`, `desktop_is`, `desktop_screenshot`, `desktop_list_surfaces` |
| **Ref Actions** | `desktop_click`, `desktop_double_click`, `desktop_triple_click`, `desktop_right_click`, `desktop_type`, `desktop_set_value`, `desktop_clear`, `desktop_focus`, `desktop_select`, `desktop_toggle`, `desktop_check`, `desktop_uncheck`, `desktop_expand`, `desktop_collapse`, `desktop_scroll`, `desktop_scroll_to` |
| **Input Synthesis** | `desktop_press`, `desktop_key_down`, `desktop_key_up`, `desktop_hover`, `desktop_drag`, `desktop_mouse_move`, `desktop_mouse_click`, `desktop_mouse_down`, `desktop_mouse_up`, `desktop_mouse_wheel` |
| **Apps & Windows** | `desktop_launch`, `desktop_close_app`, `desktop_list_apps`, `desktop_list_windows`, `desktop_list_displays`, `desktop_focus_window`, `desktop_move_window`, `desktop_resize_window`, `desktop_minimize`, `desktop_maximize`, `desktop_restore`, `desktop_open_system_surface` |
| **Notifications & Clipboard** | `desktop_list_notifications`, `desktop_dismiss_notification`, `desktop_dismiss_all_notifications`, `desktop_notification_action`, `desktop_clipboard_get`, `desktop_clipboard_set`, `desktop_clipboard_clear` |
| **Workflow & AI Fusion** | `desktop_wait`, `desktop_batch`, `desktop_session`, `desktop_trace`, **`desktop_ai_act_verify`** |
| **COSMIC Compositor & Craft** | `os_workspace_list`, `os_workspace_switch`, `os_layout_set`, `os_window_open`, `os_theme_set`, `os_session_status`, `os_apps_discover`, `craft_catalog_list`, `craft_sniff_file`, `craft_anything_status` |

## Held Input in Daemon / Session Mode

Because `aphrody_pilot` runs inside stateful `PilotEngine` (`aphrody_craft` MCP server / GPUI session), `desktop_key_down` / `desktop_key_up` and `desktop_mouse_down` / `desktop_mouse_up` track active held keys/buttons inside `HeldInputState` rather than refusing outright.
