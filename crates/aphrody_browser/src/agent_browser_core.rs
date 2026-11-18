//! Core `agent-browser 0.43.0` fusion over `aphrody_pilot::browser`, `browser-browse`, and `browser-views`.
//!
//! Bridges `aphrody_pilot`'s pure-Rust browser automation primitives (`SnapshotDiff`,
//! `SemanticLocator`, `AnnotatedElementBox`, `NetworkRouteTable`, `HarRingBuffer`,
//! `BrowserEventRingBuffer`, `BrowserStorageState`, `TabSessionManager`,
//! `BrowserActionPolicy`, batch execution) with `browser-browse` (`AriaSnapshotNode`,
//! `BrowseSessionPool`, `DomainPolicy`, `detect_handoff_gate`), `browser-views`
//! (`BrowserViewModel`), and `url::Url`.

use std::collections::BTreeMap;

use browser_browse::{
    AriaSnapshotNode, BrowseSessionPool, DomainPolicy, HandoffNotice, assign_aria_refs,
    detect_handoff_gate,
};
use browser_views::BrowserViewModel;
use browser_vpn::RouteMode;
use url::Url;

pub use aphrody_pilot::browser::{
    AnnotatedElementBox, AnnotatedScreenshot, BrowserActionPolicy, BrowserBatchReport,
    BrowserBatchStep, BrowserBatchStepResult, BrowserControllerState, BrowserCookie,
    BrowserEventRingBuffer, BrowserLaunchOptions, BrowserPolicyDecision, BrowserStorageState,
    BrowserTabSession, BrowserUrlCheck, ConsoleLogEntry, DiffLine, DiffLineKind, DomainFilter,
    HarContentMode, HarEntry, HarRingBuffer, MAX_EVENT_RING_ENTRIES, MAX_HAR_ENTRIES,
    MAX_HAR_ENTRY_BODY_BYTES, MAX_HAR_TOTAL_BYTES, NetworkRoute, NetworkRouteTable, OriginStorage,
    PageErrorEntry, RouteDisposition, RouteResponse, SemanticLocator, SemanticLocatorKind,
    SnapshotDiff, StorageEntry, TabRef, TabSessionManager, action_policy_category,
    build_annotation_overlay_js, build_chrome_launch_args, cleanup_annotation_overlay_js,
    diff_snapshot_text, filter_non_overlapping_annotations, find_ax_nodes_by_locator,
    format_console_arg, format_console_args, full_page_clip_size, is_sensitive_header,
    is_valid_tab_label, matches_status_filter, normalize_ax_role, project_annotation_boxes,
    redact_header_map, route_url_matches, sanitize_tab_url, snapshot_comparison_text,
    validate_navigation_url,
};

#[derive(Debug)]
pub struct AgentBrowserCore {
    pub controller: BrowserControllerState,
    pub domain_policy: DomainPolicy,
    pub browse_pool: BrowseSessionPool,
    pub last_aria_root: Option<AriaSnapshotNode>,
    pub active_handoff: Option<HandoffNotice>,
}

impl Default for AgentBrowserCore {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentBrowserCore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            controller: BrowserControllerState::default(),
            domain_policy: DomainPolicy::default(),
            browse_pool: BrowseSessionPool::default(),
            last_aria_root: None,
            active_handoff: None,
        }
    }

    /// Validates `raw_url` against `url::Url`, SSRF guards, and `DomainPolicy`, then checks for
    /// anti-bot/MFA handoff gates before updating tab/session navigation state.
    pub fn navigate_checked(
        &mut self,
        raw_url: &str,
        http_status: u16,
        page_title: &str,
    ) -> anyhow::Result<Option<HandoffNotice>> {
        let parsed = Url::parse(raw_url.trim())
            .map_err(|err| anyhow::anyhow!("invalid URL '{raw_url}': {err}"))?;
        let ssrf = validate_navigation_url(parsed.as_str(), &self.controller.options);
        if !ssrf.allowed {
            anyhow::bail!(
                "navigation blocked by SSRF guard: {}",
                ssrf.reason.unwrap_or_else(|| "blocked".to_owned())
            );
        }
        let host = parsed.host_str().unwrap_or("");
        let has_cookies = !self.controller.storage.cookies_for_host(Some(host)).is_empty();
        if let Err(err) = self.domain_policy.check_url(parsed.as_str(), has_cookies) {
            anyhow::bail!("navigation blocked by domain policy for host '{host}': {err}");
        }

        let target_id = self
            .controller
            .tabs
            .active_tab()
            .map(|tab| tab.target_id.clone())
            .unwrap_or_else(|| "target-1".to_owned());

        if let Some(notice) =
            detect_handoff_gate(&target_id, parsed.as_str(), http_status, page_title, None)
        {
            self.active_handoff = Some(notice.clone());
            return Ok(Some(notice));
        }

        self.active_handoff = None;
        self.controller.current_url = parsed.to_string();
        self.controller.last_title = page_title.to_owned();
        self.controller
            .tabs
            .pin_active_url(parsed.as_str(), page_title);
        Ok(None)
    }

    /// Assigns deterministic `@e1..@eN` ARIA refs, compacts the snapshot tree, and returns both
    /// rendered compact text and its [`SnapshotDiff`] against the previous baseline.
    pub fn snapshot_aria(&mut self, mut root: AriaSnapshotNode) -> (String, SnapshotDiff) {
        let _assigned = assign_aria_refs(&mut root);
        let compact = root.format_compact();
        let previous = std::mem::replace(&mut self.controller.last_snapshot_text, compact.clone());
        let diff = diff_snapshot_text(&previous, &compact);
        self.last_aria_root = Some(root);
        (compact, diff)
    }

    /// Evaluates a [`SemanticLocator`] over the latest [`AriaSnapshotNode`] tree.
    #[must_use]
    pub fn find_in_aria(&self, locator: &SemanticLocator) -> Vec<AriaSnapshotNode> {
        let Some(root) = &self.last_aria_root else {
            return Vec::new();
        };
        let mut matched = Vec::new();
        collect_aria_matches(root, locator, &mut matched);
        if let Some(nth) = locator.nth {
            let total = matched.len();
            let idx = if nth >= 0 {
                Some(nth as usize)
            } else {
                total.checked_sub(nth.unsigned_abs())
            };
            return idx
                .and_then(|i| matched.into_iter().nth(i))
                .into_iter()
                .collect();
        }
        matched
    }

    /// Generates overlap-filtered [`AnnotatedElementBox`] badges from the latest ARIA snapshot tree.
    #[must_use]
    pub fn annotate_aria_boxes(
        &self,
        bounds_lookup: &dyn Fn(&AriaSnapshotNode) -> Option<(f64, f64, f64, f64)>,
    ) -> Vec<AnnotatedElementBox> {
        let Some(root) = &self.last_aria_root else {
            return Vec::new();
        };
        let mut raw = Vec::new();
        collect_annotated_aria(root, bounds_lookup, &mut raw);
        filter_non_overlapping_annotations(raw)
    }

    /// Resolves route intercepts and pushes sensitive-header-redacted traffic into the HAR ring.
    #[allow(clippy::too_many_arguments)]
    pub fn record_network_exchange(
        &mut self,
        request_id: &str,
        started_ms: u64,
        time_ms: f64,
        method: &str,
        url: &str,
        resource_type: Option<&str>,
        mut request_headers: BTreeMap<String, String>,
        mut response_headers: BTreeMap<String, String>,
        mut status: u16,
        response_body: Option<String>,
        mime_type: &str,
    ) -> Option<NetworkRoute> {
        let matched_route = self
            .controller
            .routes
            .select_route(url, Some(method), resource_type);
        let mut final_body = response_body;
        if let Some(route) = &matched_route {
            match route.disposition {
                RouteDisposition::Abort => status = 0,
                RouteDisposition::Fulfill => {
                    if let Some(resp) = &route.response {
                        status = resp.status;
                        for (k, v) in &resp.headers {
                            response_headers.insert(k.clone(), v.clone());
                        }
                        final_body = Some(resp.body.clone());
                    }
                }
                RouteDisposition::Continue => {}
            }
        }

        request_headers = redact_header_map(&request_headers);
        response_headers = redact_header_map(&response_headers);
        let encoded_bytes = final_body.as_ref().map_or(0, String::len);
        self.controller.har.push(HarEntry {
            request_id: request_id.to_owned(),
            started_ms,
            time_ms,
            method: method.to_owned(),
            url: url.to_owned(),
            status,
            status_text: if status == 0 {
                "Aborted".to_owned()
            } else {
                "OK".to_owned()
            },
            request_headers,
            response_headers,
            request_body: None,
            response_body: final_body,
            mime_type: mime_type.to_owned(),
            encoded_bytes,
            from_route: matched_route.is_some(),
        });
        matched_route
    }

    /// Synchronizes active tab state with `browser_views::BrowserViewModel` for GPUI rendering.
    pub fn sync_from_browser_model(&mut self, model: &BrowserViewModel) {
        if let Some(active) = model.active_tab() {
            self.controller.current_url = active.url.clone();
            self.controller.last_title = active.title.clone();
            self.controller
                .tabs
                .pin_active_url(&active.url, &active.title);
        }
    }

    pub fn open_synced_tab(
        &mut self,
        model: &mut BrowserViewModel,
        title: &str,
        url: &str,
        vpn_route: RouteMode,
    ) -> anyhow::Result<BrowserTabSession> {
        let check = validate_navigation_url(url, &self.controller.options);
        if url != "about:blank" && !check.allowed {
            anyhow::bail!(
                "tab URL rejected: {}",
                check.reason.unwrap_or_else(|| "blocked".to_owned())
            );
        }
        model.open_tab(title, url, vpn_route);
        let tab = self.controller.tabs.open_tab(url, None, None, None)?.clone();
        self.controller.current_url = tab.url.clone();
        self.controller.last_title = title.to_owned();
        Ok(tab)
    }

    #[must_use]
    pub fn telemetry_summary(&self) -> String {
        let tab_ref = self
            .controller
            .tabs
            .active_tab()
            .map_or("t1", |t| t.tab_ref.as_str());
        let console_count = self.controller.events.console_entries(None).len();
        let error_count = self.controller.events.error_entries().len();
        let route_count = self.controller.routes.routes().len();
        let har_count = self.controller.har.len();
        format!(
            "Tab {tab_ref} ({} open) · {route_count} route(s) · {har_count} HAR · {console_count} console · {error_count} err",
            self.controller.tabs.tabs().len(),
        )
    }
}

fn collect_aria_matches(
    node: &AriaSnapshotNode,
    locator: &SemanticLocator,
    out: &mut Vec<AriaSnapshotNode>,
) {
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let text_ok = |hay: &str, needle: &str| -> bool {
        let a = norm(hay);
        let b = norm(needle);
        if locator.exact {
            a == b
        } else {
            a.to_ascii_lowercase().contains(&b.to_ascii_lowercase())
        }
    };

    let matched = match locator.kind {
        SemanticLocatorKind::Role => {
            let role_ok = normalize_ax_role(&node.role)
                .eq_ignore_ascii_case(normalize_ax_role(&locator.query));
            let name_ok = locator
                .name
                .as_deref()
                .is_none_or(|n| text_ok(&node.name, n));
            role_ok && name_ok
        }
        SemanticLocatorKind::Text
        | SemanticLocatorKind::Label
        | SemanticLocatorKind::AltText
        | SemanticLocatorKind::Title => text_ok(&node.name, &locator.query),
        SemanticLocatorKind::Placeholder => node
            .value
            .as_deref()
            .is_some_and(|val| text_ok(val, &locator.query)),
        SemanticLocatorKind::TestId => node
            .ref_id
            .as_deref()
            .is_some_and(|r| r == locator.query)
            || node.name == locator.query,
        SemanticLocatorKind::XPath | SemanticLocatorKind::Css => {
            let clean = locator.query.trim();
            if let Some(tag) = clean.strip_prefix("//") {
                let role_only = tag.split('[').next().unwrap_or("").trim();
                role_only.is_empty()
                    || role_only == "*"
                    || normalize_ax_role(&node.role)
                        .eq_ignore_ascii_case(normalize_ax_role(role_only))
            } else {
                false
            }
        }
    };

    if matched {
        out.push(node.clone());
    }
    for child in &node.children {
        collect_aria_matches(child, locator, out);
    }
}

fn collect_annotated_aria(
    node: &AriaSnapshotNode,
    bounds_lookup: &dyn Fn(&AriaSnapshotNode) -> Option<(f64, f64, f64, f64)>,
    out: &mut Vec<AnnotatedElementBox>,
) {
    if let Some(ref_id) = &node.ref_id
        && let Some((x, y, width, height)) = bounds_lookup(node)
    {
        let number = out.len() + 1;
        out.push(AnnotatedElementBox {
            ref_id: ref_id.clone(),
            number,
            role: node.role.clone(),
            name: node.name.clone(),
            x,
            y,
            width,
            height,
        });
    }
    for child in &node.children {
        collect_annotated_aria(child, bounds_lookup, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_aria_page(button_name: &str) -> AriaSnapshotNode {
        AriaSnapshotNode {
            ref_id: None,
            role: "document".to_owned(),
            name: "Workspace".to_owned(),
            value: None,
            disabled: false,
            focused: false,
            children: vec![
                AriaSnapshotNode {
                    ref_id: None,
                    role: "textbox".to_owned(),
                    name: "Search docs".to_owned(),
                    value: Some("Search...".to_owned()),
                    disabled: false,
                    focused: false,
                    children: Vec::new(),
                },
                AriaSnapshotNode {
                    ref_id: None,
                    role: "button".to_owned(),
                    name: button_name.to_owned(),
                    value: None,
                    disabled: false,
                    focused: true,
                    children: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn aria_snapshot_diff_semantic_find_and_annotations_fuse() {
        let mut core = AgentBrowserCore::new();
        let (_text1, diff1) = core.snapshot_aria(sample_aria_page("Save"));
        assert!(diff1.changed);

        let (_text2, diff2) = core.snapshot_aria(sample_aria_page("Publish"));
        assert_eq!(diff2.additions, 1);
        assert_eq!(diff2.removals, 1);

        let hits = core.find_in_aria(&SemanticLocator::role("button").with_name("Publish"));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].ref_id.as_deref(), Some("@e2"));

        let boxes = core.annotate_aria_boxes(&|node| {
            Some(if node.role == "textbox" {
                (10.0, 10.0, 200.0, 32.0)
            } else {
                (10.0, 64.0, 100.0, 32.0)
            })
        });
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0].number, 1);
        assert_eq!(boxes[1].ref_id, "@e2");
    }

    #[test]
    fn navigate_checked_stops_on_handoff_and_har_redacts_secrets() {
        let mut core = AgentBrowserCore::new();
        let gate = core
            .navigate_checked("https://aphrody.com/login", 403, "Just a moment...")
            .expect("handoff returns cleanly");
        assert!(gate.is_some());
        assert!(core.active_handoff.is_some());

        core.controller.har.start(HarContentMode::Full, true);
        core.controller.routes.add_route(
            "**/v1/token",
            RouteDisposition::Fulfill,
            Some(RouteResponse {
                status: 200,
                body: "{\"issued\":true}".to_owned(),
                ..Default::default()
            }),
            None,
            Vec::new(),
        );
        let mut req_headers = BTreeMap::new();
        req_headers.insert("Cookie".to_owned(), "session=xyz".to_owned());
        let matched = core.record_network_exchange(
            "r-1",
            42,
            3.2,
            "GET",
            "https://api.aphrody.com/v1/token",
            Some("fetch"),
            req_headers,
            BTreeMap::new(),
            500,
            None,
            "application/json",
        );
        assert!(matched.is_some());
        let har = core
            .controller
            .har
            .export_har_1_2("https://aphrody.com", "Login");
        assert_eq!(
            har["log"]["entries"][0]["request"]["headers"][0]["value"],
            "[REDACTED]"
        );
        assert_eq!(har["log"]["entries"][0]["response"]["status"], 200);
    }

    #[test]
    fn sync_with_browser_view_model_keeps_tabs_aligned() {
        let mut core = AgentBrowserCore::new();
        let mut model = BrowserViewModel::default();
        let tab = core
            .open_synced_tab(
                &mut model,
                "Docs",
                "https://aphrody.com/docs",
                RouteMode::SystemTunnel,
            )
            .expect("tab opened");
        assert_eq!(tab.tab_ref.as_str(), "t2");
        assert_eq!(model.tabs.len(), 2);
        assert!(core.telemetry_summary().contains("Tab t2 (2 open)"));
    }
}
