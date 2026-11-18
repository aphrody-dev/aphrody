use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use web_time::Instant;

use crate::action::{Action, ActionRequest, ActionResult, InteractionPolicy, execute_verified_action, sleep_brief};
use crate::adapter::{CommandContext, Deadline, InteractionLease, PilotEngine};
use crate::browser::{SnapshotDiff, diff_snapshot_text};
use crate::locator::{LocatorQuery, ObservationRequest, find_node_by_ref, resolve_unique};
use crate::model::{DeliverySemantics, ErrorCode, PilotError, RetryDisposition, SnapshotSurface};
use crate::refs::{stamp_tree_refs, validate_ref_id};

const MAX_TRACE_RING: usize = 512;
pub const MAX_BATCH_ENTRIES: usize = 256;
pub const MAX_BATCH_OUTPUT_BYTES: usize = 1024 * 1024;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchExecutionState {
    #[default]
    Completed,
    NotStarted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStepResult {
    pub index: usize,
    pub op: String,
    pub ok: bool,
    pub duration_ms: u64,
    #[serde(default)]
    pub execution: BatchExecutionState,
    #[serde(default)]
    pub retry_disposition: RetryDisposition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<PilotError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchReport {
    pub total: usize,
    #[serde(default)]
    pub completed: usize,
    pub succeeded: usize,
    pub failed: usize,
    #[serde(default)]
    pub not_started: usize,
    pub stopped_early: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_index: Option<usize>,
    #[serde(default)]
    pub max_output_bytes: usize,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_snapshot_diff: Option<SnapshotDiff>,
    #[serde(default)]
    pub retry_disposition: RetryDisposition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_reason: Option<String>,
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

fn extract_verification_tag(result: &ActionResult, key: &str) -> Option<String> {
    result.steps.iter().rev().find_map(|step| {
        let detail = step.detail.as_deref()?;
        detail.split(';').find_map(|segment| {
            let trimmed = segment.trim();
            trimmed
                .strip_prefix(&format!("{key}="))
                .map(str::to_owned)
        })
    })
}

pub fn act_verify(
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
            "act_verify requires either `target_ref` or `locator`",
        ));
    };

    let deadline = Deadline::from_timeout_ms(ctx.timeout_ms);
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

    let retry_disposition = action_result.retry_disposition();
    let verification_scope = extract_verification_tag(&action_result, "verification_scope");
    let verification_reason = extract_verification_tag(&action_result, "verification_reason");

    if !plan.include_post_snapshot {
        return Ok(AiActOutcome {
            action_result,
            resolved_ref: target_ref,
            before_snapshot_id: before_tree.snapshot_id,
            after_snapshot_id: None,
            after_snapshot_text: None,
            after_snapshot_diff: None,
            retry_disposition,
            verification_scope,
            verification_reason,
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
    let diff = diff_snapshot_text(&before_tree.text, &after_tree.text);
    Ok(AiActOutcome {
        action_result,
        resolved_ref: target_ref,
        before_snapshot_id: before_tree.snapshot_id,
        after_snapshot_id: Some(after_tree.snapshot_id),
        after_snapshot_text: Some(after_tree.text),
        after_snapshot_diff: Some(diff),
        retry_disposition,
        verification_scope,
        verification_reason,
        ref_count_delta: delta,
    })
}

pub fn ai_act_verify(
    engine: &mut PilotEngine,
    plan: &AiActPlan,
    ctx: &CommandContext,
) -> Result<AiActOutcome, PilotError> {
    act_verify(engine, plan, ctx)
}

fn infer_retry_from_data(data: &Value) -> RetryDisposition {
    let disp_val = data
        .get("disposition")
        .or_else(|| data.get("action_result").and_then(|ar| ar.get("disposition")));
    if let Some(val) = disp_val
        && let Ok(semantics) = serde_json::from_value::<DeliverySemantics>(val.clone())
    {
        return semantics.retry_disposition();
    }
    RetryDisposition::Unknown
}

pub fn execute_batch(
    engine: &mut PilotEngine,
    steps: &[BatchStep],
    stop_on_error: bool,
    ctx: &CommandContext,
) -> BatchReport {
    execute_batch_with_limits(engine, steps, stop_on_error, ctx, MAX_BATCH_OUTPUT_BYTES)
}

pub fn execute_batch_with_limits(
    engine: &mut PilotEngine,
    steps: &[BatchStep],
    stop_on_error: bool,
    ctx: &CommandContext,
    max_output_bytes: usize,
) -> BatchReport {
    let started = Instant::now();
    let deadline = Deadline::from_timeout_ms(ctx.timeout_ms);
    let total = steps.len();
    let mut results = Vec::with_capacity(total.min(MAX_BATCH_ENTRIES));
    let mut completed = 0_usize;
    let mut succeeded = 0_usize;
    let mut failed = 0_usize;
    let mut stopped_early = false;
    let mut stopped_reason = None;
    let mut stopped_index = None;
    let mut used_bytes = 0_usize;

    for (index, step) in steps.iter().enumerate() {
        if index >= MAX_BATCH_ENTRIES {
            stopped_early = true;
            stopped_reason = Some("entry_limit".to_owned());
            stopped_index = Some(index);
            break;
        }
        if deadline.is_expired() {
            failed += 1;
            let err = PilotError::new(
                ErrorCode::Timeout,
                format!("batch deadline elapsed before step {index} ('{}') started", step.op),
            )
            .with_disposition(DeliverySemantics::not_delivered());
            results.push(BatchStepResult {
                index,
                op: step.op.clone(),
                ok: false,
                duration_ms: 0,
                execution: BatchExecutionState::NotStarted,
                retry_disposition: err.retry_disposition(),
                data: None,
                error: Some(err),
            });
            stopped_early = true;
            stopped_reason = Some("deadline".to_owned());
            stopped_index = Some(index);
            break;
        }

        let step_start = Instant::now();
        let entry = match engine.execute_tool(&step.op, &step.args, ctx) {
            Ok(data) => {
                succeeded += 1;
                completed += 1;
                let retry = infer_retry_from_data(&data);
                BatchStepResult {
                    index,
                    op: step.op.clone(),
                    ok: true,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                    execution: BatchExecutionState::Completed,
                    retry_disposition: retry,
                    data: Some(data),
                    error: None,
                }
            }
            Err(err) => {
                failed += 1;
                completed += 1;
                let retry = err.retry_disposition();
                BatchStepResult {
                    index,
                    op: step.op.clone(),
                    ok: false,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                    execution: BatchExecutionState::Completed,
                    retry_disposition: retry,
                    data: None,
                    error: Some(err),
                }
            }
        };

        let entry_bytes = serde_json::to_vec(&entry).map_or(256, |bytes| bytes.len());
        let entry_failed = !entry.ok;
        if used_bytes.saturating_add(entry_bytes) > max_output_bytes {
            let truncated = BatchStepResult {
                index: entry.index,
                op: entry.op,
                ok: entry.ok,
                duration_ms: entry.duration_ms,
                execution: entry.execution,
                retry_disposition: entry.retry_disposition,
                data: None,
                error: Some(
                    PilotError::new(
                        ErrorCode::Internal,
                        format!("batch step {index} output exceeded byte limit ({max_output_bytes})"),
                    )
                    .with_disposition(DeliverySemantics::uncertain()),
                ),
            };
            results.push(truncated);
            stopped_early = true;
            stopped_reason = Some("output_limit".to_owned());
            stopped_index = Some(index);
            break;
        }
        used_bytes = used_bytes.saturating_add(entry_bytes);
        results.push(entry);

        if entry_failed && stop_on_error {
            stopped_early = true;
            stopped_reason = Some("stop_on_error".to_owned());
            stopped_index = Some(index);
            break;
        }
    }

    BatchReport {
        total,
        completed,
        succeeded,
        failed,
        not_started: total.saturating_sub(completed),
        stopped_early,
        stopped_reason,
        stopped_index,
        max_output_bytes,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::TargetBackend;

    #[test]
    fn act_verify_and_batch_stop_on_error_track_retry_and_not_started() {
        let mut engine = PilotEngine::new();
        engine.set_target(TargetBackend::Gpui);
        let ctx = CommandContext {
            target: TargetBackend::Gpui,
            ..Default::default()
        };

        let outcome = act_verify(
            &mut engine,
            &AiActPlan {
                app: None,
                window_id: None,
                surface: SnapshotSurface::Window,
                target_ref: None,
                locator: Some(LocatorQuery {
                    identity: crate::locator::IdentityPredicate {
                        role: Some("switch".to_owned()),
                        name: Some("Dark Appearance".to_owned()),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                action: Action::Toggle,
                policy: InteractionPolicy::Verified,
                include_post_snapshot: true,
            },
            &ctx,
        )
        .expect("act_verify toggle ok");
        assert!(outcome.action_result.verified);
        assert_eq!(outcome.retry_disposition, RetryDisposition::Unsafe);
        assert!(outcome.after_snapshot_diff.is_some());

        let batch = execute_batch(
            &mut engine,
            &[
                BatchStep {
                    op: "desktop_status".to_owned(),
                    args: json!({}),
                },
                BatchStep {
                    op: "desktop_unknown_op_for_test".to_owned(),
                    args: json!({}),
                },
                BatchStep {
                    op: "desktop_status".to_owned(),
                    args: json!({}),
                },
            ],
            true,
            &ctx,
        );
        assert_eq!(batch.total, 3);
        assert_eq!(batch.completed, 2);
        assert_eq!(batch.succeeded, 1);
        assert_eq!(batch.failed, 1);
        assert_eq!(batch.not_started, 1);
        assert!(batch.stopped_early);
        assert_eq!(batch.stopped_reason.as_deref(), Some("stop_on_error"));
        assert_eq!(batch.stopped_index, Some(1));
        assert_eq!(batch.steps[1].retry_disposition, RetryDisposition::Safe);
    }
}
