//! Production starter showcase demonstrating how to build a custom domain application
//! (Data/Query Studio, RE Workbench, Audio/CAD tool, or custom IDE) on top of
//! `gpui_ui`, `gpui_code`, and `gpui_app_shell`.
//!
//! Run with:
//! ```sh
//! cargo run -p gpui_app_shell --example studio_starter
//! ```

use std::sync::Arc;

use gpui_app_shell::gpui_ui::gpui;
use gpui_app_shell::gpui_ui::prelude::*;
use gpui_app_shell::workspace::dock::{DockPosition, Panel, PanelEvent};
use gpui_app_shell::workspace::{
    AppState, HideStatusItem, ItemHandle, StatusItemView, Workspace,
};
use gpui_app_shell::{AppModule, AppShellBuilder, create_code_editor};

actions!(
    studio_starter,
    [
        /// Toggles the custom Domain Studio dock panel.
        ToggleStudioPanel,
        /// Inserts a sample Rust snippet into a new center editor tab.
        OpenSampleScratchpad,
    ]
);

const SAMPLE_RUST_SNIPPET: &str = r#"// Welcome to GPUI Studio Starter!
// Powered by gpui_ui + gpui_code + gpui_app_shell.

pub struct PipelineStage {
    pub name: &'static str,
    pub latency_micros: u64,
}

pub fn summarize_stages(stages: &[PipelineStage]) -> u64 {
    stages.iter().map(|stage| stage.latency_micros).sum()
}
"#;

struct StudioInspectorPanel {
    focus_handle: FocusHandle,
    position: DockPosition,
    execution_count: usize,
}

impl StudioInspectorPanel {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            position: DockPosition::Right,
            execution_count: 0,
        }
    }
}

impl EventEmitter<PanelEvent> for StudioInspectorPanel {}

impl Focusable for StudioInspectorPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for StudioInspectorPanel {
    fn persistent_name() -> &'static str {
        "StudioInspectorPanel"
    }

    fn panel_key() -> &'static str {
        "studio_inspector_panel"
    }

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        self.position
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(
            position,
            DockPosition::Left | DockPosition::Right | DockPosition::Bottom
        )
    }

    fn set_position(
        &mut self,
        position: DockPosition,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.position = position;
        cx.notify();
    }

    fn default_size(&self, _window: &Window, _cx: &App) -> Pixels {
        px(320.0)
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<IconName> {
        Some(IconName::BoltFilled)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Studio Domain Inspector")
    }

    fn toggle_action(&self) -> Box<dyn Action> {
        ToggleStudioPanel.boxed_clone()
    }

    fn activation_priority(&self) -> u32 {
        10
    }
}

impl Render for StudioInspectorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .p_3()
            .gap_3()
            .bg(cx.theme().colors().panel_background)
            .child(
                h_flex()
                    .justify_between()
                    .child(Label::new("Domain Inspector").size(LabelSize::Large))
                    .child(
                        Label::new(format!("Runs: {}", self.execution_count))
                            .color(Color::Muted),
                    ),
            )
            .child(
                Label::new(
                    "Attach custom domain tools, live telemetry, SQL/AST inspectors, or hardware controls in any dock.",
                )
                .color(Color::Muted)
                .size(LabelSize::Small),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("run-pipeline", "Run Stage")
                            .style(ButtonStyle::Filled)
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.execution_count += 1;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("open-scratchpad", "New Rust Scratchpad")
                            .style(ButtonStyle::Subtle)
                            .on_click(|_event, window, cx| {
                                window.dispatch_action(OpenSampleScratchpad.boxed_clone(), cx);
                            }),
                    ),
            )
    }
}

struct StudioStatusBadge {
    active_title: Option<SharedString>,
}

impl StudioStatusBadge {
    fn new() -> Self {
        Self { active_title: None }
    }
}

impl StatusItemView for StudioStatusBadge {
    fn set_active_pane_item(
        &mut self,
        active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_title = active_pane_item.map(|item| item.tab_content_text(0, cx));
        cx.notify();
    }

    fn hide_setting(&self, _cx: &App) -> Option<HideStatusItem> {
        None
    }
}

impl Render for StudioStatusBadge {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let label = match &self.active_title {
            Some(title) => format!("Studio • {title}"),
            None => "Studio • Ready".to_owned(),
        };
        h_flex()
            .px_2()
            .child(Label::new(label).size(LabelSize::Small).color(Color::Accent))
    }
}

struct StudioDomainModule;

impl AppModule for StudioDomainModule {
    fn name(&self) -> &'static str {
        "studio-domain"
    }

    fn init(&self, _app_state: &Arc<AppState>, cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("ctrl-alt-s", ToggleStudioPanel, None),
            KeyBinding::new("ctrl-alt-n", OpenSampleScratchpad, None),
        ]);
    }

    fn configure_workspace(
        &self,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let panel = cx.new(StudioInspectorPanel::new);
        workspace.add_panel(panel, window, cx);

        let badge = cx.new(|_cx| StudioStatusBadge::new());
        workspace.status_bar().update(cx, |status_bar, cx| {
            status_bar.add_right_item(badge, window, cx);
        });

        workspace.register_action(|workspace, _: &ToggleStudioPanel, window, cx| {
            workspace.toggle_panel_focus::<StudioInspectorPanel>(window, cx);
        });

        let languages = workspace.app_state().languages.clone();
        workspace.register_action(move |workspace, _: &OpenSampleScratchpad, window, cx| {
            let editor = create_code_editor(
                SAMPLE_RUST_SNIPPET,
                Some("Rust"),
                &languages,
                window,
                cx,
            );
            workspace.add_item_to_active_pane(Box::new(editor), None, true, window, cx);
        });

        let initial_editor = create_code_editor(
            SAMPLE_RUST_SNIPPET,
            Some("Rust"),
            &workspace.app_state().languages.clone(),
            window,
            cx,
        );
        workspace.add_item_to_active_pane(Box::new(initial_editor), None, true, window, cx);
    }
}

fn main() {
    let custom_data_dir = std::env::temp_dir().join("gpui-studio-starter");
    AppShellBuilder::new("GPUI Studio Starter")
        .data_dir(custom_data_dir)
        .with_module(StudioDomainModule)
        .run();
}
