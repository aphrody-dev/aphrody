//! `cosmic-edit` buffer editor with integrated Tree-sitter AST & symbol extraction via
//! [`aphrody_pilot::analyze_with_tree_sitter`].

use aphrody_pilot::{SyntaxLanguage, TreeSitterReport, analyze_with_tree_sitter};
use aphrody_ui::{KeyValueRow, OsTheme};
use gpui::{AnyElement, App, div, prelude::*, px};
use ui::{Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::apps::section_heading;

#[derive(Debug, Clone)]
pub struct CosmicEditState {
    pub path: String,
    pub language: SyntaxLanguage,
    pub source: String,
    pub last_report: Option<TreeSitterReport>,
}

impl Default for CosmicEditState {
    fn default() -> Self {
        let source = r#"use aphrody::prelude::*;

pub struct CosmicAppletSurface {
    pub name: &'static str,
    pub active_hint_px: f32,
}

pub fn render_cosmic_dock() -> &'static str {
    "aphrody-cosmic-epoch"
}
"#
        .to_owned();
        let last_report = analyze_with_tree_sitter(SyntaxLanguage::Rust, &source).ok();
        Self {
            path: "crates/aphrody_desktop/src/apps/edit.rs".to_owned(),
            language: SyntaxLanguage::Rust,
            source,
            last_report,
        }
    }
}

impl CosmicEditState {
    pub fn load_sample(&mut self, language: SyntaxLanguage) {
        self.language = language;
        self.source = match language {
            SyntaxLanguage::Rust => {
                "pub struct Panel {\n    pub height: f32,\n}\npub fn boot() {}\n".to_owned()
            }
            SyntaxLanguage::TypeScript => {
                "export interface CosmicPanel { height: number }\nexport function mountShell(): void {}\n".to_owned()
            }
            SyntaxLanguage::Tsx => {
                "export function DockItem() { return <button>Launch</button>; }\n".to_owned()
            }
            SyntaxLanguage::Python => "def sync_wayland():\n    return True\n".to_owned(),
            SyntaxLanguage::Json => {
                "{\n  \"compositor\": \"cosmic-comp\",\n  \"tiling\": \"dwindle\"\n}\n".to_owned()
            }
        };
        self.last_report = analyze_with_tree_sitter(language, &self.source).ok();
    }
}

pub fn render_cosmic_edit_window(
    theme: &OsTheme,
    edit: &CosmicEditState,
    on_switch_lang: Box<dyn Fn(SyntaxLanguage, &mut App) + 'static>,
) -> AnyElement {
    let cb = std::rc::Rc::new(on_switch_lang);
    let langs = [
        ("Rust", SyntaxLanguage::Rust),
        ("TypeScript", SyntaxLanguage::TypeScript),
        ("TSX", SyntaxLanguage::Tsx),
        ("Python", SyntaxLanguage::Python),
    ];
    let mut tabs = h_flex().gap_1p5();
    for (idx, (label, lang)) in langs.into_iter().enumerate() {
        let on_sel = std::rc::Rc::clone(&cb);
        let active = edit.language == lang;
        tabs = tabs.child(
            div()
                .id(("cosmic-edit-lang", idx))
                .cursor_pointer()
                .px_2()
                .py_1()
                .rounded(px(6.0))
                .bg(if active {
                    theme.colors.primary_container
                } else {
                    theme.colors.surface_container_high
                })
                .on_click(move |_ev, _w, cx| on_sel(lang, cx))
                .child(Label::new(label).size(LabelSize::XSmall)),
        );
    }

    let mut code_box = v_flex()
        .flex_1()
        .p_2()
        .gap_0p5()
        .rounded(px(8.0))
        .bg(theme.colors.surface_container_lowest)
        .border_1()
        .border_color(theme.colors.outline_variant);
    for (idx, line) in edit.source.lines().take(18).enumerate() {
        code_box =
            code_box.child(Label::new(format!("{:>2} | {line}", idx + 1)).size(LabelSize::XSmall));
    }

    let mut symbols_col = v_flex().w(px(210.0)).gap_1();
    symbols_col = symbols_col.child(section_heading(theme, "Tree-sitter Outline"));
    if let Some(rep) = &edit.last_report {
        symbols_col = symbols_col
            .child(KeyValueRow::new("Errors", rep.syntax_errors.to_string()))
            .child(KeyValueRow::new("Symbols", rep.symbols.len().to_string()));
        for sym in rep.symbols.iter().take(8) {
            symbols_col = symbols_col.child(KeyValueRow::new(
                sym.kind.clone(),
                format!("{} (L{})", sym.name, sym.start_line),
            ));
        }
    }

    v_flex()
        .size_full()
        .gap_2()
        .child(
            h_flex()
                .justify_between()
                .child(Label::new(edit.path.clone()).size(LabelSize::XSmall))
                .child(tabs),
        )
        .child(h_flex().flex_1().gap_2().child(code_box).child(symbols_col))
        .into_any_element()
}
