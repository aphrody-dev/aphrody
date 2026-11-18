#!/usr/bin/env python3
"""Flatten the imported upstreams into one Bun-style Cargo workspace (idempotent).

usage: uv run --with tomlkit tools/flatten.py <monorepo> [--unify] [--root-only]

* every imported crate is a member under third_party/<upstream>/<package> (tools/layout.toml); the per-upstream [workspace] tables are removed
  and their inheritance (workspace.package, workspace.dependencies, lints) is inlined in each member;
* dependencies on crates of the monorepo become inline path dependencies (git and registry self references
  disappear); a [patch.crates-io] entry makes third-party dependents use the same copy;
* with --unify, registry requirements of the same crate on different breaking lines are raised to the
  newest line found in the workspace (one version per crate).
Root manifest = the committed one: its members and [patch.crates-io] follow the lock, profiles of Servo and Obscura are added.
"""
import copy, json, os, re, sys, tomllib
import tomlkit
from tomlkit import inline_table, array

def layout(mono):
    """tools/layout.toml: where Bun, first-party crates and third-party upstreams live."""
    with open(os.path.join(mono, "tools", "layout.toml"), "rb") as f:
        return tomllib.load(f)


DEPT = ("dependencies", "dev-dependencies", "build-dependencies")
PKG_INHERIT = ("version", "edition", "authors", "license", "license-file", "repository", "homepage",
               "description", "documentation", "readme", "keywords", "categories", "rust-version",
               "publish", "exclude", "include")


def load(p):
    with open(p, encoding="utf-8") as f:
        return tomlkit.parse(f.read())


def save(doc, p):
    with open(p, "w", encoding="utf-8", newline="\n") as f:
        f.write(tomlkit.dumps(doc))


def plain(x):
    return x.unwrap() if hasattr(x, "unwrap") else x


def as_inline(d):
    t = inline_table()
    for k, v in d.items():
        if isinstance(v, list):
            a = array()
            a.extend(v)
            t[k] = a
        else:
            t[k] = v
    return t


def vkey(req):
    """(breaking-line key, numeric tuple) of a plain requirement, or None for pins and ranges."""
    r = req.strip()
    if r.startswith("=") or "," in r or "<" in r or r in ("*", ""):
        return None
    r = r.lstrip("^~>= ")
    parts = r.split("-")[0].split(".")
    try:
        nums = [int(p) for p in parts if p != "*"]
    except ValueError:
        return None
    if not nums:
        return None
    line = (0, nums[1]) if nums[0] == 0 and len(nums) > 1 else (nums[0],)
    return line, tuple(nums)


def main():
    mono = sys.argv[1]
    unify = "--unify" in sys.argv
    root_only = "--root-only" in sys.argv   # manifests of the crates are already flattened: rebuild the root manifest only
    lock = json.load(open(os.path.join(mono, "tools", "upstreams.lock.json")))
    lay = layout(mono)
    third, first = lay["third_party"]["dir"], lay["first_party"]["dir"]
    root = load(os.path.join(mono, "Cargo.toml"))

    # --- index: package name -> (dir, upstream) ---------------------------------------------------
    pkgs, owner = {}, {}
    for u, v in lock.items():
        for _, tgt in v["crates"].items():
            mf = os.path.join(mono, tgt, "Cargo.toml")
            if not os.path.exists(mf):
                continue
            t = load(mf)
            name = str(t["package"]["name"])
            if name in pkgs:
                print("DUPLICATE package", name, pkgs[name], tgt, "-> excluded", tgt)
                owner[tgt] = None
                continue
            pkgs[name] = tgt
            owner[tgt] = u

    # --- per-upstream workspace source ------------------------------------------------------------
    W = {}
    for u, v in lock.items():
        virt = os.path.join(mono, third, u, "Cargo.toml")
        saved = os.path.join(mono, third, u, "Cargo.workspace.toml")
        rootcrate = v["crates"].get(".")
        src = None
        if rootcrate and os.path.exists(os.path.join(mono, rootcrate, "Cargo.toml")):
            t = load(os.path.join(mono, rootcrate, "Cargo.toml"))
            if "workspace" in t:
                src = t
                os.makedirs(os.path.dirname(saved), exist_ok=True)
                w = tomlkit.document()
                for k in ("workspace", "patch", "profile", "replace"):
                    if k in t:
                        w[k] = copy.deepcopy(t[k])
                save(w, saved)
        if src is None and os.path.exists(virt):
            src = load(virt)
        if src is None and os.path.exists(saved):
            src = load(saved)
        W[u] = src

    dirs_of = {u: {tgt: d for d, tgt in v["crates"].items()} for u, v in lock.items()}  # tgt -> orig dir
    new_of = {u: {d: tgt for d, tgt in v["crates"].items()} for u, v in lock.items()}   # orig dir -> tgt

    extras_patch, extras_profile = {}, {}
    reqs = {}          # crate -> [(manifest, table, key, version-string)]
    unresolved = []
    n_done = 0

    for tgt, u in sorted(owner.items()):
        if u is None or root_only:
            continue
        mp = os.path.join(mono, tgt, "Cargo.toml")
        doc = load(mp)
        w = W.get(u)
        wdoc = plain(w.get("workspace", {})) if w is not None else {}
        wpkg, wdeps, wlints = wdoc.get("package", {}), wdoc.get("dependencies", {}), wdoc.get("lints", {})
        orig = dirs_of[u].get(tgt)          # original dir of this crate, e.g. components/script
        wroot_rel = "."                      # workspace root is the repo root in every imported upstream

        # workspace-only sections -> root extras
        for sec, store in (("patch", extras_patch), ("profile", extras_profile)):
            if sec in doc:
                store[(u, tgt)] = copy.deepcopy(doc[sec])
                del doc[sec]
        if "replace" in doc:
            del doc["replace"]
        if "workspace" in doc:
            del doc["workspace"]

        # [package] inheritance
        pk = doc["package"]
        for k in PKG_INHERIT:
            if k in pk and isinstance(plain(pk[k]), dict) and plain(pk[k]).get("workspace") is True:
                if k in wpkg:
                    pk[k] = wpkg[k]
                else:
                    del pk[k]
        if "lints" in doc and plain(doc["lints"]).get("workspace") is True:
            del doc["lints"]
            if wlints:
                doc["lints"] = copy.deepcopy(w["workspace"]["lints"])

        def fix_table(tbl, where):
            for k in list(tbl.keys()):
                spec = plain(tbl[k])
                d = {"version": spec} if isinstance(spec, str) else dict(spec)
                if d.get("workspace") is True:
                    base = wdeps.get(k)
                    if base is None:
                        unresolved.append((tgt, k, "missing in workspace.dependencies"))
                        continue
                    base = {"version": base} if isinstance(base, str) else dict(base)
                    feats = list(base.get("features", [])) + [f for f in d.get("features", []) if f not in base.get("features", [])]
                    base.update({kk: vv for kk, vv in d.items() if kk not in ("workspace", "features")})
                    if feats:
                        base["features"] = feats
                    d, from_ws = base, True
                else:
                    from_ws = False
                pname = d.get("package", k)
                internal = None
                if "path" in d:
                    rel = d["path"]
                    if from_ws:
                        resolved = os.path.normpath(os.path.join(wroot_rel, rel)).replace("\\", "/")
                    else:
                        resolved = os.path.normpath(os.path.join(orig or ".", rel)).replace("\\", "/")
                    internal = new_of[u].get(resolved)
                    if internal is None:
                        rc = new_of[u].get(".")
                        cands = ([os.path.join(rc, resolved)] if rc else []) + [os.path.join(third, u, resolved)]
                        hit = next((c.replace("\\", "/") for c in cands if os.path.exists(os.path.join(mono, c, "Cargo.toml"))), None)
                        if hit is None:
                            unresolved.append((tgt, k, "path dep not found: " + resolved))
                        else:
                            d["path"] = os.path.relpath(hit, tgt).replace("\\", "/")
                elif pname in pkgs and owner.get(pkgs[pname]) is not None:
                    # registry or git spec of a crate that lives in the monorepo: use the local copy
                    internal = pkgs[pname]
                    d.pop("version", None)
                if internal is not None:
                    newpath = os.path.relpath(internal, tgt).replace("\\", "/")
                    for dd in ("git", "rev", "branch", "tag"):
                        d.pop(dd, None)
                    d["path"] = newpath
                    # keep `version` for publishable layout; dropped when it would not match the local copy
                elif "git" in d:
                    unresolved.append((tgt, k, "git dep outside the monorepo: " + d["git"]))
                if "version" in d and "path" not in d and "git" not in d:
                    reqs.setdefault(pname, []).append((tgt, where, k))
                tbl[k] = as_inline(d) if (len(d) > 1 or "version" not in d) else d["version"]

        for sec in DEPT:
            if sec in doc:
                fix_table(doc[sec], sec)
        if "target" in doc:
            for cfg in doc["target"].values():
                for sec in DEPT:
                    if sec in cfg:
                        fix_table(cfg[sec], "target." + sec)
        # relative paths in the manifest that pointed outside the crate
        for k in ("readme", "license-file"):
            if k in pk and isinstance(plain(pk[k]), str) and plain(pk[k]).startswith(".."):
                del pk[k]
        save(doc, mp)
        n_done += 1

    # --- root manifest ---------------------------------------------------------------------------
    members = sorted(t for t, u in owner.items() if u is not None)
    # crates written in this repository (not imported): components/<package>
    for d in sorted(os.listdir(os.path.join(mono, first))) if os.path.isdir(os.path.join(mono, first)) else []:
        mf = os.path.join(mono, first, d, "Cargo.toml")
        if os.path.exists(mf) and first + "/" + d not in members:
            members.append(first + "/" + d)
    root["workspace"]["members"] = members
    # patches over imported crates (Bun's patches/rust-argon2/*), applied once
    import subprocess
    for name_dir in sorted(os.listdir(os.path.join(mono, "tools", "patches"))) if os.path.isdir(os.path.join(mono, "tools", "patches")) else []:
        tgt = pkgs.get(name_dir.replace("_", "-")) or pkgs.get(name_dir) or pkgs.get("rust-argon2")
        for p in sorted(os.listdir(os.path.join(mono, "tools", "patches", name_dir))):
            pf = os.path.join(mono, "tools", "patches", name_dir, p)
            args = ["git", "apply", "-p1", "--directory=" + tgt, pf]
            if subprocess.run(args[:2] + ["--reverse", "--check"] + args[2:], cwd=mono, capture_output=True).returncode == 0:
                continue  # already applied
            r = subprocess.run(args, cwd=mono, capture_output=True, text=True)
            print("patch", name_dir, p, "ok" if r.returncode == 0 else "FAILED: " + r.stderr[:200])
    patch = tomlkit.table()
    cio = tomlkit.table()
    for name, d in sorted(pkgs.items()):
        if owner[d] is None:
            continue
        it = inline_table()
        it["path"] = d
        cio[name] = it
    patch["crates-io"] = cio
    root["patch"] = patch
    # profiles: Bun's root wins; other upstreams add only profiles/package overrides it does not define.
    # `panic` is profile-wide, so the browser binary builds with --profile browser (unwind, see ARCHITECTURE.md).
    rp = root.setdefault("profile", tomlkit.table())
    for (u, t), prof in extras_profile.items():
        for prof_name, pv in plain(prof).items():
            if prof_name not in rp:
                if u in ("servo", "obscura"):
                    rp[prof_name] = pv
                continue
            for k, v in pv.items():
                if k == "package":
                    for pn, ps in v.items():
                        rp[prof_name].setdefault("package", tomlkit.table())
                        if pn not in rp[prof_name]["package"] and (pn in pkgs):
                            rp[prof_name]["package"][pn] = ps
    save(root, os.path.join(mono, "Cargo.toml"))
    json.dump({"unresolved": unresolved, "profiles_not_merged": [f"{u}:{t}" for (u, t) in extras_profile],
               "patches_not_merged": [f"{u}:{t}" for (u, t) in extras_patch]},
              open(os.path.join(mono, ".local", "reports", "flatten.report.json"), "w"), indent=1)
    print("flattened", n_done, "manifests;", len(members), "members;", len(unresolved), "unresolved")


main()
