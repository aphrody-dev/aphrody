//! The Aphrody VPN daemon, its client, and its C ABI v1 surface.
//!
//! The daemon is Mullvad's `mullvad-daemon`, built with its `embedded` feature so that the host
//! process keeps its own logger and release lifecycle. The client speaks the daemon's own
//! management IPC; this crate defines no protocol of its own.

#[cfg(not(target_os = "android"))]
pub mod cli;
#[cfg(not(target_os = "android"))]
pub mod ffi;

use std::path::PathBuf;

pub use mullvad_management_interface::Error;
pub use mullvad_types;
pub use mullvad_types::states::TunnelState;

/// A started daemon; [`run`] drives it until it is told to stop.
pub type Daemon = mullvad_daemon::Daemon;

/// Mullvad keeps its settings and cache in system directories that only an elevated session can
/// create. Any other session gets per-user directories under the shared Aphrody state directory.
/// `MULLVAD_SETTINGS_DIR` and `MULLVAD_CACHE_DIR` still win when they are set.
fn user_dir(kind: &str) -> Result<PathBuf, String> {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("Aphrody"))
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
            .map(|dir| dir.join("aphrody"))
    }
    .ok_or("no per-user state directory (LOCALAPPDATA or HOME is unset)")?;
    let dir = base.join("vpn").join(kind);
    std::fs::create_dir_all(&dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    Ok(dir)
}

/// Starts the daemon in this process with Mullvad's platform paths and endpoint settings.
pub async fn start() -> Result<Daemon, String> {
    let settings_dir = mullvad_paths::settings_dir()
        .or_else(|_| user_dir("settings"))
        .map_err(|error| error.to_string())?;
    let cache_dir = mullvad_paths::cache_dir()
        .or_else(|_| user_dir("cache"))
        .map_err(|error| error.to_string())?;
    let log_handle = mullvad_daemon::logging::detached_log_handle().map_err(|error| error.to_string())?;
    let config = mullvad_daemon::DaemonConfig {
        log_dir: None,
        resource_dir: mullvad_paths::get_resource_dir(),
        settings_dir,
        cache_dir,
        rpc_socket_path: mullvad_paths::get_rpc_socket_path(),
        endpoint: mullvad_api::ApiEndpoint::from_env_vars(),
        log_handle,
    };
    mullvad_daemon::Daemon::start(config, mullvad_daemon::DaemonCommandChannel::new())
        .await
        .map_err(|error| error.to_string())
}

/// Starts the daemon and runs it until it stops.
pub async fn run() -> Result<(), String> {
    start().await?.run().await.map_err(|error| error.to_string())
}

#[cfg(not(target_os = "android"))]
use mullvad_types::{
    account::AccountData,
    device::DeviceState,
    relay_constraints::RelaySettings,
    relay_list::RelayList,
    settings::{DnsOptions, Settings},
    wireguard::QuantumResistantState,
};

/// A connection to a running daemon's management IPC.
#[cfg(not(target_os = "android"))]
pub struct Client(mullvad_management_interface::client::MullvadProxyClient);

#[cfg(not(target_os = "android"))]
impl Client {
    /// Connects to the platform's management endpoint.
    pub async fn connect() -> Result<Self, Error> {
        mullvad_management_interface::client::MullvadProxyClient::new().await.map(Self)
    }

    /// The tunnel state the daemon reports.
    pub async fn state(&mut self) -> Result<TunnelState, Error> {
        self.0.get_tunnel_state().await
    }

    /// Logs in with an account number; the number never appears in an error or a log line.
    pub async fn login(&mut self, account: String) -> Result<(), Error> {
        self.0.login_account(account).await
    }

    /// Asks the daemon to connect with its current settings; `true` when the state changed.
    pub async fn connect_tunnel(&mut self) -> Result<bool, Error> {
        self.0.connect_tunnel().await
    }

    /// Asks the daemon to disconnect; `true` when the state changed.
    pub async fn disconnect_tunnel(&mut self) -> Result<bool, Error> {
        self.0.disconnect_tunnel("ayvpn").await
    }

    /// The daemon's settings: relay constraints, tunnel options, DNS options and the toggles.
    pub async fn settings(&mut self) -> Result<Settings, Error> {
        self.0.get_settings().await
    }

    /// The relay list: countries, their cities and the relays in each.
    pub async fn relay_list(&mut self) -> Result<RelayList, Error> {
        self.0.get_relay_locations().await
    }

    /// Whether a device is logged in, logged out or revoked.
    pub async fn device(&mut self) -> Result<DeviceState, Error> {
        self.0.get_device().await
    }

    /// The expiry of an account, looked up through the daemon's API connection.
    pub async fn account_data(&mut self, account: String) -> Result<AccountData, Error> {
        self.0.get_account_data(account).await
    }

    /// Logs the current device out.
    pub async fn logout(&mut self) -> Result<(), Error> {
        self.0.logout_account("ayvpn").await
    }

    /// Replaces the relay settings, which carry the selected location.
    pub async fn set_relay_settings(&mut self, settings: RelaySettings) -> Result<(), Error> {
        self.0.set_relay_settings(settings).await
    }

    /// Replaces the DNS options, which carry the content blockers.
    pub async fn set_dns_options(&mut self, options: DnsOptions) -> Result<(), Error> {
        self.0.set_dns_options(options).await
    }

    pub async fn set_allow_lan(&mut self, state: bool) -> Result<(), Error> {
        self.0.set_allow_lan(state).await
    }

    pub async fn set_lockdown_mode(&mut self, state: bool) -> Result<(), Error> {
        self.0.set_lockdown_mode(state).await
    }

    pub async fn set_auto_connect(&mut self, state: bool) -> Result<(), Error> {
        self.0.set_auto_connect(state).await
    }

    pub async fn set_enable_ipv6(&mut self, state: bool) -> Result<(), Error> {
        self.0.set_enable_ipv6(state).await
    }

    pub async fn set_daita(&mut self, state: bool) -> Result<(), Error> {
        self.0.set_enable_daita(state).await
    }

    pub async fn set_quantum_resistant(&mut self, state: bool) -> Result<(), Error> {
        let state = if state { QuantumResistantState::On } else { QuantumResistantState::Off };
        self.0.set_quantum_resistant_tunnel(state).await
    }
}
