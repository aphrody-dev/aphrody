#!/bin/bash
# Builds the WSL2 kernel from the Microsoft tree with Microsoft's config-wsl, following the build steps of README-Microsoft.WSL2.
# usage: build.sh ROOT REF [MARCH]   run on the build host. ROOT holds src/ and build/; REF is a tag or branch of the WSL2 kernel tree.
# env: WSL_GIT (repository URL, required), JOBS (default nproc). MARCH (for example raptorlake) is passed to the compiler as -march and -mtune;
# without it the build is Microsoft's generic one. Output: ROOT/build/<ref>[-march]/{bzImage,modules.vhdx,config,build.json,sha256sums}.
set -euo pipefail
root=$(readlink -f "$1"); ref=$2; march=${3:-}
: "${WSL_GIT:?WSL_GIT must name the kernel repository}"
jobs=${JOBS:-$(nproc)}
src=$root/src
out=$root/build/${ref}${march:+-$march}
log() { echo "[$(date +%H:%M:%S)] $*"; }

if [ ! -d "$src/.git" ]; then git clone -q --depth 1 --branch "$ref" "$WSL_GIT" "$src"; fi
if [ "$(git -C "$src" describe --tags --exact-match 2>/dev/null || git -C "$src" rev-parse --abbrev-ref HEAD)" != "$ref" ]; then
  git -C "$src" fetch -q --depth 1 origin "refs/tags/$ref:refs/tags/$ref" || git -C "$src" fetch -q --depth 1 origin "$ref"
  git -C "$src" checkout -q "$ref"
fi
commit=$(git -C "$src" rev-parse HEAD)

rm -rf "$out"; mkdir -p "$out/stage"
cp "$src/Microsoft/config-wsl" "$out/.config"
[ -n "$march" ] && "$src/scripts/config" --file "$out/.config" --set-str LOCALVERSION "-microsoft-standard-WSL2-$march"
kcflags=""; [ -n "$march" ] && kcflags="-march=$march -mtune=$march"
make -C "$src" O="$out" olddefconfig >/dev/null
started=$(date +%s)
log "building bzImage and modules with -j$jobs KCFLAGS='$kcflags'"
make -C "$src" O="$out" -j"$jobs" KCFLAGS="$kcflags" bzImage modules >"$out/build.log" 2>&1 || { tail -30 "$out/build.log"; exit 1; }
built=$(date +%s)
release=$(make -s -C "$src" O="$out" kernelrelease)
make -C "$src" O="$out" INSTALL_MOD_PATH="$out/stage/modules" modules_install >>"$out/build.log" 2>&1
make -C "$src" O="$out" INSTALL_HDR_PATH="$out/stage/headers" headers_install >>"$out/build.log" 2>&1
make -C "$src/tools/perf" O="$out/perf-build" NO_JEVENTS=1 NO_JVMTI=1 NO_LIBTRACEEVENT=1 install DESTDIR="$out/stage/perf" prefix=/ >>"$out/build.log" 2>&1 \
  || { echo "perf build failed, shipping an empty perf directory" >&2; mkdir -p "$out/stage/perf/bin"; }
"$src/Microsoft/scripts/gen_artifacts_vhdx.sh" "$out/stage/modules" "$out/stage/headers" "$out/stage/perf" "$release" "$out/modules.vhdx" >>"$out/build.log" 2>&1
install -m644 "$out/arch/x86/boot/bzImage" "$out/bzImage"
cp "$out/.config" "$out/config"
finished=$(date +%s)
(cd "$out" && sha256sum bzImage modules.vhdx config > sha256sums)
cat > "$out/build.json" <<JSON
{
  "ref": "$ref",
  "commit": "$commit",
  "release": "$release",
  "march": "${march:-generic}",
  "kcflags": "$kcflags",
  "compiler": "$(gcc --version | head -1)",
  "jobs": $jobs,
  "buildSeconds": $((built - started)),
  "totalSeconds": $((finished - started)),
  "bzImageBytes": $(stat -c %s "$out/bzImage"),
  "modulesVhdxBytes": $(stat -c %s "$out/modules.vhdx"),
  "bzImageSha256": "$(sha256sum "$out/bzImage" | cut -d' ' -f1)",
  "modulesVhdxSha256": "$(sha256sum "$out/modules.vhdx" | cut -d' ' -f1)"
}
JSON
rm -rf "$out/stage" "$out/perf-build"
log "done: $release"; cat "$out/build.json"
