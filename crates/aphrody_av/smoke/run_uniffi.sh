#!/bin/sh
# Builds the cdylib with UniFFI, generates the Python, Kotlin and Swift bindings, runs the Python smoke test.
# usage: smoke/run_uniffi.sh [OUT_DIR]   (run from anywhere inside the workspace)
set -eu
here=$(cd "$(dirname "$0")" && pwd)
crate=$(dirname "$here")
out=${1:-$(mktemp -d)}
cd "$crate"
cargo build -p aphrody_av --lib --features uniffi
target=${CARGO_TARGET_DIR:-$(cargo metadata --format-version 1 --no-deps | python3 -c 'import sys,json;print(json.load(sys.stdin)["target_directory"])')}
case $(uname -s) in Darwin) lib=libaphrody_av.dylib ;; *) lib=libaphrody_av.so ;; esac
for language in python kotlin swift; do
    cargo run -q -p aphrody_av --features uniffi-cli --bin aphrody-av-uniffi-bindgen -- generate --library "$target/debug/$lib" --language "$language" --out-dir "$out/$language"
done
cp "$target/debug/$lib" "$out/python/"
ls -l "$out"/python "$out"/kotlin "$out"/swift
AV_BINDINGS="$out/python" python3 -I "$here/python_smoke.py" "$here/fixtures/clip.mp4"
