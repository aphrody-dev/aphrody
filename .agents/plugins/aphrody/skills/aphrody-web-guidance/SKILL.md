---
name: aphrody-web-guidance
description: GoogleChrome/modern-web-guidance-src Baseline web standards, Coherence Auditor, and modern CSS/WebGPU/Wasm checks for Aphrody web & NSH applications.
---

# Modern Web Guidance (`GoogleChrome/modern-web-guidance-src`)

Applies Google Chrome Baseline web standards natively across Aphrody web builds, `gpui_web`, NSH HTML/TSX surfaces, and `templates/wasm-wgpu/*` via `aphrody_design::modern_web_guidance` and `gpui-wgpu design web-guidance`.

## Checklist

- Use semantic HTML landmarks (`<main>`, `<header>`, `<section>`, `<canvas>`) and responsive `<meta name="viewport">`.
- Drive theming via M3 Expressive CSS custom properties (`--md-sys-*`) and variable fonts (`Google Sans Flex` `font-variation-settings`), never hardcoded inline palette hex values in views.
- Pair `WebGPU` (`navigator.gpu`) with graceful capability fallback messaging and Wasm CSP `'wasm-unsafe-eval'` only where Wasm instantiation requires it.
- Audit projects via `gpui-wgpu design web-guidance <dir>` or MCP tool `modern_web_guidance_audit`.
