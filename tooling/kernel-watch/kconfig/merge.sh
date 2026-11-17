#!/bin/bash
# Merges config fragments into a base configuration of a kernel tree and reports which requested symbols did not take effect.
# usage: merge.sh TREE OUT BASE_CONFIG FRAGMENT...   (TREE: kernel source, OUT: build directory, built with the LLVM toolchain)
set -euo pipefail
tree=$(readlink -f "$1"); mkdir -p "$2"; out=$(readlink -f "$2"); base=$(readlink -f "$3"); shift 3
fragments=(); for f in "$@"; do fragments+=("$(readlink -f "$f")"); done
set -- "${fragments[@]}"
mkdir -p "$out"; cp "$base" "$out/.config"
make -C "$tree" O="$out" LLVM=1 olddefconfig >/dev/null
(cd "$tree" && ARCH=x86 KCONFIG_CONFIG="$out/.config" scripts/kconfig/merge_config.sh -m -O "$out" "$out/.config" "$@" >/dev/null)
make -C "$tree" O="$out" LLVM=1 olddefconfig >/dev/null
cat "$@" | grep -E '^(# )?CONFIG_[A-Za-z0-9_]+(=| is not set)' | sed -E 's/^# (CONFIG_[A-Za-z0-9_]+) is not set$/\1=n/' | sort -u > "$out/requested.txt"
: > "$out/not-applied.txt"
while IFS= read -r line; do
  sym=${line%%=*}; want=${line#*=}
  have=$(grep -E "^$sym=" "$out/.config" | cut -d= -f2- || true)
  if [ "$want" = n ]; then grep -q "^$sym=" "$out/.config" && echo "$sym: wanted n, got $have" >> "$out/not-applied.txt"
  elif [ "$have" != "$want" ]; then echo "$sym: wanted $want, got ${have:-unset}" >> "$out/not-applied.txt"; fi
done < "$out/requested.txt"
echo "requested $(wc -l < "$out/requested.txt"), not applied $(wc -l < "$out/not-applied.txt")"
cat "$out/not-applied.txt"
