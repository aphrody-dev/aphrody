#!/bin/bash
# Builds the Arch base root filesystem the guests start from. Result: /lane/rootfs/base.
. "$(dirname "$0")/common.sh"
mkdir -p /lane/rootfs
pacman -Sy --noconfirm --needed arch-install-scripts e2fsprogs >/dev/null
pacman-key --init >/dev/null 2>&1
pacman-key --populate archlinux >/dev/null 2>&1
plain_pacman_conf
rm -rf /lane/rootfs/base.new
mkdir -p /lane/rootfs/base.new
pacstrap -C /tmp/pacman-plain.conf -c /lane/rootfs/base.new base mkinitcpio python >/dev/null
rm -rf /lane/rootfs/base
mv /lane/rootfs/base.new /lane/rootfs/base
echo "rootfs-packages $(pacman -r /lane/rootfs/base -Q 2>/dev/null | wc -l)"
