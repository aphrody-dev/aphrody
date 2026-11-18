#!/usr/bin/env bash
# Integration fixes on top of flatten.py output; idempotent, no-op when third_party uses lockfile pins (`mode = "pin"`).
set -e
cd "$(dirname "$0")/.."

if ! compgen -G "third_party/*/Cargo.toml" > /dev/null && ! compgen -G "third_party/*/*/Cargo.toml" > /dev/null; then
  exit 0
fi

crate() { printf '%s\n' third_party/*/"$1"; }
drop() { d=$(crate "$1"); sed -i "s#\"$d\", ##; s#, \"$d\"##; \#^$2 = {path = \"$d\"}#d" Cargo.toml; }

rg -l "sm-angle-default" third_party --glob Cargo.toml | xargs -r sed -i "s/sm-angle-default/sm-angle-builtin/g"
drop wrench wrench
drop webrender-examples webrender-examples
for c in obscura-js obscura-browser obscura-cdp obscura-cli obscura obscura-mcp; do drop $c $c; done
