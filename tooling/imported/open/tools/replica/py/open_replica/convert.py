"""Public sources of a site beyond its screens: a published OpenAPI description (well-known locations and links
found while crawling) and linked documents. Each download is typed by Magika from its bytes, then converted:
an OpenAPI description to an endpoint and data-model summary, PDF by docling, other documents by markitdown.

Writes sources/<n> (the bytes), sources/<n>.md and sources.json in the replica directory."""

from __future__ import annotations

import json
import os
import re
import time
import urllib.request
import urllib.robotparser
from pathlib import Path
from urllib.parse import urlsplit

# a source larger than this is skipped, not truncated
MAX_BYTES = int(os.environ.get("REPLICA_SOURCE_MAX_BYTES", str(64 * 1024 * 1024)))


class _SameOriginRedirects(urllib.request.HTTPRedirectHandler):
    """Follows redirects only within the site: a source link never leads the download to another host."""

    def __init__(self, origin: str) -> None:
        super().__init__()
        self.origin = origin

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        u = urlsplit(newurl)
        if f"{u.scheme}://{u.netloc}" != self.origin:
            return None
        return super().redirect_request(req, fp, code, msg, headers, newurl)

WELL_KNOWN = ["/.well-known/openapi.json", "/.well-known/openapi.yaml", "/openapi.json", "/api/openapi.json", "/swagger.json", "/api-docs"]
DOC_LINK = re.compile(r"\.(pdf|docx?|pptx?|xlsx?|csv|json|ya?ml|md|txt|epub)(\?|$)", re.I)
MARKITDOWN = {"doc", "docx", "ppt", "pptx", "xls", "xlsx", "csv", "html", "markdown", "txt", "epub", "xml", "rtf", "outlook"}


def identify(paths: list[Path]) -> dict[Path, tuple[str, str]]:
    """Magika's label (json, pdf, html...) and group (code, document, image...) of each file."""
    from magika import Magika

    m = Magika()
    out = {}
    for p, r in zip(paths, m.identify_paths(paths)):
        if r.ok:
            out[p] = (r.output.label, r.output.group)
    return out


def to_markdown(path: Path, label: str) -> str | None:
    if label == "pdf":
        try:
            from docling.document_converter import DocumentConverter

            return DocumentConverter().convert(str(path)).document.export_to_markdown()
        except Exception:
            pass
    if label in MARKITDOWN or label == "pdf":
        from markitdown import MarkItDown

        return MarkItDown().convert(str(path)).text_content
    return None


def summarize_openapi(doc: dict) -> str:
    """Endpoint groups (first tag, else the first two path segments) and the fields their JSON answers carry."""
    groups: dict[str, dict] = {}
    for path, methods in (doc.get("paths") or {}).items():
        for method, op in methods.items():
            if not isinstance(op, dict) or method == "parameters":
                continue
            g = (op.get("tags") or ["/".join([s for s in path.split("/") if s][:2])])[0]
            e = groups.setdefault(g, {"ops": [], "fields": []})
            summary = re.sub(r"\s+", " ", op.get("summary") or "")[:80]
            e["ops"].append(f"{method.upper()} {path}" + (f" ({summary})" if summary else ""))
            schema = (((op.get("responses") or {}).get("200") or {}).get("content") or {}).get("application/json", {}).get("schema") or {}
            obj = schema.get("items", {}) if schema.get("type") == "array" else schema
            for k in (obj.get("properties") or {}):
                if k not in e["fields"]:
                    e["fields"].append(k)
    info = doc.get("info") or {}
    total = sum(len(e["ops"]) for e in groups.values())
    lines = [f"# {info.get('title', 'API')} {info.get('version', '')}".strip(), "", f"{total} operations in {len(groups)} groups.", ""]
    for g, e in sorted(groups.items(), key=lambda kv: -len(kv[1]["ops"])):
        lines += [f"## {g} ({len(e['ops'])})", ""]
        if e["fields"]:
            lines += [f"Fields: {', '.join(e['fields'][:40])}", ""]
        lines += [f"- {o}" for o in e["ops"]] + [""]
    return "\n".join(lines)


def gather(origin: str, out: Path, per_shape: int = 2) -> list[dict]:
    """Downloads the sources not fetched yet; linked documents are capped per route shape (a site's item files,
    such as the files of one repository each, are its users' content, not documentation of the site)."""
    from collections import Counter

    from .crawl import Shapes, load, same_origin

    d = out / "sources"
    d.mkdir(parents=True, exist_ok=True)
    index = out / "sources.json"
    known: list[dict] = json.loads(index.read_text(encoding="utf-8")) if index.exists() else []
    have = {s["url"] for s in known}
    screens = load(out)
    shapes = Shapes()
    for s in screens:
        shapes.learn(urlsplit(s["final"]).path)
        if s.get("depth") == 0 and s.get("dom"):
            shapes.add_nav([urlsplit(u).path for u in s["dom"]["nav"] if same_origin(u, origin)])
    counts: Counter[str] = Counter(shapes.of(urlsplit(s["url"]).path) for s in known)
    urls = [origin + p for p in WELL_KNOWN]
    for s in screens:
        for link in (s.get("dom") or {}).get("links", []):
            u = urlsplit(link)
            plain = f"{u.scheme}://{u.netloc}{u.path}"
            shape = shapes.of(u.path)
            if same_origin(plain, origin) and DOC_LINK.search(u.path) and plain not in urls and counts[shape] < per_shape:
                counts[shape] += 1
                urls.append(plain)
    robots = urllib.robotparser.RobotFileParser(origin + "/robots.txt")
    try:
        robots.read()
    except Exception:
        robots.allow_all = True
    opener = urllib.request.build_opener(_SameOriginRedirects(origin))
    fetched: list[tuple[str, Path]] = []
    for url in urls:
        if url in have or not robots.can_fetch("*", url):
            continue
        try:
            with opener.open(urllib.request.Request(url, headers={"accept": "*/*"}), timeout=60) as r:
                if r.status != 200:
                    continue
                body = r.read(MAX_BYTES + 1)
        except Exception:
            continue
        if len(body) > MAX_BYTES:
            continue
        f = d / f"{len(known) + len(fetched) + 1:03d}"
        f.write_bytes(body)
        fetched.append((url, f))
        time.sleep(1)
    types = identify([f for _, f in fetched]) if fetched else {}
    for url, f in fetched:
        label, group = types.get(f, ("unknown", "unknown"))
        src = {"url": url, "file": str(f), "label": label, "group": group, "markdown": None, "openapi": False}
        md = None
        if label in ("json", "yaml"):
            try:
                text = f.read_text(encoding="utf-8")
                if label == "json":
                    doc = json.loads(text)
                else:
                    import yaml  # markitdown[all] brings PyYAML

                    doc = yaml.safe_load(text)
                if isinstance(doc, dict) and (doc.get("openapi") or doc.get("swagger")):
                    src["openapi"] = True
                    md = summarize_openapi(doc)
            except Exception:
                pass
        if md is None:
            try:
                md = to_markdown(f, label)
            except Exception:
                md = None
        if md:
            Path(f"{f}.md").write_text(md, encoding="utf-8")
            src["markdown"] = f"{f}.md"
        print(f"source {urlsplit(url).path} -> {label}{' (OpenAPI)' if src['openapi'] else ''}", flush=True)
        known.append(src)
    index.write_text(json.dumps(known, indent=2), encoding="utf-8")
    return known
