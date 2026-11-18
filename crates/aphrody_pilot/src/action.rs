use std::time::Duration;

use serde::{Deserialize, Serialize};
use web_time::Instant;

use crate::adapter::{Deadline, InteractionLease, LiveElement, NativeHandle, PlatformAdapter};
use crate::model::{DeliverySemantics, ElementState, ErrorCode, PilotError, RetryDisposition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Shift,
    Control,
    Alt,
    Meta,
}

impl Modifier {
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "shift" => Some(Self::Shift),
            "ctrl" | "control" => Some(Self::Control),
            "alt" | "option" => Some(Self::Alt),
            "cmd" | "command" | "super" | "meta" | "win" | "windows" => Some(Self::Meta),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyCombo {
    pub key: String,
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
}

impl KeyCombo {
    pub fn parse(raw: &str) -> Result<Self, PilotError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(PilotError::invalid_input("empty key combo"));
        }
        let parts: Vec<&str> = trimmed.split('+').map(str::trim).collect();
        let Some((last, mods)) = parts.split_last() else {
            return Err(PilotError::invalid_input("empty key combo"));
        };
        if last.is_empty() {
            return Err(PilotError::invalid_input(format!(
                "missing primary key in combo '{raw}'"
            )));
        }
        let mut modifiers = Vec::with_capacity(mods.len());
        for m in mods {
            let parsed = Modifier::parse(m).ok_or_else(|| {
                PilotError::invalid_input(format!("unknown modifier '{m}' in combo '{raw}'"))
            })?;
            if !modifiers.contains(&parsed) {
                modifiers.push(parsed);
            }
        }
        Ok(Self {
            key: (*last).to_owned(),
            modifiers,
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}

impl MouseButton {
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "middle" => Some(Self::Middle),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseEventKind {
    Move,
    Click,
    Down,
    Up,
    Wheel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub button: MouseButton,
    #[serde(default = "default_click_count")]
    pub click_count: u8,
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
    #[serde(default)]
    pub wheel_dx: i32,
    #[serde(default)]
    pub wheel_dy: i32,
}

const fn default_click_count() -> u8 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DragParams {
    pub from_x: f64,
    pub from_y: f64,
    pub to_x: f64,
    pub to_y: f64,
    #[serde(default)]
    pub button: MouseButton,
    #[serde(default = "default_drag_steps")]
    pub steps: u16,
    #[serde(default = "default_drag_duration_ms")]
    pub duration_ms: u64,
}

const fn default_drag_steps() -> u16 {
    12
}

const fn default_drag_duration_ms() -> u64 {
    240
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "arg", rename_all = "snake_case")]
pub enum Action {
    Click,
    DoubleClick,
    TripleClick,
    RightClick,
    TypeText(String),
    SetValue(String),
    Clear,
    Focus,
    Select(String),
    Toggle,
    Check,
    Uncheck,
    Expand,
    Collapse,
    Scroll { direction: Direction, amount: i32 },
    ScrollTo,
}

impl Action {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::DoubleClick => "double_click",
            Self::TripleClick => "triple_click",
            Self::RightClick => "right_click",
            Self::TypeText(_) => "type",
            Self::SetValue(_) => "set_value",
            Self::Clear => "clear",
            Self::Focus => "focus",
            Self::Select(_) => "select",
            Self::Toggle => "toggle",
            Self::Check => "check",
            Self::Uncheck => "uncheck",
            Self::Expand => "expand",
            Self::Collapse => "collapse",
            Self::Scroll { .. } => "scroll",
            Self::ScrollTo => "scroll_to",
        }
    }

    #[must_use]
    pub const fn requires_state_readback(&self) -> bool {
        matches!(
            self,
            Self::TypeText(_)
                | Self::SetValue(_)
                | Self::Clear
                | Self::Toggle
                | Self::Check
                | Self::Uncheck
                | Self::Expand
                | Self::Collapse
                | Self::Select(_)
        )
    }

    #[must_use]
    pub const fn writes_element_value(&self) -> bool {
        matches!(self, Self::TypeText(_) | Self::SetValue(_) | Self::Clear)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionPolicy {
    Fast,
    #[default]
    Verified,
    Headed,
}

impl InteractionPolicy {
    #[must_use]
    pub const fn is_headed(self) -> bool {
        matches!(self, Self::Headed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRequest {
    pub action: Action,
    #[serde(default)]
    pub policy: InteractionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStepOutcome {
    pub step: String,
    pub mechanism: String,
    pub ok: bool,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
    pub disposition: DeliverySemantics,
    pub verified: bool,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_state: Option<ElementState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_state: Option<ElementState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<ActionStepOutcome>,
}

impl ActionResult {
    #[must_use]
    pub const fn retry_disposition(&self) -> RetryDisposition {
        self.disposition.retry_disposition()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostconditionVerdict {
    Satisfied,
    Mismatch(&'static str),
    InsufficientEvidence(&'static str),
}

fn normalize_ws_value(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn state_value(state: &ElementState) -> Option<&str> {
    state.value.as_deref()
}

fn checked_state(state: &ElementState) -> Option<bool> {
    if state.has_state("indeterminate") {
        return None;
    }
    state
        .checked
        .or_else(|| {
            if state.has_state("checked") {
                Some(true)
            } else if !state.states.is_empty() {
                Some(false)
            } else {
                None
            }
        })
}

fn secure_value_redacted(action: &Action, observed: &ElementState) -> bool {
    action.writes_element_value()
        && observed.value.is_none()
        && observed.has_state("secure")
}

#[must_use]
pub fn evaluate_postcondition(
    action: &Action,
    before: Option<&ElementState>,
    after: &LiveElement,
) -> PostconditionVerdict {
    let state = &after.state;
    if secure_value_redacted(action, state) {
        return PostconditionVerdict::InsufficientEvidence("secure_field");
    }
    match action {
        Action::Check | Action::Uncheck => match checked_state(state) {
            Some(is_checked) if is_checked == matches!(action, Action::Check) => {
                PostconditionVerdict::Satisfied
            }
            Some(_) => PostconditionVerdict::Mismatch("checked_state_mismatch"),
            None => PostconditionVerdict::InsufficientEvidence("indeterminate_or_missing_checked"),
        },
        Action::Toggle => {
            let prev = before.and_then(checked_state);
            let next = checked_state(state);
            match (prev, next) {
                (Some(p), Some(n)) if p != n => PostconditionVerdict::Satisfied,
                (Some(_), Some(_)) => PostconditionVerdict::Mismatch("toggle_unchanged"),
                _ => PostconditionVerdict::InsufficientEvidence("missing_toggle_baseline"),
            }
        }
        Action::Expand | Action::Collapse => {
            let wants_expanded = matches!(action, Action::Expand);
            let is_expanded = state.expanded.unwrap_or_else(|| state.has_state("expanded"));
            if state.expanded.is_none() && state.states.is_empty() {
                PostconditionVerdict::InsufficientEvidence("missing_expanded_state")
            } else if is_expanded == wants_expanded {
                PostconditionVerdict::Satisfied
            } else {
                PostconditionVerdict::Mismatch("expanded_state_mismatch")
            }
        }
        Action::Clear => match state_value(state) {
            Some("") => PostconditionVerdict::Satisfied,
            Some(_) => PostconditionVerdict::Mismatch("value_not_cleared"),
            None => PostconditionVerdict::InsufficientEvidence("missing_value_readback"),
        },
        Action::SetValue(expected) => match state_value(state) {
            Some(actual)
                if actual == expected.as_str()
                    || normalize_ws_value(actual) == normalize_ws_value(expected) =>
            {
                PostconditionVerdict::Satisfied
            }
            Some(_) => PostconditionVerdict::Mismatch("value_mismatch"),
            None => PostconditionVerdict::InsufficientEvidence("missing_value_readback"),
        },
        Action::TypeText(delta) => match state_value(state) {
            Some(actual)
                if actual.contains(delta.as_str())
                    || normalize_ws_value(actual).contains(&normalize_ws_value(delta)) =>
            {
                PostconditionVerdict::Satisfied
            }
            Some(_) => PostconditionVerdict::Mismatch("typed_text_absent"),
            None => PostconditionVerdict::InsufficientEvidence("missing_value_readback"),
        },
        Action::Select(option) => {
            if state.selected == Some(true)
                || state.has_state("selected")
                || state_value(state).is_some_and(|val| {
                    val == option.as_str()
                        || normalize_ws_value(val) == normalize_ws_value(option)
                })
            {
                PostconditionVerdict::Satisfied
            } else {
                PostconditionVerdict::Mismatch("selection_mismatch")
            }
        }
        _ => PostconditionVerdict::Satisfied,
    }
}

fn typed_by_synthetic_keys(action: &Action, result: &ActionResult) -> bool {
    matches!(action, Action::TypeText(_))
        && result.steps.iter().any(|step| {
            step.ok
                && (step.mechanism.contains("physical")
                    || step.mechanism.contains("synthetic")
                    || step.mechanism.contains("send_input"))
        })
}

/// Executes an action once, then verifies stateful postconditions (`checked`, `expanded`,
/// `value`, `selected`) via bounded 50 ms readback polling (up to 400 ms settle window,
/// extending while synthetic text drains) against live platform observations.
pub fn execute_verified_action(
    adapter: &dyn PlatformAdapter,
    handle: &NativeHandle,
    target_ref: Option<&str>,
    mut request: ActionRequest,
    lease: &InteractionLease,
) -> Result<ActionResult, PilotError> {
    let started = Instant::now();
    let original_action = request.action.clone();
    let deadline = lease.deadline();
    let before = adapter
        .get_live_element(handle, deadline)
        .ok()
        .map(|el| el.state);

    if let Some(state) = &before
        && state.has_state("disabled")
    {
        return Err(PilotError::new(
            ErrorCode::ElementDisabled,
            format!(
                "cannot execute '{}' on a disabled element",
                original_action.name()
            ),
        )
        .with_disposition(DeliverySemantics::not_delivered()));
    }

    if matches!(original_action, Action::Toggle)
        && let Some(state) = &before
        && let Some(checked) = checked_state(state)
    {
        request.action = if checked {
            Action::Uncheck
        } else {
            Action::Check
        };
    }

    let mut result = adapter.execute_action(handle, &request, lease)?;
    result.action = original_action.name().to_owned();
    result.target_ref = target_ref.map(str::to_owned);
    result.before_state = before.clone();

    if !original_action.requires_state_readback() || request.policy == InteractionPolicy::Fast {
        result.duration_ms = started.elapsed().as_millis() as u64;
        return Ok(result);
    }

    let settle = Duration::from_millis(50);
    let settle_deadline = deadline.capped(Duration::from_millis(400));
    let drains = typed_by_synthetic_keys(&original_action, &result);
    let mut after_live = adapter.get_live_element(handle, deadline).ok();
    let mut changing = true;

    while let Some(obs) = &after_live {
        let verdict = evaluate_postcondition(&original_action, before.as_ref(), obs);
        if matches!(
            verdict,
            PostconditionVerdict::Satisfied | PostconditionVerdict::InsufficientEvidence(_)
        ) {
            break;
        }
        if settle_deadline.remaining() <= settle && !(drains && changing && deadline.remaining() > settle) {
            break;
        }
        sleep_brief(settle);
        let next = adapter.get_live_element(handle, deadline).ok();
        let next_val = next.as_ref().and_then(|el| el.state.value.as_deref());
        let prev_val = after_live.as_ref().and_then(|el| el.state.value.as_deref());
        changing = next_val != prev_val;
        after_live = next;
    }

    result.after_state = after_live.as_ref().map(|el| el.state.clone());
    let (scope, reason) = match &after_live {
        Some(obs) => match evaluate_postcondition(&original_action, before.as_ref(), obs) {
            PostconditionVerdict::Satisfied => {
                result.verified = true;
                result.disposition = DeliverySemantics::delivered_verified();
                let scope = if original_action.writes_element_value() {
                    "element_value"
                } else {
                    "element_state"
                };
                (scope, "postcondition_satisfied")
            }
            PostconditionVerdict::InsufficientEvidence(why) => {
                result.verified = false;
                result.disposition = DeliverySemantics::delivered_unverified();
                ("unavailable", why)
            }
            PostconditionVerdict::Mismatch(why) => {
                result.verified = false;
                result.disposition = DeliverySemantics::delivered_unverified();
                ("element_state", why)
            }
        },
        None => {
            result.verified = false;
            result.disposition = DeliverySemantics::delivered_unverified();
            ("unavailable", "readback_unavailable")
        }
    };
    result.steps.push(ActionStepOutcome {
        step: "post_action_verification".to_owned(),
        mechanism: "live_readback".to_owned(),
        ok: result.verified,
        duration_ms: started.elapsed().as_millis() as u64,
        detail: Some(format!(
            "verification_scope={scope}; verification_reason={reason}; retry={:?}",
            result.retry_disposition()
        )),
    });
    result.duration_ms = started.elapsed().as_millis() as u64;
    Ok(result)
}

#[must_use]
pub fn postcondition_satisfied(
    action: &Action,
    before: Option<&ElementState>,
    after: &ElementState,
) -> bool {
    match action {
        Action::Check => after.has_state("checked") || after.checked == Some(true),
        Action::Uncheck => !after.has_state("checked") && after.checked != Some(true),
        Action::Toggle => match before.map(|b| b.checked.unwrap_or_else(|| b.has_state("checked")))
        {
            Some(prev) => (after.checked.unwrap_or_else(|| after.has_state("checked"))) != prev,
            None => true,
        },
        Action::Expand => after.has_state("expanded") || after.expanded == Some(true),
        Action::Collapse => !after.has_state("expanded") && after.expanded != Some(true),
        Action::Clear => after.value.as_deref().unwrap_or("").is_empty(),
        Action::SetValue(expected) => after.value.as_deref() == Some(expected.as_str()),
        Action::TypeText(delta) => after
            .value
            .as_deref()
            .is_some_and(|v| v.contains(delta.as_str())),
        Action::Select(option) => {
            after.has_state("selected") || after.value.as_deref() == Some(option.as_str())
        }
        _ => true,
    }
}

pub fn sleep_brief(dur: Duration) {
    #[cfg(not(target_family = "wasm"))]
    std::thread::sleep(dur);
    #[cfg(target_family = "wasm")]
    let _ = dur;
}

#[must_use]
pub fn validate_deadline(timeout_ms: Option<u64>) -> Deadline {
    Deadline::from_timeout_ms(timeout_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Rect;

    fn live_with(state: ElementState) -> LiveElement {
        LiveElement {
            id: "n1".to_owned(),
            role: "textfield".to_owned(),
            name: Some("Test".to_owned()),
            bounds: Some(Rect::new(0.0, 0.0, 100.0, 24.0)),
            state,
            actions: Vec::new(),
        }
    }

    #[test]
    fn postcondition_readback_handles_secure_fields_whitespace_and_toggle() {
        let mut secure_state = ElementState::default();
        secure_state.push_flag("secure");
        let secure_el = live_with(secure_state);
        assert_eq!(
            evaluate_postcondition(&Action::SetValue("secret".to_owned()), None, &secure_el),
            PostconditionVerdict::InsufficientEvidence("secure_field")
        );

        let spaced = live_with(ElementState {
            value: Some("hello   world".to_owned()),
            ..Default::default()
        });
        assert_eq!(
            evaluate_postcondition(&Action::SetValue("hello world".to_owned()), None, &spaced),
            PostconditionVerdict::Satisfied
        );

        let before_toggle = ElementState {
            checked: Some(false),
            ..Default::default()
        };
        let after_toggle = live_with(ElementState {
            checked: Some(true),
            ..Default::default()
        });
        assert_eq!(
            evaluate_postcondition(&Action::Toggle, Some(&before_toggle), &after_toggle),
            PostconditionVerdict::Satisfied
        );
        assert_eq!(
            ActionResult {
                action: "toggle".to_owned(),
                target_ref: None,
                disposition: DeliverySemantics::delivered_verified(),
                verified: true,
                duration_ms: 1,
                before_state: None,
                after_state: None,
                steps: Vec::new(),
            }
            .retry_disposition(),
            RetryDisposition::Unsafe
        );
    }
}
