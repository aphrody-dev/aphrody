#!/bin/sh
# Builds the wasm32-unknown-unknown module, generates the JS glue with wasm-bindgen 0.2.130 and runs the Bun smoke test.
# usage: smoke/run_wasm.sh [OUT_DIR]
set -eu
here=$(cd "$(dirname "$0")" && pwd)
crate=$(dirname "$here")
out=${1:-$(mktemp -d)}
cd "$crate"
cargo build -p aphrody_av --lib --target wasm32-unknown-unknown --no-default-features --features wasm --release
target=${CARGO_TARGET_DIR:-$(cargo metadata --format-version 1 --no-deps | python3 -c 'import sys,json;print(json.load(sys.stdin)["target_directory"])')}
wasm-bindgen "$target/wasm32-unknown-unknown/release/aphrody_av.wasm" --target nodejs --out-dir "$out"
AV_WASM="$out" bun "$here/bun_smoke.ts" "$here/fixtures/clip.mp4"
