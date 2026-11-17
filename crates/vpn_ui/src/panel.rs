//! The VPN view and the dock panel and pane item that host it.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use gpui_ui::gpui::ClickEvent;
use gpui_ui::prelude::*;
use gpui_ui::ui::{Callout, Divider, Indicator, Switch, TintColor};
use gpui_ui::{InputField, menu};
use util::ResultExt as _;
use workspace::Workspace;
use workspace::dock::{DockPosition, Panel, PanelEvent};
use workspace::item::{Item, ItemEvent};

use crate::backend::{
    AccountStatus, ConnectionState, Country, LocationSelection, Toggle, VpnBackend, VpnCommand,
    VpnSnapshot,
};

actions!(
    vpn,
    [
        /// Shows or hides the VPN dock panel.
        ToggleFocus,
        /// Opens the VPN view as a tab in the active pane.
        OpenVpn,
        /// Connects when the tunnel is down, disconnects otherwise.
        ToggleConnection,
    ]
);

pub(crate) const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// What the main button does in a given tunnel state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimaryAction {
    Connect,
    Cancel,
    Disconnect,
    Wait,
}

/// How a tunnel state is shown: the banner headline, its tone and the main button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusPresentation {
    pub headline: String,
    pub severity: Severity,
    pub action: PrimaryAction,
}

impl PrimaryAction {
    pub fn label(self) -> &'static str {
        match self {
            PrimaryAction::Connect => "Connect",
            PrimaryAction::Cancel => "Cancel",
            PrimaryAction::Disconnect => "Disconnect",
            PrimaryAction::Wait => "Disconnecting",
        }
    }

    pub fn command(self) -> Option<VpnCommand> {
        match self {
            PrimaryAction::Connect => Some(VpnCommand::Connect),
            PrimaryAction::Cancel | PrimaryAction::Disconnect => Some(VpnCommand::Disconnect),
            PrimaryAction::Wait => None,
        }
    }
}

pub fn present(state: &ConnectionState) -> StatusPresentation {
    let (headline, severity, action) = match state {
        ConnectionState::Disconnected { locked_down: false } => (
            "Unsecured connection".to_owned(),
            Severity::Error,
            PrimaryAction::Connect,
        ),
        ConnectionState::Disconnected { locked_down: true } => (
            "Blocked: lockdown mode".to_owned(),
            Severity::Warning,
            PrimaryAction::Connect,
        ),
        ConnectionState::Connecting => (
            "Connecting".to_owned(),
            Severity::Warning,
            PrimaryAction::Cancel,
        ),
        ConnectionState::Connected => (
            "Secure connection".to_owned(),
            Severity::Success,
            PrimaryAction::Disconnect,
        ),
        ConnectionState::Disconnecting => (
            "Disconnecting".to_owned(),
            Severity::Warning,
            PrimaryAction::Wait,
        ),
        ConnectionState::Error { reason } => (
            format!("Blocked connection: {reason}"),
            Severity::Error,
            PrimaryAction::Disconnect,
        ),
    };
    StatusPresentation {
        headline,
        severity,
        action,
    }
}

/// The account line: time left, or that the account has run out.
pub fn time_left(expiry_unix: i64, now_unix: i64) -> String {
    let seconds = expiry_unix - now_unix;
    if seconds <= 0 {
        return "Out of time".to_owned();
    }
    match seconds / 86_400 {
        0 => "Less than a day left".to_owned(),
        1 => "1 day left".to_owned(),
        days => format!("{days} days left"),
    }
}

/// Accepts an account number with or without separators; Mullvad accounts are sixteen digits.
pub fn normalize_account(input: &str) -> Result<String, &'static str> {
    let digits: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    if digits.len() == 16 && digits.chars().all(|c| c.is_ascii_digit()) {
        Ok(digits)
    } else {
        Err("An account number is sixteen digits")
    }
}

fn severity_color(severity: Severity) -> Color {
    match severity {
        Severity::Info => Color::Accent,
        Severity::Success => Color::Success,
        Severity::Warning => Color::Warning,
        Severity::Error => Color::Error,
    }
}

fn selection_label(selection: &LocationSelection, countries: &[Country]) -> String {
    match selection {
        LocationSelection::Any => "Any location".to_owned(),
        LocationSelection::Country(code) => countries
            .iter()
            .find(|country| &country.code == code)
            .map_or_else(|| code.clone(), |country| country.name.clone()),
        LocationSelection::City(code, city_code) => {
            let country = countries.iter().find(|country| &country.code == code);
            let city = country.and_then(|country| {
                country
                    .cities
                    .iter()
                    .find(|city| &city.code == city_code)
                    .map(|city| (country.name.as_str(), city.name.as_str()))
            });
            match city {
                Some((country, city)) => format!("{city}, {country}"),
                None => format!("{city_code}, {code}"),
            }
        }
        LocationSelection::Other(label) => label.clone(),
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

/// The VPN screen: live tunnel state, connect control, location, account and settings.
pub struct VpnView {
    pub(crate) backend: Arc<dyn VpnBackend>,
    pub(crate) snapshot: Option<VpnSnapshot>,
    pub(crate) countries: Vec<Country>,
    pub(crate) daemon_error: Option<SharedString>,
    pub(crate) action_error: Option<SharedString>,
    pub(crate) locations_open: bool,
    pub(crate) expanded_country: Option<String>,
    pub(crate) busy: bool,
    account_input: Option<Entity<InputField>>,
    login_error: Option<SharedString>,
    focus_handle: FocusHandle,
    _poll: Task<()>,
}

impl VpnView {
    pub fn new(backend: Arc<dyn VpnBackend>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // The text editor registers its factory when the editor crate starts; a bare test app has none.
        let account_input = gpui_ui::ui_input::ERASED_EDITOR_FACTORY
            .get()
            .map(|_| cx.new(|cx| InputField::new(window, cx, "Account number").masked(true)));
        let poll = cx.spawn(async move |this, cx| {
            loop {
                let Ok(task) = this.update(cx, |this, cx| this.backend.snapshot(cx)) else {
                    break;
                };
                let result = task.await;
                if this
                    .update(cx, |this, cx| this.apply_snapshot(result, cx))
                    .is_err()
                {
                    break;
                }
                cx.background_executor().timer(POLL_INTERVAL).await;
            }
        });
        let mut view = Self {
            backend,
            snapshot: None,
            countries: Vec::new(),
            daemon_error: None,
            action_error: None,
            locations_open: false,
            expanded_country: None,
            busy: false,
            account_input,
            login_error: None,
            focus_handle: cx.focus_handle(),
            _poll: poll,
        };
        view.load_locations(cx);
        view
    }

    pub fn snapshot(&self) -> Option<&VpnSnapshot> {
        self.snapshot.as_ref()
    }

    /// Why the daemon could not be read, when it could not.
    pub fn daemon_error(&self) -> Option<&SharedString> {
        self.daemon_error.as_ref()
    }

    pub fn show_locations(&mut self, open: bool, cx: &mut Context<Self>) {
        self.locations_open = open;
        cx.notify();
    }

    pub(crate) fn apply_snapshot(&mut self, result: Result<VpnSnapshot>, cx: &mut Context<Self>) {
        match result {
            Ok(snapshot) => {
                self.daemon_error = None;
                self.snapshot = Some(snapshot);
                if self.countries.is_empty() {
                    self.load_locations(cx);
                }
            }
            Err(error) => {
                self.daemon_error = Some(format!("{error:#}").into());
                self.snapshot = None;
            }
        }
        cx.notify();
    }

    fn load_locations(&mut self, cx: &mut Context<Self>) {
        let task = self.backend.locations(cx);
        cx.spawn(async move |this, cx| match task.await {
            Ok(countries) => {
                this.update(cx, |this, cx| {
                    this.countries = countries;
                    cx.notify();
                })
                .log_err();
            }
            Err(error) => log::warn!("relay locations unavailable: {error:#}"),
        })
        .detach();
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let task = self.backend.snapshot(cx);
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| this.apply_snapshot(result, cx))
                .log_err();
        })
        .detach();
    }

    /// Sends one change to the daemon, then reads the state back; a failure is shown, not dropped.
    pub fn run(&mut self, command: VpnCommand, cx: &mut Context<Self>) {
        self.busy = true;
        self.action_error = None;
        cx.notify();
        let task = self.backend.execute(command, cx);
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.busy = false;
                if let Err(error) = result {
                    this.action_error = Some(format!("{error:#}").into());
                }
                this.refresh(cx);
                cx.notify();
            })
            .log_err();
        })
        .detach();
    }

    pub fn toggle_connection(&mut self, cx: &mut Context<Self>) {
        let action = self
            .snapshot
            .as_ref()
            .map(|snapshot| present(&snapshot.tunnel).action);
        if let Some(command) = action.and_then(PrimaryAction::command) {
            self.run(command, cx);
        }
    }

    pub fn select_location(&mut self, selection: LocationSelection, cx: &mut Context<Self>) {
        self.locations_open = false;
        self.run(VpnCommand::SelectLocation(selection), cx);
    }

    pub fn set_toggle(&mut self, toggle: Toggle, enabled: bool, cx: &mut Context<Self>) {
        self.run(VpnCommand::Set(toggle, enabled), cx);
    }

    /// Validates and sends an account number; the number is never kept or logged.
    pub fn login(&mut self, account: &str, cx: &mut Context<Self>) {
        match normalize_account(account) {
            Ok(account) => {
                self.login_error = None;
                self.run(VpnCommand::Login(account), cx);
            }
            Err(message) => {
                self.login_error = Some(message.into());
                cx.notify();
            }
        }
    }

    fn submit_login(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.account_input.clone() else {
            return;
        };
        let account = input.read(cx).text(cx);
        input.update(cx, |input, cx| {
            input.set_error(None::<SharedString>, cx);
        });
        self.login(&account, cx);
        if self.login_error.is_none() {
            let editor = input.read(cx).editor().clone();
            editor.clear(window, cx);
        }
    }

    fn render_error(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let message = self.action_error.clone()?;
        Some(
            Callout::new()
                .severity(Severity::Error)
                .icon(IconName::XCircle)
                .title("The VPN daemon refused the change")
                .description(message)
                .dismiss_action(
                    IconButton::new("vpn-dismiss-error", IconName::Close).on_click(cx.listener(
                        |this, _: &ClickEvent, _window, cx| {
                            this.action_error = None;
                            cx.notify();
                        },
                    )),
                )
                .into_any_element(),
        )
    }

    fn render_status(&self, snapshot: &VpnSnapshot, cx: &mut Context<Self>) -> impl IntoElement {
        let presentation = present(&snapshot.tunnel);
        let color = severity_color(presentation.severity);
        let exit = snapshot.exit.clone();
        let action = presentation.action;
        let connect_style = match action {
            PrimaryAction::Connect => ButtonStyle::Tinted(TintColor::Success),
            PrimaryAction::Cancel | PrimaryAction::Disconnect => {
                ButtonStyle::Tinted(TintColor::Error)
            }
            PrimaryAction::Wait => ButtonStyle::Subtle,
        };
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Indicator::dot().color(color))
                    .child(
                        Label::new(presentation.headline)
                            .size(LabelSize::Large)
                            .color(color),
                    ),
            )
            .when_some(exit, |this, exit| {
                this.child(
                    v_flex()
                        .child(Label::new(match &exit.city {
                            Some(city) => format!("{city}, {}", exit.country),
                            None => exit.country.clone(),
                        }))
                        .when_some(exit.hostname.clone(), |this, hostname| {
                            this.child(
                                Label::new(hostname)
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            )
                        })
                        .when_some(exit.address.clone(), |this, address| {
                            this.child(
                                Label::new(format!("Out IP: {address}"))
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            )
                        }),
                )
            })
            .child(
                Button::new("vpn-primary", action.label())
                    .full_width()
                    .style(connect_style)
                    .disabled(action == PrimaryAction::Wait || self.busy)
                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                        this.toggle_connection(cx);
                    })),
            )
    }

    fn render_location(&self, snapshot: &VpnSnapshot, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.locations_open;
        let current = selection_label(&snapshot.selection, &self.countries);
        let selectable = !matches!(snapshot.selection, LocationSelection::Other(_));
        v_flex()
            .gap_1()
            .child(
                Label::new("Location")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
            .child(
                Button::new("vpn-location", current)
                    .full_width()
                    .style(ButtonStyle::Outlined)
                    .end_icon(Icon::new(if open {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    }))
                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                        this.locations_open = !this.locations_open;
                        cx.notify();
                    })),
            )
            .when(open, |this| {
                let mut list = v_flex().gap_0p5().pl_2().child(
                    Button::new("vpn-location-any", "Any location")
                        .full_width()
                        .style(ButtonStyle::Subtle)
                        .toggle_state(snapshot.selection == LocationSelection::Any && selectable)
                        .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                            this.select_location(LocationSelection::Any, cx);
                        })),
                );
                for country in &self.countries {
                    list = list.child(self.render_country(country, &snapshot.selection, cx));
                }
                this.child(list)
            })
    }

    fn render_country(
        &self,
        country: &Country,
        selection: &LocationSelection,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let expanded = self.expanded_country.as_deref() == Some(country.code.as_str());
        let code = country.code.clone();
        let select_code = country.code.clone();
        v_flex()
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        IconButton::new(
                            SharedString::from(format!("vpn-expand-{}", country.code)),
                            if expanded {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            },
                        )
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _window, cx| {
                                this.expanded_country =
                                    if this.expanded_country.as_deref() == Some(&code) {
                                        None
                                    } else {
                                        Some(code.clone())
                                    };
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        Button::new(
                            SharedString::from(format!("vpn-country-{}", country.code)),
                            country.name.clone(),
                        )
                        .full_width()
                        .style(ButtonStyle::Subtle)
                        .toggle_state(
                            *selection == LocationSelection::Country(country.code.clone()),
                        )
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _window, cx| {
                                this.select_location(
                                    LocationSelection::Country(select_code.clone()),
                                    cx,
                                );
                            },
                        )),
                    ),
            )
            .when(expanded, |this| {
                this.child(v_flex().pl_6().children(country.cities.iter().map(|city| {
                    let target = LocationSelection::City(country.code.clone(), city.code.clone());
                    let selected = *selection == target;
                    Button::new(
                        SharedString::from(format!("vpn-city-{}-{}", country.code, city.code)),
                        format!("{} ({})", city.name, city.relay_count),
                    )
                    .full_width()
                    .style(ButtonStyle::Subtle)
                    .toggle_state(selected)
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _window, cx| {
                            this.select_location(target.clone(), cx);
                        },
                    ))
                })))
            })
    }

    fn render_account(&self, snapshot: &VpnSnapshot, cx: &mut Context<Self>) -> impl IntoElement {
        let section = v_flex().gap_1().child(
            Label::new("Account")
                .size(LabelSize::Small)
                .color(Color::Muted),
        );
        match &snapshot.account {
            AccountStatus::LoggedIn {
                device,
                expiry_unix,
            } => section
                .child(Label::new(format!("Device: {device}")))
                .when_some(*expiry_unix, |this, expiry| {
                    let text = time_left(expiry, now_unix());
                    let color = if expiry <= now_unix() {
                        Color::Error
                    } else {
                        Color::Muted
                    };
                    this.child(Label::new(text).size(LabelSize::Small).color(color))
                })
                .child(
                    Button::new("vpn-logout", "Log out")
                        .style(ButtonStyle::Outlined)
                        .disabled(self.busy)
                        .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                            this.run(VpnCommand::Logout, cx);
                        })),
                )
                .into_any_element(),
            status => section
                .when(matches!(status, AccountStatus::Revoked), |this| {
                    this.child(
                        Label::new("This device was removed from the account. Log in again.")
                            .size(LabelSize::Small)
                            .color(Color::Warning),
                    )
                })
                .when_some(self.account_input.clone(), |this, input| this.child(input))
                .when_some(self.login_error.clone(), |this, message| {
                    this.child(
                        Label::new(message)
                            .size(LabelSize::Small)
                            .color(Color::Error),
                    )
                })
                .child(
                    Button::new("vpn-login", "Log in")
                        .full_width()
                        .style(ButtonStyle::Filled)
                        .disabled(self.busy || self.account_input.is_none())
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.submit_login(window, cx);
                        })),
                )
                .into_any_element(),
        }
    }

    fn render_settings(&self, snapshot: &VpnSnapshot, cx: &mut Context<Self>) -> impl IntoElement {
        let mut column = v_flex().gap_1();
        let mut group = "";
        for toggle in Toggle::ALL {
            if toggle.group() != group {
                group = toggle.group();
                column = column.child(Label::new(group).size(LabelSize::Small).color(Color::Muted));
            }
            let enabled = snapshot.is_enabled(toggle);
            column = column.child(
                Switch::new(
                    SharedString::from(format!("vpn-toggle-{toggle:?}")),
                    if enabled {
                        ToggleState::Selected
                    } else {
                        ToggleState::Unselected
                    },
                )
                .label(toggle.label())
                .full_width(true)
                .disabled(self.busy)
                .on_click(cx.listener(
                    move |this, state: &ToggleState, _window, cx| {
                        this.set_toggle(toggle, *state == ToggleState::Selected, cx);
                    },
                )),
            );
        }
        column
    }
}

impl Focusable for VpnView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for VpnView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.snapshot.clone();
        let error = self.render_error(cx);
        let daemon_error = self.daemon_error.clone();
        v_flex()
            .id("vpn-view")
            .key_context("VpnView")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &menu::Confirm, window, cx| {
                this.submit_login(window, cx);
            }))
            .size_full()
            .overflow_y_scroll()
            .p_3()
            .gap_3()
            .bg(cx.theme().colors().panel_background)
            .children(error)
            .when_some(daemon_error, |this, message| {
                this.child(
                    Callout::new()
                        .severity(Severity::Error)
                        .icon(IconName::Warning)
                        .title("The VPN daemon is not reachable")
                        .description(message),
                )
            })
            .when_some(snapshot, |this, snapshot| {
                this.child(self.render_status(&snapshot, cx))
                    .child(Divider::horizontal())
                    .child(self.render_location(&snapshot, cx))
                    .child(Divider::horizontal())
                    .child(self.render_account(&snapshot, cx))
                    .child(Divider::horizontal())
                    .child(self.render_settings(&snapshot, cx))
            })
    }
}

/// The VPN view as a dock panel.
pub struct VpnPanel {
    view: Entity<VpnView>,
    position: DockPosition,
}

impl VpnPanel {
    pub fn new(view: Entity<VpnView>) -> Self {
        Self {
            view,
            position: DockPosition::Right,
        }
    }
}

impl EventEmitter<PanelEvent> for VpnPanel {}

impl Focusable for VpnPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.view.focus_handle(cx)
    }
}

impl Render for VpnPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.view.clone()
    }
}

impl Panel for VpnPanel {
    fn persistent_name() -> &'static str {
        "VpnPanel"
    }

    fn panel_key() -> &'static str {
        "vpn_panel"
    }

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        self.position
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(position, DockPosition::Left | DockPosition::Right)
    }

    fn set_position(
        &mut self,
        position: DockPosition,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.position = position;
        cx.notify();
    }

    fn default_size(&self, _window: &Window, _cx: &App) -> Pixels {
        px(340.0)
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<IconName> {
        Some(IconName::Lock)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("VPN")
    }

    fn toggle_action(&self) -> Box<dyn Action> {
        ToggleFocus.boxed_clone()
    }

    fn activation_priority(&self) -> u32 {
        20
    }
}

/// The VPN view as a tab in a pane.
pub struct VpnPane {
    view: Entity<VpnView>,
}

impl VpnPane {
    pub fn new(view: Entity<VpnView>) -> Self {
        Self { view }
    }
}

impl EventEmitter<ItemEvent> for VpnPane {}

impl Focusable for VpnPane {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.view.focus_handle(cx)
    }
}

impl Render for VpnPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .justify_center()
            .bg(cx.theme().colors().editor_background)
            .child(
                div()
                    .h_full()
                    .w(px(520.0))
                    .max_w_full()
                    .child(self.view.clone()),
            )
    }
}

impl Item for VpnPane {
    type Event = ItemEvent;

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::Lock))
    }

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        "VPN".into()
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(ItemEvent)) {
        f(*event);
    }
}

pub(crate) fn register_actions(workspace: &mut Workspace, view: Entity<VpnView>) {
    workspace.register_action(|workspace, _: &ToggleFocus, window, cx| {
        workspace.toggle_panel_focus::<VpnPanel>(window, cx);
    });
    workspace.register_action({
        let view = view.clone();
        move |workspace, _: &OpenVpn, window, cx| {
            let pane = cx.new(|_| VpnPane::new(view.clone()));
            workspace.add_item_to_active_pane(Box::new(pane), None, true, window, cx);
        }
    });
    workspace.register_action(move |_, _: &ToggleConnection, _window, cx| {
        view.update(cx, |view, cx| view.toggle_connection(cx));
    });
}
