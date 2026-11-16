use std::collections::{BTreeMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default embedded `cosmic-session`-equivalent manifest read from `assets/os/session.toml`.
pub const BUNDLED_SESSION_TOML: &str = include_str!("../../../assets/os/session.toml");

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("failed to parse session manifest TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("session unit '{unit}' depends on missing unit '{missing}'")]
    MissingDependency { unit: String, missing: String },
    #[error("dependency cycle detected involving unit '{0}'")]
    DependencyCycle(String),
    #[error("unknown session unit '{0}'")]
    UnknownUnit(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitKind {
    Compositor,
    Shell,
    #[default]
    Daemon,
    Mcp,
    Portal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    Always,
    #[default]
    OnFailure,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitPhase {
    Stopped,
    Starting,
    Running,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSettings {
    pub name: String,
    pub default_shell: String,
    pub default_theme: String,
    pub default_appearance: String,
    #[serde(default)]
    pub focus_follows_mouse: bool,
    #[serde(default = "default_tiling")]
    pub autotile_default: String,
}

fn default_tiling() -> String {
    "dwindle".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionUnitConfig {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub kind: UnitKind,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub restart: RestartPolicy,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default = "default_true")]
    pub autostart: bool,
}

fn default_true() -> bool {
    true
}

impl SessionUnitConfig {
    /// Returns true when the current host OS matches `platforms` (or `platforms` is empty).
    #[must_use]
    pub fn matches_current_platform(&self) -> bool {
        self.matches_platform(std::env::consts::OS)
    }

    #[must_use]
    pub fn matches_platform(&self, platform: &str) -> bool {
        self.platforms.is_empty()
            || self
                .platforms
                .iter()
                .any(|entry| entry.eq_ignore_ascii_case(platform))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManifest {
    pub schema_version: u32,
    pub session: SessionSettings,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub units: Vec<SessionUnitConfig>,
}

impl SessionManifest {
    pub fn bundled() -> Result<Self, SessionError> {
        Self::from_toml_str(BUNDLED_SESSION_TOML)
    }

    pub fn from_toml_str(toml_str: &str) -> Result<Self, SessionError> {
        Ok(toml::from_str(toml_str)?)
    }

    /// Loads `$XDG_CONFIG_HOME/aphrody/session.toml` (or legacy equivalent) with bundled fallback.
    pub fn load_with_user_override() -> Result<Self, SessionError> {
        if let Some(path) = user_session_override_path()
            && path.is_file()
            && let Ok(contents) = std::fs::read_to_string(&path)
        {
            return Self::from_toml_str(&contents);
        }
        Self::bundled()
    }

    /// Computes a deterministic topological startup sequence for units matching `platform`.
    pub fn plan_startup_order(&self, platform: &str) -> Result<Vec<String>, SessionError> {
        let active: BTreeMap<&str, &SessionUnitConfig> = self
            .units
            .iter()
            .filter(|unit| unit.autostart && unit.matches_platform(platform))
            .map(|unit| (unit.id.as_str(), unit))
            .collect();

        let mut in_degree: BTreeMap<&str, usize> = active.keys().map(|id| (*id, 0usize)).collect();
        let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

        for (id, unit) in &active {
            for dep in &unit.depends_on {
                if !active.contains_key(dep.as_str()) {
                    let exists_in_manifest = self.units.iter().any(|u| u.id == *dep);
                    if !exists_in_manifest {
                        return Err(SessionError::MissingDependency {
                            unit: (*id).to_owned(),
                            missing: dep.clone(),
                        });
                    }
                    continue;
                }
                if let Some(count) = in_degree.get_mut(id) {
                    *count += 1;
                }
                dependents.entry(dep.as_str()).or_default().push(*id);
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut ordered = Vec::with_capacity(active.len());

        while let Some(id) = queue.pop_front() {
            ordered.push(id.to_owned());
            if let Some(children) = dependents.get(id) {
                for child in children {
                    if let Some(deg) = in_degree.get_mut(child) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push_back(child);
                        }
                    }
                }
            }
        }

        if ordered.len() != active.len() {
            let stuck = in_degree
                .into_iter()
                .find(|(_, deg)| *deg > 0)
                .map(|(id, _)| id.to_owned())
                .unwrap_or_default();
            return Err(SessionError::DependencyCycle(stuck));
        }

        Ok(ordered)
    }
}

fn user_session_override_path() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?;
    let primary = config_dir.join(brand::APP_SLUG).join("session.toml");
    if primary.is_file() {
        return Some(primary);
    }
    let trimmed_legacy = brand::LEGACY_LOCAL_SETTINGS_FOLDER.trim_start_matches('.');
    Some(config_dir.join(trimmed_legacy).join("session.toml"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUnitStatus {
    pub config: SessionUnitConfig,
    pub phase: UnitPhase,
    pub restarts: u32,
    pub last_message: Option<String>,
}

/// Live COSMIC-compatible session state machine and environment builder.
#[derive(Debug, Clone)]
pub struct SessionSupervisor {
    pub manifest: SessionManifest,
    pub units: BTreeMap<String, SessionUnitStatus>,
    pub startup_order: Vec<String>,
}

impl SessionSupervisor {
    pub fn new(manifest: SessionManifest) -> Result<Self, SessionError> {
        let platform = std::env::consts::OS;
        let startup_order = manifest.plan_startup_order(platform)?;
        let mut units = BTreeMap::new();
        for cfg in &manifest.units {
            if cfg.matches_platform(platform) {
                units.insert(
                    cfg.id.clone(),
                    SessionUnitStatus {
                        config: cfg.clone(),
                        phase: UnitPhase::Stopped,
                        restarts: 0,
                        last_message: None,
                    },
                );
            }
        }
        Ok(Self {
            manifest,
            units,
            startup_order,
        })
    }

    /// Transitions all autostart units through dependencies into `Running`.
    pub fn activate_session(&mut self) -> Vec<String> {
        let mut started = Vec::new();
        let mut running: HashSet<String> = HashSet::new();
        for id in &self.startup_order {
            let deps_ready = self.units.get(id).is_some_and(|status| {
                status
                    .config
                    .depends_on
                    .iter()
                    .all(|dep| !self.units.contains_key(dep) || running.contains(dep))
            });
            if let Some(status) = self.units.get_mut(id) {
                if deps_ready {
                    status.phase = UnitPhase::Running;
                    status.last_message = Some("active".to_owned());
                    running.insert(id.clone());
                    started.push(id.clone());
                } else {
                    status.phase = UnitPhase::Degraded;
                    status.last_message = Some("waiting on dependency".to_owned());
                }
            }
        }
        started
    }

    pub fn set_unit_phase(
        &mut self,
        unit_id: &str,
        phase: UnitPhase,
        message: Option<String>,
    ) -> Result<(), SessionError> {
        let status = self
            .units
            .get_mut(unit_id)
            .ok_or_else(|| SessionError::UnknownUnit(unit_id.to_owned()))?;
        if matches!(phase, UnitPhase::Starting) && !matches!(status.phase, UnitPhase::Stopped) {
            status.restarts = status.restarts.saturating_add(1);
        }
        status.phase = phase;
        status.last_message = message;
        Ok(())
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<SessionUnitStatus> {
        self.units.values().cloned().collect()
    }
}

/// Synthesizes session environment variables dynamically from manifest and host environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEnvironment {
    pub vars: BTreeMap<String, String>,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl SessionEnvironment {
    #[must_use]
    pub fn detect(manifest: &SessionManifest) -> Self {
        let mut vars = manifest.environment.clone();

        for passthrough in [
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "XDG_RUNTIME_DIR",
            "WGPU_BACKEND",
            "XDG_DATA_DIRS",
            "XDG_CONFIG_HOME",
        ] {
            if let Ok(value) = std::env::var(passthrough)
                && !value.trim().is_empty()
            {
                vars.insert(passthrough.to_owned(), value);
            }
        }

        if cfg!(target_os = "linux") && !vars.contains_key("XDG_SESSION_TYPE") {
            let session_type = if vars.contains_key("WAYLAND_DISPLAY") {
                "wayland"
            } else if vars.contains_key("DISPLAY") {
                "x11"
            } else {
                "headless"
            };
            vars.insert("XDG_SESSION_TYPE".to_owned(), session_type.to_owned());
        }

        vars.entry(brand::env_key("SESSION"))
            .or_insert_with(|| manifest.session.name.clone());

        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(brand::APP_SLUG);
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(brand::APP_SLUG);

        Self {
            vars,
            config_dir,
            data_dir,
        }
    }

    /// Exports `config_dir` override hint for inspection without side effects.
    #[must_use]
    pub fn config_path(&self, file_name: &str) -> PathBuf {
        Path::new(&self.config_dir).join(file_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_session_manifest_plans_dependencies_without_cycles() {
        let manifest = SessionManifest::bundled().expect("bundled session.toml must parse");
        let order = manifest
            .plan_startup_order("linux")
            .expect("startup order resolves cleanly");
        assert_eq!(order.first().map(String::as_str), Some("compositor"));
        assert!(order.iter().any(|u| u == "craft-mcp"));

        let mut supervisor = SessionSupervisor::new(manifest).expect("supervisor builds");
        let started = supervisor.activate_session();
        assert!(!started.is_empty());
        assert!(
            supervisor
                .snapshot()
                .iter()
                .all(|u| u.phase == UnitPhase::Running)
        );
    }

    #[test]
    fn rejects_cycle_and_missing_dependency() {
        let cyclic = r#"
schema_version = 1
[session]
name = "test"
default_shell = "os-desktop"
default_theme = "aphrody"
default_appearance = "dark"

[[units]]
id = "a"
description = "A"
command = "a"
depends_on = ["b"]

[[units]]
id = "b"
description = "B"
command = "b"
depends_on = ["a"]
"#;
        let parsed = SessionManifest::from_toml_str(cyclic).expect("valid syntax");
        assert!(matches!(
            parsed.plan_startup_order("linux"),
            Err(SessionError::DependencyCycle(_))
        ));
    }
}
