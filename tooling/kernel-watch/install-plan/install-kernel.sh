#!/bin/bash
# Installs a custom kernel package NEXT TO the distribution kernel on an Arch host, boots it once, and falls back to the old kernel by itself.
# Default is a dry run that only prints what it would do. Nothing is changed without --apply, and nothing here is meant for a build or test host:
# it is for the machine that should run the kernel, after the same kernel passed the lane on the build host.
#
# usage: install-kernel.sh --package FILE.pkg.tar.zst [--pkgbase NAME] [--apply] [--phase install|verify|promote|rollback]
#   install   snapshot (btrfs root only), pacman -U the package, rebuild the bootloader config, arm a ONE-TIME boot of the new kernel, print the reboot command
#   verify    after the reboot: prints the checklist results (running release, failed units, units that were active before, network, dmesg errors)
#   promote   makes the new kernel the default (only run after verify looked right)
#   rollback  re-arms the old kernel as default, removes the one-time entry, removes the package
# Bootloaders: GRUB (grub-reboot with GRUB_DEFAULT=saved) and systemd-boot (bootctl set-oneshot). The old default is never edited by install.
set -euo pipefail

apply=0; phase=install; package=""; pkgbase=""
while [ $# -gt 0 ]; do
  case $1 in
    --apply) apply=1 ;;
    --phase) phase=$2; shift ;;
    --package) package=$2; shift ;;
    --pkgbase) pkgbase=$2; shift ;;
    *) echo "unknown argument $1" >&2; exit 2 ;;
  esac
  shift
done
state=/var/lib/kernel-install-plan
run() { echo "+ $*"; if [ "$apply" = 1 ]; then "$@"; fi; }
note() { echo "# $*"; }

bootloader() {
  if [ -d /boot/loader/entries ] || [ -d /efi/loader/entries ]; then echo systemd-boot
  elif [ -f /boot/grub/grub.cfg ]; then echo grub
  else echo unknown; fi
}
pkgbase_of() { [ -n "$pkgbase" ] && { echo "$pkgbase"; return; }; bsdtar -xOf "$package" .PKGINFO | awk -F' = ' '$1=="pkgbase" {print $2}'; }

old_kernel=$(uname -r)
loader=$(bootloader)
note "running kernel $old_kernel; bootloader $loader; mode $([ "$apply" = 1 ] && echo APPLY || echo dry-run)"

case $phase in
  install)
    [ -n "$package" ] || { echo "--package is required for install" >&2; exit 2; }
    base=$(pkgbase_of)
    free_mb=$(stat -f -c '%a %S' /boot | awk '{print int($1 * $2 / 1048576)}')
    note "package base $base; free space on /boot $free_mb MiB (need about 300)"
    [ "$free_mb" -ge 300 ] || { echo "not enough space on /boot" >&2; exit 1; }
    run install -d "$state"
    if [ "$(findmnt -n -o FSTYPE /)" = btrfs ] && command -v btrfs >/dev/null; then
      note "root is btrfs: read-only snapshot before the change (remove it after promote)"
      run btrfs subvolume snapshot -r / "/pre-kernel-$(date +%Y%m%d%H%M)"
    fi
    note "units active before the change, for the verify phase"
    if [ "$apply" = 1 ]; then systemctl list-units --type=service --state=active --no-legend --plain | awk '{print $1}' | sort > "$state/active-before.txt"; echo "$old_kernel" > "$state/old-kernel"; fi
    run pacman -U --noconfirm "$package"
    case $loader in
      grub)
        note "GRUB_DEFAULT must be 'saved' so grub-reboot can arm a one-time entry; the current default entry stays the distribution kernel"
        run sed -i 's/^GRUB_DEFAULT=.*/GRUB_DEFAULT=saved/' /etc/default/grub
        run grub-mkconfig -o /boot/grub/grub.cfg
        note "check the entry title before arming it: grep -E 'menuentry|submenu' /boot/grub/grub.cfg (apply mode prints it here)"
        if [ "$apply" = 1 ]; then grep -E '^\s*(menuentry|submenu)' /boot/grub/grub.cfg | cut -c1-120; fi
        run grub-reboot "Advanced options for Arch Linux>Arch Linux, with Linux $base"
        ;;
      systemd-boot)
        run bootctl set-oneshot "$(basename "$(ls /boot/loader/entries/*"$base"*.conf | head -1)")"
        ;;
      *) echo "unknown bootloader: stop here and arm a one-time boot by hand" >&2; exit 1 ;;
    esac
    note "fallback: the one-time entry is consumed by the first boot, so any later reboot (panic=, watchdog, a power cycle from the provider) returns to the distribution kernel"
    note "the new kernel's command line should carry panic=15 so a kernel panic reboots into the old kernel without a human"
    note "next: reboot, then run this script with --phase verify"
    ;;
  verify)
    echo "running kernel: $(uname -r) (before: $(cat "$state/old-kernel" 2>/dev/null || echo unknown))"
    echo "failed units:"; systemctl --failed --no-legend --plain || true
    if [ -f "$state/active-before.txt" ]; then
      echo "units active before and not active now:"
      comm -23 "$state/active-before.txt" <(systemctl list-units --type=service --state=active --no-legend --plain | awk '{print $1}' | sort) || true
    fi
    echo "kernel errors:"; dmesg --level=err,crit,alert,emerg | head -20 || true
    echo "network: $(getent hosts archlinux.org >/dev/null && echo dns-ok || echo dns-FAILED)"
    echo "docker: $(systemctl is-active docker 2>/dev/null || true)"
    echo "decide: promote if the output is clean, otherwise reboot (the old kernel is still the default) and run rollback"
    ;;
  promote)
    case $loader in
      grub) run grub-set-default "Advanced options for Arch Linux>Arch Linux, with Linux $(pkgbase_of)" ;;
      systemd-boot) run bootctl set-default "$(basename "$(ls /boot/loader/entries/*"$(pkgbase_of)"*.conf | head -1)")" ;;
    esac
    ;;
  rollback)
    case $loader in
      grub) run grub-set-default 0; run grub-editenv - unset next_entry ;;
      systemd-boot) run bootctl set-default "" ;;
    esac
    [ -n "$pkgbase" ] && run pacman -R --noconfirm "$pkgbase"
    note "reboot into the distribution kernel; remove the pre-kernel btrfs snapshot when satisfied"
    ;;
  *) echo "unknown phase $phase" >&2; exit 2 ;;
esac
