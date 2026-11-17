# Aphrody's Feature Development Process

This guide covers moderate-to-large features — new UI, behavior changes, or work that spans multiple crates in Aphrody. Small keybindings, settings tweaks, or targeted bug fixes do not need all of this.

> **Before you start:** Make sure the feature aligns with project direction before investing significant effort. Read the [Contributing Guide](../../../CONTRIBUTING.md) and, if there isn't already a confirmed GitHub issue, open a [GitHub Discussion](https://github.com/aphrody-dev/aphrody/discussions/new?category=feature-requests) first. Feature PRs that skip early design alignment have a much lower merge rate.

## 1. Why does this matter?

Every feature starts as a concrete problem statement:

- **What problem does this solve?**
- **What's the evidence?** GitHub issues, discussions, reproduction reports, or benchmarks.
- **Is there prior art?** Name how comparable tools or crates solve it and why that pattern fits Aphrody (or Zed/GPUI/NSH conventions).

## 2. What is it?

Write a short, concrete feature statement backed by the context above. If you cannot describe it in a few sentences, narrow the scope first.

Example structure:

**Feature:** Inline Git Blame

**Purpose:** Show the last commit author and message for each line directly after editor text so developers can inspect history without opening a separate blame view.

**Background & Decisions:**
Compare existing editor approaches and explain key architectural choices (for example, shelling out safely via `smol::process::Command` vs. an in-process git library, or keeping `gpui_code --no-default-features` decoupled).

## 3. What else does this affect?

Walk through this checklist before writing code (skip items that do not apply):

- **Actions & keybindings.** What actions are introduced, and do default bindings conflict with existing Zed-compatible keymaps?
- **Settings & Zed compatibility.** Is behavior configurable per-user/per-project/per-language? Does it work cleanly in both `.aphrody/` and `.zed/` project configs?
- **Themes & design tokens.** Does this need semantic tokens that work cleanly across both light and dark modes (`ui-checklist.md`)?
- **Vim mode.** Do Vim motions/modes interact predictably with the change?
- **Remote development.** Does it work over remote projects (SSH/WSL), where paths, shells, and env vars differ?
- **Persistence across restarts.** Should state persist in workspace/SQLite storage?
- **Accessibility.** Is it keyboard-navigable with clear focus indicators and contrast?
- **Platform & GPU differences.** Does behavior differ across Linux (Wayland/X11), macOS (Metal), Windows (DX12/Vulkan), or WebGPU/Wasm?
- **Performance.** Does it stay responsive (<8 ms frame budget for 120 FPS) on large files and big workspaces?
- **Security.** Does it respect Workspace Trust and sandboxing boundaries without introducing unsafe command/shell surfaces?

If your feature touches the **editor** specifically, verify interactions with gutter elements, inline blocks, multiple cursors, folding, edit predictions, code intelligence popovers, and the minimap — and ensure features intended only for single buffers behave properly in multi-buffers.

## 4. Ship it

Use this write-up as the basis for your GitHub Discussion, issue, or PR description so reviewers can align on goals, tradeoffs, and verification steps.
