"""Recon crawl on crawl4ai: breadth-first deep crawl of one site, robots.txt honoured, one request at a time with a
randomised human-pace delay, a rendered screenshot, the page as Markdown and a DOM summary per page. Pages of the
same route shape are capped so a site with millions of item pages yields its screens, not its catalogue.

Writes, in the replica directory: screens.jsonl (one record per page, the format tools/replica/recon.ts reads),
screens/<id>.png and pages/<id>.md."""

from __future__ import annotations

import asyncio
import base64
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from urllib.parse import urljoin, urlsplit, urlunsplit

from lxml import html as lxml_html


def same_origin(url: str, origin: str) -> bool:
    """Same scheme, host and port: a prefix test would accept https://site.example.evil for https://site.example."""
    u = urlsplit(url)
    return f"{u.scheme}://{u.netloc}" == origin


class Shapes:
    """Route shape of a path: navigation roots and route words kept, every other segment a wildcard. A route word
    (tree, discussions) follows variable segments: it is seen after two different segments that are not navigation
    roots. A namespace seen under several roots (/datasets/acme, /spaces/acme) is not one."""

    def __init__(self) -> None:
        self.roots: set[str] = set()
        self.words: dict[str, set[str]] = defaultdict(set)

    def add_nav(self, paths: list[str]) -> None:
        for p in paths:
            segs = [s for s in p.split("/") if s]
            if segs:
                self.roots.add(segs[0])

    def learn(self, path: str) -> None:
        segs = [s for s in path.split("/") if s]
        for i, s in enumerate(segs[1:], start=1):
            self.words[f"{i}:{s}"].add(segs[i - 1])

    def of(self, path: str) -> str:
        segs = [s for s in path.split("/") if s]

        def keep(s: str, i: int) -> bool:
            if i == 0:
                return s in self.roots
            before = self.words.get(f"{i}:{s}", set())
            return len(before) >= 2 and not (before & self.roots) and re.fullmatch(r"[a-z][a-z-]*", s) is not None

        return "/" + "/".join(s if keep(s, i) else "*" for i, s in enumerate(segs))


def summarize(page_html: str, base: str) -> dict:
    """The parts of a rendered page the recon map needs: headings, controls, forms, links, roles, components."""
    doc = lxml_html.fromstring(page_html or "<html></html>")

    def text(e) -> str:
        return re.sub(r"\s+", " ", e.text_content() or "").strip()[:80]

    def absolute(href: str | None) -> str | None:
        if not href or href.startswith(("javascript:", "mailto:", "tel:")):
            return None
        u = urlsplit(urljoin(base, href))
        return urlunsplit((u.scheme, u.netloc, u.path, u.query, ""))

    roles = Counter(e.get("role") for e in doc.iter() if isinstance(e.tag, str) and e.get("role"))
    names: list[str] = []
    for e in doc.iter():
        if not isinstance(e.tag, str):
            continue
        n = e.get("data-target") or e.get("data-component") or e.get("data-testid") or (e.tag if "-" in e.tag else None)
        if n and n not in names:
            names.append(n)
    buttons: list[str] = []
    for b in doc.xpath("//button|//*[@role='button']"):
        label = text(b) or (b.get("aria-label") or "")
        if label and label not in buttons:
            buttons.append(label)
    title = doc.findtext(".//title") or ""
    return {
        "title": re.sub(r"\s+", " ", title).strip()[:120],
        "headings": [f"{h.tag} {text(h)}" for h in doc.xpath("//h1|//h2|//h3") if text(h)][:40],
        "buttons": buttons[:60],
        "inputs": [f"{i.get('type') or i.tag}:{(i.get('name') or i.get('aria-label') or i.get('placeholder') or '')[:60]}" for i in doc.xpath("//input[not(@type='hidden')]|//select|//textarea")][:40],
        "forms": [{"method": (f.get("method") or "get").lower(), "action": f.get("action") or "", "fields": len(f.xpath(".//input|.//select|.//textarea"))} for f in doc.xpath("//form")][:20],
        "links": list(dict.fromkeys(filter(None, (absolute(a.get("href")) for a in doc.xpath("//a[@href]"))))),
        "nav": list(dict.fromkeys(filter(None, (absolute(a.get("href")) for a in doc.xpath("//header//a[@href]|//nav//a[@href]"))))),
        "roles": dict(roles),
        "components": names[:80],
        "landmarks": [t for t in ("header", "nav", "main", "aside", "footer", "dialog") if doc.find(f".//{t}") is not None],
        "text": len(doc.text_content() or ""),
    }


def load(out: Path) -> list[dict]:
    f = out / "screens.jsonl"
    return [json.loads(line) for line in f.read_text(encoding="utf-8").splitlines() if line.strip()] if f.exists() else []


async def crawl(start: list[str], out: Path, depth: int, limit: int, per_shape: int, delay: float, viewport: tuple[int, int], exclude: str | None) -> list[dict]:
    from crawl4ai import AsyncWebCrawler, BrowserConfig, CacheMode, CrawlerRunConfig
    from crawl4ai.deep_crawling import BFSDeepCrawlStrategy
    from crawl4ai.deep_crawling.filters import FilterChain, URLFilter

    (out / "screens").mkdir(parents=True, exist_ok=True)
    (out / "pages").mkdir(parents=True, exist_ok=True)
    origin = "{0.scheme}://{0.netloc}".format(urlsplit(start[0]))
    done = load(out)
    # pages refused by a rate limit in an earlier run are crawled again; pages of other origins (a crawler treating
    # subdomains as internal) are not part of this site's recon
    kept = [s for s in done if same_origin(s["url"], origin) and not re.search(r"\b429\b|too many requests", s.get("error") or "", re.I)]
    if len(kept) != len(done):
        done = kept
        (out / "screens.jsonl").write_text("".join(json.dumps(s) + "\n" for s in done), encoding="utf-8")
    seen = {s["url"] for s in done} | {s["final"] for s in done}
    shapes = Shapes()
    counts: Counter[str] = Counter()
    for s in done:
        shapes.learn(urlsplit(s["final"]).path)
        if s.get("dom"):
            shapes.add_nav([urlsplit(u).path for u in s["dom"]["nav"] if same_origin(u, origin)])
    for s in done:
        counts[shapes.of(urlsplit(s["url"]).path)] += 1
    skip = re.compile(exclude) if exclude else None

    class ShapeFilter(URLFilter):
        def apply(self, url: str) -> bool:
            u = urlsplit(url)
            ok = same_origin(url, origin) and url not in seen and not (skip and skip.search(url)) and counts[shapes.of(u.path)] < per_shape
            if ok:
                counts[shapes.of(u.path)] += 1
            self._update_stats(ok)
            return ok

    browser = BrowserConfig(headless=True, viewport_width=viewport[0], viewport_height=viewport[1])
    ids: dict[str, str] = {s["url"]: s["id"] for s in done}
    captured: list[dict] = []
    n = max((int(s["id"][1:]) for s in done if s["id"][1:].isdigit()), default=0)
    async with AsyncWebCrawler(config=browser) as crawler:
        for k, seed in enumerate(start):
            if len(captured) >= limit:
                break
            if k:
                # seeds are separate crawls: the same pace applies between them
                await asyncio.sleep(delay)
            run = CrawlerRunConfig(
                deep_crawl_strategy=BFSDeepCrawlStrategy(max_depth=depth, include_external=False, max_pages=limit - len(captured), filter_chain=FilterChain([ShapeFilter()])),
                screenshot=True,
                check_robots_txt=True,
                cache_mode=CacheMode.BYPASS,
                semaphore_count=1,
                mean_delay=delay,
                max_range=delay / 2,
                wait_until="domcontentloaded",
                delay_before_return_html=1.0,
                stream=True,
            )
            backoff = 30.0
            async for r in await crawler.arun(seed, config=run):
                if r.url in seen or not same_origin(r.url, origin):
                    # a page captured before (the seed of a later round, typically) is crawled for its links only
                    continue
                if not r.success:
                    # the shape quota was taken when the link was queued; a page that did not load gives it back
                    counts[shapes.of(urlsplit(r.url).path)] -= 1
                if not r.success and re.search(r"\b429\b|too many requests", r.error_message or "", re.I):
                    # rate limited: nothing recorded (a later round finds the page again), the crawl pauses with a
                    # doubling wait; the stream is consumed lazily, so pausing here pauses the crawler
                    print(f"rate limited at {urlsplit(r.url).path}; waiting {int(backoff)} s", flush=True)
                    await asyncio.sleep(backoff)
                    backoff = min(backoff * 2, 600.0)
                    continue
                backoff = 30.0
                n += 1
                sid = f"S{n:02d}"
                meta = r.metadata or {}
                final = r.redirected_url or r.url
                seen.update({r.url, final})
                ids[r.url] = sid
                rec = {"id": sid, "url": r.url, "final": final, "depth": meta.get("depth", 0), "from": ids.get(meta.get("parent_url") or ""), "viewport": f"{viewport[0]}x{viewport[1]}", "screenshot": None, "mobile": None, "markdown": None, "dom": None}
                if not r.success:
                    rec["error"] = (r.error_message or "failed")[:300]
                else:
                    if r.screenshot:
                        (out / "screens" / f"{sid}.png").write_bytes(base64.b64decode(r.screenshot))
                        rec["screenshot"] = f"screens/{sid}.png"
                    md = getattr(r.markdown, "raw_markdown", None) or (str(r.markdown) if r.markdown else "")
                    if md:
                        (out / "pages" / f"{sid}.md").write_text(md, encoding="utf-8")
                        rec["markdown"] = f"pages/{sid}.md"
                    rec["dom"] = summarize(r.html or "", final)
                    shapes.learn(urlsplit(final).path)
                    if rec["depth"] == 0:
                        shapes.add_nav([urlsplit(u).path for u in rec["dom"]["nav"] if same_origin(u, origin)])
                print(f"{sid} {urlsplit(r.url).path} {(rec['dom'] or {}).get('title', rec.get('error', ''))[:60]}", flush=True)
                with (out / "screens.jsonl").open("a", encoding="utf-8") as f:
                    f.write(json.dumps(rec) + "\n")
                captured.append(rec)
    return captured


def run(start: list[str], out: Path, **kw) -> list[dict]:
    return asyncio.run(crawl(start, out, **kw))
