"""open-replica <command> <replica dir> ...: the Python steps of tools/replica, each printing one JSON document last."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def devices() -> dict:
    out: dict = {}
    try:
        import torch

        out["torch"] = {"version": torch.__version__, "cuda": torch.cuda.is_available(), "device": torch.cuda.get_device_name(0) if torch.cuda.is_available() else "cpu"}
    except Exception as e:
        out["torch"] = {"error": str(e)[:200]}
    try:
        import tensorflow as tf

        out["tensorflow"] = {"version": tf.__version__, "devices": [d.name for d in tf.config.list_physical_devices()]}
    except Exception as e:
        out["tensorflow"] = {"error": str(e)[:200]}
    return out


def main() -> None:
    ap = argparse.ArgumentParser(prog="open-replica")
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("crawl", help="deep crawl with crawl4ai")
    c.add_argument("dir")
    c.add_argument("urls", nargs="+")
    c.add_argument("--depth", type=int, default=2)
    c.add_argument("--limit", type=int, default=60)
    c.add_argument("--per-shape", type=int, default=2)
    c.add_argument("--delay", type=float, default=2.0)
    c.add_argument("--viewport", default="1440x900")
    c.add_argument("--exclude")
    s = sub.add_parser("sources", help="OpenAPI and linked documents, typed by Magika, converted to Markdown")
    s.add_argument("dir")
    s.add_argument("origin")
    sub.add_parser("index", help="build the retrieval index").add_argument("dir")
    q = sub.add_parser("ask", help="retrieve passages for a question")
    q.add_argument("dir")
    q.add_argument("question")
    q.add_argument("-k", type=int, default=6)
    v = sub.add_parser("visual", help="layout parity against a running clone")
    v.add_argument("dir")
    v.add_argument("clone_url")
    v.add_argument("--viewport", default="1440x900")
    r = sub.add_parser("rea", help="binary facts with LIEF and pyghidra")
    r.add_argument("dir")
    r.add_argument("binaries", nargs="+")
    r.add_argument("--no-ghidra", action="store_true")
    sub.add_parser("devices", help="PyTorch and TensorFlow devices")
    a = ap.parse_args()

    if a.cmd == "devices":
        result: object = devices()
    elif a.cmd == "crawl":
        from .crawl import run

        w, h = (int(x) for x in a.viewport.split("x"))
        caps = run(a.urls, Path(a.dir), depth=a.depth, limit=a.limit, per_shape=a.per_shape, delay=a.delay, viewport=(w, h), exclude=a.exclude)
        result = {"captured": len(caps), "failed": sum(1 for x in caps if x.get("error"))}
    elif a.cmd == "sources":
        from .convert import gather

        srcs = gather(a.origin.rstrip("/"), Path(a.dir))
        result = {"sources": len(srcs), "openapi": sum(1 for x in srcs if x["openapi"])}
    elif a.cmd == "index":
        from .rag import build

        result = build(Path(a.dir))
    elif a.cmd == "ask":
        from .rag import query

        result = query(Path(a.dir), a.question, a.k)
    elif a.cmd == "visual":
        from .visual import compare

        w, h = (int(x) for x in a.viewport.split("x"))
        result = compare(Path(a.dir), a.clone_url, (w, h))
    else:
        from .rea import report

        result = [str(report(Path(b), Path(a.dir), ghidra=not a.no_ghidra)) for b in a.binaries]
    print(json.dumps(result))


if __name__ == "__main__":
    main()
