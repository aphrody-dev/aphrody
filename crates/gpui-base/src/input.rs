//! Text input: the shared editing engine and the three states built on it.
//!
//! Nothing here should be `pub` unless it is reachable from outside the crate.
//! A `pub` on an item behind a private module says something the module path
//! contradicts, and it hides the real API surface from anyone reading it.
#![warn(unreachable_pub)]

use gpui::App;

/// Character used by masked editor modes.
pub(crate) const MASK_CHAR: char = '•';

#[path = "input/editor/auto_close.rs"]
mod auto_close;
mod base;
#[path = "input/base/blink_cursor.rs"]
pub(crate) mod blink_cursor;
#[path = "input/base/change.rs"]
mod change;
#[path = "input/base/cursor.rs"]
mod cursor;
#[path = "input/editor/decorations.rs"]
mod decorations;
#[path = "input/editor/diagnostics.rs"]
mod diagnostics;
#[path = "input/editor/display_map/mod.rs"]
mod display_map;
mod editor;
#[path = "input/base/element.rs"]
mod element;
#[path = "input/editor/highlighting.rs"]
mod highlighting;
#[path = "input/editor/indent.rs"]
mod indent;
#[path = "input/base/inline_tokens.rs"]
mod inline_tokens;
mod input;
#[path = "input/base/token_presentation.rs"]
mod token_presentation;
pub use inline_tokens::{InlineToken, InlineTokenError, InlineTokenSpan, InputContent};
pub(crate) use token_presentation::InlineTokenPresentation;
pub use token_presentation::{
    InlineTokenClickEvent, InlineTokenClickListener, InlineTokenContext, InlineTokenHoverEvent,
    InlineTokenHoverListener, InlineTokenRenderer,
};
#[path = "input/base/a11y_text.rs"]
mod a11y_text;
#[path = "input/base/kind.rs"]
mod kind;
#[path = "input/editor/language.rs"]
mod language;
#[path = "input/editor/language_config.rs"]
pub mod language_config;
#[path = "input/base/layout.rs"]
mod layout;
#[path = "input/editor/lsp/mod.rs"]
mod lsp;
#[path = "input/base/mask_pattern.rs"]
mod mask_pattern;
#[path = "input/base/mode.rs"]
mod mode;
#[path = "input/base/movement.rs"]
mod movement;
#[path = "input/base/native.rs"]
mod native;
#[path = "input/base/rope_ext.rs"]
mod rope_ext;
mod scrollbar_presentation;
#[path = "input/editor/search.rs"]
mod search;
#[path = "input/base/selection.rs"]
mod selection;
#[path = "input/base/state.rs"]
mod state;
#[path = "input/base/tab_stops.rs"]
mod tab_stops;
mod textarea;
pub use scrollbar_presentation::{EditorScrollbarContext, EditorScrollbarRenderer};
#[path = "input/base/touch.rs"]
mod touch;
#[path = "input/base/undo_manager.rs"]
mod undo_manager;

pub(crate) fn init(cx: &mut App) {
    state::init(cx);
}

pub use crate::number_input::{NumberInputEvent, NumberStep};
pub use base::{InputBase, InputContextMenuCapabilities, InputStyles};
pub use blink_cursor::CursorBlink;
pub use cursor::Selection;
pub use decorations::{
    GutterDecoration, GutterDecorationCollection, OverviewRulerDecoration,
    OverviewRulerDecorationCollection, OverviewRulerLane, RangeDecoration,
    RangeDecorationCollection, RangeDecorationStyle, TextDecoration, TextDecorationCollection,
};
pub use diagnostics::{
    Diagnostic, DiagnosticEntry, DiagnosticRelatedInformation, DiagnosticSet, DiagnosticSeverity,
    DiagnosticSummary, DiagnosticTag, RelatedInformation,
};
pub use display_map::{BufferPoint, DisplayMap, DisplayPoint, FoldRange, WrappingIndent};
pub use editor::{Editor, EditorState};
pub use highlighting::{
    DiagnosticColors, FoldIconRenderer, FoldPlaceholderRenderer, HighlightStyleResolver,
    InputEditorStyle, InputHighlighter, InputHighlighterFactory, SharedHighlightStyleResolver,
    SyntaxContext, SyntaxContextProvider,
};
pub use indent::TabSize;
pub use input::{Input, InputState};
pub use kind::{
    EditorExtras, EditorMode, InputExtras, InputMode, InputModeKind, MultiLineMode, TextareaExtras,
    TextareaMode,
};
pub(crate) use language::EditorLanguage;
pub use language::{LanguageProvider, set_language_config, set_language_provider};
pub(crate) use language_config::LanguageConfig;
pub use language_config::{AutoClosingPair, BracketPair, IndentationRules};
pub use lsp::{
    CodeActionItem, CodeActionMenuState, CodeActionProvider, CompletionHighlighter,
    CompletionMenuOptions, CompletionMenuState, CompletionProvider, DefinitionProvider,
    DocumentColorProvider, DocumentRangeSemanticTokensProvider, HoverPopoverState, HoverProvider,
    InputOverlayKind, Lsp, ShowDocumentHandler, highlight_characters, highlight_contains,
    highlight_prefix,
};
pub use lsp_types::Position;
pub use mask_pattern::MaskPattern;
#[cfg(target_os = "macos")]
#[doc(hidden)]
pub use native::set_text_content_type;
pub use native::{NativeMenu, NativeMenuItem};
pub use rope_ext::{InputEdit, Point, RopeExt, RopeLines};
pub use ropey::Rope;
pub use search::{SearchMatcher, SearchSession};
pub use state::*;
pub use textarea::{Textarea, TextareaState};
