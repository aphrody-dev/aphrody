use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use gpui::{Entity, Task, TestAppContext, VisualTestContext};
use gpui_ui::gpui::App;

use crate::backend::{
    AccountStatus, City, ConnectionState, Country, ExitLocation, LocationSelection, Toggle,
    VpnBackend, VpnCommand, VpnSnapshot,
};
use crate::panel::{POLL_INTERVAL, PrimaryAction, VpnView, normalize_account, present, time_left};
use gpui_ui::ui::Severity;

const FAKE_ACCOUNT: &str = "0000 0000 0000 0000";

struct FakeState {
    snapshot: Result<VpnSnapshot, String>,
    countries: Vec<Country>,
    commands: Vec<VpnCommand>,
    refuse_next: Option<String>,
}

#[derive(Clone)]
struct FakeBackend(Arc<Mutex<FakeState>>);

fn sample_snapshot(tunnel: ConnectionState) -> VpnSnapshot {
    VpnSnapshot {
        tunnel,
        exit: Some(ExitLocation {
            country: "Exampleland".into(),
            city: Some("Sample City".into()),
            hostname: Some("sample-relay-001".into()),
            address: None,
        }),
        account: AccountStatus::LoggedIn {
            device: "Quiet Example".into(),
            expiry_unix: Some(1_000_000),
        },
        selection: LocationSelection::Any,
        enabled: BTreeSet::new(),
    }
}

fn sample_countries() -> Vec<Country> {
    vec![Country {
        code: "ex".into(),
        name: "Exampleland".into(),
        cities: vec![City {
            code: "sc".into(),
            name: "Sample City".into(),
            relay_count: 3,
        }],
    }]
}

impl FakeBackend {
    fn new(tunnel: ConnectionState) -> Self {
        Self(Arc::new(Mutex::new(FakeState {
            snapshot: Ok(sample_snapshot(tunnel)),
            countries: sample_countries(),
            commands: Vec::new(),
            refuse_next: None,
        })))
    }

    fn with_state<R>(&self, update: impl FnOnce(&mut FakeState) -> R) -> R {
        update(&mut self.0.lock().expect("fake state lock"))
    }

    fn commands(&self) -> Vec<VpnCommand> {
        self.with_state(|state| state.commands.clone())
    }
}

impl VpnBackend for FakeBackend {
    fn snapshot(&self, _cx: &mut App) -> Task<Result<VpnSnapshot>> {
        Task::ready(self.with_state(|state| state.snapshot.clone().map_err(|error| anyhow!(error))))
    }

    fn locations(&self, _cx: &mut App) -> Task<Result<Vec<Country>>> {
        Task::ready(Ok(self.with_state(|state| state.countries.clone())))
    }

    fn execute(&self, command: VpnCommand, _cx: &mut App) -> Task<Result<()>> {
        Task::ready(self.with_state(|state| {
            if let Some(reason) = state.refuse_next.take() {
                return Err(anyhow!(reason));
            }
            state.commands.push(command.clone());
            if let Ok(snapshot) = state.snapshot.as_mut() {
                match command {
                    VpnCommand::Connect => snapshot.tunnel = ConnectionState::Connected,
                    VpnCommand::Disconnect => {
                        snapshot.tunnel = ConnectionState::Disconnected { locked_down: false }
                    }
                    VpnCommand::Login(_) => {
                        snapshot.account = AccountStatus::LoggedIn {
                            device: "Quiet Example".into(),
                            expiry_unix: None,
                        }
                    }
                    VpnCommand::Logout => snapshot.account = AccountStatus::LoggedOut,
                    VpnCommand::SelectLocation(selection) => snapshot.selection = selection,
                    VpnCommand::Set(toggle, true) => {
                        snapshot.enabled.insert(toggle);
                    }
                    VpnCommand::Set(toggle, false) => {
                        snapshot.enabled.remove(&toggle);
                    }
                }
            }
            Ok(())
        }))
    }
}

fn open_view<'a>(
    backend: &FakeBackend,
    cx: &'a mut TestAppContext,
) -> (Entity<VpnView>, &'a mut VisualTestContext) {
    cx.update(gpui_ui::init);
    let backend: Arc<dyn VpnBackend> = Arc::new(backend.clone());
    let (view, cx) = cx.add_window_view(|window, cx| VpnView::new(backend, window, cx));
    cx.run_until_parked();
    (view, cx)
}

fn tunnel_of(view: &Entity<VpnView>, cx: &VisualTestContext) -> Option<ConnectionState> {
    view.read_with(cx, |view, _| {
        view.snapshot().map(|snapshot| snapshot.tunnel.clone())
    })
}

#[test]
fn every_tunnel_state_maps_to_a_headline_tone_and_action() {
    let cases = [
        (
            ConnectionState::Disconnected { locked_down: false },
            "Unsecured connection",
            Severity::Error,
            PrimaryAction::Connect,
        ),
        (
            ConnectionState::Disconnected { locked_down: true },
            "Blocked: lockdown mode",
            Severity::Warning,
            PrimaryAction::Connect,
        ),
        (
            ConnectionState::Connecting,
            "Connecting",
            Severity::Warning,
            PrimaryAction::Cancel,
        ),
        (
            ConnectionState::Connected,
            "Secure connection",
            Severity::Success,
            PrimaryAction::Disconnect,
        ),
        (
            ConnectionState::Disconnecting,
            "Disconnecting",
            Severity::Warning,
            PrimaryAction::Wait,
        ),
        (
            ConnectionState::Error {
                reason: "no route".into(),
            },
            "Blocked connection: no route",
            Severity::Error,
            PrimaryAction::Disconnect,
        ),
    ];
    for (state, headline, severity, action) in cases {
        let presentation = present(&state);
        assert_eq!(presentation.headline, headline);
        assert_eq!(presentation.severity, severity);
        assert_eq!(presentation.action, action);
    }
    assert_eq!(PrimaryAction::Wait.command(), None);
    assert_eq!(
        PrimaryAction::Cancel.command(),
        Some(VpnCommand::Disconnect)
    );
}

#[test]
fn account_input_and_expiry_text() {
    assert_eq!(
        normalize_account(FAKE_ACCOUNT).as_deref(),
        Ok("0000000000000000")
    );
    assert_eq!(
        normalize_account("0000-0000-0000-0000").as_deref(),
        Ok("0000000000000000")
    );
    assert!(normalize_account("12345").is_err());
    assert!(normalize_account("abcd abcd abcd abcd").is_err());
    assert_eq!(time_left(100, 100), "Out of time");
    assert_eq!(time_left(100 + 3_600, 100), "Less than a day left");
    assert_eq!(time_left(100 + 86_400, 100), "1 day left");
    assert_eq!(time_left(100 + 5 * 86_400, 100), "5 days left");
}

#[gpui::test]
async fn first_poll_fills_the_snapshot_and_the_locations(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Connected);
    let (view, cx) = open_view(&backend, cx);
    assert_eq!(tunnel_of(&view, cx), Some(ConnectionState::Connected));
    view.read_with(cx, |view, _| {
        assert_eq!(view.countries.len(), 1);
        assert!(view.daemon_error.is_none());
    });
}

#[gpui::test]
async fn polling_follows_the_daemon(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Disconnected { locked_down: false });
    let (view, cx) = open_view(&backend, cx);
    backend.with_state(|state| {
        if let Ok(snapshot) = state.snapshot.as_mut() {
            snapshot.tunnel = ConnectionState::Connecting;
        }
    });
    cx.executor().advance_clock(POLL_INTERVAL);
    cx.run_until_parked();
    assert_eq!(tunnel_of(&view, cx), Some(ConnectionState::Connecting));
}

#[gpui::test]
async fn an_unreachable_daemon_is_shown_then_recovers(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Connected);
    backend.with_state(|state| state.snapshot = Err("cannot reach the VPN daemon".into()));
    let (view, cx) = open_view(&backend, cx);
    view.read_with(cx, |view, _| {
        assert!(view.snapshot().is_none());
        let message = view.daemon_error.as_ref().expect("daemon error");
        assert!(message.contains("cannot reach the VPN daemon"));
    });

    backend.with_state(|state| state.snapshot = Ok(sample_snapshot(ConnectionState::Connected)));
    cx.executor().advance_clock(POLL_INTERVAL);
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(view.daemon_error.is_none());
        assert!(view.snapshot().is_some());
    });
}

#[gpui::test]
async fn the_main_button_connects_then_disconnects(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Disconnected { locked_down: false });
    let (view, cx) = open_view(&backend, cx);

    view.update(cx, |view, cx| view.toggle_connection(cx));
    cx.run_until_parked();
    assert_eq!(tunnel_of(&view, cx), Some(ConnectionState::Connected));

    view.update(cx, |view, cx| view.toggle_connection(cx));
    cx.run_until_parked();
    assert_eq!(
        tunnel_of(&view, cx),
        Some(ConnectionState::Disconnected { locked_down: false })
    );
    assert_eq!(
        backend.commands(),
        vec![VpnCommand::Connect, VpnCommand::Disconnect]
    );
}

#[gpui::test]
async fn cancelling_a_connection_attempt_disconnects(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Connecting);
    let (view, cx) = open_view(&backend, cx);
    view.update(cx, |view, cx| view.toggle_connection(cx));
    cx.run_until_parked();
    assert_eq!(backend.commands(), vec![VpnCommand::Disconnect]);
}

#[gpui::test]
async fn nothing_is_sent_while_the_tunnel_is_disconnecting(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Disconnecting);
    let (view, cx) = open_view(&backend, cx);
    view.update(cx, |view, cx| view.toggle_connection(cx));
    cx.run_until_parked();
    assert!(backend.commands().is_empty());
}

#[gpui::test]
async fn a_refused_command_surfaces_in_the_view(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Disconnected { locked_down: false });
    let (view, cx) = open_view(&backend, cx);
    backend.with_state(|state| state.refuse_next = Some("no account".into()));

    view.update(cx, |view, cx| view.toggle_connection(cx));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        let message = view.action_error.as_ref().expect("action error");
        assert!(message.contains("no account"));
        assert!(!view.busy);
    });
    assert!(backend.commands().is_empty());

    view.update(cx, |view, cx| view.toggle_connection(cx));
    cx.run_until_parked();
    view.read_with(cx, |view, _| assert!(view.action_error.is_none()));
}

#[gpui::test]
async fn login_validates_before_it_reaches_the_daemon(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Disconnected { locked_down: false });
    backend.with_state(|state| {
        if let Ok(snapshot) = state.snapshot.as_mut() {
            snapshot.account = AccountStatus::LoggedOut;
        }
    });
    let (view, cx) = open_view(&backend, cx);

    view.update(cx, |view, cx| view.login("not an account", cx));
    cx.run_until_parked();
    assert!(backend.commands().is_empty());

    view.update(cx, |view, cx| view.login(FAKE_ACCOUNT, cx));
    cx.run_until_parked();
    assert_eq!(
        backend.commands(),
        vec![VpnCommand::Login("0000000000000000".into())]
    );
    view.read_with(cx, |view, _| {
        assert!(matches!(
            view.snapshot().map(|snapshot| &snapshot.account),
            Some(AccountStatus::LoggedIn { .. })
        ));
    });
}

#[gpui::test]
async fn logout_returns_to_the_login_state(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Disconnected { locked_down: false });
    let (view, cx) = open_view(&backend, cx);
    view.update(cx, |view, cx| view.run(VpnCommand::Logout, cx));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.snapshot().map(|snapshot| snapshot.account.clone()),
            Some(AccountStatus::LoggedOut)
        );
    });
}

#[gpui::test]
async fn choosing_a_city_selects_it_and_closes_the_list(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Disconnected { locked_down: false });
    let (view, cx) = open_view(&backend, cx);
    view.update(cx, |view, _| view.locations_open = true);

    let city = LocationSelection::City("ex".into(), "sc".into());
    view.update(cx, |view, cx| view.select_location(city.clone(), cx));
    cx.run_until_parked();

    assert_eq!(
        backend.commands(),
        vec![VpnCommand::SelectLocation(city.clone())]
    );
    view.read_with(cx, |view, _| {
        assert!(!view.locations_open);
        assert_eq!(
            view.snapshot().map(|snapshot| snapshot.selection.clone()),
            Some(city)
        );
    });
}

#[gpui::test]
async fn switches_write_settings_and_read_them_back(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Disconnected { locked_down: false });
    let (view, cx) = open_view(&backend, cx);

    view.update(cx, |view, cx| view.set_toggle(Toggle::Daita, true, cx));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(
            view.snapshot()
                .is_some_and(|snapshot| snapshot.is_enabled(Toggle::Daita))
        );
    });

    view.update(cx, |view, cx| view.set_toggle(Toggle::Daita, false, cx));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(
            view.snapshot()
                .is_some_and(|snapshot| !snapshot.is_enabled(Toggle::Daita))
        );
    });
    assert_eq!(
        backend.commands(),
        vec![
            VpnCommand::Set(Toggle::Daita, true),
            VpnCommand::Set(Toggle::Daita, false)
        ]
    );
}

#[gpui::test]
async fn the_view_renders_with_its_location_list_open(cx: &mut TestAppContext) {
    let backend = FakeBackend::new(ConnectionState::Connected);
    let (view, cx) = open_view(&backend, cx);
    view.update(cx, |view, cx| {
        view.locations_open = true;
        view.expanded_country = Some("ex".into());
        cx.notify();
    });
    cx.run_until_parked();
}

#[test]
fn the_account_number_never_appears_in_debug_output() {
    let command = VpnCommand::Login("0000000000000000".into());
    assert!(!format!("{command:?}").contains("0000"));
}
