#!/usr/bin/env python3
"""One version line per crate: raise every direct registry requirement to the newest line used in the workspace.

usage: cargo metadata --format-version 1 --offline | uv run --with tomlkit tools/unify.py <monorepo> [--skip a,b]

Pinned (=x), ranged and non-registry requirements are left alone. Crates listed in tools/unify.skip (one per line,
with the reason after #) are not touched: those are the ones whose newest line broke a member.
"""
import json, os, sys, collections
import tomlkit

DEPT = ("dependencies", "dev-dependencies", "build-dependencies")


def vkey(req):
    r = req.strip()
    if not r or r.startswith("=") or "," in r or "<" in r or r in ("*",):
        return None
    r = r.lstrip("^~>= ")
    try:
        nums = [int(p) for p in r.split("-")[0].split(".") if p != "*"]
    except ValueError:
        return None
    if not nums:
        return None
    return ((0, nums[1]) if nums[0] == 0 and len(nums) > 1 else (nums[0],)), tuple(nums)


def load(p):
    return tomlkit.parse(open(p, encoding="utf-8").read())


def main():
    mono = sys.argv[1]
    skip = set()
    sp = os.path.join(mono, "tools", "unify.skip")
    if os.path.exists(sp):
        skip = {l.split("#")[0].strip() for l in open(sp) if l.split("#")[0].strip()}
    m = json.load(sys.stdin)
    ws = set(m["workspace_members"])
    byid = {p["id"]: p for p in m["packages"]}
    reqs = collections.defaultdict(set)
    for i in ws:
        for d in byid[i]["dependencies"]:
            if not d.get("path") and d.get("source", "").startswith("registry"):
                reqs[d["name"]].add(d["req"])
    # inherited (workspace = true) requirements are the root's; they show up in the same metadata
    target = {}
    for name, rs in reqs.items():
        ks = {r: vkey(r) for r in rs}
        valid = {r: k for r, k in ks.items() if k}
        if name in skip or len({k[0] for k in valid.values()}) < 2:
            continue
        best = max(valid, key=lambda r: valid[r][1])
        # cargo normalises "0.23.0" to "^0.23.0"; write the bare form
        target[name] = best.lstrip("^")
    changed = collections.Counter()

    def fix(tbl, where):
        for k in list(tbl.keys()):
            spec = tbl[k]
            pname = (spec.get("package", k) if hasattr(spec, "get") and not isinstance(spec, str) else k)
            if pname not in target:
                continue
            cur = str(spec) if isinstance(spec, str) else (str(spec.get("version")) if spec.get("version") is not None else None)
            if cur is None or vkey(cur) is None:
                continue
            if vkey(cur)[0] == vkey(target[pname])[0] and vkey(cur)[1] >= vkey(target[pname])[1]:
                continue
            if isinstance(spec, str):
                tbl[k] = target[pname]
            else:
                spec["version"] = target[pname]
            changed[pname] += 1

    files = [os.path.join(mono, "Cargo.toml")]
    for root_, dirs, fs in os.walk(os.path.join(mono, "src")):
        if "Cargo.toml" in fs:
            files.append(os.path.join(root_, "Cargo.toml"))
        dirs[:] = [d for d in dirs if d not in ("target", "node_modules", "vendor")]
    for f in files:
        doc = load(f)
        if "workspace" in doc and "dependencies" in doc["workspace"]:
            fix(doc["workspace"]["dependencies"], "ws")
        for sec in DEPT:
            if sec in doc:
                fix(doc[sec], sec)
        for cfg in (doc["target"].values() if "target" in doc else []):
            for sec in DEPT:
                if sec in cfg:
                    fix(cfg[sec], sec)
        open(f, "w", encoding="utf-8", newline="\n").write(tomlkit.dumps(doc))
    json.dump({"target": target, "edits": changed}, open(os.path.join(mono, ".local", "reports", "unify.report.json"), "w"), indent=1)
    print(len(target), "crates unified;", sum(changed.values()), "requirement edits")


main()
