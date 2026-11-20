//! Awesome GPUI Store catalog, cross-platform compatibility matrix,
//! unified `upt` OS package-manager bridge, NSH + GPUI store UI, and 22-target packager.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod extensions_catalog;
pub mod nsh_store;
pub mod packagers;

/// Embedded live-synced Awesome GPUI catalog (`assets/os/awesome_gpui_store.toml`).
#[cfg(not(feature = "slim"))]
pub const BUNDLED_STORE_TOML: &str = include_str!("../../../assets/os/awesome_gpui_store.toml");

/// Slim Wasm starter catalog hydrated from R2 at runtime (`web-slim` saves ~193 KiB `.rodata`).
#[cfg(feature = "slim")]
pub const BUNDLED_STORE_TOML: &str = r#"schema_version = 1
source_repo = "aphrody-labs/aphrody"
source_url = "https://aphrody.com"
synced_at = "2026-11-20"
upstream_stars = 0
total_entries = 0
entries = []
"#;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("failed to parse Awesome GPUI Store TOML catalog: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("store IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("package vendor resolution error: {0}")]
    Vendor(String),
}

#[cfg(not(target_family = "wasm"))]
impl From<upt::UptError> for StoreError {
    fn from(err: upt::UptError) -> Self {
        Self::Vendor(err.to_string())
    }
}

/// Supported target store, distro package manager, container, and runtime ecosystems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StoreTarget {
    #[serde(rename = "winget")]
    WinGet,
    #[serde(rename = "msstore")]
    MicrosoftStore,
    #[serde(rename = "flatpak")]
    Flatpak,
    #[serde(rename = "playstore")]
    PlayStore,
    #[serde(rename = "appstore")]
    AppStore,
    #[serde(rename = "snap")]
    UbuntuSnap,
    #[serde(rename = "arch")]
    ArchPacman,
    #[serde(rename = "nix")]
    NixFlake,
    #[serde(rename = "cargo")]
    CargoCrate,
    #[serde(rename = "npm")]
    NpmPackage,
    #[serde(rename = "chrome")]
    ChromeWebStore,
    #[serde(rename = "apk")]
    AlpineApk,
    #[serde(rename = "rpm")]
    FedoraRpm,
    #[serde(rename = "deb")]
    DebianDeb,
    #[serde(rename = "xbps")]
    VoidXbps,
    #[serde(rename = "gentoo")]
    GentooEmerge,
    #[serde(rename = "freebsd")]
    FreeBsdPkg,
    #[serde(rename = "brew")]
    Homebrew,
    #[serde(rename = "scoop")]
    Scoop,
    #[serde(rename = "choco")]
    Chocolatey,
    #[serde(rename = "appimage")]
    AppImage,
    #[serde(rename = "oci")]
    DockerOci,
}

impl StoreTarget {
    pub const ALL: [Self; 22] = [
        Self::WinGet,
        Self::MicrosoftStore,
        Self::Flatpak,
        Self::PlayStore,
        Self::AppStore,
        Self::UbuntuSnap,
        Self::ArchPacman,
        Self::NixFlake,
        Self::CargoCrate,
        Self::NpmPackage,
        Self::ChromeWebStore,
        Self::AlpineApk,
        Self::FedoraRpm,
        Self::DebianDeb,
        Self::VoidXbps,
        Self::GentooEmerge,
        Self::FreeBsdPkg,
        Self::Homebrew,
        Self::Scoop,
        Self::Chocolatey,
        Self::AppImage,
        Self::DockerOci,
    ];

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::WinGet => "winget",
            Self::MicrosoftStore => "msstore",
            Self::Flatpak => "flatpak",
            Self::PlayStore => "playstore",
            Self::AppStore => "appstore",
            Self::UbuntuSnap => "snap",
            Self::ArchPacman => "arch",
            Self::NixFlake => "nix",
            Self::CargoCrate => "cargo",
            Self::NpmPackage => "npm",
            Self::ChromeWebStore => "chrome",
            Self::AlpineApk => "apk",
            Self::FedoraRpm => "rpm",
            Self::DebianDeb => "deb",
            Self::VoidXbps => "xbps",
            Self::GentooEmerge => "gentoo",
            Self::FreeBsdPkg => "freebsd",
            Self::Homebrew => "brew",
            Self::Scoop => "scoop",
            Self::Chocolatey => "choco",
            Self::AppImage => "appimage",
            Self::DockerOci => "oci",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::WinGet => "WinGet",
            Self::MicrosoftStore => "Microsoft Store (MSIX)",
            Self::Flatpak => "Flatpak / Flathub",
            Self::PlayStore => "Google Play Store",
            Self::AppStore => "Apple App Store",
            Self::UbuntuSnap => "Ubuntu Snap",
            Self::ArchPacman => "Arch Linux / AUR",
            Self::NixFlake => "Nix Flake",
            Self::CargoCrate => "Cargo (crates.io)",
            Self::NpmPackage => "npm (NSH + Wasm)",
            Self::ChromeWebStore => "Chrome Web Store (MV3)",
            Self::AlpineApk => "Alpine Linux (apk)",
            Self::FedoraRpm => "Fedora / RHEL / openSUSE (rpm)",
            Self::DebianDeb => "Debian / Ubuntu (.deb)",
            Self::VoidXbps => "Void Linux (xbps)",
            Self::GentooEmerge => "Gentoo (emerge)",
            Self::FreeBsdPkg => "FreeBSD (pkg)",
            Self::Homebrew => "Homebrew (brew)",
            Self::Scoop => "Scoop",
            Self::Chocolatey => "Chocolatey",
            Self::AppImage => "AppImage",
            Self::DockerOci => "Docker / OCI",
        }
    }

    /// Runtime surface category served by this target.
    #[must_use]
    pub const fn surface(self) -> RuntimeSurface {
        match self {
            Self::PlayStore => RuntimeSurface::Mobile,
            Self::ChromeWebStore | Self::NpmPackage => RuntimeSurface::Web,
            Self::DockerOci => RuntimeSurface::Container,
            _ => RuntimeSurface::Desktop,
        }
    }

    /// Corresponding `upt` vendor tool identifier when directly mapped to a live OS package manager.
    #[must_use]
    pub const fn upt_vendor_name(self) -> Option<&'static str> {
        match self {
            Self::WinGet => Some("winget"),
            Self::Flatpak => Some("flatpak"),
            Self::UbuntuSnap => Some("snap"),
            Self::ArchPacman => Some("pacman"),
            Self::NixFlake => Some("nix-env"),
            Self::AlpineApk => Some("apk"),
            Self::FedoraRpm => Some("dnf"),
            Self::DebianDeb => Some("apt"),
            Self::VoidXbps => Some("xbps"),
            Self::GentooEmerge => Some("emerge"),
            Self::FreeBsdPkg => Some("pkg"),
            Self::Homebrew => Some("brew"),
            Self::Scoop => Some("scoop"),
            Self::Chocolatey => Some("choco"),
            _ => None,
        }
    }

    #[must_use]
    pub fn from_slug(raw: &str) -> Option<Self> {
        let lower = raw.trim().to_ascii_lowercase();
        Self::ALL.into_iter().find(|t| t.slug() == lower)
    }
}

impl fmt::Display for StoreTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreCategory {
    App,
    Library,
    Showcase,
    Tool,
    Resource,
}

impl StoreCategory {
    pub const ALL: [Self; 5] = [
        Self::App,
        Self::Library,
        Self::Showcase,
        Self::Tool,
        Self::Resource,
    ];

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Library => "library",
            Self::Showcase => "showcase",
            Self::Tool => "tool",
            Self::Resource => "resource",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::App => "Apps",
            Self::Library => "Libraries & UI Kits",
            Self::Showcase => "Showcases",
            Self::Tool => "Tooling",
            Self::Resource => "Resources",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryStatus {
    #[default]
    Active,
    Quiet,
    Dormant,
    Archived,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformTarget {
    Windows,
    Macos,
    Linux,
    Freebsd,
    Web,
    Ios,
    Android,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSurface {
    Desktop,
    Web,
    Mobile,
    Container,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityMatrix {
    pub desktop: bool,
    pub web_wasm: bool,
    pub mobile_ios: bool,
    pub mobile_android: bool,
    pub freebsd: bool,
    pub browser_extension: bool,
    pub target_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreEntry {
    pub id: String,
    pub name: String,
    pub tagline: String,
    pub description: String,
    pub category: StoreCategory,
    #[serde(default)]
    pub subcategory: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub icon_url: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub publisher_id: String,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub crate_name: String,
    #[serde(default)]
    pub npm_package: String,
    #[serde(default)]
    pub bin_name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_license")]
    pub license: String,
    #[serde(default)]
    pub stars: u64,
    #[serde(default)]
    pub status: EntryStatus,
    #[serde(default)]
    pub platforms: Vec<PlatformTarget>,
    #[serde(default)]
    pub targets: Vec<StoreTarget>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub first_party: bool,
}

fn default_version() -> String {
    "0.1.0".to_owned()
}

fn default_license() -> String {
    "MIT OR Apache-2.0".to_owned()
}

impl StoreEntry {
    #[must_use]
    pub fn supports_target(&self, target: StoreTarget) -> bool {
        self.targets.contains(&target)
    }

    #[must_use]
    pub fn supports_platform(&self, platform: PlatformTarget) -> bool {
        self.platforms.contains(&platform)
    }

    #[must_use]
    pub fn compatibility_matrix(&self) -> CompatibilityMatrix {
        CompatibilityMatrix {
            desktop: self.platforms.iter().any(|p| {
                matches!(
                    p,
                    PlatformTarget::Windows | PlatformTarget::Macos | PlatformTarget::Linux
                )
            }),
            web_wasm: self.supports_platform(PlatformTarget::Web)
                || self.supports_target(StoreTarget::NpmPackage),
            mobile_ios: self.supports_platform(PlatformTarget::Ios),
            mobile_android: self.supports_platform(PlatformTarget::Android)
                || self.supports_target(StoreTarget::PlayStore),
            freebsd: self.supports_platform(PlatformTarget::Freebsd)
                || self.supports_target(StoreTarget::FreeBsdPkg),
            browser_extension: self.supports_target(StoreTarget::ChromeWebStore),
            target_count: self.targets.len(),
        }
    }

    #[must_use]
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return true;
        }
        self.id.to_ascii_lowercase().contains(&q)
            || self.name.to_ascii_lowercase().contains(&q)
            || self.tagline.to_ascii_lowercase().contains(&q)
            || self.description.to_ascii_lowercase().contains(&q)
            || self.subcategory.to_ascii_lowercase().contains(&q)
            || self.crate_name.to_ascii_lowercase().contains(&q)
            || self
                .tags
                .iter()
                .any(|tag| tag.to_ascii_lowercase().contains(&q))
    }
}

/// Complete Awesome GPUI Store catalog parsed from `assets/os/awesome_gpui_store.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwesomeGpuiCatalog {
    pub schema_version: u32,
    pub source_repo: String,
    pub source_url: String,
    pub synced_at: String,
    #[serde(default)]
    pub upstream_stars: u64,
    #[serde(default)]
    pub total_entries: usize,
    pub entries: Vec<StoreEntry>,
}

impl AwesomeGpuiCatalog {
    pub fn bundled() -> Result<Self, StoreError> {
        Self::from_toml_str(BUNDLED_STORE_TOML)
    }

    pub fn from_toml_str(raw: &str) -> Result<Self, StoreError> {
        Ok(toml::from_str(raw)?)
    }

    /// Loads bundled catalog and merges `$XDG_CONFIG_HOME/aphrody/awesome_gpui_store.toml` overrides when present.
    pub fn load_with_user_override() -> Result<Self, StoreError> {
        let mut base = Self::bundled()?;
        if let Some(cfg_dir) = dirs::config_dir() {
            let user_path = cfg_dir
                .join(brand::APP_SLUG)
                .join("awesome_gpui_store.toml");
            if user_path.is_file()
                && let Ok(raw) = std::fs::read_to_string(&user_path)
                && let Ok(extra) = Self::from_toml_str(&raw)
            {
                for entry in extra.entries {
                    if let Some(existing) = base.entries.iter_mut().find(|e| e.id == entry.id) {
                        *existing = entry;
                    } else {
                        base.entries.push(entry);
                    }
                }
                base.total_entries = base.entries.len();
            }
        }
        Ok(base)
    }

    #[must_use]
    pub fn by_id(&self, id: &str) -> Option<&StoreEntry> {
        let needle = id.trim();
        self.entries
            .iter()
            .find(|e| e.id.eq_ignore_ascii_case(needle) || e.name.eq_ignore_ascii_case(needle))
    }

    #[must_use]
    pub fn search(
        &self,
        query: &str,
        category: Option<StoreCategory>,
        target: Option<StoreTarget>,
        platform: Option<PlatformTarget>,
    ) -> Vec<&StoreEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                if let Some(cat) = category
                    && entry.category != cat
                {
                    return false;
                }
                if let Some(tgt) = target
                    && !entry.supports_target(tgt)
                {
                    return false;
                }
                if let Some(plat) = platform
                    && !entry.supports_platform(plat)
                {
                    return false;
                }
                entry.matches_query(query)
            })
            .collect()
    }

    #[must_use]
    pub fn category_counts(&self) -> Vec<(StoreCategory, usize)> {
        StoreCategory::ALL
            .into_iter()
            .map(|cat| {
                let count = self.entries.iter().filter(|e| e.category == cat).count();
                (cat, count)
            })
            .collect()
    }

    #[must_use]
    pub fn target_counts(&self) -> Vec<(StoreTarget, usize)> {
        StoreTarget::ALL
            .into_iter()
            .map(|tgt| {
                let count = self
                    .entries
                    .iter()
                    .filter(|e| e.supports_target(tgt))
                    .count();
                (tgt, count)
            })
            .collect()
    }
}

/// Actions normalized across all OS package managers via `upt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UptAction {
    Install,
    Remove,
    Upgrade,
    Search,
    Info,
    UpdateIndex,
    UpgradeAll,
    ListInstalled,
}

impl UptAction {
    #[must_use]
    pub const fn as_cli_verb(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Remove => "remove",
            Self::Upgrade => "upgrade",
            Self::Search => "search",
            Self::Info => "info",
            Self::UpdateIndex => "update",
            Self::UpgradeAll => "upgrade",
            Self::ListInstalled => "list",
        }
    }

    #[must_use]
    pub const fn takes_package(self) -> bool {
        matches!(
            self,
            Self::Install | Self::Remove | Self::Upgrade | Self::Search | Self::Info
        )
    }

    #[must_use]
    pub const fn accepts_confirm(self) -> bool {
        matches!(
            self,
            Self::Install | Self::Remove | Self::Upgrade | Self::UpgradeAll
        )
    }
}

/// Resolved command line produced by `upt` for installing/managing a store package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UptResolvedPlan {
    pub detected_os: Option<String>,
    pub vendor_name: String,
    pub package_ref: String,
    pub command: Vec<String>,
    pub shell_line: String,
}

/// All package manager vendors supported by `upt 0.9.0`.
#[must_use]
pub const fn supported_upt_vendors() -> &'static [&'static str] {
    &[
        "apk",
        "apt",
        "brew",
        "cards",
        "choco",
        "dnf",
        "emerge",
        "eopkg",
        "flatpak",
        "guix",
        "nala",
        "nix-env",
        "opkg",
        "pacman",
        "pkg",
        "pkg(termux)",
        "pkgman",
        "prt-get",
        "scoop",
        "slackpkg",
        "snap",
        "upt",
        "urpm",
        "winget",
        "xbps",
        "yay",
        "yum",
        "zypper",
    ]
}

/// Canonical package name/identifier for a store entry under a given `upt` vendor.
#[must_use]
pub fn package_ref_for_vendor(entry: &StoreEntry, vendor_name: &str) -> String {
    match vendor_name {
        "winget" => packagers::winget_identifier(entry),
        "flatpak" => entry.app_id.clone(),
        _ => packagers::entry_slug(entry),
    }
}

/// Detect the active host OS and available `upt` package manager vendor.
#[cfg(not(target_family = "wasm"))]
#[must_use]
pub fn detect_host_vendor() -> (Option<String>, Option<String>) {
    let os = upt::detect_os();
    let vendor = os
        .as_deref()
        .and_then(|os_id| upt::detect_vendor(os_id).ok())
        .map(|v| v.name().to_owned());
    (os, vendor)
}

#[cfg(target_family = "wasm")]
#[must_use]
pub fn detect_host_vendor() -> (Option<String>, Option<String>) {
    (Some("web".to_owned()), None)
}

/// Resolve an exact OS package manager command via `upt::{init_vendor, Vendor}`.
#[cfg(not(target_family = "wasm"))]
pub fn resolve_upt_command(
    vendor_name: &str,
    action: UptAction,
    package: &str,
    confirm: bool,
) -> Result<Vec<String>, StoreError> {
    let upt_cli = upt::init_vendor("upt")?;
    let mut args = vec!["upt".to_owned(), action.as_cli_verb().to_owned()];
    if confirm && action.accepts_confirm() {
        args.push("-y".to_owned());
    }
    if action.takes_package() {
        args.push(package.to_owned());
    }
    let task = upt_cli.parse(&args, "gpui-store")?;
    let vendor: upt::Vendor = upt::init_vendor(vendor_name)?;
    Ok(vendor.eval(&task)?)
}

#[cfg(target_family = "wasm")]
pub fn resolve_upt_command(
    vendor_name: &str,
    action: UptAction,
    package: &str,
    _confirm: bool,
) -> Result<Vec<String>, StoreError> {
    let mut out = vec![vendor_name.to_owned(), action.as_cli_verb().to_owned()];
    if action.takes_package() {
        out.push(package.to_owned());
    }
    Ok(out)
}

/// Resolve a full [`UptResolvedPlan`] for a [`StoreEntry`] using a preferred vendor or live host auto-detection.
pub fn resolve_store_entry_command(
    entry: &StoreEntry,
    preferred_vendor: Option<&str>,
    action: UptAction,
    confirm: bool,
) -> Result<UptResolvedPlan, StoreError> {
    let (detected_os, detected_vendor) = detect_host_vendor();
    let vendor_name = preferred_vendor
        .map(str::to_owned)
        .or(detected_vendor)
        .unwrap_or_else(|| "upt".to_owned());
    let package_ref = package_ref_for_vendor(entry, &vendor_name);
    let command = resolve_upt_command(&vendor_name, action, &package_ref, confirm)?;
    let shell_line = command.join(" ");
    Ok(UptResolvedPlan {
        detected_os,
        vendor_name,
        package_ref,
        command,
        shell_line,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_loads_all_upstream_entries_and_aphrody() {
        let catalog =
            AwesomeGpuiCatalog::bundled().expect("awesome_gpui_store.toml parses cleanly");
        assert!(catalog.entries.len() >= 180);
        let aphrody = catalog.by_id("aphrody").expect("aphrody flagship present");
        assert!(aphrody.first_party);
        assert_eq!(aphrody.targets.len(), StoreTarget::ALL.len());
        let matrix = aphrody.compatibility_matrix();
        assert!(matrix.desktop);
        assert!(matrix.web_wasm);
        assert!(matrix.mobile_ios);
        assert!(matrix.mobile_android);
        assert!(matrix.browser_extension);
    }

    #[test]
    fn search_filters_by_query_category_and_target() {
        let catalog = AwesomeGpuiCatalog::bundled().unwrap();
        let libs = catalog.search("", Some(StoreCategory::Library), None, None);
        assert!(!libs.is_empty());
        let winget_apps = catalog.search(
            "aphrody",
            Some(StoreCategory::App),
            Some(StoreTarget::WinGet),
            None,
        );
        assert!(!winget_apps.is_empty());
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn upt_resolves_all_supported_os_vendors_for_store_entries() {
        let catalog = AwesomeGpuiCatalog::bundled().unwrap();
        let aphrody = catalog.by_id("aphrody").unwrap();

        let winget =
            resolve_store_entry_command(aphrody, Some("winget"), UptAction::Install, false)
                .unwrap();
        assert_eq!(winget.shell_line, "winget install AphrodyLabs.Aphrody");

        let pacman =
            resolve_store_entry_command(aphrody, Some("pacman"), UptAction::Install, true).unwrap();
        assert_eq!(pacman.shell_line, "pacman -S aphrody --noconfirm");

        let apk =
            resolve_store_entry_command(aphrody, Some("apk"), UptAction::Install, false).unwrap();
        assert_eq!(apk.shell_line, "apk add aphrody");

        let apt =
            resolve_store_entry_command(aphrody, Some("apt"), UptAction::Install, true).unwrap();
        assert_eq!(apt.shell_line, "apt install aphrody -y");

        let dnf =
            resolve_store_entry_command(aphrody, Some("dnf"), UptAction::Search, false).unwrap();
        assert_eq!(dnf.shell_line, "dnf search aphrody");

        let brew =
            resolve_store_entry_command(aphrody, Some("brew"), UptAction::Upgrade, false).unwrap();
        assert_eq!(brew.shell_line, "brew upgrade aphrody");

        let scoop =
            resolve_store_entry_command(aphrody, Some("scoop"), UptAction::Install, false).unwrap();
        assert_eq!(scoop.shell_line, "scoop install aphrody");

        let choco =
            resolve_store_entry_command(aphrody, Some("choco"), UptAction::Install, true).unwrap();
        assert_eq!(choco.shell_line, "choco install aphrody -y");

        let xbps =
            resolve_store_entry_command(aphrody, Some("xbps"), UptAction::Install, true).unwrap();
        assert_eq!(xbps.shell_line, "xbps-install aphrody -y");

        let emerge =
            resolve_store_entry_command(aphrody, Some("emerge"), UptAction::Install, false)
                .unwrap();
        assert_eq!(emerge.shell_line, "emerge aphrody");

        let freebsd =
            resolve_store_entry_command(aphrody, Some("pkg"), UptAction::Install, true).unwrap();
        assert_eq!(freebsd.shell_line, "pkg install aphrody -y");

        let nix = resolve_store_entry_command(aphrody, Some("nix-env"), UptAction::Install, false)
            .unwrap();
        assert_eq!(nix.shell_line, "nix-env -i aphrody");

        let zypper =
            resolve_store_entry_command(aphrody, Some("zypper"), UptAction::Install, true).unwrap();
        assert_eq!(zypper.shell_line, "zypper install aphrody -y");
    }
}
