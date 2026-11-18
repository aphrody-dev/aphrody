"""Layout parity of a clone against the original's screenshots.

For every original screen the clone is captured at the same route (crawl4ai, same viewport). Each pair gets:
- a layout score: cosine similarity of TensorFlow image embeddings (a Keras ImageNet backbone with global average
  pooling, REPLICA_BACKBONE or EfficientNetV2B0), reported 0-100;
- obscura's pixel and structural-edge diagnostics (render-repros/check.py, absorbed as obscura_check.py), which
  carry no verdict of their own.

Writes clone-screens/<id>.png and visual.json in the replica directory."""

from __future__ import annotations

import asyncio
import base64
import json
import os
from pathlib import Path
from urllib.parse import urlsplit


async def capture(clone_url: str, screens: list[dict], out: Path, viewport: tuple[int, int]) -> dict[str, Path]:
    from crawl4ai import AsyncWebCrawler, BrowserConfig, CacheMode, CrawlerRunConfig

    d = out / "clone-screens"
    d.mkdir(parents=True, exist_ok=True)
    base = clone_url.rstrip("/")
    # the route the original screenshot shows: the page reached after redirects
    targets = {s["id"]: base + (urlsplit(s["final"]).path or "/") + (f"?{urlsplit(s['final']).query}" if urlsplit(s["final"]).query else "") for s in screens if s.get("screenshot")}
    shots: dict[str, Path] = {}
    run = CrawlerRunConfig(screenshot=True, cache_mode=CacheMode.BYPASS, wait_until="domcontentloaded", delay_before_return_html=1.0)
    async with AsyncWebCrawler(config=BrowserConfig(headless=True, viewport_width=viewport[0], viewport_height=viewport[1])) as crawler:
        for sid, url in targets.items():
            r = await crawler.arun(url, config=run)
            if r.success and r.screenshot:
                p = d / f"{sid}.png"
                p.write_bytes(base64.b64decode(r.screenshot))
                shots[sid] = p
    return shots


def embedder():
    import tensorflow as tf

    name = os.environ.get("REPLICA_BACKBONE", "EfficientNetV2B0")
    apps = tf.keras.applications
    cls = getattr(apps, name)
    # the backbone's own preprocessing, from the submodule that defines it; identity when none is published
    module = next((m for m in vars(apps).values() if getattr(m, name, None) is cls and hasattr(m, "preprocess_input")), None)
    preprocess = module.preprocess_input if module else (lambda x: x)
    model = cls(include_top=False, weights="imagenet", pooling="avg")
    size = model.input_shape[1] or 224

    def embed(path: Path):
        # PNG, JPEG or WebP, whatever the capture produced
        img = tf.io.decode_image(tf.io.read_file(str(path)), channels=3, expand_animations=False)
        img = tf.image.resize(img, (size, size))
        v = model(preprocess(img[None, ...]), training=False)[0]
        return v / (tf.norm(v) + 1e-9)

    return name, [d.name for d in tf.config.list_physical_devices()], embed


def compare(out: Path, clone_url: str, viewport: tuple[int, int]) -> dict:
    import numpy as np
    from PIL import Image

    from .crawl import load
    from .obscura_check import pair_metrics

    screens = [s for s in load(out) if s.get("screenshot")]
    shots = asyncio.run(capture(clone_url, screens, out, viewport))
    name, devices, embed = embedder()
    result = {"backbone": name, "devices": devices, "clone": clone_url, "screens": {}}
    for s in screens:
        clone = shots.get(s["id"])
        if not clone:
            result["screens"][s["id"]] = {"layout": 0.0, "error": "clone route did not render"}
            continue
        original = out / s["screenshot"]
        cos = float((embed(original) * embed(clone)).numpy().sum())
        a = Image.open(original).convert("RGB")
        b = Image.open(clone).convert("RGB").resize(a.size)
        result["screens"][s["id"]] = {"layout": round(max(0.0, cos) * 100, 1), "diagnostics": pair_metrics(np.asarray(b), np.asarray(a))}
    scores = [v["layout"] for v in result["screens"].values()]
    result["mean"] = round(sum(scores) / len(scores), 1) if scores else None
    (out / "visual.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    return result
