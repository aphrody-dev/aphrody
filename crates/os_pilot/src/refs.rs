use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};

use crate::model::{AccessibilityNode, ElementState, ErrorCode, PilotError, Rect, SnapshotSurface};

const MAX_SAVED_SNAPSHOTS: usize = 128;
const PRUNE_LOW_WATER: usize = 96;
const LATEST_SNAPSHOT_FILE: &str = "latest_snapshot_id";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityMatch {
    Match,
    NoMatch,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefScope {
    pub app: String,
    pub window_id: String,
    pub surface: SnapshotSurface,
    pub pid: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RefEntryIdentity {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub ancestor_path_hash: u64,
    pub child_index: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefCapabilities {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    pub interactive: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RefEntry {
    pub ref_id: String,
    pub node_id: String,
    pub scope: Option<RefScope>,
    pub identity: RefEntryIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Rect>,
    pub capabilities: RefCapabilities,
    #[serde(default)]
    pub state: ElementState,
    #[serde(default)]
    pub depth: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RefMap {
    pub snapshot_id: String,
    pub created_ms: u64,
    pub scope: Option<RefScope>,
    pub entries: BTreeMap<String, RefEntry>,
}

impl RefMap {
    #[must_use]
    pub fn from_tree(
        snapshot_id: impl Into<String>,
        scope: Option<RefScope>,
        root: &AccessibilityNode,
        created_ms: u64,
    ) -> Self {
        let sid = snapshot_id.into();
        let mut entries = BTreeMap::new();
        let mut counter = 1_usize;
        collect_entries(
            &sid,
            &scope,
            root,
            0,
            0,
            0x9e37_79b9_7f4a_7c15,
            &mut counter,
            &mut entries,
        );
        Self {
            snapshot_id: sid,
            created_ms,
            scope,
            entries,
        }
    }

    #[must_use]
    pub fn get(&self, ref_id: &str) -> Option<&RefEntry> {
        let normalized = normalize_ref_token(ref_id);
        self.entries.get(normalized)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_entries(
    snapshot_id: &str,
    scope: &Option<RefScope>,
    node: &AccessibilityNode,
    depth: usize,
    child_index: usize,
    parent_hash: u64,
    counter: &mut usize,
    entries: &mut BTreeMap<String, RefEntry>,
) {
    let mut hasher = FxHasher::default();
    parent_hash.hash(&mut hasher);
    node.role.hash(&mut hasher);
    node.name.hash(&mut hasher);
    node.identifier.hash(&mut hasher);
    child_index.hash(&mut hasher);
    let path_hash = hasher.finish();

    if let Some(ref_id) = &node.ref_id {
        let key = normalize_ref_token(ref_id).to_owned();
        entries.insert(
            key.clone(),
            RefEntry {
                ref_id: format!("@{key}"),
                node_id: node.id.clone(),
                scope: scope.clone(),
                identity: RefEntryIdentity {
                    role: node.role.clone(),
                    name: node.name.clone(),
                    identifier: node.identifier.clone(),
                    description: node.description.clone(),
                    ancestor_path_hash: path_hash,
                    child_index,
                },
                bounds: node.bounds,
                capabilities: RefCapabilities {
                    actions: node.actions.clone(),
                    interactive: node.interactive,
                },
                state: node.state.clone(),
                depth,
            },
        );
    } else if node.interactive || !node.actions.is_empty() || node.name.is_some() {
        let key = format!("{snapshot_id}:e{}", *counter);
        *counter += 1;
        entries.insert(
            key.clone(),
            RefEntry {
                ref_id: format!("@{key}"),
                node_id: node.id.clone(),
                scope: scope.clone(),
                identity: RefEntryIdentity {
                    role: node.role.clone(),
                    name: node.name.clone(),
                    identifier: node.identifier.clone(),
                    description: node.description.clone(),
                    ancestor_path_hash: path_hash,
                    child_index,
                },
                bounds: node.bounds,
                capabilities: RefCapabilities {
                    actions: node.actions.clone(),
                    interactive: node.interactive,
                },
                state: node.state.clone(),
                depth,
            },
        );
    }

    for (idx, child) in node.children.iter().enumerate() {
        collect_entries(
            snapshot_id,
            scope,
            child,
            depth + 1,
            idx,
            path_hash,
            counter,
            entries,
        );
    }
}

/// Assigns deterministic snapshot ref IDs `@<snapshot_id>:e<n>` onto a tree and returns both the
/// mutated tree and the lookup [`RefMap`].
#[must_use]
pub fn stamp_tree_refs(
    snapshot_id: &str,
    scope: Option<RefScope>,
    mut root: AccessibilityNode,
    created_ms: u64,
) -> (AccessibilityNode, RefMap) {
    let mut counter = 1_usize;
    stamp_node(snapshot_id, &mut root, &mut counter);
    let map = RefMap::from_tree(snapshot_id, scope, &root, created_ms);
    (root, map)
}

fn stamp_node(snapshot_id: &str, node: &mut AccessibilityNode, counter: &mut usize) {
    let keep = node.interactive
        || !node.actions.is_empty()
        || node.name.as_deref().is_some_and(|n| !n.trim().is_empty())
        || node.identifier.is_some();
    if keep {
        node.ref_id = Some(format!("@{snapshot_id}:e{}", *counter));
        *counter += 1;
    }
    for child in &mut node.children {
        stamp_node(snapshot_id, child, counter);
    }
}

/// Verifies whether a candidate live node still matches the saved ref fingerprint (protecting
/// against OS list reordering, reused indices, and stale refs).
#[must_use]
pub fn verify_entry_identity(expected: &RefEntry, live: &AccessibilityNode) -> IdentityMatch {
    if !live.role.eq_ignore_ascii_case(&expected.identity.role) {
        return IdentityMatch::NoMatch;
    }
    if let (Some(exp_id), Some(live_id)) = (&expected.identity.identifier, &live.identifier) {
        if exp_id != live_id {
            return IdentityMatch::NoMatch;
        }
        return IdentityMatch::Match;
    }
    if let Some(exp_name) = &expected.identity.name {
        match &live.name {
            Some(live_name) if live_name == exp_name => return IdentityMatch::Match,
            Some(_) => return IdentityMatch::NoMatch,
            None => return IdentityMatch::Unknown,
        }
    }
    if let (Some(a), Some(b)) = (expected.bounds, live.bounds) {
        let dx = (a.x - b.x).abs();
        let dy = (a.y - b.y).abs();
        if dx <= 24.0 && dy <= 24.0 {
            return IdentityMatch::Match;
        }
    }
    IdentityMatch::Unknown
}

#[must_use]
fn wasm_safe_pid() -> u32 {
    #[cfg(not(target_family = "wasm"))]
    {
        std::process::id()
    }
    #[cfg(target_family = "wasm")]
    {
        1
    }
}

#[must_use]
pub fn new_snapshot_id(seed: u64) -> String {
    let mut hasher = FxHasher::default();
    seed.hash(&mut hasher);
    wasm_safe_pid().hash(&mut hasher);
    let hash = hasher.finish();
    format!("s{:08x}", (hash & 0xffff_ffff) as u32)
}

#[must_use]
pub fn normalize_ref_token(raw: &str) -> &str {
    raw.trim().strip_prefix('@').unwrap_or(raw.trim())
}

pub fn validate_ref_id(raw: &str) -> Result<String, PilotError> {
    let token = normalize_ref_token(raw);
    let Some((snap, elem)) = token.split_once(':') else {
        return Err(PilotError::invalid_input(format!(
            "invalid ref '{raw}': expected '@<snapshot_id>:e<index>'"
        ))
        .with_suggestion("Pass a ref emitted by `snapshot`, e.g. '@s12ab34cd:e1'."));
    };
    validate_snapshot_id(snap)?;
    if !elem.starts_with('e') || elem[1..].parse::<usize>().is_err() {
        return Err(PilotError::invalid_input(format!(
            "invalid element index in ref '{raw}'"
        )));
    }
    Ok(format!("@{snap}:{elem}"))
}

pub fn validate_snapshot_id(snapshot_id: &str) -> Result<(), PilotError> {
    let clean = snapshot_id.trim();
    if clean.len() < 3
        || !clean.starts_with('s')
        || !clean[1..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(PilotError::invalid_input(format!(
            "invalid snapshot id '{snapshot_id}'"
        )));
    }
    Ok(())
}

pub fn validate_session_id(session_id: &str) -> Result<(), PilotError> {
    if session_id.is_empty()
        || session_id.len() > 64
        || !session_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(PilotError::invalid_input(format!(
            "invalid session id '{session_id}': use 1..=64 ASCII alphanumeric, '-' or '_'"
        )));
    }
    Ok(())
}

/// Session-scoped on-disk + in-memory snapshot ref store (wasm-safe in-memory fallback when no
/// host filesystem state dir is available).
#[derive(Debug, Clone)]
pub struct RefStore {
    base_dir: PathBuf,
}

impl RefStore {
    pub fn for_session(session_id: Option<&str>) -> Result<Self, PilotError> {
        let root = resolve_state_root();
        if let Some(sid) = session_id {
            validate_session_id(sid)?;
            return Ok(Self {
                base_dir: root.join("sessions").join(sid),
            });
        }
        Ok(Self { base_dir: root })
    }

    pub fn at_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    pub fn save_snapshot(&self, map: &RefMap) -> Result<(), PilotError> {
        validate_snapshot_id(&map.snapshot_id)?;
        let snap_dir = self.base_dir.join("snapshots");
        std::fs::create_dir_all(&snap_dir).map_err(|err| {
            PilotError::new(
                ErrorCode::Internal,
                format!(
                    "failed to create snapshot dir {}: {err}",
                    snap_dir.display()
                ),
            )
        })?;
        let target = snap_dir.join(format!("{}.json", map.snapshot_id));
        let tmp = snap_dir.join(format!("{}.tmp.{}", map.snapshot_id, wasm_safe_pid()));
        let bytes = serde_json::to_vec(map).map_err(|err| {
            PilotError::new(ErrorCode::Internal, format!("serialize refmap: {err}"))
        })?;
        std::fs::write(&tmp, bytes).map_err(|err| {
            PilotError::new(ErrorCode::Internal, format!("write refmap temp: {err}"))
        })?;
        std::fs::rename(&tmp, &target)
            .map_err(|err| PilotError::new(ErrorCode::Internal, format!("commit refmap: {err}")))?;
        let _ = std::fs::write(
            self.base_dir.join(LATEST_SNAPSHOT_FILE),
            map.snapshot_id.as_bytes(),
        );
        self.prune_old_snapshots(&snap_dir);
        Ok(())
    }

    pub fn load_snapshot(&self, snapshot_id: &str) -> Result<RefMap, PilotError> {
        validate_snapshot_id(snapshot_id)?;
        let path = self
            .base_dir
            .join("snapshots")
            .join(format!("{snapshot_id}.json"));
        let bytes = std::fs::read(&path).map_err(|err| {
            PilotError::new(
                ErrorCode::StaleRef,
                format!("snapshot '{snapshot_id}' not found in store: {err}"),
            )
            .with_suggestion("Capture a fresh `snapshot` before resolving refs.")
        })?;
        serde_json::from_slice(&bytes).map_err(|err| {
            PilotError::new(
                ErrorCode::Internal,
                format!("corrupt snapshot '{snapshot_id}': {err}"),
            )
        })
    }

    pub fn load_latest_snapshot(&self) -> Result<RefMap, PilotError> {
        let latest_path = self.base_dir.join(LATEST_SNAPSHOT_FILE);
        let id = std::fs::read_to_string(&latest_path).map_err(|_| {
            PilotError::new(
                ErrorCode::ElementNotFound,
                "no snapshot exists yet in this session",
            )
            .with_suggestion("Run `snapshot` first.")
        })?;
        self.load_snapshot(id.trim())
    }

    pub fn resolve_ref(&self, ref_id: &str) -> Result<(RefMap, RefEntry), PilotError> {
        let normalized = validate_ref_id(ref_id)?;
        let token = normalize_ref_token(&normalized);
        let (snap_id, _) = token
            .split_once(':')
            .ok_or_else(|| PilotError::invalid_input(format!("invalid ref '{ref_id}'")))?;
        let map = self.load_snapshot(snap_id)?;
        let entry = map
            .get(&normalized)
            .cloned()
            .ok_or_else(|| PilotError::stale_ref(ref_id))?;
        Ok((map, entry))
    }

    #[must_use]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    fn prune_old_snapshots(&self, snap_dir: &Path) {
        let Ok(entries) = std::fs::read_dir(snap_dir) else {
            return;
        };
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("json") {
                    return None;
                }
                let modified = e
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                Some((modified, p))
            })
            .collect();
        if files.len() <= MAX_SAVED_SNAPSHOTS {
            return;
        }
        files.sort_by_key(|(ts, _)| *ts);
        let remove_count = files.len().saturating_sub(PRUNE_LOW_WATER);
        for (_, old) in files.into_iter().take(remove_count) {
            let _ = std::fs::remove_file(old);
        }
    }
}

#[must_use]
pub fn resolve_state_root() -> PathBuf {
    if let Ok(override_dir) = std::env::var("APHRODY_PILOT_STATE_DIR")
        && !override_dir.trim().is_empty()
    {
        return PathBuf::from(override_dir);
    }
    #[cfg(not(target_family = "wasm"))]
    {
        std::env::temp_dir().join(format!("{}-pilot", brand::APP_SLUG))
    }
    #[cfg(target_family = "wasm")]
    {
        PathBuf::from(format!("/tmp/{}-pilot", brand::APP_SLUG))
    }
}
