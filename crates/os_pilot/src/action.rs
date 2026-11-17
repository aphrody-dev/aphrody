use std::time::Duration;

use serde::{Deserialize, Serialize};
use web_time::Instant;

use crate::adapter::{Deadline, InteractionLease, NativeHandle, PlatformAdapter};
use crate::model::{DeliverySemantics, ElementState, ErrorCode, PilotError};

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

/// Executes an action once, then verifies stateful postconditions (`checked`, `expanded`,
/// `value`, `selected`) via bounded 50 ms polling against live platform observations.
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
        && let Some(checked) = state.checked.or_else(|| Some(state.has_state("checked")))
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
    let settle_deadline = deadline.capped(Duration::from_millis(350));
    let mut after = adapter
        .get_live_element(handle, deadline)
        .ok()
        .map(|e| e.state);
    while let Some(obs) = &after {
        if postcondition_satisfied(&original_action, before.as_ref(), obs) {
            result.verified = true;
            result.disposition = DeliverySemantics::delivered_verified();
            result.after_state = Some(obs.clone());
            result.duration_ms = started.elapsed().as_millis() as u64;
            return Ok(result);
        }
        if settle_deadline.remaining() <= settle {
            break;
        }
        sleep_brief(settle);
        after = adapter
            .get_live_element(handle, deadline)
            .ok()
            .map(|e| e.state);
    }

    result.after_state = after.clone();
    if let Some(obs) = &after
        && postcondition_satisfied(&original_action, before.as_ref(), obs)
    {
        result.verified = true;
        result.disposition = DeliverySemantics::delivered_verified();
    } else {
        result.verified = false;
        result.disposition = DeliverySemantics::delivered_unverified();
    }
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
