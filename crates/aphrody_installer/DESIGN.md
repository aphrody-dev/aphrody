# Aphrody OS Installer & DE Switcher Design Specification

## Target Matrix

- **Desktop (GPUI / Native)**: Primary side-rail workspace + multi-pane step planner and dry-run log inspector.
- **Web (WebGPU / Browser OS)**: Adaptive responsive layout supporting environment switching preview and headless plan export.
- **TUI (`nsh-platform-tui`)**: Cell-aligned keyboard navigation matching CachyOS `New-Cli-Installer` step parity (`Prepare`, `Install`, `Switch DE`, `Rescue`).
- **Mobile**: Compact single-column stacked wizard cards with safe-area padding (`p_safe_area`).

## Design Tokens

All visual surfaces read exclusively from semantic `theme()` tokens in [`theme.json`](./theme.json) (`background`, `card`, `primary`, `secondary`, `muted`, `accent`, `destructive`, `border`, `input`, `ring`, `radius`) anchored to Material 3 Expressive (`assets/os/google_design_tokens.toml`).

## Motion Inventory

1. **Tab / Navigation Selection Indicator**: `LayoutTransition::spring(Spring::new(280.0).damping_ratio(0.85))` across `Tabs` (`tabs` indicator tracking active workflow mode: `Guided`, `Switch DE`, `Advanced`, `Rescue`).
2. **Choice Pill & Action Button Press**: `active(|s| s.opacity(0.75))` + smooth border/surface token transitions on press, honoring system reduced motion automatically via `nsh_base`.
3. **Controlled Switch Thumb & Track**: `transition-colors` on track (`bg-muted` -> `checked:bg-primary`) and `transition-transform` on thumb (`checked:translate-x-5`) for LUKS encryption, ZRAM swap, autologin, and hardware driver toggles.
4. **Dry-Run Plan Execution Progress**: Determinate width animation across steps (`0%` -> `100%`) using `nsh_base::progress()`.
