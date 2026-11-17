//! [`VpnBackend`] over the `ayvpn-daemon` client; the protocol and the types are the daemon's own.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow};
use ayvpn_daemon::mullvad_types::constraints::Constraint;
use ayvpn_daemon::mullvad_types::device::DeviceState;
use ayvpn_daemon::mullvad_types::location::GeoIpLocation;
use ayvpn_daemon::mullvad_types::relay_constraints::{
    GeographicLocationConstraint, LocationConstraint, RelaySettings,
};
use ayvpn_daemon::mullvad_types::relay_list::RelayList;
use ayvpn_daemon::mullvad_types::settings::Settings;
use ayvpn_daemon::{Client, Error, TunnelState};
use futures::lock::Mutex;
use gpui_tokio::Tokio;
use gpui_ui::gpui::{App, Task};

use crate::backend::{
    AccountStatus, City, ConnectionState, Country, ExitLocation, LocationSelection, Toggle,
    VpnBackend, VpnCommand, VpnSnapshot,
};

/// The daemon is asked for the account expiry at most this often: it is an API round trip.
const EXPIRY_REFRESH: Duration = Duration::from_secs(300);

struct CachedExpiry {
    fetched: Instant,
    expiry_unix: Option<i64>,
}

/// Talks to the daemon's management IPC, reconnecting after any failed call.
#[derive(Clone)]
pub struct DaemonBackend {
    client: Arc<Mutex<Option<Client>>>,
    expiry: Arc<StdMutex<Option<CachedExpiry>>>,
}

impl DaemonBackend {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
            expiry: Arc::new(StdMutex::new(None)),
        }
    }

    fn forget_expiry(expiry: &StdMutex<Option<CachedExpiry>>) {
        if let Ok(mut cached) = expiry.lock() {
            *cached = None;
        }
    }
}

impl Default for DaemonBackend {
    fn default() -> Self {
        Self::new()
    }
}

async fn with_client<T>(
    slot: &Mutex<Option<Client>>,
    operation: impl AsyncFnOnce(&mut Client) -> Result<T, Error>,
) -> Result<T> {
    let mut guard = slot.lock().await;
    let client = match guard.as_mut() {
        Some(client) => client,
        None => guard.insert(
            Client::connect()
                .await
                .context("cannot reach the VPN daemon")?,
        ),
    };
    match operation(client).await {
        Ok(value) => Ok(value),
        Err(error) => {
            *guard = None;
            Err(anyhow!(error))
        }
    }
}

impl VpnBackend for DaemonBackend {
    fn snapshot(&self, cx: &mut App) -> Task<Result<VpnSnapshot>> {
        let client = self.client.clone();
        let expiry = self.expiry.clone();
        Tokio::spawn_result(cx, async move {
            with_client(&client, async |client| read_snapshot(client, &expiry).await).await
        })
    }

    fn locations(&self, cx: &mut App) -> Task<Result<Vec<Country>>> {
        let client = self.client.clone();
        Tokio::spawn_result(cx, async move {
            let list = with_client(&client, async |client| client.relay_list().await).await?;
            Ok(countries(&list))
        })
    }

    fn execute(&self, command: VpnCommand, cx: &mut App) -> Task<Result<()>> {
        let client = self.client.clone();
        let expiry = self.expiry.clone();
        Tokio::spawn_result(cx, async move {
            with_client(&client, async |client| match command {
                VpnCommand::Connect => client.connect_tunnel().await.map(|_| ()),
                VpnCommand::Disconnect => client.disconnect_tunnel().await.map(|_| ()),
                VpnCommand::Login(account) => {
                    Self::forget_expiry(&expiry);
                    client.login(account).await
                }
                VpnCommand::Logout => {
                    Self::forget_expiry(&expiry);
                    client.logout().await
                }
                VpnCommand::SelectLocation(selection) => select_location(client, selection).await,
                VpnCommand::Set(toggle, enabled) => set_toggle(client, toggle, enabled).await,
            })
            .await
        })
    }
}

async fn read_snapshot(
    client: &mut Client,
    expiry: &StdMutex<Option<CachedExpiry>>,
) -> Result<VpnSnapshot, Error> {
    let tunnel = client.state().await?;
    let settings = client.settings().await?;
    let device = client.device().await?;

    let account = match device {
        DeviceState::LoggedOut => AccountStatus::LoggedOut,
        DeviceState::Revoked => AccountStatus::Revoked,
        DeviceState::LoggedIn(logged_in) => {
            let cached = expiry.lock().ok().and_then(|cached| {
                cached
                    .as_ref()
                    .filter(|cached| cached.fetched.elapsed() < EXPIRY_REFRESH)
                    .map(|cached| cached.expiry_unix)
            });
            let expiry_unix = match cached {
                Some(expiry_unix) => expiry_unix,
                None => {
                    let fetched = match client.account_data(logged_in.account_number).await {
                        Ok(data) => Some(data.expiry.timestamp()),
                        Err(error) => {
                            log::warn!("account expiry unavailable: {error}");
                            None
                        }
                    };
                    if let Ok(mut cached) = expiry.lock() {
                        *cached = Some(CachedExpiry {
                            fetched: Instant::now(),
                            expiry_unix: fetched,
                        });
                    }
                    fetched
                }
            };
            AccountStatus::LoggedIn {
                device: logged_in.device.pretty_name(),
                expiry_unix,
            }
        }
    };

    let (tunnel, exit) = map_tunnel(&tunnel);
    Ok(VpnSnapshot {
        tunnel,
        exit,
        account,
        selection: map_selection(&settings),
        enabled: map_toggles(&settings),
    })
}

fn map_tunnel(state: &TunnelState) -> (ConnectionState, Option<ExitLocation>) {
    match state {
        TunnelState::Disconnected {
            location,
            locked_down,
        } => (
            ConnectionState::Disconnected {
                locked_down: *locked_down,
            },
            location.as_ref().map(map_exit),
        ),
        TunnelState::Connecting { location, .. } => {
            (ConnectionState::Connecting, location.as_ref().map(map_exit))
        }
        TunnelState::Connected { location, .. } => {
            (ConnectionState::Connected, location.as_ref().map(map_exit))
        }
        TunnelState::Disconnecting(_) => (ConnectionState::Disconnecting, None),
        TunnelState::Error(error) => (
            ConnectionState::Error {
                reason: error.cause().to_string(),
            },
            None,
        ),
    }
}

fn map_exit(location: &GeoIpLocation) -> ExitLocation {
    let hostname = match &location.location {
        Some(GeographicLocationConstraint::Hostname(_, _, hostname)) => Some(hostname.clone()),
        _ => None,
    };
    ExitLocation {
        country: location.country.clone(),
        city: location.city.clone(),
        hostname,
        address: location
            .ipv4
            .map(|address| address.to_string())
            .or_else(|| location.ipv6.map(|address| address.to_string())),
    }
}

fn map_selection(settings: &Settings) -> LocationSelection {
    match &settings.relay_settings {
        RelaySettings::CustomTunnelEndpoint(_) => {
            LocationSelection::Other("Custom endpoint".into())
        }
        RelaySettings::Normal(constraints) => match &constraints.location {
            Constraint::Any => LocationSelection::Any,
            Constraint::Only(LocationConstraint::Location(location)) => match location {
                GeographicLocationConstraint::Country(country) => {
                    LocationSelection::Country(country.clone())
                }
                GeographicLocationConstraint::City(country, city) => {
                    LocationSelection::City(country.clone(), city.clone())
                }
                GeographicLocationConstraint::Hostname(_, _, hostname) => {
                    LocationSelection::Other(hostname.clone())
                }
            },
            Constraint::Only(LocationConstraint::CustomList { .. }) => {
                LocationSelection::Other("Custom list".into())
            }
        },
    }
}

fn map_toggles(settings: &Settings) -> BTreeSet<Toggle> {
    let dns = &settings.tunnel_options.dns_options.default_options;
    let wireguard = &settings.tunnel_options.wireguard;
    [
        (Toggle::AutoConnect, settings.auto_connect),
        (Toggle::AllowLan, settings.allow_lan),
        (Toggle::LockdownMode, settings.lockdown_mode),
        (Toggle::Ipv6, settings.tunnel_options.generic.enable_ipv6),
        (
            Toggle::QuantumResistant,
            wireguard.quantum_resistant.enabled(),
        ),
        (Toggle::Daita, wireguard.daita),
        (Toggle::BlockAds, dns.block_ads),
        (Toggle::BlockTrackers, dns.block_trackers),
        (Toggle::BlockMalware, dns.block_malware),
    ]
    .into_iter()
    .filter_map(|(toggle, enabled)| enabled.then_some(toggle))
    .collect()
}

fn countries(list: &RelayList) -> Vec<Country> {
    let mut countries: Vec<Country> = list
        .countries
        .iter()
        .map(|country| Country {
            code: country.code.clone(),
            name: country.name.clone(),
            cities: country
                .cities
                .iter()
                .filter(|city| city.relays.iter().any(|relay| relay.inner.active))
                .map(|city| City {
                    code: city.code.clone(),
                    name: city.name.clone(),
                    relay_count: city
                        .relays
                        .iter()
                        .filter(|relay| relay.inner.active)
                        .count(),
                })
                .collect(),
        })
        .filter(|country| !country.cities.is_empty())
        .collect();
    countries.sort_by(|left, right| left.name.cmp(&right.name));
    countries
}

async fn select_location(client: &mut Client, selection: LocationSelection) -> Result<(), Error> {
    let location = match selection {
        LocationSelection::Any => Constraint::Any,
        LocationSelection::Country(country) => Constraint::Only(LocationConstraint::Location(
            GeographicLocationConstraint::Country(country),
        )),
        LocationSelection::City(country, city) => Constraint::Only(LocationConstraint::Location(
            GeographicLocationConstraint::City(country, city),
        )),
        LocationSelection::Other(_) => return Ok(()),
    };
    match client.settings().await?.relay_settings {
        RelaySettings::Normal(mut constraints) => {
            constraints.location = location;
            client
                .set_relay_settings(RelaySettings::Normal(constraints))
                .await
        }
        RelaySettings::CustomTunnelEndpoint(_) => Ok(()),
    }
}

async fn set_toggle(client: &mut Client, toggle: Toggle, enabled: bool) -> Result<(), Error> {
    match toggle {
        Toggle::AutoConnect => client.set_auto_connect(enabled).await,
        Toggle::AllowLan => client.set_allow_lan(enabled).await,
        Toggle::LockdownMode => client.set_lockdown_mode(enabled).await,
        Toggle::Ipv6 => client.set_enable_ipv6(enabled).await,
        Toggle::QuantumResistant => client.set_quantum_resistant(enabled).await,
        Toggle::Daita => client.set_daita(enabled).await,
        Toggle::BlockAds | Toggle::BlockTrackers | Toggle::BlockMalware => {
            let mut options = client.settings().await?.tunnel_options.dns_options;
            let blockers = &mut options.default_options;
            match toggle {
                Toggle::BlockAds => blockers.block_ads = enabled,
                Toggle::BlockTrackers => blockers.block_trackers = enabled,
                _ => blockers.block_malware = enabled,
            }
            client.set_dns_options(options).await
        }
    }
}
