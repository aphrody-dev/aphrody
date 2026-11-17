#!/bin/bash
# Stages a built kernel tree for the rc lane: modules installed under usr/, the bzImage and the release string.
# usage: stage.sh TREE OUT STAGE   (OUT: the build directory used with O=, STAGE: destination, created fresh)
set -euo pipefail
tree=$(readlink -f "$1"); out=$(readlink -f "$2"); stage=$3
rm -rf "$stage"; mkdir -p "$stage/usr"; stage=$(readlink -f "$stage")
make -C "$tree" O="$out" LLVM=1 INSTALL_MOD_PATH="$stage/usr" INSTALL_MOD_STRIP=1 modules_install >/dev/null
rm -f "$stage"/usr/lib/modules/*/build "$stage"/usr/lib/modules/*/source
install -m644 "$out/arch/x86/boot/bzImage" "$stage/bzImage"
make -s -C "$tree" O="$out" LLVM=1 kernelrelease > "$stage/release"
echo "$(git -C "$tree" describe --tags --always 2>/dev/null || echo tree) $(git -C "$tree" rev-parse HEAD 2>/dev/null || echo unknown)" > "$stage/source"
cat "$stage/release" "$stage/source"
