#!/usr/bin/env python3
"""Direct-dependency requirements of workspace members per crate, for crates resolved at >1 version.
usage: cargo metadata --format-version 1 | directdups.py"""
import json, sys, collections
m = json.load(sys.stdin)
ws = set(m["workspace_members"])
byid = {p["id"]: p for p in m["packages"]}
vers = collections.defaultdict(set)
for p in m["packages"]:
    if p["source"]:
        vers[p["name"]].add(p["version"])
dup = {n for n, v in vers.items() if len(v) > 1}
direct = collections.defaultdict(lambda: collections.defaultdict(list))
for i in ws:
    p = byid[i]
    for d in p["dependencies"]:
        if d["name"] in dup and not d.get("path"):
            direct[d["name"]][d["req"]].append(p["name"])
print(json.dumps({"dup_names": len(dup), "dup_with_direct_requirements": len(direct),
                  "direct": {n: {r: sorted(set(c))[:4] for r, c in rq.items()} for n, rq in sorted(direct.items())}}, indent=1))
