//! Full WebGPU OS Runtime fusing `ai-os` + `cloudflare-os` + `nysh` (`nsh` + `aysh`) + `browser` + `ayvpn`.
//!
//! Runs identically on native Desktop GPUI (Vulkan / DirectX 12 / Metal) and in-browser
//! `wasm32-unknown-unknown` WebGPU (`aphrody_desktop` `web_os`), providing one coherent state
//! model across Durable Object Gadgets/Gatekeepers, MCP/A2A agent tasks, and the `nysh` GPU shell.

use browser_browse::{BrowseSessionPool, DomainPolicy};
use browser_views::BrowserViewModel;
use browser_vpn::{BrowserVpnRouter, RouteMode, TunnelState};
use serde::{Deserialize, Serialize};

/// Canonical outbound Gatekeepers from `aphrody-labs/cloudflare-os` (`packages/gatekeeper-*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CloudflareGatekeeperKind {
    Anthropic,
    CloudflareApi,
    Discord,
    Gemini,
    GitHub,
    GoogleCalendar,
    GoogleDocs,
    GoogleDrive,
    GoogleGmail,
    GoogleSheets,
    Linear,
    Notion,
    OpenAi,
    Slack,
}

impl CloudflareGatekeeperKind {
    #[must_use]
    pub const fn worker_package(self) -> &'static str {
        match self {
            Self::Anthropic => "gatekeeper-anthropic",
            Self::CloudflareApi => "gatekeeper-cloudflare",
            Self::Discord => "gatekeeper-discord",
            Self::Gemini => "gatekeeper-gemini",
            Self::GitHub => "gatekeeper-github",
            Self::GoogleCalendar => "gatekeeper-google-calendar",
            Self::GoogleDocs => "gatekeeper-google-docs",
            Self::GoogleDrive => "gatekeeper-google-drive",
            Self::GoogleGmail => "gatekeeper-google-gmail",
            Self::GoogleSheets => "gatekeeper-google-sheets",
            Self::Linear => "gatekeeper-linear",
            Self::Notion => "gatekeeper-notion",
            Self::OpenAi => "gatekeeper-openai",
            Self::Slack => "gatekeeper-slack",
        }
    }
}

/// Isolated capability Gadget running inside a `cloudflare-os` Durable Object Workshop or local Wasm isolate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudflareOsGadget {
    pub id: String,
    pub title: String,
    pub workshop_durable_object: String,
    pub gatekeepers: Vec<CloudflareGatekeeperKind>,
    pub webgpu_accelerated: bool,
}

/// `ai-os` ring-3 agent kernel task bridged over MCP / A2A and Cap'n Web RPC.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiOsKernelTask {
    pub task_id: String,
    pub protocol: &'static str,
    pub summary: String,
    pub target_gadget: Option<String>,
    pub completed: bool,
}

/// Single GPU-instanced glyph cell in the `nysh` (`nsh` + `aysh` `rio-vt` / `niubash`) WebGPU terminal surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NyshShellCell {
    pub ch: char,
    pub fg_rgba: u32,
    pub bg_rgba: u32,
    pub bold: bool,
}

impl Default for NyshShellCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg_rgba: 0xE6E8_F0FF,
            bg_rgba: 0x0F11_17FF,
            bold: false,
        }
    }
}

/// Fused `nysh` (`nsh` TSX/GPUI reactive shell + `aysh` `niubash`/`rubash` Bash 5.3 + `rio-vt` grid).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NyshTerminalGrid {
    pub cols: u16,
    pub rows: u16,
    pub cwd: String,
    pub active_shell: &'static str,
    lines: Vec<String>,
}

impl Default for NyshTerminalGrid {
    fn default() -> Self {
        Self::new(120, 36)
    }
}

impl NyshTerminalGrid {
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            cwd: "~/workspace".into(),
            active_shell: "nysh (nsh + aysh/niubash + rio-vt WebGPU)",
            lines: vec![
                "Aphrody WebGPU OS — ai-os + cloudflare-os + nysh unified shell".into(),
            ],
        }
    }

    pub fn push_command(&mut self, cmd: &str, output: &str) {
        self.lines.push(format!("niu$ {cmd}"));
        for line in output.lines() {
            self.lines.push(line.to_owned());
        }
        let max_keep = usize::from(self.rows).saturating_mul(8).max(64);
        if self.lines.len() > max_keep {
            let drain = self.lines.len() - max_keep;
            self.lines.drain(0..drain);
        }
    }

    #[must_use]
    pub fn transcript(&self) -> &[String] {
        &self.lines
    }

    /// Packs the visible viewport into GPU-ready `NyshShellCell` instances for WebGPU/WGSL rendering.
    #[must_use]
    pub fn pack_visible_cells(&self) -> Vec<NyshShellCell> {
        let cols = usize::from(self.cols.max(1));
        let rows = usize::from(self.rows.max(1));
        let mut grid = vec![NyshShellCell::default(); cols * rows];
        let start = self.lines.len().saturating_sub(rows);
        for (row_idx, line) in self.lines[start..].iter().enumerate() {
            let is_prompt = line.starts_with("niu$ ");
            for (col_idx, ch) in line.chars().take(cols).enumerate() {
                grid[row_idx * cols + col_idx] = NyshShellCell {
                    ch,
                    fg_rgba: if is_prompt { 0x7AA2_F7FF } else { 0xE6E8_F0FF },
                    bg_rgba: 0x0F11_17FF,
                    bold: is_prompt,
                };
            }
        }
        grid
    }
}

/// Top-level Full WebGPU OS state uniting `ai-os`, `cloudflare-os`, `nysh`, `browser`, and `ayvpn`.
#[derive(Debug)]
pub struct WebGpuOsRuntime {
    pub browser_model: BrowserViewModel,
    pub vpn_router: BrowserVpnRouter,
    pub browse_pool: BrowseSessionPool,
    pub gadgets: Vec<CloudflareOsGadget>,
    pub kernel_tasks: Vec<AiOsKernelTask>,
    pub nysh_terminal: NyshTerminalGrid,
}

impl Default for WebGpuOsRuntime {
    fn default() -> Self {
        Self::new_default()
    }
}

impl WebGpuOsRuntime {
    #[must_use]
    pub fn new_default() -> Self {
        let vpn_router = BrowserVpnRouter::new();
        vpn_router.set_state(TunnelState::Connected);
        let gadgets = vec![
            CloudflareOsGadget {
                id: "workshop-browser".into(),
                title: "Kitesurf + Browse MCP Gadget".into(),
                workshop_durable_object: "WorkshopBrowserDO".into(),
                gatekeepers: vec![
                    CloudflareGatekeeperKind::GitHub,
                    CloudflareGatekeeperKind::CloudflareApi,
                    CloudflareGatekeeperKind::Gemini,
                ],
                webgpu_accelerated: true,
            },
            CloudflareOsGadget {
                id: "workshop-nysh".into(),
                title: "Nysh WebGPU Terminal & Sandbox Gadget".into(),
                workshop_durable_object: "WorkshopSandboxDO".into(),
                gatekeepers: vec![
                    CloudflareGatekeeperKind::Anthropic,
                    CloudflareGatekeeperKind::OpenAi,
                ],
                webgpu_accelerated: true,
            },
        ];
        let kernel_tasks = vec![AiOsKernelTask {
            task_id: "aios-boot-1".into(),
            protocol: "MCP+A2A/CapnWeb",
            summary: "Synchronize Gatekeepers, WireGuard exits, and WebGPU compositor".into(),
            target_gadget: Some("workshop-browser".into()),
            completed: true,
        }];
        Self {
            browser_model: BrowserViewModel::new_default_window(),
            vpn_router,
            browse_pool: BrowseSessionPool::new(DomainPolicy::default()),
            gadgets,
            kernel_tasks,
            nysh_terminal: NyshTerminalGrid::default(),
        }
    }

    pub fn open_isolated_tab(&mut self, title: &str, url: &str, route: RouteMode) -> u64 {
        self.browser_model.open_tab(title, url, route)
    }

    pub fn exec_nysh_line(&mut self, command: &str) {
        let trimmed = command.trim();
        let reply = if trimmed.starts_with("ay vpn") {
            let exit = self.vpn_router.active_exit();
            format!(
                "state={:?} exit={} ({}/{}) proto={:?}",
                self.vpn_router.state(),
                exit.id,
                exit.country,
                exit.city,
                exit.protocol
            )
        } else if trimmed.starts_with("cf-os gadgets") {
            format!("{} Durable Object gadget(s) mounted on WebGPU bus", self.gadgets.len())
        } else {
            format!("ok: executed `{trimmed}` in {}", self.nysh_terminal.active_shell)
        };
        self.nysh_terminal.push_command(trimmed, &reply);
    }
}
