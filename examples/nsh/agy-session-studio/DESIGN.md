# AGY & Storytold Parity Studio — NSH Design Specification

Anchored to **`https://design.google/`** (*Material 3 Neural Expressive* + *Code is a Design Material*) and [`docs/design/manifest.json`](../../../docs/design/manifest.json).

## Visual Atmosphere & Token Rules
- **Zero raw colors in presentation code**: Every control in `src/ui/` and every page in `src/pages/` consumes semantic theme tokens (`bg-background`, `bg-card`, `bg-popover`, `text-foreground`, `text-muted-foreground`, `bg-primary`, `border-border`) defined in [`theme.json`](./theme.json).
- **Typography**: `Google Sans Flex` (`opsz`, `wght`, `ROND`) for titles, navigation pills, and KPI summaries; `JetBrains Mono` for session UUIDs, step timings, SHA-256 digests, and tool calls.
- **Targets**: Web (Chrome SSE auto-reload + portable offline `.html`) and Desktop (`nsh_workspace` inside Aphrody IDE).

## Motion Inventory (Normative NSH Design Guide Requirement)

| Element / Transition | Trigger | Mechanism | Duration / Spring | Reduced-Motion Fallback |
| --- | --- | --- | --- | --- |
| Segmented View Bar (`Overview` / `Timeline` / `Storytold 82` / `GUI Parity`) | Tab selection | Spatial spring (`stiffness: 1400, damping: 0.9`) | `180ms` sliding pill | Instant state highlight (`0ms`) |
| Session Omnibar Filter & Cascading Mode Picker | Focus / open | Spatial scale `0.97 -> 1.0` + Effects opacity | `200ms` / `120ms` | Opacity-only `80ms` |
| Live SSE Auto-Reload Pulse (`ThinkingAuraBadge`) | `/api/events` message | Effects spring (`damping: 1.0`, zero bounce) | `240ms` aura wash | Static `"LIVE"` pill |
| Step Inspector Drawer / Card Expansion | Row click | `LayoutAnimation` height + fade | `220ms` | Instant expand |
| Light / Dark Appearance Toggle | Theme button | Effects cross-fade across `--md-sys-color-*` | `160ms` | Instant token swap |
