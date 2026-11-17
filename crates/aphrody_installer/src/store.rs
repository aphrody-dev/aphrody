//! Reactive global NSH signal store for `aphrody_installer`.

use std::collections::BTreeSet;

use nsh_core::prelude::*;

use crate::planner::{
    DesktopSwitchPlan, InstallExecutionPlan, InstallerConfig, InstallTargetMode,
    plan_desktop_switch, plan_installation,
};
use crate::profiles::{BootloaderKind, FilesystemKind, InstallerProfileCatalog};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallerTab {
    Guided,
    SwitchDe,
    Advanced,
    Rescue,
}

#[derive(Clone)]
struct InstallerStore {
    catalog: InstallerProfileCatalog,
    config: InstallerConfig,
    active_tab: InstallerTab,
    installing: bool,
    progress_pct: f32,
    status_line: String,
    logs: Vec<String>,
    last_switch: Option<DesktopSwitchPlan>,
}

impl Default for InstallerStore {
    fn default() -> Self {
        let catalog = InstallerProfileCatalog::bundled().unwrap_or_else(|_| {
            InstallerProfileCatalog::from_toml_strs(
                "[base-packages]\npackages = [\"base\"]\n",
                "[server]\ndefault_profile=\"minimal\"\ndefault_kernel=\"linux-cachyos-server\"\n[server.base]\n[server.profiles.minimal]\nname=\"Minimal\"\ndescription=\"Minimal\"\n",
            )
            .unwrap_or_else(|_| panic!("minimal fallback profile catalog must parse"))
        });
        Self {
            catalog,
            config: InstallerConfig::default(),
            active_tab: InstallerTab::Guided,
            installing: false,
            progress_pct: 0.0,
            status_line: "Ready — review profile or switch Rust DE".to_owned(),
            logs: vec!["Aphrody NSH Installer initialized (CachyOS Net-Profile + Rust Userlands).".to_owned()],
            last_switch: None,
        }
    }
}

fn store() -> Signal<InstallerStore> {
    global::<InstallerStore>()
}

pub(crate) fn active_tab() -> InstallerTab {
    store().with(|s| s.active_tab)
}

pub(crate) fn set_tab(tab: InstallerTab) {
    store().update(|s| s.active_tab = tab);
}

pub(crate) fn config_snapshot() -> InstallerConfig {
    store().with(|s| s.config.clone())
}

pub(crate) fn environments() -> Vec<crate::profiles::DesktopEnvironmentSpec> {
    store().with(|s| s.catalog.environments.clone())
}

pub(crate) fn server_profiles() -> Vec<crate::profiles::ServerProfileSpec> {
    store().with(|s| s.catalog.server_profiles.clone())
}

pub(crate) fn netinstall_groups() -> Vec<crate::profiles::NetinstallGroupSpec> {
    store().with(|s| s.catalog.netinstall_groups.clone())
}

pub(crate) fn set_desktop(id: &str) {
    let target = id.to_owned();
    store().update(|s| {
        let from = s.config.desktop.clone();
        s.config.target_mode = InstallTargetMode::Desktop;
        s.config.desktop = target.clone();
        if let Ok(plan) = plan_desktop_switch(&from, &target, &s.catalog) {
            s.status_line = format!("DE switched: {} -> {}", plan.from_env, plan.to_env);
            s.logs.push(format!(
                "DE Switch [{}]: session='{}' dm='{}'",
                plan.toolkit_badge, plan.session_exec, plan.display_manager
            ));
            s.last_switch = Some(plan);
        }
    });
}

pub(crate) fn set_target_mode(mode: InstallTargetMode) {
    store().update(|s| s.config.target_mode = mode);
}

pub(crate) fn set_server_profile(id: &str) {
    let id = id.to_owned();
    store().update(|s| {
        s.config.target_mode = InstallTargetMode::Server;
        s.config.server_profile = id;
    });
}

pub(crate) fn set_kernel(kernel_id: &str) {
    let id = kernel_id.to_owned();
    store().update(|s| s.config.kernel = id);
}

pub(crate) fn set_filesystem(fs: FilesystemKind) {
    store().update(|s| s.config.fs_name = fs.slug().to_owned());
}

pub(crate) fn set_bootloader(bl: BootloaderKind) {
    store().update(|s| s.config.bootloader = bl.slug().to_owned());
}

pub(crate) fn set_device(dev: &str) {
    let dev = dev.trim().to_owned();
    if dev.is_empty() {
        return;
    }
    store().update(|s| s.config.device = dev);
}

pub(crate) fn set_hostname(host: &str) {
    let host = host.trim().to_owned();
    if host.is_empty() {
        return;
    }
    store().update(|s| s.config.hostname = host);
}

pub(crate) fn set_encrypt_luks(enabled: bool) {
    store().update(|s| s.config.encrypt_luks = enabled);
}

pub(crate) fn set_zram_swap(enabled: bool) {
    store().update(|s| s.config.zram_swap = enabled);
}

pub(crate) fn set_chwd_drivers(enabled: bool) {
    store().update(|s| s.config.chwd_auto_drivers = enabled);
}

pub(crate) fn set_autologin(enabled: bool) {
    store().update(|s| s.config.autologin = enabled);
}

pub(crate) fn toggle_optional_group(name: &str) {
    let name = name.to_owned();
    store().update(|s| {
        let mut set: BTreeSet<String> = s.config.optional_groups.iter().cloned().collect();
        if set.contains(&name) {
            set.remove(&name);
        } else {
            set.insert(name);
        }
        s.config.optional_groups = set.into_iter().collect();
    });
}

pub(crate) fn execution_plan() -> InstallExecutionPlan {
    store().with(|s| plan_installation(&s.config, &s.catalog))
}

pub(crate) fn progress_pct() -> f32 {
    store().with(|s| s.progress_pct)
}

pub(crate) fn status_line() -> String {
    store().with(|s| s.status_line.clone())
}

pub(crate) fn logs() -> Vec<String> {
    store().with(|s| s.logs.clone())
}

pub(crate) fn run_dry_install() {
    store().update(|s| {
        let plan = plan_installation(&s.config, &s.catalog);
        if !plan.preflight.valid {
            s.status_line = format!("Preflight blocked: {}", plan.preflight.errors.join("; "));
            return;
        }
        s.installing = true;
        s.logs.clear();
        for (idx, step) in plan.steps.iter().enumerate() {
            s.logs
                .push(format!("[{}/{}] {}: {}", idx + 1, plan.steps.len(), step.phase, step.command_preview));
        }
        s.installing = false;
        s.progress_pct = 100.0;
        s.status_line = format!(
            "Dry-run plan ready: {} steps, {} packages, target={}",
            plan.steps.len(),
            plan.packages.len(),
            s.config.desktop
        );
    });
}

pub(crate) fn trigger_rescue(task_id: &str) {
    if let Some(task) = crate::planner::RESCUE_TASKS.iter().find(|t| t.id == task_id) {
        store().update(|s| {
            s.status_line = format!("Queued rescue: {}", task.title);
            s.logs.push(format!("[RESCUE] {}", task.command_preview));
        });
    }
}
