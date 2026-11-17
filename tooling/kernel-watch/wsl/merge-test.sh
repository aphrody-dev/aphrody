#!/bin/bash
# Measures how well Microsoft's WSL2 patches carry to a newer upstream tag, with git merge-tree in a throwaway blob-less repository
# (no working tree is touched, no shared checkout is used). Two measurements:
#   delta: Microsoft's own changes, i.e. stable tag -> Microsoft tag, replayed onto the upstream tag (the question that matters);
#   naive: a plain merge of the upstream tag into the Microsoft tag, which also trips over every stable backport.
# usage: merge-test.sh ROOT MSFT_TAG STABLE_TAG UPSTREAM_TAG   env: WSL_GIT, STABLE_GIT, KERNEL_GIT
# Output: ROOT/merge/<msft>-into-<upstream>/{report.json,delta-conflicts.txt,naive-conflicts.txt,dxgkrnl.txt}
set -euo pipefail
root=$(readlink -f "$1"); msft_tag=$2; stable_tag=$3; up_tag=$4
: "${WSL_GIT:?}" "${STABLE_GIT:?}" "${KERNEL_GIT:?}"
work=$root/merge-scratch
out=$root/merge/${msft_tag}-into-${up_tag}
rm -rf "$work"; mkdir -p "$work" "$out"
git init -q "$work"; cd "$work"
git config gc.auto 0; git config merge.renames true; git config diff.renameLimit 20000; git config merge.renameLimit 20000
git remote add msft "$WSL_GIT"; git remote add stable "$STABLE_GIT"; git remote add linux "$KERNEL_GIT"
git fetch -q --no-tags linux "refs/tags/$up_tag:refs/tags/$up_tag"
git fetch -q --no-tags stable "refs/tags/$stable_tag:refs/tags/$stable_tag"
git fetch -q --no-tags msft "refs/tags/$msft_tag:refs/tags/$msft_tag"

ms_delta_files=$(git diff --name-only "$stable_tag" "$msft_tag" | wc -l)
git diff --name-only "$stable_tag" "$msft_tag" > "$out/msft-delta-files.txt"
git diff --name-only "$stable_tag" "$up_tag" > "$out/upstream-since-stable-files.txt"
git diff --shortstat "$stable_tag" "$msft_tag" > "$out/msft-delta-shortstat.txt"
common=$(comm -12 <(sort "$out/msft-delta-files.txt") <(sort "$out/upstream-since-stable-files.txt") | wc -l)

delta_status=0
git merge-tree --write-tree --name-only --no-messages --merge-base="$stable_tag" "$up_tag" "$msft_tag" > "$out/delta-merge-tree.txt" 2>"$out/delta.err" || delta_status=$?
sed -n '2,$p' "$out/delta-merge-tree.txt" | sed '/^$/d' | sort -u > "$out/delta-conflicts.txt"

base=$(git merge-base "$msft_tag" "$up_tag")
naive_status=0
git merge-tree --write-tree --name-only --no-messages "$msft_tag" "$up_tag" > "$out/naive-merge-tree.txt" 2>"$out/naive.err" || naive_status=$?
sed -n '2,$p' "$out/naive-merge-tree.txt" | sed '/^$/d' | sort -u > "$out/naive-conflicts.txt"

git ls-tree -r --name-only "$msft_tag" -- drivers/hv/dxgkrnl include/uapi/misc/d3dkmthk.h > "$out/dxgkrnl.txt"
cat > "$out/report.json" <<JSON
{
  "msftTag": "$msft_tag",
  "stableTag": "$stable_tag",
  "upstreamTag": "$up_tag",
  "mergeBase": "$base",
  "commitsOnMsftSinceBase": $(git rev-list --count "$base..$msft_tag"),
  "commitsOnUpstreamSinceBase": $(git rev-list --count "$base..$up_tag"),
  "delta": {
    "shortstat": "$(tr -d '\n' < "$out/msft-delta-shortstat.txt" | sed 's/^ *//')",
    "filesChangedByMsft": $ms_delta_files,
    "filesChangedByUpstreamSinceStable": $(wc -l < "$out/upstream-since-stable-files.txt"),
    "filesChangedByBoth": $common,
    "mergeExitStatus": $delta_status,
    "conflictedFiles": $(wc -l < "$out/delta-conflicts.txt"),
    "dxgkrnlFilesInMsftTree": $(git ls-tree -r --name-only "$msft_tag" -- drivers/hv/dxgkrnl | wc -l),
    "dxgkrnlFilesInUpstream": $(git ls-tree -r --name-only "$up_tag" -- drivers/hv/dxgkrnl | wc -l),
    "dxgkrnlConflicts": $(grep -c '^drivers/hv/dxgkrnl/' "$out/delta-conflicts.txt" || true),
    "dxgkrnlSurroundingConflicts": $(grep -cE '^(drivers/hv/(Kconfig|Makefile|vmbus_drv.c|hv_common.c)|include/linux/hyperv.h|include/uapi/misc/d3dkmthk.h)$' "$out/delta-conflicts.txt" || true)
  },
  "naive": {
    "mergeExitStatus": $naive_status,
    "conflictedFiles": $(wc -l < "$out/naive-conflicts.txt"),
    "dxgkrnlConflicts": $(grep -c '^drivers/hv/dxgkrnl/' "$out/naive-conflicts.txt" || true)
  }
}
JSON
cat "$out/report.json"
echo "--- delta conflicts"; cat "$out/delta-conflicts.txt"
cd /; rm -rf "$work"
