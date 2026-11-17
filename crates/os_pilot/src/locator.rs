use serde::{Deserialize, Serialize};

use crate::model::{AccessibilityNode, ErrorCode, PilotError, SnapshotSurface, WindowInfo};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationRequest {
    #[serde(default)]
    pub surface: SnapshotSurface,
    #[serde(default)]
    pub interactive_only: bool,
    #[serde(default)]
    pub include_bounds: bool,
    #[serde(default)]
    pub compact: bool,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default = "default_max_nodes")]
    pub max_nodes: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<String>,
}

const fn default_max_depth() -> usize {
    18
}

const fn default_max_nodes() -> usize {
    2500
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationBudgetStats {
    pub nodes_visited: usize,
    pub max_depth_reached: usize,
    pub truncated: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedTree {
    pub snapshot_id: String,
    pub window: WindowInfo,
    pub surface: SnapshotSurface,
    pub root: AccessibilityNode,
    pub complete: bool,
    pub stats: ObservationBudgetStats,
    pub text: String,
    pub ref_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityPredicate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_contains: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatePredicate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
    #[serde(default)]
    pub interactive_only: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainmentPredicate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocatorQuery {
    #[serde(flatten)]
    pub identity: IdentityPredicate,
    #[serde(flatten)]
    pub state: StatePredicate,
    #[serde(flatten)]
    pub containment: ContainmentPredicate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nth: Option<usize>,
    #[serde(default)]
    pub all: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocatorMatch {
    pub ref_id: Option<String>,
    pub node_id: String,
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    pub interactive: bool,
    pub depth: usize,
    pub bounds: Option<crate::model::Rect>,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocatorResolution {
    pub query: LocatorQuery,
    pub matches: Vec<LocatorMatch>,
    pub total_found: usize,
}

#[must_use]
pub fn node_matches(node: &AccessibilityNode, query: &LocatorQuery) -> bool {
    if query.state.interactive_only && !node.interactive && node.actions.is_empty() {
        return false;
    }
    if let Some(role) = &query.identity.role
        && !node.role.eq_ignore_ascii_case(role.trim())
    {
        return false;
    }
    if let Some(name) = &query.identity.name
        && node.name.as_deref() != Some(name.as_str())
    {
        return false;
    }
    if let Some(needle) = &query.identity.name_contains {
        let lower = needle.to_ascii_lowercase();
        let hay = node.name.as_deref().unwrap_or("").to_ascii_lowercase();
        if !hay.contains(&lower) {
            return false;
        }
    }
    if let Some(id) = &query.identity.identifier
        && node.identifier.as_deref() != Some(id.as_str())
    {
        return false;
    }
    if let Some(val_needle) = &query.identity.value_contains {
        let lower = val_needle.to_ascii_lowercase();
        let hay = node
            .value
            .as_deref()
            .or(node.state.value.as_deref())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !hay.contains(&lower) {
            return false;
        }
    }
    if let Some(enabled) = query.state.enabled
        && node.state.has_state("enabled") != enabled
    {
        return false;
    }
    if let Some(focused) = query.state.focused
        && node.state.has_state("focused") != focused
    {
        return false;
    }
    if let Some(checked) = query.state.checked
        && node.state.has_state("checked") != checked
    {
        return false;
    }
    if let Some(selected) = query.state.selected
        && node.state.has_state("selected") != selected
    {
        return false;
    }
    if let Some(expanded) = query.state.expanded
        && node.state.has_state("expanded") != expanded
    {
        return false;
    }
    true
}

#[must_use]
pub fn evaluate_locator_tree(root: &AccessibilityNode, query: &LocatorQuery) -> LocatorResolution {
    let start_root = match &query.containment.within_ref {
        Some(within) => find_node_by_ref(root, within).unwrap_or(root),
        None => root,
    };
    let mut matches = Vec::new();
    collect_matches(start_root, query, 0, &mut matches);
    let total_found = matches.len();
    if let Some(nth) = query.nth {
        matches = matches.into_iter().nth(nth).into_iter().collect();
    } else if !query.all && matches.len() > 1 {
        // Return all matches so callers see ambiguity clearly unless `--nth` or `--all` specified.
    }
    LocatorResolution {
        query: query.clone(),
        matches,
        total_found,
    }
}

pub fn resolve_query(root: &AccessibilityNode, query: &LocatorQuery) -> LocatorResolution {
    evaluate_locator_tree(root, query)
}

pub fn resolve_unique(
    root: &AccessibilityNode,
    query: &LocatorQuery,
) -> Result<LocatorMatch, PilotError> {
    let res = evaluate_locator_tree(root, query);
    match res.matches.len() {
        0 => Err(PilotError::new(
            ErrorCode::ElementNotFound,
            "no accessibility element matched locator query",
        )
        .with_suggestion("Inspect the latest `snapshot` and broaden the query.")),
        1 => Ok(res
            .matches
            .into_iter()
            .next()
            .unwrap_or_else(|| unreachable_match())),
        count => Err(PilotError::new(
            ErrorCode::AmbiguousRef,
            format!("locator matched {count} elements; specify --nth or refine predicate"),
        )
        .with_suggestion("Set `nth` or narrow role/name/identifier.")),
    }
}

fn unreachable_match() -> LocatorMatch {
    LocatorMatch {
        ref_id: None,
        node_id: String::new(),
        role: String::new(),
        name: None,
        value: None,
        identifier: None,
        interactive: false,
        depth: 0,
        bounds: None,
        actions: Vec::new(),
    }
}

pub fn find_node_by_ref<'a>(
    node: &'a AccessibilityNode,
    ref_or_id: &str,
) -> Option<&'a AccessibilityNode> {
    let clean = ref_or_id.trim();
    let stripped = clean.strip_prefix('@').unwrap_or(clean);
    if node
        .ref_id
        .as_deref()
        .is_some_and(|r| r == clean || r.strip_prefix('@').unwrap_or(r) == stripped)
        || node.id == clean
    {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_node_by_ref(child, ref_or_id) {
            return Some(found);
        }
    }
    None
}

fn collect_matches(
    node: &AccessibilityNode,
    query: &LocatorQuery,
    depth: usize,
    out: &mut Vec<LocatorMatch>,
) {
    if node_matches(node, query) {
        out.push(LocatorMatch {
            ref_id: node.ref_id.clone(),
            node_id: node.id.clone(),
            role: node.role.clone(),
            name: node.name.clone(),
            value: node.value.clone().or_else(|| node.state.value.clone()),
            identifier: node.identifier.clone(),
            interactive: node.interactive,
            depth,
            bounds: node.bounds,
            actions: node.actions.clone(),
        });
    }
    for child in &node.children {
        collect_matches(child, query, depth + 1, out);
    }
}

/// Renders an AI-agent compact hierarchical snapshot string (`[ref] role "name" states bounds`).
#[must_use]
pub fn render_snapshot_text(root: &AccessibilityNode, request: &ObservationRequest) -> String {
    let mut lines = Vec::new();
    render_node_line(root, 0, request, &mut lines);
    lines.join("\n")
}

fn render_node_line(
    node: &AccessibilityNode,
    depth: usize,
    req: &ObservationRequest,
    out: &mut Vec<String>,
) {
    let has_content = node.interactive
        || !node.actions.is_empty()
        || node.name.as_deref().is_some_and(|n| !n.trim().is_empty())
        || node.value.as_deref().is_some_and(|v| !v.trim().is_empty());

    let show = if req.interactive_only {
        node.interactive || !node.actions.is_empty() || depth == 0
    } else if req.compact {
        has_content || depth == 0
    } else {
        true
    };

    if show {
        let indent = "  ".repeat(depth);
        let ref_badge = node
            .ref_id
            .as_deref()
            .map(|r| format!("[{r}] "))
            .unwrap_or_default();
        let name_part = node
            .name
            .as_deref()
            .filter(|n| !n.trim().is_empty())
            .map(|n| format!(" {n:?}"))
            .unwrap_or_default();
        let val_part = node
            .value
            .as_deref()
            .or(node.state.value.as_deref())
            .filter(|v| !v.trim().is_empty())
            .map(|v| format!(" value={v:?}"))
            .unwrap_or_default();
        let states_part = if node.state.states.is_empty() {
            String::new()
        } else {
            format!(" ({})", node.state.states.join(","))
        };
        let bounds_part = if req.include_bounds {
            node.bounds
                .map(|b| format!(" @({:.0},{:.0} {:.0}x{:.0})", b.x, b.y, b.width, b.height))
                .unwrap_or_default()
        } else {
            String::new()
        };
        out.push(format!(
            "{indent}{ref_badge}{}{name_part}{val_part}{states_part}{bounds_part}",
            node.role
        ));
    }

    for child in &node.children {
        render_node_line(child, depth + 1, req, out);
    }
}
