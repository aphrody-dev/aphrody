//! Desktop environment, Rust userland shell, kernel, bootloader, and server profiles
//! imported directly from `vendor/new-cli-installer` (`net-profiles.toml`, `server-profiles.toml`)
//! and extended with Aphrody Rust Userland shells (`Aphrody GPUI`, `Iced COSMIC`, `egui Craft`, `WebGPU`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Synchronized copy of `vendor/new-cli-installer/net-profiles.toml`.
pub const BUNDLED_NET_PROFILES_TOML: &str =
    include_str!("../../../assets/os/installer_net_profiles.toml");

/// Synchronized copy of `vendor/new-cli-installer/server-profiles.toml`.
pub const BUNDLED_SERVER_PROFILES_TOML: &str =
    include_str!("../../../assets/os/installer_server_profiles.toml");

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("failed to parse installer profile TOML: {0}")]
    Toml(#[from] toml::de::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentToolkit {
    /// Aphrody GPUI OS (Pop!_OS + COSMIC Epoch compositor written natively in GPUI).
    AphrodyGpui,
    /// COSMIC Epoch (`libcosmic` + `iced` + `cosmic-comp` Wayland session).
    IcedCosmic,
    /// Storytold Craft Studio + `egui 0.37` + `bevy_gpui` Unreal 5.8 workstation shell.
    EguiCraft,
    /// Browser / Wasm32 WebGPU + `nsh-platform-web` shell.
    WebGpuBrowser,
    /// GTK4 / libadwaita GNOME session.
    GtkGnome,
    /// Qt6 KDE Plasma session.
    QtKde,
    /// Independent Wayland compositor (`Hyprland`, `Sway`, `Niri`, `Wayfire`).
    WaylandTiling,
    /// Lightweight X11 / GTK / Qt desktop (`Xfce`, `LXQt`, `Cinnamon`, `MATE`, `Budgie`, `i3wm`, `LXDE`, `Qtile`, `UKUI`).
    ClassicDesktop,
}

impl EnvironmentToolkit {
    #[must_use]
    pub const fn badge(self) -> &'static str {
        match self {
            Self::AphrodyGpui => "Rust GPUI",
            Self::IcedCosmic => "Iced / COSMIC",
            Self::EguiCraft => "egui / Bevy",
            Self::WebGpuBrowser => "WebGPU / Wasm",
            Self::GtkGnome => "GTK4 / Adwaita",
            Self::QtKde => "Qt6 / Kirigami",
            Self::WaylandTiling => "Wayland",
            Self::ClassicDesktop => "X11 / Classic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopEnvironmentSpec {
    pub id: String,
    pub display_name: String,
    pub toolkit: EnvironmentToolkit,
    pub display_manager: String,
    pub session_exec: String,
    pub aphrody_userland: bool,
    pub summary: String,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KernelSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub scheduler: &'static str,
    pub packages: &'static [&'static str],
}

pub const SUPPORTED_KERNELS: &[KernelSpec] = &[
    KernelSpec {
        id: "linux-cachyos",
        label: "CachyOS BORE + LTO",
        scheduler: "BORE + ThinLTO",
        packages: &["linux-cachyos", "linux-cachyos-headers"],
    },
    KernelSpec {
        id: "linux-aphrody",
        label: "Aphrody GPUI Kernel (7.x io_uring + sched_ext)",
        scheduler: "sched_ext + lavd",
        packages: &["linux-cachyos-bore", "linux-cachyos-bore-headers", "scx-scheds"],
    },
    KernelSpec {
        id: "linux-cachyos-lts",
        label: "CachyOS LTS",
        scheduler: "BORE LTS",
        packages: &["linux-cachyos-lts", "linux-cachyos-lts-headers"],
    },
    KernelSpec {
        id: "linux-cachyos-server",
        label: "CachyOS Server (1000Hz / Headless)",
        scheduler: "EEVDF Server",
        packages: &["linux-cachyos-server", "linux-cachyos-server-headers"],
    },
    KernelSpec {
        id: "linux-zen",
        label: "Arch Zen",
        scheduler: "Interactive Zen",
        packages: &["linux-zen", "linux-zen-headers"],
    },
    KernelSpec {
        id: "linux-lts",
        label: "Arch Linux LTS",
        scheduler: "Upstream LTS",
        packages: &["linux-lts", "linux-lts-headers"],
    },
    KernelSpec {
        id: "linux",
        label: "Arch Stock",
        scheduler: "Upstream EEVDF",
        packages: &["linux", "linux-headers"],
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BootloaderKind {
    #[default]
    SystemdBoot,
    Grub,
    Refind,
    Limine,
}

impl BootloaderKind {
    pub const ALL: [Self; 4] = [Self::SystemdBoot, Self::Grub, Self::Refind, Self::Limine];

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::SystemdBoot => "systemd-boot",
            Self::Grub => "grub",
            Self::Refind => "refind",
            Self::Limine => "limine",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SystemdBoot => "systemd-boot (UEFI)",
            Self::Grub => "GRUB2 (UEFI/BIOS + Btrfs Snapshots)",
            Self::Refind => "rEFInd (Multi-Boot)",
            Self::Limine => "Limine (UEFI/BIOS + Snapshot Hook)",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|b| b.slug().eq_ignore_ascii_case(raw.trim()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemKind {
    #[default]
    Btrfs,
    Ext4,
    Xfs,
    F2fs,
    Zfs,
}

impl FilesystemKind {
    pub const ALL: [Self; 5] = [Self::Btrfs, Self::Ext4, Self::Xfs, Self::F2fs, Self::Zfs];

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Btrfs => "btrfs",
            Self::Ext4 => "ext4",
            Self::Xfs => "xfs",
            Self::F2fs => "f2fs",
            Self::Zfs => "zfs",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Btrfs => "Btrfs (Snapper @ / @home subvolumes)",
            Self::Ext4 => "ext4 (Classic Journaling)",
            Self::Xfs => "XFS (Reflink Build Hosts)",
            Self::F2fs => "F2FS (Flash/NVMe)",
            Self::Zfs => "OpenZFS (Snapshots & Compression)",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|fs| fs.slug().eq_ignore_ascii_case(raw.trim()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetinstallGroupSpec {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub selected: bool,
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceUnitSpec {
    pub name: String,
    pub action: String,
    #[serde(default)]
    pub urgent: bool,
    #[serde(default)]
    pub user: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerProfileSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub packages: Vec<String>,
    pub units: Vec<ServiceUnitSpec>,
    pub firewall_tcp_ports: Vec<u16>,
    pub firewall_udp_ports: Vec<u16>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawNetProfiles {
    #[serde(rename = "base-packages")]
    base_packages: RawBasePackages,
    #[serde(default)]
    services: RawServices,
    #[serde(default)]
    desktop: BTreeMap<String, RawPackageList>,
    #[serde(default)]
    netinstall: RawNetinstall,
}

#[derive(Debug, Default, Deserialize)]
struct RawBasePackages {
    #[serde(default)]
    packages: Vec<String>,
    #[serde(default)]
    desktop: Option<RawPackageList>,
}

#[derive(Debug, Default, Deserialize)]
struct RawPackageList {
    #[serde(default)]
    packages: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawServices {
    #[serde(default)]
    units: Vec<ServiceUnitSpec>,
    #[serde(default)]
    desktop: Option<RawServicesDesktop>,
}

#[derive(Debug, Default, Deserialize)]
struct RawServicesDesktop {
    #[serde(default)]
    units: Vec<ServiceUnitSpec>,
}

#[derive(Debug, Default, Deserialize)]
struct RawNetinstall {
    #[serde(default)]
    group: Vec<NetinstallGroupSpec>,
}

#[derive(Debug, Deserialize)]
struct RawServerRoot {
    server: RawServerSection,
}

#[derive(Debug, Deserialize)]
struct RawServerSection {
    default_profile: String,
    default_kernel: String,
    #[serde(default)]
    profile_order: Vec<String>,
    base: RawServerBase,
    profiles: BTreeMap<String, RawServerProfileEntry>,
}

#[derive(Debug, Deserialize)]
struct RawServerBase {
    #[serde(default)]
    packages: Vec<String>,
    #[serde(default)]
    units: Vec<ServiceUnitSpec>,
    #[serde(default)]
    firewall_tcp_ports: Vec<u16>,
}

#[derive(Debug, Deserialize)]
struct RawServerProfileEntry {
    name: String,
    description: String,
    #[serde(default)]
    packages: Vec<String>,
    #[serde(default)]
    units: Vec<ServiceUnitSpec>,
    #[serde(default)]
    firewall_tcp_ports: Vec<u16>,
    #[serde(default)]
    firewall_udp_ports: Vec<u16>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct InstallerProfileCatalog {
    pub base_cli_packages: Vec<String>,
    pub base_desktop_packages: Vec<String>,
    pub base_services: Vec<ServiceUnitSpec>,
    pub desktop_services: Vec<ServiceUnitSpec>,
    pub environments: Vec<DesktopEnvironmentSpec>,
    pub netinstall_groups: Vec<NetinstallGroupSpec>,
    pub server_default_profile: String,
    pub server_default_kernel: String,
    pub server_base_packages: Vec<String>,
    pub server_base_units: Vec<ServiceUnitSpec>,
    pub server_base_tcp_ports: Vec<u16>,
    pub server_profiles: Vec<ServerProfileSpec>,
}

impl InstallerProfileCatalog {
    pub fn bundled() -> Result<Self, ProfileError> {
        Self::from_toml_strs(BUNDLED_NET_PROFILES_TOML, BUNDLED_SERVER_PROFILES_TOML)
    }

    pub fn from_toml_strs(net_toml: &str, server_toml: &str) -> Result<Self, ProfileError> {
        let net: RawNetProfiles = toml::from_str(net_toml)?;
        let srv: RawServerRoot = toml::from_str(server_toml)?;

        let mut environments = vec![
            DesktopEnvironmentSpec {
                id: "aphrody-gpui".to_owned(),
                display_name: "Aphrody GPUI OS (Pop!_OS + COSMIC)".to_owned(),
                toolkit: EnvironmentToolkit::AphrodyGpui,
                display_manager: "cosmic-greeter".to_owned(),
                session_exec: "aphrody-os desktop".to_owned(),
                aphrody_userland: true,
                summary: "Pure-Rust GPUI compositor, Pop Shell tiling/stacking, COSMIC panels & ArtCraft.".to_owned(),
                packages: vec![
                    "mesa".to_owned(),
                    "vulkan-icd-loader".to_owned(),
                    "wayland".to_owned(),
                    "xorg-xwayland".to_owned(),
                    "pipewire".to_owned(),
                    "wireplumber".to_owned(),
                ],
            },
            DesktopEnvironmentSpec {
                id: "iced-cosmic".to_owned(),
                display_name: "COSMIC Epoch (Iced / libcosmic)".to_owned(),
                toolkit: EnvironmentToolkit::IcedCosmic,
                display_manager: "cosmic-greeter".to_owned(),
                session_exec: "cosmic-session".to_owned(),
                aphrody_userland: true,
                summary: "System76 COSMIC Epoch Wayland compositor (cosmic-comp) + Iced applets suite.".to_owned(),
                packages: net
                    .desktop
                    .get("cosmic")
                    .map(|p| p.packages.clone())
                    .unwrap_or_default(),
            },
            DesktopEnvironmentSpec {
                id: "egui-craft".to_owned(),
                display_name: "Craft Studio Workstation (egui + Bevy 5.8)".to_owned(),
                toolkit: EnvironmentToolkit::EguiCraft,
                display_manager: "cosmic-greeter".to_owned(),
                session_exec: "aphrody-os desktop --app figcraft".to_owned(),
                aphrody_userland: true,
                summary: "egui_gpui 0.37 immediate UI + Bevy Unreal 5.8 Solari GI + 18 Storytold Craft apps.".to_owned(),
                packages: vec![
                    "vulkan-icd-loader".to_owned(),
                    "mesa".to_owned(),
                    "wayland".to_owned(),
                    "pipewire".to_owned(),
                ],
            },
            DesktopEnvironmentSpec {
                id: "webgpu-os".to_owned(),
                display_name: "WebGPU / Browser Shell (gpui_web + NSH)".to_owned(),
                toolkit: EnvironmentToolkit::WebGpuBrowser,
                display_manager: "greetd".to_owned(),
                session_exec: "bun tooling/bun/cli.ts web serve".to_owned(),
                aphrody_userland: true,
                summary: "Wasm32 WebGPU / WASI Preview 2 desktop shell served via Bun OS & NSH Web.".to_owned(),
                packages: vec!["chromium".to_owned(), "wayland".to_owned(), "vulkan-icd-loader".to_owned()],
            },
        ];

        let mut ordered_arch_ids: Vec<&str> = vec![
            "gnome", "kde", "hyprland", "sway", "niri", "xfce", "cinnamon", "mate", "budgie",
            "i3wm", "lxqt", "lxde", "wayfire", "qtile", "ukui",
        ];
        for key in net.desktop.keys() {
            if key != "xorg" && key != "cosmic" && !ordered_arch_ids.contains(&key.as_str()) {
                ordered_arch_ids.push(key.as_str());
            }
        }

        for id in ordered_arch_ids {
            let Some(list) = net.desktop.get(id) else {
                continue;
            };
            environments.push(build_arch_env_spec(id, list.packages.clone()));
        }

        let mut server_profiles = Vec::new();
        for key in &srv.server.profile_order {
            if let Some(entry) = srv.server.profiles.get(key) {
                server_profiles.push(ServerProfileSpec {
                    id: key.clone(),
                    name: entry.name.clone(),
                    description: entry.description.clone(),
                    packages: entry.packages.clone(),
                    units: entry.units.clone(),
                    firewall_tcp_ports: entry.firewall_tcp_ports.clone(),
                    firewall_udp_ports: entry.firewall_udp_ports.clone(),
                    warnings: entry.warnings.clone(),
                });
            }
        }
        for (key, entry) in srv.server.profiles {
            if !srv.server.profile_order.contains(&key) {
                server_profiles.push(ServerProfileSpec {
                    id: key,
                    name: entry.name,
                    description: entry.description,
                    packages: entry.packages,
                    units: entry.units,
                    firewall_tcp_ports: entry.firewall_tcp_ports,
                    firewall_udp_ports: entry.firewall_udp_ports,
                    warnings: entry.warnings,
                });
            }
        }

        Ok(Self {
            base_cli_packages: net.base_packages.packages,
            base_desktop_packages: net
                .base_packages
                .desktop
                .map(|d| d.packages)
                .unwrap_or_default(),
            base_services: net.services.units,
            desktop_services: net
                .services
                .desktop
                .map(|d| d.units)
                .unwrap_or_default(),
            environments,
            netinstall_groups: net.netinstall.group,
            server_default_profile: srv.server.default_profile,
            server_default_kernel: srv.server.default_kernel,
            server_base_packages: srv.server.base.packages,
            server_base_units: srv.server.base.units,
            server_base_tcp_ports: srv.server.base.firewall_tcp_ports,
            server_profiles,
        })
    }

    #[must_use]
    pub fn environment(&self, id: &str) -> Option<&DesktopEnvironmentSpec> {
        let needle = id.trim();
        self.environments
            .iter()
            .find(|env| env.id.eq_ignore_ascii_case(needle))
    }
}

fn build_arch_env_spec(id: &str, packages: Vec<String>) -> DesktopEnvironmentSpec {
    let (display_name, toolkit, display_manager, session_exec, summary) = match id {
        "gnome" => (
            "GNOME (GTK4 / libadwaita)",
            EnvironmentToolkit::GtkGnome,
            "gdm",
            "gnome-session",
            "Arch/CachyOS GNOME desktop with GDM, libadwaita apps, and PipeWire.",
        ),
        "kde" => (
            "KDE Plasma 6 (Qt6)",
            EnvironmentToolkit::QtKde,
            "plasma-login-manager",
            "startplasma-wayland",
            "Arch/CachyOS KDE Plasma 6 desktop with KWin Wayland and CachyOS Qt settings.",
        ),
        "hyprland" => (
            "Hyprland (Dynamic Wayland)",
            EnvironmentToolkit::WaylandTiling,
            "sddm",
            "Hyprland",
            "Dynamic tiling Wayland compositor with xdg-desktop-portal-hyprland.",
        ),
        "sway" => (
            "Sway (i3-compatible Wayland)",
            EnvironmentToolkit::WaylandTiling,
            "ly",
            "sway",
            "Keyboard-driven Wayland tiling window manager with Waybar & Foot.",
        ),
        "niri" => (
            "Niri (Scrollable Tiling Wayland)",
            EnvironmentToolkit::WaylandTiling,
            "sddm",
            "niri-session",
            "Scrollable-tiling Wayland compositor paired with Noctalia shell.",
        ),
        "xfce" => (
            "Xfce 4 (GTK Lightweight)",
            EnvironmentToolkit::ClassicDesktop,
            "lightdm",
            "startxfce4",
            "Modular lightweight GTK desktop environment.",
        ),
        "cinnamon" => (
            "Cinnamon (GTK)",
            EnvironmentToolkit::ClassicDesktop,
            "lightdm",
            "cinnamon-session",
            "Traditional Linux Mint desktop environment.",
        ),
        "mate" => (
            "MATE",
            EnvironmentToolkit::ClassicDesktop,
            "lightdm",
            "mate-session",
            "Classic GNOME 2 continuation desktop.",
        ),
        "budgie" => (
            "Budgie Desktop",
            EnvironmentToolkit::ClassicDesktop,
            "lightdm",
            "budgie-desktop",
            "Streamlined GTK desktop environment.",
        ),
        "i3wm" => (
            "i3wm (X11 Tiling)",
            EnvironmentToolkit::ClassicDesktop,
            "ly",
            "i3",
            "Minimal X11 tiling window manager with Polybar & Rofi.",
        ),
        "lxqt" => (
            "LXQt (Lightweight Qt)",
            EnvironmentToolkit::ClassicDesktop,
            "sddm",
            "startlxqt",
            "Fast low-memory Qt desktop environment.",
        ),
        "lxde" => (
            "LXDE (Minimal X11)",
            EnvironmentToolkit::ClassicDesktop,
            "lightdm",
            "startlxde",
            "Ultra-lightweight X11 Openbox desktop.",
        ),
        "wayfire" => (
            "Wayfire (3D Wayland)",
            EnvironmentToolkit::WaylandTiling,
            "ly",
            "wayfire",
            "Modular wlroots 3D Wayland compositor.",
        ),
        "qtile" => (
            "Qtile (Python Tiling)",
            EnvironmentToolkit::ClassicDesktop,
            "ly",
            "qtile start",
            "Hackable Python-scripted window manager.",
        ),
        "ukui" => (
            "UKUI (Qt)",
            EnvironmentToolkit::ClassicDesktop,
            "sddm",
            "ukui-session",
            "Lightweight Qt desktop with unified panel.",
        ),
        other => (
            other,
            EnvironmentToolkit::ClassicDesktop,
            "sddm",
            other,
            "Upstream Arch/CachyOS net-profile session.",
        ),
    };

    DesktopEnvironmentSpec {
        id: id.to_owned(),
        display_name: display_name.to_owned(),
        toolkit,
        display_manager: display_manager.to_owned(),
        session_exec: session_exec.to_owned(),
        aphrody_userland: false,
        summary: summary.to_owned(),
        packages,
    }
}
