#!/bin/bash
# Installs one kernel into a copy of the base root filesystem and writes a bootable ext4 image plus vmlinuz and initramfs.
# env: ID, SOURCE (repo|file), PACKAGE (name for repo, files in /lane/work/$ID/pkgs for file), TESTING_REPOS, TESTING_REPOS, DISK_GB
. "$(dirname "$0")/common.sh"
pacman -Sy --noconfirm --needed arch-install-scripts e2fsprogs >/dev/null
dst=/lane/work/$ID
root=$dst/root
rm -rf "$root"
mkdir -p "$dst"
cp -a --reflink=auto /lane/rootfs/base "$root"
# arch-chroot wants the root to be a mountpoint, or pacman cannot size its cache.
mount --bind "$root" "$root"
if [ "$SOURCE" = repo ]; then
  arch-chroot "$root" pacman -Sy --noconfirm "$PACKAGE" >/dev/null
else
  # arch-chroot mounts a tmpfs on /tmp, so the packages go elsewhere; the glob must expand inside the chroot.
  mkdir -p "$root/var/lane-pkgs"
  cp "$dst"/pkgs/*.pkg.tar.zst "$root/var/lane-pkgs/"
  # CachyOS v3 and v4 packages carry their own architecture tag, which pacman rejects unless the configuration lists it.
  if ls "$dst"/pkgs/ | grep -qE 'x86_64_v[34]'; then sed -i 's/^#\?Architecture.*/Architecture = x86_64 x86_64_v3 x86_64_v4/' "$root/etc/pacman.conf"; fi
  arch-chroot "$root" sh -c 'pacman -U --noconfirm /var/lane-pkgs/*.pkg.tar.zst'
  rm -rf "$root/var/lane-pkgs"
fi
kver=$(cd "$root/usr/lib/modules" && for d in */; do [ -f "$d/vmlinuz" ] && echo "${d%/}"; done)
[ "$(echo "$kver" | wc -l)" = 1 ] || { echo "expected one installed kernel, found: $kver" >&2; exit 1; }
# autodetect is skipped: it would trim the image to the container host's hardware instead of the guest's virtio devices.
arch-chroot "$root" mkinitcpio -S autodetect -k "$kver" -g /boot/initramfs-lane.img >/dev/null 2>&1
install -m644 "$root/usr/lib/modules/$kver/vmlinuz" "$dst/vmlinuz"
install -m644 "$root/boot/initramfs-lane.img" "$dst/initramfs.img"
echo "$kver" > "$dst/kernel-release"
echo "$(arch-chroot "$root" pacman -Qqo "/usr/lib/modules/$kver/vmlinuz" | head -1) $(arch-chroot "$root" pacman -Q "$(arch-chroot "$root" pacman -Qqo "/usr/lib/modules/$kver/vmlinuz" | head -1)" | cut -d' ' -f2)" > "$dst/kernel-package"

install -m755 "$(dirname "$0")/smoke.sh" "$root/usr/local/bin/lane-smoke"
install -m644 "$(dirname "$0")/lane-smoke.service" "$root/etc/systemd/system/lane-smoke.service"
mkdir -p "$root/etc/systemd/system/multi-user.target.wants" "$root/etc/systemd/network"
ln -sf /etc/systemd/system/lane-smoke.service "$root/etc/systemd/system/multi-user.target.wants/lane-smoke.service"
ln -sf /usr/lib/systemd/system/systemd-networkd.service "$root/etc/systemd/system/multi-user.target.wants/systemd-networkd.service"
printf '[Match]\nName=e*\n\n[Network]\nDHCP=yes\n' > "$root/etc/systemd/network/20-wired.network"
rm -f "$root/etc/resolv.conf"
echo "nameserver 10.0.2.3" > "$root/etc/resolv.conf"
echo "lane-guest" > "$root/etc/hostname"
echo "TESTING_REPOS=\"$TESTING_REPOS\"" > "$root/etc/lane.env"
[ -d /lane/work/kselftest ] && cp -a /lane/work/kselftest "$root/opt/kselftest"
arch-chroot "$root" passwd -d root >/dev/null
arch-chroot "$root" systemctl mask systemd-firstboot.service >/dev/null 2>&1 || true

umount -R "$root"
truncate -s "${DISK_GB}G" "$dst/root.img"
mkfs.ext4 -q -F -L lane-root -d "$root" "$dst/root.img"
rm -rf "$root"
chown "${HOST_UID:-1000}" "$dst"/*
ls -l "$dst"
