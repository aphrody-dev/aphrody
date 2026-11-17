//! Pure-Rust installer orchestrator, partition & Btrfs subvolume planner, LUKS2 setup,
//! CachyOS `settings.json` compatibility, System Rescue plan, and live DE Switcher planner.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::profiles::{
    BootloaderKind, FilesystemKind, InstallerProfileCatalog, SUPPORTED_KERNELS,
};

#[derive(Debug, Error)]
pub enum PlannerError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("unknown desktop environment or stack '{0}'")]
    UnknownEnvironment(String),
    #[error("JSON config error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallTargetMode {
    #[default]
    Desktop,
    Server,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionEntry {
    pub name: String,
    pub mountpoint: String,
    pub size: String,
    #[serde(default)]
    pub fs_name: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BtrfsSubvolume {
    pub subvol: String,
    pub mountpoint: String,
    pub compress_zstd: bool,
}

#[must_use]
pub fn default_btrfs_subvolumes() -> Vec<BtrfsSubvolume> {
    vec![
        BtrfsSubvolume {
            subvol: "@".to_owned(),
            mountpoint: "/".to_owned(),
            compress_zstd: true,
        },
        BtrfsSubvolume {
            subvol: "@home".to_owned(),
            mountpoint: "/home".to_owned(),
            compress_zstd: true,
        },
        BtrfsSubvolume {
            subvol: "@root".to_owned(),
            mountpoint: "/root".to_owned(),
            compress_zstd: true,
        },
        BtrfsSubvolume {
            subvol: "@srv".to_owned(),
            mountpoint: "/srv".to_owned(),
            compress_zstd: true,
        },
        BtrfsSubvolume {
            subvol: "@cache".to_owned(),
            mountpoint: "/var/cache".to_owned(),
            compress_zstd: false,
        },
        BtrfsSubvolume {
            subvol: "@tmp".to_owned(),
            mountpoint: "/var/tmp".to_owned(),
            compress_zstd: false,
        },
        BtrfsSubvolume {
            subvol: "@log".to_owned(),
            mountpoint: "/var/log".to_owned(),
            compress_zstd: true,
        },
    ]
}

/// Compatible superset of `vendor/new-cli-installer/settings.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallerConfig {
    pub device: String,
    pub fs_name: String,
    pub partitions: Vec<PartitionEntry>,
    pub hostname: String,
    pub locale: String,
    pub xkbmap: String,
    pub timezone: String,
    pub user_name: String,
    #[serde(default = "default_shell")]
    pub user_shell: String,
    pub kernel: String,
    pub desktop: String,
    pub bootloader: String,
    #[serde(default)]
    pub target_mode: InstallTargetMode,
    #[serde(default = "default_server_profile")]
    pub server_profile: String,
    #[serde(default)]
    pub encrypt_luks: bool,
    #[serde(default = "default_true")]
    pub zram_swap: bool,
    #[serde(default = "default_true")]
    pub chwd_auto_drivers: bool,
    #[serde(default)]
    pub autologin: bool,
    #[serde(default)]
    pub optional_groups: Vec<String>,
}

fn default_shell() -> String {
    "/usr/bin/fish".to_owned()
}

fn default_server_profile() -> String {
    "minimal".to_owned()
}

const fn default_true() -> bool {
    true
}

impl Default for InstallerConfig {
    fn default() -> Self {
        Self {
            device: "/dev/nvme0n1".to_owned(),
            fs_name: FilesystemKind::Btrfs.slug().to_owned(),
            partitions: vec![
                PartitionEntry {
                    name: "/dev/nvme0n1p1".to_owned(),
                    mountpoint: "/boot".to_owned(),
                    size: "1024M".to_owned(),
                    fs_name: Some("vfat".to_owned()),
                    kind: "boot".to_owned(),
                },
                PartitionEntry {
                    name: "/dev/nvme0n1p2".to_owned(),
                    mountpoint: "/".to_owned(),
                    size: "100%".to_owned(),
                    fs_name: None,
                    kind: "root".to_owned(),
                },
            ],
            hostname: "aphrody-os".to_owned(),
            locale: "en_US.UTF-8".to_owned(),
            xkbmap: "us".to_owned(),
            timezone: "UTC".to_owned(),
            user_name: "aphrody".to_owned(),
            user_shell: default_shell(),
            kernel: "linux-aphrody".to_owned(),
            desktop: "aphrody-gpui".to_owned(),
            bootloader: BootloaderKind::SystemdBoot.slug().to_owned(),
            target_mode: InstallTargetMode::Desktop,
            server_profile: default_server_profile(),
            encrypt_luks: false,
            zram_swap: true,
            chwd_auto_drivers: true,
            autologin: false,
            optional_groups: vec!["Multimedia & Codecs".to_owned()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightReport {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallStepSpec {
    pub id: String,
    pub phase: String,
    pub title: String,
    pub command_preview: String,
    pub fatal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallExecutionPlan {
    pub preflight: PreflightReport,
    pub packages: Vec<String>,
    pub enabled_units: Vec<String>,
    pub btrfs_subvolumes: Vec<BtrfsSubvolume>,
    pub steps: Vec<InstallStepSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopSwitchPlan {
    pub from_env: String,
    pub to_env: String,
    pub toolkit_badge: String,
    pub display_manager: String,
    pub session_exec: String,
    pub install_packages: Vec<String>,
    pub enable_units: Vec<String>,
    pub commands: Vec<String>,
}

pub fn validate_config(
    cfg: &InstallerConfig,
    catalog: &InstallerProfileCatalog,
) -> PreflightReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if cfg.device.trim().is_empty() {
        errors.push("Target block device cannot be empty.".to_owned());
    }
    if FilesystemKind::parse(&cfg.fs_name).is_none() {
        errors.push(format!("Unsupported filesystem '{}'.", cfg.fs_name));
    }
    if BootloaderKind::parse(&cfg.bootloader).is_none() {
        errors.push(format!("Unsupported bootloader '{}'.", cfg.bootloader));
    }
    if !SUPPORTED_KERNELS.iter().any(|k| k.id == cfg.kernel) {
        errors.push(format!("Unknown kernel '{}'.", cfg.kernel));
    }
    let has_root = cfg.partitions.iter().any(|p| p.mountpoint == "/");
    if !has_root {
        errors.push("Partition layout must declare a '/' root partition.".to_owned());
    }
    let has_boot = cfg
        .partitions
        .iter()
        .any(|p| p.kind == "boot" || p.mountpoint == "/boot" || p.mountpoint == "/boot/efi");
    if !has_boot {
        warnings.push("No separate EFI /boot partition declared (assumed BIOS MBR).".to_owned());
    }
    if cfg.hostname.trim().is_empty() {
        errors.push("Hostname cannot be empty.".to_owned());
    }
    if cfg.target_mode == InstallTargetMode::Desktop && catalog.environment(&cfg.desktop).is_none()
    {
        errors.push(format!("Unknown desktop environment '{}'.", cfg.desktop));
    }
    if cfg.encrypt_luks && cfg.bootloader == BootloaderKind::SystemdBoot.slug() {
        warnings.push("LUKS2 + systemd-boot uses sd-encrypt initramfs hook.".to_owned());
    }

    PreflightReport {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

/// Generates an end-to-end deterministic installation execution plan mirroring
/// `cachyos/new-cli-installer/installer-lib/src/orchestrator.cpp::plan_steps`.
#[must_use]
pub fn plan_installation(
    cfg: &InstallerConfig,
    catalog: &InstallerProfileCatalog,
) -> InstallExecutionPlan {
    let preflight = validate_config(cfg, catalog);
    let mut pkg_set = BTreeSet::new();
    for p in &catalog.base_cli_packages {
        pkg_set.insert(p.clone());
    }
    if let Some(kernel) = SUPPORTED_KERNELS.iter().find(|k| k.id == cfg.kernel) {
        for p in kernel.packages {
            pkg_set.insert((*p).to_owned());
        }
    }

    let mut units = BTreeSet::new();
    for u in &catalog.base_services {
        if u.action == "enable" {
            units.insert(u.name.clone());
        }
    }

    match cfg.target_mode {
        InstallTargetMode::Desktop => {
            for p in &catalog.base_desktop_packages {
                pkg_set.insert(p.clone());
            }
            if let Some(env) = catalog.environment(&cfg.desktop) {
                for p in &env.packages {
                    pkg_set.insert(p.clone());
                }
                if !env.display_manager.is_empty() {
                    units.insert(env.display_manager.clone());
                }
            }
            for grp in &catalog.netinstall_groups {
                if cfg
                    .optional_groups
                    .iter()
                    .any(|g| g.eq_ignore_ascii_case(&grp.name))
                {
                    for p in &grp.packages {
                        pkg_set.insert(p.clone());
                    }
                }
            }
            for u in &catalog.desktop_services {
                if u.action == "enable" {
                    units.insert(u.name.clone());
                }
            }
        }
        InstallTargetMode::Server => {
            for p in &catalog.server_base_packages {
                pkg_set.insert(p.clone());
            }
            for u in &catalog.server_base_units {
                if u.action == "enable" {
                    units.insert(u.name.clone());
                }
            }
            if let Some(srv) = catalog
                .server_profiles
                .iter()
                .find(|s| s.id == cfg.server_profile)
            {
                for p in &srv.packages {
                    pkg_set.insert(p.clone());
                }
                for u in &srv.units {
                    if u.action == "enable" {
                        units.insert(u.name.clone());
                    }
                }
            }
        }
    }

    let btrfs_subvolumes = if cfg.fs_name.eq_ignore_ascii_case("btrfs") {
        default_btrfs_subvolumes()
    } else {
        Vec::new()
    };

    let packages: Vec<String> = pkg_set.into_iter().collect();
    let enabled_units: Vec<String> = units.into_iter().collect();

    let mut steps = vec![
        InstallStepSpec {
            id: "partition".to_owned(),
            phase: "Prepare".to_owned(),
            title: format!("Prepare {} ({})", cfg.device, cfg.fs_name),
            command_preview: if cfg.encrypt_luks {
                format!(
                    "cryptsetup luksFormat {} && mkfs.{} /dev/mapper/cryptroot",
                    cfg.device, cfg.fs_name
                )
            } else {
                format!("sgdisk --zap-all {} && mkfs.{}", cfg.device, cfg.fs_name)
            },
            fatal: true,
        },
        InstallStepSpec {
            id: "base".to_owned(),
            phase: "Install".to_owned(),
            title: format!("Pacstrap Base + Kernel ({})", cfg.kernel),
            command_preview: format!(
                "pacstrap -K /mnt base {} ({} total packages)",
                cfg.kernel,
                packages.len()
            ),
            fatal: true,
        },
        InstallStepSpec {
            id: "fstab".to_owned(),
            phase: "Configure".to_owned(),
            title: "Generate /etc/fstab & Machine ID".to_owned(),
            command_preview: "genfstab -U /mnt >> /mnt/etc/fstab && systemd-machine-id-setup --root=/mnt".to_owned(),
            fatal: true,
        },
        InstallStepSpec {
            id: "system_settings".to_owned(),
            phase: "Configure".to_owned(),
            title: format!("Hostname '{}', Locale '{}', Keymap '{}'", cfg.hostname, cfg.locale, cfg.xkbmap),
            command_preview: format!(
                "echo {} > /mnt/etc/hostname && timedatectl --root=/mnt set-timezone {}",
                cfg.hostname, cfg.timezone
            ),
            fatal: true,
        },
        InstallStepSpec {
            id: "users".to_owned(),
            phase: "Configure".to_owned(),
            title: format!("Create User '{}' ({})", cfg.user_name, cfg.user_shell),
            command_preview: format!(
                "arch-chroot /mnt useradd -mG wheel,video,input -s {} {}",
                cfg.user_shell, cfg.user_name
            ),
            fatal: true,
        },
    ];

    if cfg.target_mode == InstallTargetMode::Desktop {
        steps.push(InstallStepSpec {
            id: "desktop".to_owned(),
            phase: "Desktop".to_owned(),
            title: format!("Configure Desktop Environment ({})", cfg.desktop),
            command_preview: format!("aphrody-os session activate --env {}", cfg.desktop),
            fatal: false,
        });
    }

    if cfg.chwd_auto_drivers {
        steps.push(InstallStepSpec {
            id: "chwd".to_owned(),
            phase: "Hardware".to_owned(),
            title: "CHWD Auto-Detect GPU / Network Drivers".to_owned(),
            command_preview: "arch-chroot /mnt chwd -a".to_owned(),
            fatal: false,
        });
    }

    steps.push(InstallStepSpec {
        id: "initramfs".to_owned(),
        phase: "Boot".to_owned(),
        title: "Build Initramfs (mkinitcpio / booster)".to_owned(),
        command_preview: "arch-chroot /mnt mkinitcpio -P".to_owned(),
        fatal: true,
    });
    steps.push(InstallStepSpec {
        id: "bootloader".to_owned(),
        phase: "Boot".to_owned(),
        title: format!("Install Bootloader ({})", cfg.bootloader),
        command_preview: match BootloaderKind::parse(&cfg.bootloader) {
            Some(BootloaderKind::SystemdBoot) => "arch-chroot /mnt bootctl install".to_owned(),
            Some(BootloaderKind::Grub) => {
                "arch-chroot /mnt grub-install --target=x86_64-efi --efi-directory=/boot".to_owned()
            }
            Some(BootloaderKind::Refind) => "arch-chroot /mnt refind-install".to_owned(),
            Some(BootloaderKind::Limine) => "arch-chroot /mnt limine-install".to_owned(),
            None => "bootctl install".to_owned(),
        },
        fatal: true,
    });
    steps.push(InstallStepSpec {
        id: "enable_services".to_owned(),
        phase: "Services".to_owned(),
        title: format!("Enable {} systemd units", enabled_units.len()),
        command_preview: format!("arch-chroot /mnt systemctl enable {}", enabled_units.join(" ")),
        fatal: false,
    });
    steps.push(InstallStepSpec {
        id: "final_validation".to_owned(),
        phase: "Verify".to_owned(),
        title: "Final Bootloader, LUKS & Portal Validation".to_owned(),
        command_preview: "aphrody-os doctor --verify-chroot /mnt".to_owned(),
        fatal: false,
    });

    InstallExecutionPlan {
        preflight,
        packages,
        enabled_units,
        btrfs_subvolumes,
        steps,
    }
}

/// Plans a live, non-destructive switch between Aphrody Rust userland shells (`aphrody-gpui`,
/// `iced-cosmic`, `egui-craft`, `webgpu-os`) and Arch classic DEs (`gnome`, `kde`, `hyprland`, ...).
pub fn plan_desktop_switch(
    from_env: &str,
    to_env: &str,
    catalog: &InstallerProfileCatalog,
) -> Result<DesktopSwitchPlan, PlannerError> {
    let target = catalog
        .environment(to_env)
        .ok_or_else(|| PlannerError::UnknownEnvironment(to_env.to_owned()))?;

    let mut commands = Vec::new();
    if target.aphrody_userland {
        commands.push(format!(
            "aphrody-os desktop --session-exec \"{}\" --theme aphrody",
            target.session_exec
        ));
    } else if !target.packages.is_empty() {
        commands.push(format!(
            "sudo pacman -S --needed --noconfirm {}",
            target.packages.join(" ")
        ));
    }

    let mut enable_units = Vec::new();
    if !target.display_manager.is_empty() {
        enable_units.push(format!("{}.service", target.display_manager));
        commands.push(format!(
            "sudo systemctl enable --force {}.service",
            target.display_manager
        ));
    }
    commands.push(format!(
        "loginctl activate || exec {}",
        target.session_exec
    ));

    Ok(DesktopSwitchPlan {
        from_env: from_env.to_owned(),
        to_env: target.id.clone(),
        toolkit_badge: target.toolkit.badge().to_owned(),
        display_manager: target.display_manager.clone(),
        session_exec: target.session_exec.clone(),
        install_packages: target.packages.clone(),
        enable_units,
        commands,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RescueTaskSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub command_preview: &'static str,
}

pub const RESCUE_TASKS: &[RescueTaskSpec] = &[
    RescueTaskSpec {
        id: "chwd-drivers",
        title: "Reinstall Hardware Display Drivers (chwd -a)",
        description: "Auto-detect NVIDIA/AMD/Intel GPU & Wi-Fi drivers and regenerate initramfs.",
        command_preview: "chwd -a && mkinitcpio -P",
    },
    RescueTaskSpec {
        id: "rebuild-bootloader",
        title: "Repair Bootloader & EFI Entries",
        description: "Reinstall systemd-boot / GRUB / Limine entries and scan OS prober.",
        command_preview: "bootctl update || grub-mkconfig -o /boot/grub/grub.cfg",
    },
    RescueTaskSpec {
        id: "btrfs-rollback",
        title: "Inspect & Rollback Btrfs Snapper Snapshot",
        description: "List read-only Btrfs snapshots and restore healthy root subvolume.",
        command_preview: "snapper list && snapper rollback --ambit classic",
    },
    RescueTaskSpec {
        id: "pacman-keyring",
        title: "Refresh Pacman / Aphrody / CachyOS Keyring",
        description: "Repair corrupted GPG keys and re-rank fastest package mirrors.",
        command_preview: "pacman-key --init && pacman-key --populate archlinux cachyos",
    },
    RescueTaskSpec {
        id: "journal-logs",
        title: "Collect Boot, Compositor & Dmesg Logs",
        description: "Inspect kernel dmesg, cosmic-comp, and pacman logs in one view.",
        command_preview: "journalctl -b -p err..emerg --no-pager",
    },
];
