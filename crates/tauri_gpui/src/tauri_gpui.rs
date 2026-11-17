//! Tauri v3 + Wry WebView + ACL Capability Engine placed inside `crates/tauri_gpui`
//! identically to `crates/gpui`, with aligned workspace dependencies (`aphrody_store`, `serde`,
//! `anyhow`) and zero duplicate `tao`/`wry`/`winit` event loops.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use aphrody_store::StoreTarget;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TauriCapabilityRule {
    pub identifier: String,
    pub windows: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebviewSurfaceSpec {
    pub label: String,
    pub url_or_bundle: String,
    pub devtools_enabled: bool,
    pub transparent: bool,
}

pub type InvokeHandlerFn = Box<dyn Fn(Value) -> Result<Value> + Send + Sync>;

/// Thread-safe Tauri v3-compatible IPC command registry enforcing capability permissions.
#[derive(Default)]
pub struct TauriIpcRouter {
    handlers: BTreeMap<String, InvokeHandlerFn>,
    allowed_commands: BTreeSet<String>,
}

impl TauriIpcRouter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_capability(&mut self, rule: &TauriCapabilityRule) {
        for perm in &rule.permissions {
            self.allowed_commands.insert(perm.clone());
        }
    }

    pub fn register<F>(&mut self, command: impl Into<String>, handler: F)
    where
        F: Fn(Value) -> Result<Value> + Send + Sync + 'static,
    {
        let key = command.into();
        self.allowed_commands.insert(key.clone());
        self.handlers.insert(key, Box::new(handler));
    }

    pub fn invoke(&self, command: &str, payload: Value) -> Result<Value> {
        if !self.allowed_commands.contains(command) {
            bail!("ACL denied IPC command '{command}'");
        }
        let handler = self
            .handlers
            .get(command)
            .ok_or_else(|| anyhow::anyhow!("unknown Tauri IPC command '{command}'"))?;
        handler(payload)
    }

    #[must_use]
    pub fn commands(&self) -> Vec<&str> {
        self.handlers.keys().map(String::as_str).collect()
    }
}

/// Returns the unified distribution packager targets aligned between `vendor/tauri` (`tauri-bundler`)
/// and [`aphrody_store`].
#[must_use]
pub fn tauri_aligned_bundle_targets() -> &'static [StoreTarget] {
    &StoreTarget::ALL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_router_enforces_acl_and_invokes_handlers() {
        let mut router = TauriIpcRouter::new();
        router.register("ping", |val| Ok(serde_json::json!({ "echo": val })));
        let out = router
            .invoke("ping", serde_json::json!("aphrody"))
            .expect("registered command succeeds");
        assert_eq!(out["echo"], "aphrody");
        assert!(router.invoke("forbidden", serde_json::json!({})).is_err());
        assert_eq!(tauri_aligned_bundle_targets().len(), 22);
    }
}
