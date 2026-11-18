#!/usr/bin/env bash
# Submodules of the imported upstreams arrive as bare gitlinks. harfbuzz is a build dependency of harfbuzz-sys:
# vendor its tree at the pinned commit (snapshot, no history). The html5lib test corpora are test data only:
# the gitlinks are removed and the pins stay here for `git clone` on demand.
set -euo pipefail
cd "$(dirname "$0")/.."
crate() { printf '%s\n' third_party/*/"$1"; }   # third_party/<upstream>/<name> (tools/layout.toml)
lol=$(crate lol_html); rcdom=$(crate markup5ever_rcdom); hb=$(crate harfbuzz-sys)/harfbuzz
git rm -q --cached --ignore-unmatch "$lol/tests/data/html5lib-tests" "$rcdom/html5lib-tests" "$rcdom/xml5lib-tests" >/dev/null
# on demand: html5lib-tests 85c968ac / f994590f (lol_html, rcdom), xml5lib-tests 9a7ca895 (rcdom)
if git ls-files -s "$hb" | grep -q '^160000'; then
  sha=$(git ls-files -s "$hb" | cut -d' ' -f2)
  git rm -q --cached "$hb"
  tmp=$(mktemp -d)
  git clone -q --filter=blob:none https://github.com/harfbuzz/harfbuzz "$tmp/hb"
  git -C "$tmp/hb" checkout -q "$sha"
  rm -rf "$tmp/hb/.git" "$hb"
  mv "$tmp/hb" "$hb"
  rm -rf "$tmp"
fi
# keep only what harfbuzz-sys compiles
for d in "$hb"/*; do case "$d" in */src|*/COPYING) ;; *) rm -rf "$d";; esac; done
