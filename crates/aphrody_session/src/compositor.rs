use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use taffy::TaffyTree;
use taffy::prelude::{Display, FlexDirection, LengthPercentageAuto, Style, auto, length};

pub type WorkspaceId = u32;
pub type ToplevelId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    #[must_use]
    pub fn inset(&self, margin: f32) -> Self {
        Self {
            x: self.x + margin,
            y: self.y + margin,
            w: (self.w - margin * 2.0).max(0.0),
            h: (self.h - margin * 2.0).max(0.0),
        }
    }
}

/// COSMIC + Hyprland + Niri unified workspace arrangement strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TilingMode {
    #[default]
    Dwindle,
    CosmicGrid,
    NiriScroll,
    Floating,
}

impl TilingMode {
    pub const ALL: [Self; 4] = [
        Self::Dwindle,
        Self::CosmicGrid,
        Self::NiriScroll,
        Self::Floating,
    ];

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Dwindle => "dwindle",
            Self::CosmicGrid => "cosmic_grid",
            Self::NiriScroll => "niri_scroll",
            Self::Floating => "floating",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dwindle => "Dwindle",
            Self::CosmicGrid => "COSMIC Grid",
            Self::NiriScroll => "Strip Scroll",
            Self::Floating => "Floating",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.slug().eq_ignore_ascii_case(raw.trim()))
    }

    #[must_use]
    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|&m| m == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    Background,
    Bottom,
    Top,
    Overlay,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LayerAnchor: u32 {
        const TOP = 1;
        const BOTTOM = 2;
        const LEFT = 4;
        const RIGHT = 8;
    }
}

impl Serialize for LayerAnchor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.bits())
    }
}

impl<'de> Deserialize<'de> for LayerAnchor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = u32::deserialize(deserializer)?;
        Ok(Self::from_bits_truncate(bits))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerSurface {
    pub id: u32,
    pub name: String,
    pub kind: LayerKind,
    pub anchor: LayerAnchor,
    pub exclusive_zone: f32,
}

/// Manages Wayland-compatible `zwlr_layer_shell_v1` exclusive zones using `taffy::TaffyTree`.
#[derive(Debug, Default, Clone)]
pub struct LayerShellManager {
    pub surfaces: BTreeMap<u32, LayerSurface>,
}

impl LayerShellManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_surface(&mut self, surface: LayerSurface) {
        self.surfaces.insert(surface.id, surface);
    }

    pub fn remove_surface(&mut self, id: u32) {
        self.surfaces.remove(&id);
    }

    /// Computes usable content bounds via a Taffy flex tree reserving top/bottom/left/right panels.
    #[must_use]
    pub fn usable_area(&self, screen: Rect) -> Rect {
        let mut top = 0.0_f32;
        let mut bottom = 0.0_f32;
        let mut left = 0.0_f32;
        let mut right = 0.0_f32;

        for surface in self.surfaces.values() {
            if surface.exclusive_zone <= 0.0 || matches!(surface.kind, LayerKind::Overlay) {
                continue;
            }
            if surface.anchor.contains(LayerAnchor::TOP)
                && !surface.anchor.contains(LayerAnchor::BOTTOM)
            {
                top += surface.exclusive_zone;
            } else if surface.anchor.contains(LayerAnchor::BOTTOM)
                && !surface.anchor.contains(LayerAnchor::TOP)
            {
                bottom += surface.exclusive_zone;
            } else if surface.anchor.contains(LayerAnchor::LEFT)
                && !surface.anchor.contains(LayerAnchor::RIGHT)
            {
                left += surface.exclusive_zone;
            } else if surface.anchor.contains(LayerAnchor::RIGHT)
                && !surface.anchor.contains(LayerAnchor::LEFT)
            {
                right += surface.exclusive_zone;
            }
        }

        match Self::try_taffy_content_rect(screen, top, bottom, left, right) {
            Ok(rect) => rect,
            Err(err) => {
                log::warn!("taffy layer-shell layout fallback: {err}");
                Rect::new(
                    screen.x + left,
                    screen.y + top,
                    (screen.w - left - right).max(120.0),
                    (screen.h - top - bottom).max(120.0),
                )
            }
        }
    }

    fn try_taffy_content_rect(
        screen: Rect,
        top: f32,
        bottom: f32,
        left: f32,
        right: f32,
    ) -> Result<Rect, taffy::TaffyError> {
        let mut tree: TaffyTree<()> = TaffyTree::new();
        let top_node = tree.new_leaf(Style {
            size: taffy::Size {
                width: auto(),
                height: length(top),
            },
            flex_shrink: 0.0,
            ..Default::default()
        })?;
        let bottom_node = tree.new_leaf(Style {
            size: taffy::Size {
                width: auto(),
                height: length(bottom),
            },
            flex_shrink: 0.0,
            ..Default::default()
        })?;
        let left_node = tree.new_leaf(Style {
            size: taffy::Size {
                width: length(left),
                height: auto(),
            },
            flex_shrink: 0.0,
            ..Default::default()
        })?;
        let right_node = tree.new_leaf(Style {
            size: taffy::Size {
                width: length(right),
                height: auto(),
            },
            flex_shrink: 0.0,
            ..Default::default()
        })?;
        let content_node = tree.new_leaf(Style {
            flex_grow: 1.0,
            size: taffy::Size {
                width: auto(),
                height: auto(),
            },
            ..Default::default()
        })?;

        let middle = tree.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                ..Default::default()
            },
            &[left_node, content_node, right_node],
        )?;
        let root = tree.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: taffy::Size {
                    width: length(screen.w.max(0.0)),
                    height: length(screen.h.max(0.0)),
                },
                margin: taffy::Rect {
                    left: LengthPercentageAuto::length(0.0),
                    right: LengthPercentageAuto::length(0.0),
                    top: LengthPercentageAuto::length(0.0),
                    bottom: LengthPercentageAuto::length(0.0),
                },
                ..Default::default()
            },
            &[top_node, middle, bottom_node],
        )?;
        tree.compute_layout(
            root,
            taffy::Size {
                width: taffy::AvailableSpace::Definite(screen.w.max(0.0)),
                height: taffy::AvailableSpace::Definite(screen.h.max(0.0)),
            },
        )?;
        let mid_layout = tree.layout(middle)?;
        let content_layout = tree.layout(content_node)?;

        Ok(Rect::new(
            screen.x + mid_layout.location.x + content_layout.location.x,
            screen.y + mid_layout.location.y + content_layout.location.y,
            content_layout.size.width.max(120.0),
            content_layout.size.height.max(120.0),
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToplevelState {
    pub id: ToplevelId,
    pub title: String,
    pub app_id: String,
    pub workspace_id: WorkspaceId,
    pub floating: bool,
    pub maximized: bool,
    pub minimized: bool,
    pub float_rect: Rect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub id: WorkspaceId,
    pub name: String,
    pub mode: TilingMode,
    pub scroll_offset_x: f32,
    pub window_ids: Vec<ToplevelId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacedWindow {
    pub id: ToplevelId,
    pub title: String,
    pub app_id: String,
    pub rect: Rect,
    pub focused: bool,
    pub maximized: bool,
    pub floating: bool,
}

pub type StackId = u32;

/// Spatial navigation direction for Pop Shell / COSMIC keyboard focus & swapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Pop Shell / `cosmic-comp` active focus ring & stacking bar configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActiveHintConfig {
    pub enabled: bool,
    pub width_px: f32,
    pub show_stack_tabs: bool,
}

impl Default for ActiveHintConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            width_px: 2.0,
            show_stack_tabs: true,
        }
    }
}

/// Tabbed container of windows stacked in a single tile slot (`pop-os/shell` + `cosmic-comp`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowStackGroup {
    pub id: StackId,
    pub workspace_id: WorkspaceId,
    pub tabs: Vec<ToplevelId>,
    pub active_tab: usize,
}

impl WindowStackGroup {
    #[must_use]
    pub fn active_toplevel(&self) -> Option<ToplevelId> {
        self.tabs
            .get(self.active_tab)
            .copied()
            .or_else(|| self.tabs.first().copied())
    }
}

/// Live workspace snapshot rendered by `cosmic-workspaces-epoch` overview grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceOverviewSnapshot {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub mode: TilingMode,
    pub active: bool,
    pub window_count: usize,
    pub placed: Vec<PlacedWindow>,
    pub stacks: Vec<WindowStackGroup>,
}

/// Computes tiled or floating window rectangles for a workspace inside `usable`.
#[must_use]
pub fn layout_windows(
    usable: Rect,
    mode: TilingMode,
    gap: f32,
    windows: &[&ToplevelState],
) -> Vec<Rect> {
    let count = windows.len();
    if count == 0 {
        return Vec::new();
    }

    match mode {
        TilingMode::Floating => windows.iter().map(|w| w.float_rect).collect(),
        TilingMode::Dwindle => {
            let mut rest = usable.inset(gap * 0.5);
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                if i + 1 == count {
                    out.push(rest.inset(gap * 0.5));
                    break;
                }
                if rest.w >= rest.h {
                    let first_w = rest.w * 0.5;
                    out.push(Rect::new(rest.x, rest.y, first_w, rest.h).inset(gap * 0.5));
                    rest = Rect::new(rest.x + first_w, rest.y, rest.w - first_w, rest.h);
                } else {
                    let first_h = rest.h * 0.5;
                    out.push(Rect::new(rest.x, rest.y, rest.w, first_h).inset(gap * 0.5));
                    rest = Rect::new(rest.x, rest.y + first_h, rest.w, rest.h - first_h);
                }
            }
            out
        }
        TilingMode::CosmicGrid => {
            let cols = (count as f32).sqrt().ceil() as usize;
            let rows = count.div_ceil(cols);
            let area = usable.inset(gap * 0.5);
            let cell_h = area.h / rows as f32;
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                let row = i / cols;
                let col = i % cols;
                let items_in_row = if row + 1 == rows {
                    count - row * cols
                } else {
                    cols
                };
                let cell_w = area.w / items_in_row as f32;
                out.push(
                    Rect::new(
                        area.x + col as f32 * cell_w,
                        area.y + row as f32 * cell_h,
                        cell_w,
                        cell_h,
                    )
                    .inset(gap * 0.5),
                );
            }
            out
        }
        TilingMode::NiriScroll => {
            let area = usable.inset(gap * 0.5);
            let col_w = (area.w * 0.62).max(280.0);
            (0..count)
                .map(|i| {
                    Rect::new(area.x + i as f32 * col_w, area.y, col_w, area.h).inset(gap * 0.5)
                })
                .collect()
        }
    }
}

/// Unified desktop workspace + toplevel window + LayerShell compositor model.
#[derive(Debug, Clone)]
pub struct CompositorState {
    pub screen: Rect,
    pub gap: f32,
    pub layers: LayerShellManager,
    pub workspaces: BTreeMap<WorkspaceId, WorkspaceState>,
    pub active_workspace: WorkspaceId,
    pub toplevels: BTreeMap<ToplevelId, ToplevelState>,
    pub focused_toplevel: Option<ToplevelId>,
    pub active_hint: ActiveHintConfig,
    pub stacks: BTreeMap<StackId, WindowStackGroup>,
    next_id: ToplevelId,
    next_stack_id: StackId,
}

impl CompositorState {
    #[must_use]
    pub fn new(screen: Rect, workspace_names: &[&str], default_mode: TilingMode) -> Self {
        let mut workspaces = BTreeMap::new();
        for (idx, name) in workspace_names.iter().enumerate() {
            let id = (idx + 1) as WorkspaceId;
            workspaces.insert(
                id,
                WorkspaceState {
                    id,
                    name: (*name).to_owned(),
                    mode: default_mode,
                    scroll_offset_x: 0.0,
                    window_ids: Vec::new(),
                },
            );
        }
        if workspaces.is_empty() {
            workspaces.insert(
                1,
                WorkspaceState {
                    id: 1,
                    name: "Studio".to_owned(),
                    mode: default_mode,
                    scroll_offset_x: 0.0,
                    window_ids: Vec::new(),
                },
            );
        }

        Self {
            screen,
            gap: 8.0,
            layers: LayerShellManager::new(),
            workspaces,
            active_workspace: 1,
            toplevels: BTreeMap::new(),
            focused_toplevel: None,
            active_hint: ActiveHintConfig::default(),
            stacks: BTreeMap::new(),
            next_id: 1,
            next_stack_id: 1,
        }
    }

    pub fn open_window(
        &mut self,
        title: impl Into<String>,
        app_id: impl Into<String>,
    ) -> ToplevelId {
        let id = self.next_id;
        self.next_id += 1;
        let offset = ((id - 1) % 6) as f32 * 28.0;
        let usable = self.layers.usable_area(self.screen);
        let float_rect = Rect::new(
            usable.x + 32.0 + offset,
            usable.y + 32.0 + offset,
            (usable.w * 0.56).max(360.0),
            (usable.h * 0.64).max(260.0),
        );

        self.toplevels.insert(
            id,
            ToplevelState {
                id,
                title: title.into(),
                app_id: app_id.into(),
                workspace_id: self.active_workspace,
                floating: false,
                maximized: false,
                minimized: false,
                float_rect,
            },
        );
        if let Some(ws) = self.workspaces.get_mut(&self.active_workspace) {
            ws.window_ids.push(id);
        }
        self.focused_toplevel = Some(id);
        id
    }

    pub fn switch_workspace(&mut self, workspace_id: WorkspaceId) -> bool {
        if !self.workspaces.contains_key(&workspace_id) {
            return false;
        }
        self.active_workspace = workspace_id;
        let last_win = self
            .workspaces
            .get(&workspace_id)
            .and_then(|ws| ws.window_ids.last().copied());
        self.focused_toplevel = last_win;
        true
    }

    pub fn set_active_mode(&mut self, mode: TilingMode) {
        if let Some(ws) = self.workspaces.get_mut(&self.active_workspace) {
            ws.mode = mode;
        }
    }

    pub fn cycle_active_mode(&mut self) -> TilingMode {
        if let Some(ws) = self.workspaces.get_mut(&self.active_workspace) {
            ws.mode = ws.mode.next();
            ws.mode
        } else {
            TilingMode::Dwindle
        }
    }

    pub fn focus_window(&mut self, id: ToplevelId) -> bool {
        if let Some(win) = self.toplevels.get(&id) {
            self.active_workspace = win.workspace_id;
            self.focused_toplevel = Some(id);
            true
        } else {
            false
        }
    }

    pub fn close_window(&mut self, id: ToplevelId) -> Option<ToplevelState> {
        let removed = self.toplevels.remove(&id)?;
        let _unstacked = self.unstack_window(id);
        if let Some(ws) = self.workspaces.get_mut(&removed.workspace_id) {
            ws.window_ids.retain(|&wid| wid != id);
        }
        if self.focused_toplevel == Some(id) {
            self.focused_toplevel = self
                .workspaces
                .get(&self.active_workspace)
                .and_then(|ws| ws.window_ids.last().copied());
        }
        Some(removed)
    }

    pub fn toggle_maximize(&mut self, id: ToplevelId) -> bool {
        if let Some(win) = self.toplevels.get_mut(&id) {
            win.maximized = !win.maximized;
            true
        } else {
            false
        }
    }

    pub fn toggle_floating(&mut self, id: ToplevelId) -> bool {
        if let Some(win) = self.toplevels.get_mut(&id) {
            win.floating = !win.floating;
            if win.floating {
                let _unstacked = self.unstack_window(id);
            }
            true
        } else {
            false
        }
    }

    pub fn set_active_hint(&mut self, enabled: bool, width_px: f32, show_stack_tabs: bool) {
        self.active_hint = ActiveHintConfig {
            enabled,
            width_px: width_px.clamp(0.0, 8.0),
            show_stack_tabs,
        };
    }

    #[must_use]
    pub fn active_stack_for(&self, id: ToplevelId) -> Option<&WindowStackGroup> {
        self.stacks.values().find(|s| s.tabs.contains(&id))
    }

    pub fn stack_windows(&mut self, primary: ToplevelId, secondary: ToplevelId) -> Option<StackId> {
        if primary == secondary {
            return None;
        }
        let ws_a = self.toplevels.get(&primary)?.workspace_id;
        let ws_b = self.toplevels.get(&secondary)?.workspace_id;
        if ws_a != ws_b {
            return None;
        }
        if let Some(win) = self.toplevels.get_mut(&primary) {
            win.floating = false;
        }
        if let Some(win) = self.toplevels.get_mut(&secondary) {
            win.floating = false;
        }
        if let Some(existing_id) = self.stacks.iter().find_map(|(sid, grp)| {
            if grp.tabs.contains(&primary) {
                Some(*sid)
            } else {
                None
            }
        }) {
            let _unstacked = self.unstack_window(secondary);
            if let Some(grp) = self.stacks.get_mut(&existing_id) {
                if !grp.tabs.contains(&secondary) {
                    grp.tabs.push(secondary);
                    grp.active_tab = grp.tabs.len().saturating_sub(1);
                }
                self.focused_toplevel = Some(secondary);
            }
            return Some(existing_id);
        }

        let _unstacked = self.unstack_window(secondary);
        let id = self.next_stack_id;
        self.next_stack_id += 1;
        self.stacks.insert(
            id,
            WindowStackGroup {
                id,
                workspace_id: ws_a,
                tabs: vec![primary, secondary],
                active_tab: 1,
            },
        );
        self.focused_toplevel = Some(secondary);
        Some(id)
    }

    pub fn stack_with_focused(&mut self) -> Option<StackId> {
        let focused = self.focused_toplevel?;
        let ws_id = self.toplevels.get(&focused)?.workspace_id;
        let partner = self
            .workspaces
            .get(&ws_id)?
            .window_ids
            .iter()
            .copied()
            .find(|&wid| wid != focused)?;
        self.stack_windows(partner, focused)
    }

    pub fn select_stack_tab(&mut self, stack_id: StackId, toplevel_id: ToplevelId) -> bool {
        let Some(grp) = self.stacks.get_mut(&stack_id) else {
            return false;
        };
        if let Some(pos) = grp.tabs.iter().position(|&t| t == toplevel_id) {
            grp.active_tab = pos;
            self.focused_toplevel = Some(toplevel_id);
            true
        } else {
            false
        }
    }

    pub fn cycle_stack_tab(&mut self, stack_id: StackId, delta: i32) -> Option<ToplevelId> {
        let grp = self.stacks.get_mut(&stack_id)?;
        if grp.tabs.is_empty() {
            return None;
        }
        let len = grp.tabs.len() as isize;
        let next = ((grp.active_tab as isize + delta as isize).rem_euclid(len)) as usize;
        grp.active_tab = next;
        let active = grp.tabs[next];
        self.focused_toplevel = Some(active);
        Some(active)
    }

    pub fn unstack_window(&mut self, id: ToplevelId) -> bool {
        let mut to_remove = None;
        let mut removed = false;
        for (&sid, grp) in &mut self.stacks {
            if let Some(pos) = grp.tabs.iter().position(|&t| t == id) {
                grp.tabs.remove(pos);
                removed = true;
                if grp.tabs.len() < 2 {
                    to_remove = Some(sid);
                } else if grp.active_tab >= grp.tabs.len() {
                    grp.active_tab = grp.tabs.len() - 1;
                }
                break;
            }
        }
        if let Some(sid) = to_remove {
            self.stacks.remove(&sid);
        }
        removed
    }

    pub fn move_window_to_workspace(
        &mut self,
        id: ToplevelId,
        target_workspace: WorkspaceId,
    ) -> bool {
        if !self.workspaces.contains_key(&target_workspace) {
            return false;
        }
        let Some(old_ws) = self.toplevels.get(&id).map(|w| w.workspace_id) else {
            return false;
        };
        if old_ws == target_workspace {
            return true;
        }
        let _unstacked = self.unstack_window(id);
        if let Some(ws) = self.workspaces.get_mut(&old_ws) {
            ws.window_ids.retain(|&wid| wid != id);
        }
        if let Some(win) = self.toplevels.get_mut(&id) {
            win.workspace_id = target_workspace;
        }
        if let Some(ws) = self.workspaces.get_mut(&target_workspace) {
            ws.window_ids.push(id);
        }
        self.active_workspace = target_workspace;
        self.focused_toplevel = Some(id);
        true
    }

    pub fn focus_direction(&mut self, dir: FocusDirection) -> Option<ToplevelId> {
        let placed = self.arrange_active_workspace();
        let focused = self.focused_toplevel?;
        let cur = placed.iter().find(|p| p.id == focused)?;
        let cx = cur.rect.x + cur.rect.w * 0.5;
        let cy = cur.rect.y + cur.rect.h * 0.5;
        let candidate = placed
            .iter()
            .filter(|p| p.id != focused)
            .filter(|p| {
                let px = p.rect.x + p.rect.w * 0.5;
                let py = p.rect.y + p.rect.h * 0.5;
                match dir {
                    FocusDirection::Left => px < cx - 1.0,
                    FocusDirection::Right => px > cx + 1.0,
                    FocusDirection::Up => py < cy - 1.0,
                    FocusDirection::Down => py > cy + 1.0,
                }
            })
            .min_by(|a, b| {
                let ax = a.rect.x + a.rect.w * 0.5 - cx;
                let ay = a.rect.y + a.rect.h * 0.5 - cy;
                let bx = b.rect.x + b.rect.w * 0.5 - cx;
                let by = b.rect.y + b.rect.h * 0.5 - cy;
                (ax * ax + ay * ay)
                    .partial_cmp(&(bx * bx + by * by))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })?;
        self.focused_toplevel = Some(candidate.id);
        Some(candidate.id)
    }

    #[must_use]
    pub fn arrange_workspace(&self, workspace_id: WorkspaceId) -> Vec<PlacedWindow> {
        let usable = self.layers.usable_area(self.screen);
        let Some(ws) = self.workspaces.get(&workspace_id) else {
            return Vec::new();
        };
        let mut hidden_stack_tabs = Vec::new();
        for stack in self.stacks.values().filter(|s| s.workspace_id == workspace_id) {
            let active = stack.active_toplevel();
            for &tab_id in &stack.tabs {
                if Some(tab_id) != active {
                    hidden_stack_tabs.push(tab_id);
                }
            }
        }

        let visible: Vec<&ToplevelState> = ws
            .window_ids
            .iter()
            .filter(|id| !hidden_stack_tabs.contains(id))
            .filter_map(|id| self.toplevels.get(id))
            .filter(|w| !w.minimized)
            .collect();

        if let Some(max_win) = visible.iter().find(|w| w.maximized) {
            return vec![PlacedWindow {
                id: max_win.id,
                title: max_win.title.clone(),
                app_id: max_win.app_id.clone(),
                rect: usable.inset(self.gap * 0.5),
                focused: self.focused_toplevel == Some(max_win.id),
                maximized: true,
                floating: max_win.floating,
            }];
        }

        let tiled: Vec<&ToplevelState> = visible.iter().copied().filter(|w| !w.floating).collect();
        let tiled_rects = layout_windows(usable, ws.mode, self.gap, &tiled);
        let mut tiled_map = BTreeMap::new();
        for (win, rect) in tiled.iter().zip(tiled_rects) {
            tiled_map.insert(win.id, rect);
        }

        visible
            .into_iter()
            .map(|win| PlacedWindow {
                id: win.id,
                title: win.title.clone(),
                app_id: win.app_id.clone(),
                rect: if win.floating {
                    win.float_rect
                } else {
                    tiled_map.get(&win.id).copied().unwrap_or(win.float_rect)
                },
                focused: self.focused_toplevel == Some(win.id),
                maximized: win.maximized,
                floating: win.floating,
            })
            .collect()
    }

    #[must_use]
    pub fn arrange_active_workspace(&self) -> Vec<PlacedWindow> {
        self.arrange_workspace(self.active_workspace)
    }

    #[must_use]
    pub fn workspace_overviews(&self) -> Vec<WorkspaceOverviewSnapshot> {
        self.workspaces
            .values()
            .map(|ws| WorkspaceOverviewSnapshot {
                workspace_id: ws.id,
                name: ws.name.clone(),
                mode: ws.mode,
                active: ws.id == self.active_workspace,
                window_count: ws.window_ids.len(),
                placed: self.arrange_workspace(ws.id),
                stacks: self
                    .stacks
                    .values()
                    .filter(|s| s.workspace_id == ws.id)
                    .cloned()
                    .collect(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taffy_layer_shell_reserves_panel_and_dock_zones() {
        let mut comp = CompositorState::new(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            &["Studio", "Code"],
            TilingMode::Dwindle,
        );
        comp.layers.set_surface(LayerSurface {
            id: 1,
            name: "top-panel".to_owned(),
            kind: LayerKind::Top,
            anchor: LayerAnchor::TOP | LayerAnchor::LEFT | LayerAnchor::RIGHT,
            exclusive_zone: 40.0,
        });
        comp.layers.set_surface(LayerSurface {
            id: 2,
            name: "bottom-dock".to_owned(),
            kind: LayerKind::Top,
            anchor: LayerAnchor::BOTTOM,
            exclusive_zone: 64.0,
        });

        let usable = comp.layers.usable_area(comp.screen);
        assert_eq!(usable.y, 40.0);
        assert_eq!(usable.h, 796.0);
        assert_eq!(usable.w, 1440.0);

        let a = comp.open_window("ArtCraft Studio", "aphrody.studio");
        let b = comp.open_window("Craft Inspector", "aphrody.inspector");
        assert_ne!(a, b);
        let placed = comp.arrange_active_workspace();
        assert_eq!(placed.len(), 2);
        assert!(placed[0].rect.w > 600.0 && placed[1].rect.w > 600.0);
    }

    #[test]
    fn pop_shell_stacking_directional_focus_and_workspace_overviews() {
        let mut comp = CompositorState::new(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            &["Studio", "Code", "Shell"],
            TilingMode::Dwindle,
        );
        let a = comp.open_window("PhotoCraft", "photocraft");
        let b = comp.open_window("COSMIC Files", "cosmic-files");
        let c = comp.open_window("COSMIC Terminal", "cosmic-term");

        assert_eq!(comp.focused_toplevel, Some(c));
        assert!(comp.focus_direction(FocusDirection::Left).is_some());

        let stack_id = comp.stack_windows(a, b).expect("stack two windows");
        assert_eq!(comp.arrange_active_workspace().len(), 2);
        assert_eq!(comp.active_stack_for(b).map(|s| s.id), Some(stack_id));

        let cycled = comp.cycle_stack_tab(stack_id, 1).expect("cycle tab");
        assert_eq!(cycled, a);
        assert!(comp.move_window_to_workspace(c, 2));
        assert_eq!(comp.active_workspace, 2);

        let overviews = comp.workspace_overviews();
        assert_eq!(overviews.len(), 3);
        assert_eq!(overviews[0].window_count, 2);
        assert_eq!(overviews[1].window_count, 1);
        assert!(overviews[1].active);

        assert!(comp.unstack_window(a));
        assert!(comp.stacks.is_empty());
    }
}
