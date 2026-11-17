//! The data the VPN view renders and the contract a daemon connection fulfils.

use std::collections::BTreeSet;
use std::fmt;

use anyhow::Result;
use gpui_ui::gpui::{App, Task};

/// What the tunnel is doing, reduced to what the view distinguishes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected { locked_down: bool },
    Connecting,
    Connected,
    Disconnecting,
    Error { reason: String },
}

/// Where the daemon says the traffic leaves from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExitLocation {
    pub country: String,
    pub city: Option<String>,
    pub hostname: Option<String>,
    pub address: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccountStatus {
    LoggedOut,
    Revoked,
    LoggedIn {
        device: String,
        expiry_unix: Option<i64>,
    },
}

/// The relay location the user asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocationSelection {
    Any,
    Country(String),
    City(String, String),
    /// A hostname, a custom list or a custom endpoint: shown, not selectable from the list.
    Other(String),
}

/// A daemon setting the view exposes as a switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Toggle {
    AutoConnect,
    AllowLan,
    LockdownMode,
    Ipv6,
    QuantumResistant,
    Daita,
    BlockAds,
    BlockTrackers,
    BlockMalware,
}

impl Toggle {
    pub const ALL: [Toggle; 9] = [
        Toggle::AutoConnect,
        Toggle::AllowLan,
        Toggle::LockdownMode,
        Toggle::Ipv6,
        Toggle::QuantumResistant,
        Toggle::Daita,
        Toggle::BlockAds,
        Toggle::BlockTrackers,
        Toggle::BlockMalware,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Toggle::AutoConnect => "Auto-connect",
            Toggle::AllowLan => "Local network sharing",
            Toggle::LockdownMode => "Lockdown mode",
            Toggle::Ipv6 => "IPv6 in the tunnel",
            Toggle::QuantumResistant => "Quantum-resistant tunnel",
            Toggle::Daita => "DAITA",
            Toggle::BlockAds => "Block ads",
            Toggle::BlockTrackers => "Block trackers",
            Toggle::BlockMalware => "Block malware",
        }
    }

    pub fn group(self) -> &'static str {
        match self {
            Toggle::AutoConnect | Toggle::AllowLan | Toggle::LockdownMode => "General",
            Toggle::Ipv6 | Toggle::QuantumResistant | Toggle::Daita => "VPN settings",
            Toggle::BlockAds | Toggle::BlockTrackers | Toggle::BlockMalware => {
                "DNS content blockers"
            }
        }
    }
}

/// Everything the daemon reports that the view shows, read in one pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VpnSnapshot {
    pub tunnel: ConnectionState,
    pub exit: Option<ExitLocation>,
    pub account: AccountStatus,
    pub selection: LocationSelection,
    pub enabled: BTreeSet<Toggle>,
}

impl VpnSnapshot {
    pub fn is_enabled(&self, toggle: Toggle) -> bool {
        self.enabled.contains(&toggle)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct City {
    pub code: String,
    pub name: String,
    pub relay_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Country {
    pub code: String,
    pub name: String,
    pub cities: Vec<City>,
}

/// A change the view asks the daemon to make.
#[derive(Clone, PartialEq, Eq)]
pub enum VpnCommand {
    Connect,
    Disconnect,
    Login(String),
    Logout,
    SelectLocation(LocationSelection),
    Set(Toggle, bool),
}

impl fmt::Debug for VpnCommand {
    // The account number is a credential: it never reaches a log line.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VpnCommand::Connect => formatter.write_str("Connect"),
            VpnCommand::Disconnect => formatter.write_str("Disconnect"),
            VpnCommand::Login(_) => formatter.write_str("Login(..)"),
            VpnCommand::Logout => formatter.write_str("Logout"),
            VpnCommand::SelectLocation(selection) => formatter
                .debug_tuple("SelectLocation")
                .field(selection)
                .finish(),
            VpnCommand::Set(toggle, enabled) => formatter
                .debug_tuple("Set")
                .field(toggle)
                .field(enabled)
                .finish(),
        }
    }
}

/// A connection to a VPN daemon. Every call returns a task so the view never blocks on the daemon.
pub trait VpnBackend: 'static {
    fn snapshot(&self, cx: &mut App) -> Task<Result<VpnSnapshot>>;
    fn locations(&self, cx: &mut App) -> Task<Result<Vec<Country>>>;
    fn execute(&self, command: VpnCommand, cx: &mut App) -> Task<Result<()>>;
}
