---
name: stitch-design-md
description: Native Google Stitch (stitch.withgoogle.com), stitch-sdk, stitch-skills, and google-labs-code/design.md workflow inside Aphrody (`google_design_stitch` + `nsh_workspace` + `gpui-wgpu design`).
---

# Stitch & DESIGN.md Integration

Combines `google-labs-code/design.md`, `google-labs-code/stitch-sdk`, and `google-labs-code/stitch-skills` directly in native Rust inside [`google_design_stitch`](../../../../crates/google_design_stitch/src/google_design_stitch.rs) and [`nsh_workspace`](../../../../crates/nsh_workspace/src/design_stitch_studio.rs).

## Core Capabilities

1. **`DESIGN.md` Visual Identity**:
   - YAML frontmatter (`colors`, `typography`, `rounded`, `spacing`, `motion`) + fenced `yaml` token blocks + `{path.to.token}` cross-references.
   - Parsed, WCAG-linted, diffed, and exported to W3C `design_tokens.json`, CSS custom properties (`--md-sys-*`), Tailwind config JSON, and WGSL `GoogleDesignTheme` uniforms.
2. **Stitch Projects, Screens & `data-stitch-id` Codegen**:
   - Preserves `data-stitch-id` attributes across generated NSH TSX and GPUI Rust views for round-trip sync.
   - Downloads signed `htmlCode.downloadUrl` & `screenshot.downloadUrl` assets using pure-Rust `ureq` (`http_mirror`), or generates offline PNG screen captures via `screen_capture`.
3. **CLI & Workspace Actions**:
   - CLI: `gpui-wgpu design design-md lint`, `gpui-wgpu design stitch capture --out target/stitch.png`
   - Studio Actions in Aphrody: `nsh_workspace::OpenStitchStudio`, `nsh_workspace::OpenGoogleDesignMirror`, `nsh_workspace::CaptureDesignScreen`
