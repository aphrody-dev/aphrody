---
page: design
---
The design page: tokens, DESIGN.md and icons as files; the canvas-to-code loop.

**DESIGN SYSTEM (REQUIRED):**
Copy section 6 of .stitch/DESIGN.md verbatim.

**Page Structure:**
1. Workspace 4 in the panel and the dock; same WGSL wallpaper, panel, dock and notification as the other workspaces.
2. Section `fw/crates/fw-m3/tokens/google_design_tokens.toml`: one row per `sys.color.dark` role with its hex and a swatch, plus the file size in bytes.
3. Section `sys.motion.springs`: one row per spring (stiffness, damping ratio) with a live spring curve drawn in WGSL or SVG from those two numbers; no images.
4. Section `assets/icons/`: icon count, total bytes, and the inlined Material Symbols count of this site build.
5. Section `DESIGN.md` files: one row per DESIGN.md in the workspace (path, bytes, heading count).
6. Footer identical to the other pages.
Generate by adding a page to site/build.ts; zero runtime requests; no prose.
