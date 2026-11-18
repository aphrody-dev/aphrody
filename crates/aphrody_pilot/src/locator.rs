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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub testid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xpath: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exact: Option<bool>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nth_signed: Option<isize>,
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

fn normalize_ws(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_matches(candidate: Option<&str>, expected: &str, exact: Option<bool>) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    match exact {
        Some(false) => {
            let needle = normalize_ws(expected).to_ascii_lowercase();
            let hay = normalize_ws(candidate).to_ascii_lowercase();
            hay.contains(&needle)
        }
        Some(true) => normalize_ws(candidate) == normalize_ws(expected),
        None => candidate == expected,
    }
}

fn matches_xpath_segment(node: &AccessibilityNode, segment: &str) -> bool {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        return false;
    }
    let (role_part, predicates_part) = match trimmed.find('[') {
        Some(idx) => (&trimmed[..idx], Some(&trimmed[idx..])),
        None => (trimmed, None),
    };
    let role_part = role_part.trim();
    if !role_part.is_empty() && role_part != "*" && !node.role.eq_ignore_ascii_case(role_part) {
        return false;
    }
    let Some(mut rest) = predicates_part else {
        return true;
    };
    while let Some(open) = rest.find('[') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(']') else {
            return false;
        };
        let clause_group = &after_open[..close];
        for clause in clause_group.split(" and ") {
            if !matches_xpath_clause(node, clause.trim()) {
                return false;
            }
        }
        rest = &after_open[close + 1..];
    }
    true
}

fn unquote_xpath_literal(raw: &str) -> &str {
    let trimmed = raw.trim();
    trimmed
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .or_else(|| {
            trimmed
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
        })
        .unwrap_or(trimmed)
}

fn xpath_attr_value<'a>(node: &'a AccessibilityNode, attr: &str) -> Option<&'a str> {
    match attr.trim().to_ascii_lowercase().as_str() {
        "@role" => Some(node.role.as_str()),
        "@name" | "@label" | "@aria-label" | "text()" => node.name.as_deref(),
        "@value" => node.value.as_deref().or(node.state.value.as_deref()),
        "@id" | "@identifier" | "@data-testid" | "@testid" => node.identifier.as_deref(),
        "@placeholder" | "@hint" => node.hint.as_deref().or(node.description.as_deref()),
        "@description" => node.description.as_deref(),
        "@ref" => node.ref_id.as_deref(),
        _ => None,
    }
}

fn matches_xpath_clause(node: &AccessibilityNode, clause: &str) -> bool {
    if let Some(inner) = clause
        .strip_prefix("contains(")
        .and_then(|s| s.strip_suffix(')'))
        && let Some((attr, lit)) = inner.split_once(',')
    {
        let expected = unquote_xpath_literal(lit).to_ascii_lowercase();
        return xpath_attr_value(node, attr)
            .is_some_and(|val| val.to_ascii_lowercase().contains(&expected));
    }
    if let Some((attr, lit)) = clause.split_once('=') {
        let expected = unquote_xpath_literal(lit);
        return match attr.trim().to_ascii_lowercase().as_str() {
            "@checked" => {
                node.state.has_state("checked").to_string() == expected.to_ascii_lowercase()
            }
            "@enabled" => {
                node.state.has_state("enabled").to_string() == expected.to_ascii_lowercase()
            }
            "@focused" => {
                node.state.has_state("focused").to_string() == expected.to_ascii_lowercase()
            }
            "@selected" => {
                node.state.has_state("selected").to_string() == expected.to_ascii_lowercase()
            }
            "@expanded" => {
                node.state.has_state("expanded").to_string() == expected.to_ascii_lowercase()
            }
            _ => xpath_attr_value(node, attr).is_some_and(|val| val == expected),
        };
    }
    false
}

fn matches_xpath_with_ancestors(
    ancestors: &[&AccessibilityNode],
    node: &AccessibilityNode,
    xpath: &str,
) -> bool {
    let raw_segments: Vec<&str> = xpath
        .trim()
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect();
    let Some((leaf, prefix)) = raw_segments.split_last() else {
        return false;
    };
    if !matches_xpath_segment(node, leaf) {
        return false;
    }
    if prefix.is_empty() {
        return true;
    }
    let mut anc_idx = ancestors.len();
    for seg in prefix.iter().rev() {
        let mut matched = false;
        while anc_idx > 0 {
            anc_idx -= 1;
            if matches_xpath_segment(ancestors[anc_idx], seg) {
                matched = true;
                break;
            }
        }
        if !matched {
            return false;
        }
    }
    true
}

#[must_use]
pub fn node_matches(node: &AccessibilityNode, query: &LocatorQuery) -> bool {
    node_matches_with_ancestors(&[], node, query)
}

fn node_matches_with_ancestors(
    ancestors: &[&AccessibilityNode],
    node: &AccessibilityNode,
    query: &LocatorQuery,
) -> bool {
    if query.state.interactive_only && !node.interactive && node.actions.is_empty() {
        return false;
    }
    if let Some(role) = &query.identity.role
        && !node.role.eq_ignore_ascii_case(role.trim())
    {
        return false;
    }
    if let Some(name) = &query.identity.name
        && !text_matches(node.name.as_deref(), name, query.identity.exact)
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
    if let Some(label) = &query.identity.label {
        let matches_label = text_matches(node.name.as_deref(), label, query.identity.exact)
            || text_matches(node.description.as_deref(), label, query.identity.exact)
            || text_matches(node.hint.as_deref(), label, query.identity.exact);
        if !matches_label {
            return false;
        }
    }
    if let Some(placeholder) = &query.identity.placeholder {
        let matches_placeholder = text_matches(
            node.hint.as_deref(),
            placeholder,
            query.identity.exact,
        ) || text_matches(
            node.description.as_deref(),
            placeholder,
            query.identity.exact,
        ) || text_matches(
            node.value.as_deref().or(node.state.value.as_deref()),
            placeholder,
            query.identity.exact,
        );
        if !matches_placeholder {
            return false;
        }
    }
    if let Some(testid) = &query.identity.testid
        && !text_matches(node.identifier.as_deref(), testid, query.identity.exact)
    {
        return false;
    }
    if let Some(xpath) = &query.identity.xpath
        && !matches_xpath_with_ancestors(ancestors, node, xpath)
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
    let mut ancestors = Vec::new();
    collect_matches(start_root, query, 0, &mut ancestors, &mut matches);
    let total_found = matches.len();
    if let Some(nth_signed) = query.nth_signed {
        let resolved_idx = if nth_signed >= 0 {
            Some(nth_signed as usize)
        } else {
            total_found.checked_sub(nth_signed.unsigned_abs())
        };
        matches = resolved_idx
            .and_then(|idx| matches.into_iter().nth(idx))
            .into_iter()
            .collect();
    } else if let Some(nth) = query.nth {
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

fn collect_matches<'a>(
    node: &'a AccessibilityNode,
    query: &LocatorQuery,
    depth: usize,
    ancestors: &mut Vec<&'a AccessibilityNode>,
    out: &mut Vec<LocatorMatch>,
) {
    if node_matches_with_ancestors(ancestors, node, query) {
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
    ancestors.push(node);
    for child in &node.children {
        collect_matches(child, query, depth + 1, ancestors, out);
    }
    ancestors.pop();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> AccessibilityNode {
        AccessibilityNode {
            id: "root".to_owned(),
            ref_id: Some("@s1:e0".to_owned()),
            role: "window".to_owned(),
            name: Some("App".to_owned()),
            children: vec![AccessibilityNode {
                id: "bar".to_owned(),
                ref_id: Some("@s1:e1".to_owned()),
                role: "toolbar".to_owned(),
                children: vec![
                    AccessibilityNode {
                        id: "search".to_owned(),
                        ref_id: Some("@s1:e2".to_owned()),
                        role: "textfield".to_owned(),
                        name: Some("Filter items".to_owned()),
                        hint: Some("Search workspace...".to_owned()),
                        identifier: Some("filter-input".to_owned()),
                        interactive: true,
                        ..Default::default()
                    },
                    AccessibilityNode {
                        id: "btn1".to_owned(),
                        ref_id: Some("@s1:e3".to_owned()),
                        role: "button".to_owned(),
                        name: Some("Export   Asset".to_owned()),
                        identifier: Some("export-btn".to_owned()),
                        interactive: true,
                        ..Default::default()
                    },
                    AccessibilityNode {
                        id: "btn2".to_owned(),
                        ref_id: Some("@s1:e4".to_owned()),
                        role: "button".to_owned(),
                        name: Some("Cancel".to_owned()),
                        identifier: Some("cancel-btn".to_owned()),
                        interactive: true,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn semantic_locators_match_label_placeholder_testid_xpath_and_nth() {
        let tree = sample_tree();

        let by_placeholder = evaluate_locator_tree(
            &tree,
            &LocatorQuery {
                identity: IdentityPredicate {
                    placeholder: Some("Search".to_owned()),
                    exact: Some(false),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert_eq!(by_placeholder.matches.len(), 1);
        assert_eq!(by_placeholder.matches[0].node_id, "search");

        let by_label_exact = evaluate_locator_tree(
            &tree,
            &LocatorQuery {
                identity: IdentityPredicate {
                    label: Some("Export Asset".to_owned()),
                    exact: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert_eq!(by_label_exact.matches.len(), 1);
        assert_eq!(by_label_exact.matches[0].node_id, "btn1");

        let by_testid = evaluate_locator_tree(
            &tree,
            &LocatorQuery {
                identity: IdentityPredicate {
                    testid: Some("export-btn".to_owned()),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert_eq!(by_testid.matches.len(), 1);
        assert_eq!(by_testid.matches[0].node_id, "btn1");

        let by_xpath = evaluate_locator_tree(
            &tree,
            &LocatorQuery {
                identity: IdentityPredicate {
                    xpath: Some("//toolbar//button[@data-testid='cancel-btn']".to_owned()),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert_eq!(by_xpath.matches.len(), 1);
        assert_eq!(by_xpath.matches[0].node_id, "btn2");

        let last_button = evaluate_locator_tree(
            &tree,
            &LocatorQuery {
                identity: IdentityPredicate {
                    role: Some("button".to_owned()),
                    ..Default::default()
                },
                nth_signed: Some(-1),
                ..Default::default()
            },
        );
        assert_eq!(last_button.total_found, 2);
        assert_eq!(last_button.matches.len(), 1);
        assert_eq!(last_button.matches[0].node_id, "btn2");
    }
}
