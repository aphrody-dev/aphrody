//! # `gpui_code` — Modular Rust Code Engine for GPUI
//!
//! Provides every building block needed to parse, index, diff, search, run, version,
//! and edit source code in Rust and GPUI applications.
//!
//! ## Feature Layers (opt-in or disable via `default-features = false`)
//! - **Core (always enabled)**: [`rope`], [`text`], [`sum_tree`], [`buffer_diff`],
//!   [`streaming_diff`], [`fuzzy`], [`fuzzy_nucleo`], [`path`].
//! - **`syntax`**: [`language_core`], [`language`], [`language_detection`], [`snippet`], [`outline`].
//! - **`grammars`**: Built-in native Tree-sitter grammars ([`grammars`], [`languages`]).
//! - **`lsp`**: Language Server Protocol client ([`lsp`], [`json_schema_store`], [`prettier`]).
//! - **`git`**: Git repository & blame/status/hunk engine ([`git`], [`git_hosting_providers`]),
//!   plus [`git_ui`] / [`git_ui_core`] with `git-ui`.
//! - **`worktree`**: Filesystem abstraction, fast file trees, and multi-root projects
//!   ([`fs`], [`worktree`], [`project`]).
//! - **`terminal`**: Cross-platform PTY & VT emulator ([`terminal`], [`shell_command_parser`]),
//!   plus [`terminal_view`] with `terminal-ui`.
//! - **`editor`**: Multi-cursor GPU [`editor::Editor`], [`multi_buffer`], [`search`],
//!   [`diagnostics`], and optional [`vim`] modal editing.

#[cfg(feature = "syntax")]
pub use buffer_diff;
pub use fuzzy;
pub use fuzzy_nucleo;
pub use gpui_ui;
pub use imara_diff;
pub use path;
pub use rope::{self, Point, Rope};
pub use streaming_diff;
pub use sum_tree::{self, SumTree};
pub use text::{self, Anchor, Bias, Buffer as TextBuffer, BufferId, OffsetUtf16, ToOffset, ToPoint};

#[cfg(feature = "syntax")]
pub use language::{self, Buffer, Language, LanguageConfig, LanguageMatcher, LanguageRegistry};
#[cfg(feature = "syntax")]
pub use language_core;
#[cfg(feature = "syntax")]
pub use language_detection;
#[cfg(feature = "syntax")]
pub use outline;
#[cfg(feature = "syntax")]
pub use snippet;

#[cfg(feature = "syntax")]
pub use languages;
#[cfg(feature = "grammars")]
pub use grammars;

#[cfg(feature = "lsp")]
pub use json_schema_store;
#[cfg(feature = "lsp")]
pub use lsp::{self, LanguageServer, LanguageServerId, LanguageServerName};
#[cfg(feature = "lsp")]
pub use prettier;

#[cfg(feature = "git")]
pub use git;
#[cfg(feature = "git")]
pub use git_hosting_providers;
#[cfg(feature = "git-ui")]
pub use git_ui;
#[cfg(feature = "git-ui")]
pub use git_ui_core;

#[cfg(feature = "worktree")]
pub use fs::{self, Fs, RealFs};
#[cfg(feature = "worktree")]
pub use project::{self, Project};
#[cfg(feature = "worktree")]
pub use worktree::{self, Worktree, WorktreeId};

#[cfg(feature = "terminal")]
pub use shell_command_parser;
#[cfg(feature = "terminal")]
pub use terminal::{self, Terminal, TerminalBuilder};
#[cfg(feature = "terminal-ui")]
pub use terminal_view::{self, TerminalView};

#[cfg(feature = "editor")]
pub use diagnostics;
#[cfg(feature = "editor")]
pub use editor::{self, Editor, EditorMode};
#[cfg(feature = "editor")]
pub use multi_buffer::{self, ExcerptRange, MultiBuffer};
#[cfg(feature = "editor")]
pub use search;

#[cfg(feature = "vim")]
pub use vim;
#[cfg(feature = "vim")]
pub use vim_mode_setting;

/// Create a standalone [`LanguageRegistry`] preloaded with built-in native Tree-sitter grammars
/// when the `grammars` feature is enabled.
#[cfg(feature = "syntax")]
pub fn create_language_registry(cx: &gpui_ui::App) -> std::sync::Arc<LanguageRegistry> {
    let registry = LanguageRegistry::new(cx.background_executor().clone());
    #[cfg(feature = "grammars")]
    registry.register_native_grammars(grammars::native_grammars());
    std::sync::Arc::new(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rope_and_fuzzy_primitives() {
        let mut rope = Rope::from("fn main() {\n    println!(\"gpui_code\");\n}\n");
        assert_eq!(rope.max_point(), Point::new(3, 0));
        rope.replace(3..7, "start");
        assert!(rope.to_string().starts_with("fn start()"));
    }
}
