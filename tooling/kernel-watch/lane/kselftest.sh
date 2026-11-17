#!/bin/bash
# Builds a small kselftest subset from the kernel source mirror. env: KERNEL_MIRROR, KERNEL_TAG, TARGETS. Result: /lane/work/kselftest.
. "$(dirname "$0")/common.sh"
pacman -Sy --noconfirm --needed base-devel git libcap libcap-ng numactl fuse3 rsync >/dev/null
rm -rf /lane/work/kselftest /tmp/ks
git config --global safe.directory '*'
git clone --depth 1 -q "file://$KERNEL_MIRROR" /tmp/ks --branch "$KERNEL_TAG" 2>/dev/null || git clone -q "file://$KERNEL_MIRROR" /tmp/ks
cd /tmp/ks
git checkout -q "$KERNEL_TAG"
make -s headers >/dev/null
make -s -C tools/testing/selftests TARGETS="$TARGETS" INSTALL_PATH=/lane/work/kselftest install >/dev/null
ls /lane/work/kselftest | head
