use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use web_time::Instant;

use crate::action::{
    Action, ActionRequest, ActionResult, InteractionPolicy, execute_verified_action, sleep_brief,
};
use crate::adapter::{CommandContext, InteractionLease, PilotEngine};
use crate::locator::{LocatorQuery, ObservationRequest, find_node_by_ref, resolve_unique};
use crate::model::{ErrorCode, PilotError, SnapshotSurface};
use crate::refs::{stamp_tree_refs, validate_ref_id};

const MAX_TRACE_RING: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionNeed {
    None,
    Accessibility,
    ScreenRecording,
    AccessibilityAndScreenRecording,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WaitSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<LocatorQuery>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_contains: Option<String>,
    #[serde(default)]
    pub gone: bool,
    #[serde(default)]
    pub window_title: Option<String>,
    #[serde(default = "default_wait_timeout_ms")]
    pub timeout_ms: u64,
}

const fn default_wait_timeout_ms() -> u64 {
    5_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitOutcome {
    pub satisfied: bool,
    pub elapsed_ms: u64,
    pub polls: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub seq: u64,
    pub timestamp_ms: u64,
    pub session_id: String,
    pub backend: String,
    pub command: String,
    pub ok: bool,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TraceRecorder {
    enabled: bool,
    next_seq: u64,
    output_path: Option<PathBuf>,
    events: VecDeque<TraceEvent>,
}

impl TraceRecorder {
    pub fn start(&mut self, output_path: Option<PathBuf>) {
        self.enabled = true;
        self.output_path = output_path;
    }

    pub fn stop(&mut self) {
        self.enabled = false;
    }

    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn record(&mut self, mut event: TraceEvent) {
        self.next_seq += 1;
        event.seq = self.next_seq;
        if let Some(path) = &self.output_path
            && self.enabled
            && let Ok(line) = serde_json::to_string(&event)
        {
            if let Some(parent) = path.parent()
                && let Err(err) = std::fs::create_dir_all(parent)
            {
                log::warn!("failed to create pilot trace directory {}: {err}", parent.display());
            }
            use std::io::Write as _;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                && let Err(err) = writeln!(file, "{line}")
            {
                log::warn!("failed to append pilot trace event: {err}");
            }
        }
        if self.events.len() >= MAX_TRACE_RING {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    #[must_use]
    pub fn snapshot(&self, limit: usize) -> Vec<TraceEvent> {
        let len = self.events.len();
        let skip = len.saturating_sub(limit);
        self.events.iter().skip(skip).cloned().collect()
    }

    #[must_use]
    pub fn output_path(&self) -> Option<&Path> {
        self.output_path.as_deref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStep {
    pub op: String,
    #[serde(default)]
    pub args: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStepResult {
    pub index: usize,
    pub op: String,
    pub ok: bool,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<PilotError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchReport {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub stopped_early: bool,
    pub duration_ms: u64,
    pub steps: Vec<BatchStepResult>,
}

/// One-shot AI observe -> resolve -> act -> post-verify -> diff-snapshot transaction that cuts
/// LLM control-loop latency from 3 roundtrips down to 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiActPlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<String>,
    #[serde(default)]
    pub surface: SnapshotSurface,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<LocatorQuery>,
    pub action: Action,
    #[serde(default)]
    pub policy: InteractionPolicy,
    #[serde(default = "default_true")]
    pub include_post_snapshot: bool,
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiActOutcome {
    pub action_result: ActionResult,
    pub resolved_ref: String,
    pub before_snapshot_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_snapshot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_snapshot_text: Option<String>,
    pub ref_count_delta: isize,
}

pub fn wait_for_selector(
    engine: &mut PilotEngine,
    selector: &WaitSelector,
    ctx: &CommandContext,
) -> Result<WaitOutcome, PilotError> {
    let started = Instant::now();
    let deadline = crate::adapter::Deadline::from_timeout_ms(Some(selector.timeout_ms));
    let mut polls = 0_u32;

    loop {
        polls += 1;
        if let Some(title_needle) = &selector.window_title {
            let windows = engine.list_windows(deadline)?;
            let found = windows.iter().any(|w| {
                w.title
                    .to_ascii_lowercase()
                    .contains(&title_needle.to_ascii_lowercase())
            });
            if found != selector.gone {
                return Ok(WaitOutcome {
                    satisfied: true,
                    elapsed_ms: started.elapsed().as_millis() as u64,
                    polls,
                    snapshot_id: None,
                    matched_ref: None,
                });
            }
        } else {
            let req = ObservationRequest {
                app: ctx.app.clone(),
                window_id: ctx.window_id.clone(),
                ..Default::default()
            };
            let tree = engine.snapshot(&req, ctx)?;
            let matched = evaluate_wait_match(&tree.root, selector);
            let satisfied = if selector.gone {
                matched.is_none()
            } else {
                matched.is_some()
            };
            if satisfied {
                return Ok(WaitOutcome {
                    satisfied: true,
                    elapsed_ms: started.elapsed().as_millis() as u64,
                    polls,
                    snapshot_id: Some(tree.snapshot_id),
                    matched_ref: matched,
                });
            }
        }

        if deadline.is_expired() {
            return Err(PilotError::new(
                ErrorCode::Timeout,
                format!(
                    "wait condition timed out after {} ms ({polls} polls)",
                    selector.timeout_ms
                ),
            ));
        }
        sleep_brief(Duration::from_millis(40));
    }
}

fn evaluate_wait_match(
    root: &crate::model::AccessibilityNode,
    selector: &WaitSelector,
) -> Option<String> {
    if let Some(ref_id) = &selector.ref_id
        && let Some(node) = find_node_by_ref(root, ref_id)
    {
        if node_satisfies_wait(node, selector) {
            return node.ref_id.clone().or_else(|| Some(node.id.clone()));
        }
        return None;
    }
    if let Some(query) = &selector.query {
        if let Ok(found) = resolve_unique(root, query) {
            return found.ref_id.or(Some(found.node_id));
        }
        return None;
    }
    if let Some(text) = &selector.text {
        let query = LocatorQuery {
            identity: crate::locator::IdentityPredicate {
                name_contains: Some(text.clone()),
                ..Default::default()
            },
            ..Default::default()
        };
        let res = crate::locator::evaluate_locator_tree(root, &query);
        if let Some(first) = res.matches.first() {
            return first.ref_id.clone().or_else(|| Some(first.node_id.clone()));
        }
        return None;
    }
    if let Some(state_name) = &selector.state {
        return find_first_matching_state(root, state_name);
    }
    None
}

fn find_first_matching_state(
    node: &crate::model::AccessibilityNode,
    state_name: &str,
) -> Option<String> {
    if node.state.has_state(state_name) {
        return node.ref_id.clone().or_else(|| Some(node.id.clone()));
    }
    for child in &node.children {
        if let Some(found) = find_first_matching_state(child, state_name) {
            return Some(found);
        }
    }
    None
}

fn node_satisfies_wait(node: &crate::model::AccessibilityNode, selector: &WaitSelector) -> bool {
    if let Some(state) = &selector.state
        && !node.state.has_state(state)
    {
        return false;
    }
    if let Some(val) = &selector.value_contains {
        let hay = node
            .value
            .as_deref()
            .or(node.state.value.as_deref())
            .unwrap_or("");
        if !hay.contains(val.as_str()) {
            return false;
        }
    }
    true
}

pub fn ai_act_verify(
    engine: &mut PilotEngine,
    plan: &AiActPlan,
    ctx: &CommandContext,
) -> Result<AiActOutcome, PilotError> {
    let obs_req = ObservationRequest {
        surface: plan.surface,
        app: plan.app.clone().or_else(|| ctx.app.clone()),
        window_id: plan.window_id.clone().or_else(|| ctx.window_id.clone()),
        include_bounds: true,
        ..Default::default()
    };
    let before_tree = engine.snapshot(&obs_req, ctx)?;
    let before_count = before_tree.ref_count as isize;

    let target_ref = if let Some(ref_id) = &plan.target_ref {
        validate_ref_id(ref_id)?
    } else if let Some(query) = &plan.locator {
        let matched = resolve_unique(&before_tree.root, query)?;
        matched.ref_id.ok_or_else(|| {
            PilotError::new(
                ErrorCode::ElementNotFound,
                "matched node had no ref id assigned",
            )
        })?
    } else {
        return Err(PilotError::invalid_input(
            "ai_act_verify requires either `target_ref` or `locator`",
        ));
    };

    let deadline = crate::adapter::Deadline::from_timeout_ms(ctx.timeout_ms);
    let handle = engine.resolve_saved_ref(&target_ref, deadline)?;
    let lease = InteractionLease::new(deadline, plan.policy);
    let req = ActionRequest {
        action: plan.action.clone(),
        policy: plan.policy,
    };
    let action_result = {
        let adapter = engine.active_adapter();
        execute_verified_action(adapter, &handle, Some(&target_ref), req, &lease)?
    };
    engine.record_action_event(&action_result);

    if !plan.include_post_snapshot {
        return Ok(AiActOutcome {
            action_result,
            resolved_ref: target_ref,
            before_snapshot_id: before_tree.snapshot_id,
            after_snapshot_id: None,
            after_snapshot_text: None,
            ref_count_delta: 0,
        });
    }

    let after_tree = engine.snapshot(&obs_req, ctx)?;
    let (_, after_map) = stamp_tree_refs(
        &after_tree.snapshot_id,
        None,
        after_tree.root.clone(),
        after_tree.stats.duration_ms,
    );
    let delta = after_map.len() as isize - before_count;
    Ok(AiActOutcome {
        action_result,
        resolved_ref: target_ref,
        before_snapshot_id: before_tree.snapshot_id,
        after_snapshot_id: Some(after_tree.snapshot_id),
        after_snapshot_text: Some(after_tree.text),
        ref_count_delta: delta,
    })
}

pub fn execute_batch(
    engine: &mut PilotEngine,
    steps: &[BatchStep],
    stop_on_error: bool,
    ctx: &CommandContext,
) -> BatchReport {
    let started = Instant::now();
    let mut results = Vec::with_capacity(steps.len());
    let mut succeeded = 0_usize;
    let mut failed = 0_usize;
    let mut stopped_early = false;

    for (index, step) in steps.iter().enumerate() {
        let step_start = Instant::now();
        match engine.execute_tool(&step.op, &step.args, ctx) {
            Ok(data) => {
                succeeded += 1;
                results.push(BatchStepResult {
                    index,
                    op: step.op.clone(),
                    ok: true,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                    data: Some(data),
                    error: None,
                });
            }
            Err(err) => {
                failed += 1;
                results.push(BatchStepResult {
                    index,
                    op: step.op.clone(),
                    ok: false,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                    data: None,
                    error: Some(err),
                });
                if stop_on_error {
                    stopped_early = true;
                    break;
                }
            }
        }
    }

    BatchReport {
        total: steps.len(),
        succeeded,
        failed,
        stopped_early,
        duration_ms: started.elapsed().as_millis() as u64,
        steps: results,
    }
}

#[must_use]
pub fn status_json(engine: &mut PilotEngine) -> Value {
    match engine.status() {
        Ok(report) => json!(report),
        Err(err) => json!({ "error": err }),
    }
}
