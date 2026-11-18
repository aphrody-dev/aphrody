//! Native GPUI mirrors of design.google library references (Transparent Screens, Gemini,
//! YouTube red, Googlebook, OpenBook Quire) and the Material 3 cross-framework component matrix
//! (Figma kit, Angular Material, Flutter, Jetpack Compose).
//!
//! Every component is value-like (`RenderOnce`): the owning view keeps the domain state and
//! receives requested changes through callbacks. Colors come from `ShadcnGpuiTheme` roles or
//! from `MirrorAccentPalette`, which resolves `ref.palette` and `comp.*` tokens.

use std::collections::BTreeSet;
use std::rc::Rc;

use gpui::{FontWeight, Hsla, KeyDownEvent, Role, linear_color_stop, linear_gradient};
use ui::prelude::*;

use crate::cosmic_theme::AppearanceMode;
use crate::google_design::MirrorAccentPalette;
use crate::starter_pack::{ShadcnGpuiTheme, StarterButton, StarterSize, StarterVariant};

type SeekHandler = Rc<dyn Fn(f32, &mut Window, &mut App) + 'static>;
type IdHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>;
type ActionHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// Formats a media position as `m:ss`, or `h:mm:ss` from one hour.
#[must_use]
pub fn format_media_timestamp(seconds: f32) -> String {
    let total = seconds.max(0.0).floor() as u64;
    let (hours, minutes, secs) = (total / 3600, (total % 3600) / 60, total % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

/// Target of a relative seek, clamped to the media range.
#[must_use]
pub fn seek_target_secs(position_secs: f32, duration_secs: f32, delta_secs: f32) -> f32 {
    (position_secs + delta_secs).clamp(0.0, duration_secs.max(0.0))
}

fn fraction_of(value: f32, total: f32) -> f32 {
    if total <= 0.0 {
        0.0
    } else {
        (value / total).clamp(0.0, 1.0)
    }
}

/// Glanceable card for additive transparent displays: the outer void renders as see-through,
/// content sits on the dark Glimmer container to stay legible over bright surroundings.
#[derive(IntoElement)]
pub struct TransparentHudCard {
    title: SharedString,
    value: SharedString,
    detail: SharedString,
    status: Option<SharedString>,
    accents: MirrorAccentPalette,
}

impl TransparentHudCard {
    #[must_use]
    pub fn new(
        title: impl Into<SharedString>,
        value: impl Into<SharedString>,
        accents: &MirrorAccentPalette,
    ) -> Self {
        Self {
            title: title.into(),
            value: value.into(),
            detail: SharedString::default(),
            status: None,
            accents: *accents,
        }
    }

    #[must_use]
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Short live status; rendered with a text label so it never relies on the dot color.
    #[must_use]
    pub fn status(mut self, status: impl Into<SharedString>) -> Self {
        self.status = Some(status.into());
        self
    }
}

impl RenderOnce for TransparentHudCard {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let accents = self.accents;
        div().p_2().bg(accents.glimmer_void).child(
            v_flex()
                .gap_1()
                .p_3()
                .rounded(rems_from_px(20.0_f32))
                .bg(accents.glimmer_surface)
                .child(
                    h_flex()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(accents.glimmer_gold)
                                .child(self.title),
                        )
                        .when_some(self.status, |row, status| {
                            row.child(
                                h_flex()
                                    .gap_1()
                                    .px_2()
                                    .rounded_full()
                                    .bg(accents.glimmer_container)
                                    .child(div().size_2().rounded_full().bg(accents.glimmer_lime))
                                    .child(div().text_xs().text_color(accents.glimmer_mint).child(status)),
                            )
                        }),
                )
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(accents.glimmer_gold)
                        .child(self.value),
                )
                .when(!self.detail.is_empty(), |card| {
                    card.child(div().text_sm().text_color(accents.glimmer_mint).child(self.detail))
                }),
        )
    }
}

/// Gemini answer card: the blue → violet → rose spectrum marks AI authorship; the thinking
/// state is spelled out in text so it reads without color or motion.
#[derive(IntoElement)]
pub struct GeminiSpectrumCard {
    title: SharedString,
    body: SharedString,
    thinking: bool,
    theme: ShadcnGpuiTheme,
    accents: MirrorAccentPalette,
}

impl GeminiSpectrumCard {
    #[must_use]
    pub fn new(
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
        theme: &ShadcnGpuiTheme,
        accents: &MirrorAccentPalette,
    ) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            thinking: false,
            theme: *theme,
            accents: *accents,
        }
    }

    #[must_use]
    pub fn thinking(mut self, thinking: bool) -> Self {
        self.thinking = thinking;
        self
    }
}

impl RenderOnce for GeminiSpectrumCard {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let [blue, violet, rose] = self.accents.gemini_spectrum;
        v_flex()
            .overflow_hidden()
            .rounded(rems_from_px(self.theme.radius_xl_px))
            .bg(self.theme.card)
            .border_1()
            .border_color(self.theme.border)
            .child(
                // GPUI gradients take two stops, so the three-stop spectrum is two adjacent segments.
                h_flex()
                    .h_1()
                    .w_full()
                    .child(div().flex_1().h_full().bg(linear_gradient(
                        90.0,
                        linear_color_stop(blue, 0.0),
                        linear_color_stop(violet, 1.0),
                    )))
                    .child(div().flex_1().h_full().bg(linear_gradient(
                        90.0,
                        linear_color_stop(violet, 0.0),
                        linear_color_stop(rose, 1.0),
                    ))),
            )
            .child(
                v_flex()
                    .gap_1p5()
                    .p_4()
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(self.theme.card_foreground)
                                    .child(self.title),
                            )
                            .when(self.thinking, |row| {
                                row.child(
                                    div()
                                        .text_xs()
                                        .text_color(self.theme.muted_foreground)
                                        .child("Thinking…"),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(self.theme.muted_foreground)
                            .child(self.body),
                    ),
            )
    }
}

/// Media scrubber with the YouTube red → magenta played range. The owner holds the position and
/// receives seek requests (arrow keys ±5 s, Home/End, or the ±10 s buttons) through `on_seek`.
#[derive(IntoElement)]
pub struct YoutubeGradientScrubber {
    id: ElementId,
    label: SharedString,
    position_secs: f32,
    buffered_secs: f32,
    duration_secs: f32,
    theme: ShadcnGpuiTheme,
    accents: MirrorAccentPalette,
    on_seek: Option<SeekHandler>,
}

impl YoutubeGradientScrubber {
    const KEY_STEP_SECS: f32 = 5.0;
    const BUTTON_STEP_SECS: f32 = 10.0;

    #[must_use]
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        duration_secs: f32,
        theme: &ShadcnGpuiTheme,
        accents: &MirrorAccentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            position_secs: 0.0,
            buffered_secs: 0.0,
            duration_secs: duration_secs.max(0.0),
            theme: *theme,
            accents: *accents,
            on_seek: None,
        }
    }

    #[must_use]
    pub fn position(mut self, position_secs: f32) -> Self {
        self.position_secs = position_secs.clamp(0.0, self.duration_secs);
        self
    }

    #[must_use]
    pub fn buffered(mut self, buffered_secs: f32) -> Self {
        self.buffered_secs = buffered_secs.clamp(0.0, self.duration_secs);
        self
    }

    /// Called with the requested position in seconds; the owner applies it and re-renders.
    #[must_use]
    pub fn on_seek(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for YoutubeGradientScrubber {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let position = self.position_secs;
        let duration = self.duration_secs;
        let played = fraction_of(position, duration);
        let buffered = fraction_of(self.buffered_secs.max(position), duration);
        let accents = self.accents;
        let theme = self.theme;
        let focus_ring = theme.ring;
        let timestamps = format!(
            "{} / {}",
            format_media_timestamp(position),
            format_media_timestamp(duration)
        );

        let mut track = div()
            .id(self.id.clone())
            .w_full()
            .py_1()
            .rounded_full()
            .border_1()
            .border_color(gpui::transparent_black())
            .focusable()
            .tab_stop(true)
            .role(Role::Slider)
            .aria_label(self.label.clone())
            .aria_value(timestamps.clone())
            .aria_numeric_value(f64::from(position))
            .aria_min_numeric_value(0.0)
            .aria_max_numeric_value(f64::from(duration))
            .aria_numeric_value_step(f64::from(Self::KEY_STEP_SECS))
            .focus_visible(move |style| style.border_color(focus_ring))
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(rems_from_px(accents.youtube_track_height_px))
                    .rounded_full()
                    .bg(theme.muted)
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .h_full()
                            .w(relative(buffered))
                            .rounded_full()
                            .bg(theme.muted_foreground.opacity(0.4)),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .h_full()
                            .w(relative(played))
                            .rounded_full()
                            .bg(linear_gradient(
                                accents.youtube_gradient_angle_deg,
                                linear_color_stop(accents.youtube_red, 0.0),
                                linear_color_stop(accents.youtube_magenta, 1.0),
                            )),
                    ),
            );

        if let Some(on_seek) = self.on_seek.clone() {
            track = track.on_key_down(move |event: &KeyDownEvent, window, cx| {
                let target = match event.keystroke.key.as_str() {
                    "left" => seek_target_secs(position, duration, -Self::KEY_STEP_SECS),
                    "right" => seek_target_secs(position, duration, Self::KEY_STEP_SECS),
                    "home" => 0.0,
                    "end" => duration,
                    _ => return,
                };
                cx.stop_propagation();
                on_seek(target, window, cx);
            });
        }

        let seek_button = |suffix: &str, label: &'static str, delta: f32| {
            let button = StarterButton::new(
                SharedString::from(format!("{}-{suffix}", self.id)),
                label,
                &theme,
            )
            .variant(StarterVariant::Ghost)
            .size(StarterSize::Sm)
            .disabled(self.on_seek.is_none());
            match self.on_seek.clone() {
                Some(on_seek) => button.on_click(move |_, window, cx| {
                    on_seek(seek_target_secs(position, duration, delta), window, cx)
                }),
                None => button,
            }
        };

        v_flex()
            .gap_1()
            .child(track)
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(timestamps),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(seek_button("back", "Back 10 s", -Self::BUTTON_STEP_SECS))
                            .child(seek_button(
                                "forward",
                                "Forward 10 s",
                                Self::BUTTON_STEP_SECS,
                            )),
                    ),
            )
    }
}

/// Launcher entry for `GlowbarShellCard`, keyed by a stable domain id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlowbarEntry {
    id: SharedString,
    label: SharedString,
}

impl GlowbarEntry {
    #[must_use]
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }

    #[must_use]
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    #[must_use]
    pub fn label(&self) -> &SharedString {
        &self.label
    }
}

/// Googlebook shell: the four-color glow bar above a launcher row with one selected entry.
#[derive(IntoElement)]
pub struct GlowbarShellCard {
    title: SharedString,
    entries: Vec<GlowbarEntry>,
    selected_id: Option<SharedString>,
    theme: ShadcnGpuiTheme,
    accents: MirrorAccentPalette,
    on_select: Option<IdHandler>,
}

impl GlowbarShellCard {
    #[must_use]
    pub fn new(
        title: impl Into<SharedString>,
        entries: Vec<GlowbarEntry>,
        theme: &ShadcnGpuiTheme,
        accents: &MirrorAccentPalette,
    ) -> Self {
        Self {
            title: title.into(),
            entries,
            selected_id: None,
            theme: *theme,
            accents: *accents,
            on_select: None,
        }
    }

    #[must_use]
    pub fn selected(mut self, id: impl Into<SharedString>) -> Self {
        self.selected_id = Some(id.into());
        self
    }

    #[must_use]
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for GlowbarShellCard {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let theme = self.theme;
        let glow = self.accents.googlebook_glow;
        let selected_id = self.selected_id;
        let on_select = self.on_select;

        v_flex()
            .overflow_hidden()
            .rounded(rems_from_px(theme.radius_xl_px))
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .h_1()
                    .w_full()
                    .children(glow.into_iter().map(|color| div().flex_1().h_full().bg(color))),
            )
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.card_foreground)
                            .child(self.title),
                    )
                    .child(h_flex().flex_wrap().gap_1().children(self.entries.into_iter().map(
                        |entry| {
                            let is_selected = selected_id.as_ref() == Some(&entry.id);
                            let button = StarterButton::new(
                                SharedString::from(format!("glowbar-entry-{}", entry.id)),
                                entry.label.clone(),
                                &theme,
                            )
                            .size(StarterSize::Sm)
                            .variant(if is_selected {
                                StarterVariant::Secondary
                            } else {
                                StarterVariant::Ghost
                            });
                            match on_select.clone() {
                                Some(on_select) => {
                                    let entry_id = entry.id.clone();
                                    button.on_click(move |_, window, cx| {
                                        on_select(&entry_id, window, cx)
                                    })
                                }
                                None => button,
                            }
                        },
                    ))),
            )
    }
}

/// One agent step in a Quire note; each step can be undone on its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuireStep {
    id: SharedString,
    summary: SharedString,
    undone: bool,
}

impl QuireStep {
    #[must_use]
    pub fn new(id: impl Into<SharedString>, summary: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            summary: summary.into(),
            undone: false,
        }
    }

    #[must_use]
    pub fn undone(mut self, undone: bool) -> Self {
        self.undone = undone;
        self
    }

    #[must_use]
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    #[must_use]
    pub fn is_undone(&self) -> bool {
        self.undone
    }
}

/// OpenBook Quire glass note: agent steps with per-step Undo, a consent card that appears only
/// for destructive operations, and a quiet line naming the agent and intake source.
///
/// GPUI has no backdrop blur, so glass is the folio color at the Quire tint opacity; economy
/// mode switches to the near-opaque paper fallback defined by the spec.
#[derive(IntoElement)]
pub struct QuireGlassNote {
    id: ElementId,
    agent_name: SharedString,
    intake_source: SharedString,
    steps: Vec<QuireStep>,
    pending_deletion: Option<SharedString>,
    economy: bool,
    theme: ShadcnGpuiTheme,
    accents: MirrorAccentPalette,
    on_undo: Option<IdHandler>,
    on_keep: Option<ActionHandler>,
    on_delete: Option<ActionHandler>,
    on_dismiss: Option<ActionHandler>,
}

impl QuireGlassNote {
    #[must_use]
    pub fn new(
        id: impl Into<ElementId>,
        agent_name: impl Into<SharedString>,
        intake_source: impl Into<SharedString>,
        theme: &ShadcnGpuiTheme,
        accents: &MirrorAccentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            agent_name: agent_name.into(),
            intake_source: intake_source.into(),
            steps: Vec::new(),
            pending_deletion: None,
            economy: false,
            theme: *theme,
            accents: *accents,
            on_undo: None,
            on_keep: None,
            on_delete: None,
            on_dismiss: None,
        }
    }

    #[must_use]
    pub fn steps(mut self, steps: Vec<QuireStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Names the object a pending destructive operation would delete; shows the consent card.
    #[must_use]
    pub fn pending_deletion(mut self, object_name: impl Into<SharedString>) -> Self {
        self.pending_deletion = Some(object_name.into());
        self
    }

    #[must_use]
    pub fn economy(mut self, economy: bool) -> Self {
        self.economy = economy;
        self
    }

    #[must_use]
    pub fn on_undo(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_undo = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn on_keep(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_keep = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn on_delete(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_delete = Some(Rc::new(handler));
        self
    }

    /// Escape while focus is inside the note stops the agent; the quiet line advertises Esc
    /// only when this handler is set.
    #[must_use]
    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }

    fn folio(&self) -> Hsla {
        let paper = match self.theme.mode {
            AppearanceMode::Light => self.accents.quire_folio_light,
            AppearanceMode::Dark => self.accents.quire_folio_dark,
        };
        let opacity = if self.economy {
            self.accents.quire_economy_paper_opacity
        } else {
            self.accents.quire_tint_opacity
        };
        paper.opacity(opacity)
    }

    fn quiet_line(&self) -> String {
        let mut parts = vec![self.agent_name.to_string(), format!("from {}", self.intake_source)];
        if self.on_dismiss.is_some() {
            parts.push("Esc to stop".to_owned());
        }
        parts.join(" · ")
    }
}

impl RenderOnce for QuireGlassNote {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let theme = self.theme;
        let folio = self.folio();
        let quiet_line = self.quiet_line();
        let rim = self.accents.quire_rim;
        let rim_height = rems_from_px(self.accents.quire_rim_width_px);
        let radius = rems_from_px(self.accents.quire_card_radius_px);
        let id_prefix = self.id.to_string();
        let on_undo = self.on_undo;

        let step_rows = self.steps.into_iter().map(|step| {
            let summary_color = if step.undone {
                theme.muted_foreground
            } else {
                theme.foreground
            };
            let button = StarterButton::new(
                SharedString::from(format!("{id_prefix}-undo-{}", step.id)),
                if step.undone { "Undone" } else { "Undo" },
                &theme,
            )
            .variant(StarterVariant::Ghost)
            .size(StarterSize::Sm)
            .disabled(step.undone || on_undo.is_none());
            let button = match on_undo.clone() {
                Some(on_undo) if !step.undone => {
                    let step_id = step.id.clone();
                    button.on_click(move |_, window, cx| on_undo(&step_id, window, cx))
                }
                _ => button,
            };
            h_flex()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .text_color(summary_color)
                        .when(step.undone, |text| text.line_through())
                        .child(step.summary),
                )
                .child(button)
        });

        let consent = self.pending_deletion.map(|object_name| {
            let keep = StarterButton::new(
                SharedString::from(format!("{id_prefix}-keep")),
                "Keep it",
                &theme,
            );
            let keep = match self.on_keep.clone() {
                Some(on_keep) => keep.on_click(move |_, window, cx| on_keep(window, cx)),
                None => keep.disabled(true),
            };
            let delete = StarterButton::new(
                SharedString::from(format!("{id_prefix}-delete")),
                "Delete",
                &theme,
            )
            .variant(StarterVariant::Destructive);
            let delete = match self.on_delete.clone() {
                Some(on_delete) => delete.on_click(move |_, window, cx| on_delete(window, cx)),
                None => delete.disabled(true),
            };
            v_flex()
                .gap_2()
                .p_3()
                .rounded(rems_from_px(theme.radius_lg_px))
                .bg(theme.muted)
                .border_1()
                .border_color(theme.destructive)
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.foreground)
                        .child(format!("Delete \u{201c}{object_name}\u{201d}?")),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("This step cannot be undone."),
                )
                .child(h_flex().justify_end().gap_2().child(keep).child(delete))
        });

        let mut note = v_flex()
            .id(self.id)
            .overflow_hidden()
            .rounded(radius)
            .bg(folio)
            .border_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .h(rim_height)
                    .w_full()
                    .children(rim.into_iter().map(|color| div().flex_1().h_full().bg(color))),
            )
            .child(
                v_flex()
                    .gap_2()
                    .p_4()
                    .children(step_rows)
                    .children(consent)
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(quiet_line),
                    ),
            );

        if let Some(on_dismiss) = self.on_dismiss {
            note = note.on_key_down(move |event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    on_dismiss(window, cx);
                }
            });
        }

        note
    }
}

/// One Material 3 category mapped across design and implementation frameworks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MaterialComponentMapping {
    pub category: &'static str,
    pub figma_kit: &'static str,
    pub angular: &'static str,
    pub flutter: &'static str,
    pub compose: &'static str,
    pub gpui: &'static str,
}

/// Rows follow Flutter's Material categories; the Figma column names component pages of the
/// Material 3 Design Kit (file `vEr6T5SQaJfMpLxjMUfD55`).
pub const MATERIAL_COMPONENT_MAPPINGS: [MaterialComponentMapping; 6] = [
    MaterialComponentMapping {
        category: "Actions",
        figma_kit: "Buttons, FAB, Icon buttons, Segmented buttons",
        angular: "MatButton, MatButtonToggle",
        flutter: "FilledButton, FloatingActionButton, IconButton, SegmentedButton",
        compose: "Button, FloatingActionButton, IconButton, SegmentedButton",
        gpui: "StarterButton, FocusableActionChip, SegmentedStudioBar",
    },
    MaterialComponentMapping {
        category: "Communication",
        figma_kit: "Badges, Progress indicators, Snackbar",
        angular: "MatBadge, MatProgressBar, MatProgressSpinner, MatSnackBar",
        flutter: "Badge, LinearProgressIndicator, CircularProgressIndicator, SnackBar",
        compose: "Badge, LinearProgressIndicator, CircularProgressIndicator, Snackbar",
        gpui: "StarterBadge, ThinkingAuraBadge, ProgressMeter, StarterAlert, GeminiSpectrumCard",
    },
    MaterialComponentMapping {
        category: "Containment",
        figma_kit: "Bottom sheets, Cards, Dialogs, Dividers, Lists",
        angular: "MatCard, MatDialog, MatBottomSheet, MatList, MatDivider",
        flutter: "Card, AlertDialog, BottomSheet, ListTile, Divider",
        compose: "Card, AlertDialog, ModalBottomSheet, ListItem, HorizontalDivider",
        gpui: "StarterCard, CascadingPopoverCard, QuireGlassNote, TransparentHudCard",
    },
    MaterialComponentMapping {
        category: "Navigation",
        figma_kit: "Navigation bar, Navigation rail, Navigation drawer, Tabs, Top app bar",
        angular: "MatToolbar, MatSidenav, MatTabGroup, MatMenu",
        flutter: "NavigationBar, NavigationRail, NavigationDrawer, TabBar, AppBar",
        compose: "NavigationBar, NavigationRail, ModalNavigationDrawer, TabRow, TopAppBar",
        gpui: "MobileBottomNavBar, GlowbarShellCard, SegmentedStudioBar",
    },
    MaterialComponentMapping {
        category: "Selection",
        figma_kit: "Checkbox, Chips, Radio button, Slider, Switch, Date pickers",
        angular: "MatCheckbox, MatChipSet, MatRadioGroup, MatSlider, MatSlideToggle, MatDatepicker",
        flutter: "Checkbox, Chip, Radio, Slider, Switch, DatePickerDialog",
        compose: "Checkbox, FilterChip, RadioButton, Slider, Switch, DatePicker",
        gpui: "FocusableActionChip, SliderControl, ToggleControl, YoutubeGradientScrubber",
    },
    MaterialComponentMapping {
        category: "Text inputs",
        figma_kit: "Text fields, Search",
        angular: "MatInput, MatFormField, MatAutocomplete",
        flutter: "TextField, SearchBar, SearchAnchor",
        compose: "TextField, OutlinedTextField, SearchBar",
        gpui: "ExpressiveOmnibar, editor::Editor (single line)",
    },
];

/// Read-only matrix comparing Material 3 implementations; six rows, so no virtualization.
#[derive(IntoElement)]
pub struct MaterialComponentMatrix {
    theme: ShadcnGpuiTheme,
}

impl MaterialComponentMatrix {
    const COLUMNS: [&'static str; 6] =
        ["Category", "Figma kit", "Angular", "Flutter", "Compose", "GPUI"];

    #[must_use]
    pub fn new(theme: &ShadcnGpuiTheme) -> Self {
        Self { theme: *theme }
    }
}

impl RenderOnce for MaterialComponentMatrix {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let theme = self.theme;
        let header = Self::COLUMNS.into_iter().map(move |column| {
            div()
                .px_2()
                .py_1p5()
                .bg(theme.muted)
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(column)
        });
        let cells = MATERIAL_COMPONENT_MAPPINGS.into_iter().flat_map(move |mapping| {
            [
                (mapping.category, true),
                (mapping.figma_kit, false),
                (mapping.angular, false),
                (mapping.flutter, false),
                (mapping.compose, false),
                (mapping.gpui, false),
            ]
            .into_iter()
            .map(move |(text, is_category)| {
                div()
                    .px_2()
                    .py_1p5()
                    .border_t_1()
                    .border_color(theme.border)
                    .text_xs()
                    .text_color(if is_category {
                        theme.foreground
                    } else {
                        theme.muted_foreground
                    })
                    .when(is_category, |cell| cell.font_weight(FontWeight::MEDIUM))
                    .child(text)
            })
        });

        div()
            .grid()
            .grid_cols(Self::COLUMNS.len() as u16)
            .overflow_hidden()
            .rounded(rems_from_px(theme.radius_lg_px))
            .border_1()
            .border_color(theme.border)
            .bg(theme.card)
            .children(header)
            .children(cells)
    }
}

/// A change requested by the reference gallery; the owner applies it with
/// [`ReferenceGalleryState::apply`] and re-renders when it returns `true`.
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceGalleryChange {
    Seek(f32),
    UndoStep(SharedString),
    SelectShellEntry(SharedString),
    KeepPendingObject,
    DeletePendingObject,
    StopAgent,
}

/// Owner-held state of the reference gallery (sample content for each mirrored reference).
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceGalleryState {
    position_secs: f32,
    undone_step_ids: BTreeSet<SharedString>,
    selected_shell_entry: SharedString,
    pending_deletion: Option<SharedString>,
    status: SharedString,
}

impl Default for ReferenceGalleryState {
    fn default() -> Self {
        Self {
            position_secs: 74.0,
            undone_step_ids: BTreeSet::new(),
            selected_shell_entry: SharedString::from("gemini"),
            pending_deletion: Some(SharedString::from("Old drafts")),
            status: SharedString::default(),
        }
    }
}

impl ReferenceGalleryState {
    pub const MEDIA_DURATION_SECS: f32 = 212.0;
    const BUFFER_AHEAD_SECS: f32 = 40.0;
    const QUIRE_STEPS: [(&'static str, &'static str); 3] = [
        ("archive-newsletters", "Archived 42 newsletters"),
        ("label-receipts", "Labeled 7 receipts as Finance"),
        ("draft-reply", "Drafted a reply to the venue"),
    ];
    const SHELL_ENTRIES: [(&'static str, &'static str); 4] = [
        ("gemini", "Gemini"),
        ("files", "Files"),
        ("studio", "Studio"),
        ("settings", "Settings"),
    ];

    /// Applies a requested change; returns whether anything visible changed.
    pub fn apply(&mut self, change: &ReferenceGalleryChange) -> bool {
        match change {
            ReferenceGalleryChange::Seek(target) => {
                let target = target.clamp(0.0, Self::MEDIA_DURATION_SECS);
                let changed = (self.position_secs - target).abs() > f32::EPSILON;
                self.position_secs = target;
                changed
            }
            ReferenceGalleryChange::UndoStep(step_id) => {
                Self::QUIRE_STEPS.iter().any(|(id, _)| *id == step_id.as_ref())
                    && self.undone_step_ids.insert(step_id.clone())
            }
            ReferenceGalleryChange::SelectShellEntry(entry_id) => {
                let known = Self::SHELL_ENTRIES.iter().any(|(id, _)| *id == entry_id.as_ref());
                let changed = known && self.selected_shell_entry != *entry_id;
                if changed {
                    self.selected_shell_entry = entry_id.clone();
                }
                changed
            }
            ReferenceGalleryChange::KeepPendingObject => self.resolve_pending("Kept"),
            ReferenceGalleryChange::DeletePendingObject => self.resolve_pending("Deleted"),
            ReferenceGalleryChange::StopAgent => {
                self.status = SharedString::from("Agent stopped");
                true
            }
        }
    }

    fn resolve_pending(&mut self, verb: &str) -> bool {
        match self.pending_deletion.take() {
            Some(object_name) => {
                self.status = SharedString::from(format!("{verb} \u{201c}{object_name}\u{201d}"));
                true
            }
            None => false,
        }
    }

    #[must_use]
    pub fn position_secs(&self) -> f32 {
        self.position_secs
    }

    #[must_use]
    pub fn is_step_undone(&self, step_id: &str) -> bool {
        self.undone_step_ids.contains(step_id)
    }

    #[must_use]
    pub fn selected_shell_entry(&self) -> &SharedString {
        &self.selected_shell_entry
    }

    #[must_use]
    pub fn pending_deletion(&self) -> Option<&SharedString> {
        self.pending_deletion.as_ref()
    }

    /// Outcome of the last resolved consent or stop request; empty until one happens.
    #[must_use]
    pub fn status(&self) -> &SharedString {
        &self.status
    }
}

type GalleryChangeHandler = Rc<dyn Fn(&ReferenceGalleryChange, &mut Window, &mut App) + 'static>;

/// Composition of every mirrored reference with sample content, shared by the Zed workspace
/// studio and the web desktop. Value-like: it renders the owner's state and reports changes.
#[derive(IntoElement)]
pub struct ReferenceGallery {
    id: ElementId,
    state: ReferenceGalleryState,
    thinking: bool,
    theme: ShadcnGpuiTheme,
    accents: MirrorAccentPalette,
    on_change: Option<GalleryChangeHandler>,
}

impl ReferenceGallery {
    #[must_use]
    pub fn new(
        id: impl Into<ElementId>,
        state: &ReferenceGalleryState,
        theme: &ShadcnGpuiTheme,
        accents: &MirrorAccentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            thinking: false,
            theme: *theme,
            accents: *accents,
            on_change: None,
        }
    }

    /// Marks the Gemini card as working, e.g. while the owner refreshes the live mirror.
    #[must_use]
    pub fn thinking(mut self, thinking: bool) -> Self {
        self.thinking = thinking;
        self
    }

    #[must_use]
    pub fn on_change(
        mut self,
        handler: impl Fn(&ReferenceGalleryChange, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ReferenceGallery {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let theme = self.theme;
        let accents = self.accents;
        let state = self.state;
        let id_prefix = self.id.to_string();
        let on_change = self.on_change;
        let emit = |change: ReferenceGalleryChange| {
            on_change.clone().map(|on_change| {
                move |window: &mut Window, cx: &mut App| on_change(&change, window, cx)
            })
        };

        let steps = ReferenceGalleryState::QUIRE_STEPS
            .into_iter()
            .map(|(id, summary)| QuireStep::new(id, summary).undone(state.is_step_undone(id)))
            .collect();
        let shell_entries = ReferenceGalleryState::SHELL_ENTRIES
            .into_iter()
            .map(|(id, label)| GlowbarEntry::new(id, label))
            .collect();

        let mut quire_note = QuireGlassNote::new(
            SharedString::from(format!("{id_prefix}-quire-note")),
            "Gemini",
            "Inbox",
            &theme,
            &accents,
        )
        .steps(steps);
        if let Some(object_name) = state.pending_deletion() {
            quire_note = quire_note.pending_deletion(object_name.clone());
        }
        if let Some(keep) = emit(ReferenceGalleryChange::KeepPendingObject) {
            quire_note = quire_note.on_keep(keep);
        }
        if let Some(delete) = emit(ReferenceGalleryChange::DeletePendingObject) {
            quire_note = quire_note.on_delete(delete);
        }
        if let Some(stop) = emit(ReferenceGalleryChange::StopAgent) {
            quire_note = quire_note.on_dismiss(stop);
        }

        let mut scrubber = YoutubeGradientScrubber::new(
            SharedString::from(format!("{id_prefix}-scrubber")),
            "Playback position",
            ReferenceGalleryState::MEDIA_DURATION_SECS,
            &theme,
            &accents,
        )
        .position(state.position_secs())
        .buffered(state.position_secs() + ReferenceGalleryState::BUFFER_AHEAD_SECS);

        let mut shell = GlowbarShellCard::new("Googlebook shell", shell_entries, &theme, &accents)
            .selected(state.selected_shell_entry().clone());

        if let Some(on_change) = on_change.clone() {
            quire_note = quire_note.on_undo({
                let on_change = on_change.clone();
                move |step_id, window, cx| {
                    on_change(&ReferenceGalleryChange::UndoStep(step_id.clone()), window, cx)
                }
            });
            scrubber = scrubber.on_seek({
                let on_change = on_change.clone();
                move |target, window, cx| {
                    on_change(&ReferenceGalleryChange::Seek(target), window, cx)
                }
            });
            shell = shell.on_select(move |entry_id, window, cx| {
                on_change(
                    &ReferenceGalleryChange::SelectShellEntry(entry_id.clone()),
                    window,
                    cx,
                )
            });
        }

        let status = state.status().clone();
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_3()
                    .items_start()
                    .child(
                        div().flex_1().min_w_0().child(
                            TransparentHudCard::new("Next turn", "120 m", &accents)
                                .detail("Rue de Rivoli, then left")
                                .status("Live"),
                        ),
                    )
                    .child(
                        div().flex_1().min_w_0().child(
                            GeminiSpectrumCard::new(
                                "Gemini",
                                "Summarized the design.google references into tokens.",
                                &theme,
                                &accents,
                            )
                            .thinking(self.thinking),
                        ),
                    ),
            )
            .child(scrubber)
            .child(
                h_flex()
                    .gap_3()
                    .items_start()
                    .child(div().flex_1().min_w_0().child(shell))
                    .child(div().flex_1().min_w_0().child(quire_note)),
            )
            .when(!status.is_empty(), |gallery| {
                gallery.child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(status),
                )
            })
            .child(MaterialComponentMatrix::new(&theme))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::cosmic_theme::contrast_ratio;

    #[test]
    fn reference_gallery_state_applies_each_change_once() {
        let mut state = ReferenceGalleryState::default();
        assert!(state.apply(&ReferenceGalleryChange::Seek(500.0)));
        assert_eq!(state.position_secs(), ReferenceGalleryState::MEDIA_DURATION_SECS);
        assert!(!state.apply(&ReferenceGalleryChange::Seek(500.0)));

        let undo = ReferenceGalleryChange::UndoStep(SharedString::from("label-receipts"));
        assert!(state.apply(&undo));
        assert!(!state.apply(&undo));
        assert!(state.is_step_undone("label-receipts"));
        assert!(!state.apply(&ReferenceGalleryChange::UndoStep(SharedString::from("unknown"))));

        assert!(!state.apply(&ReferenceGalleryChange::SelectShellEntry(SharedString::from("gemini"))));
        assert!(state.apply(&ReferenceGalleryChange::SelectShellEntry(SharedString::from("files"))));
        assert_eq!(state.selected_shell_entry().as_ref(), "files");

        assert!(state.status().is_empty());
        assert!(state.apply(&ReferenceGalleryChange::DeletePendingObject));
        assert!(state.pending_deletion().is_none());
        assert_eq!(state.status().as_ref(), "Deleted \u{201c}Old drafts\u{201d}");
        assert!(!state.apply(&ReferenceGalleryChange::KeepPendingObject));
    }

    #[test]
    fn media_timestamps_and_seek_targets_are_clamped() {
        assert_eq!(format_media_timestamp(0.0), "0:00");
        assert_eq!(format_media_timestamp(65.9), "1:05");
        assert_eq!(format_media_timestamp(3725.0), "1:02:05");
        assert_eq!(format_media_timestamp(-4.0), "0:00");
        assert_eq!(seek_target_secs(3.0, 100.0, -10.0), 0.0);
        assert_eq!(seek_target_secs(95.0, 100.0, 10.0), 100.0);
        assert_eq!(seek_target_secs(40.0, 100.0, 5.0), 45.0);
        assert_eq!(fraction_of(5.0, 0.0), 0.0);
        assert_eq!(fraction_of(50.0, 100.0), 0.5);
    }

    #[test]
    fn quire_note_folio_and_quiet_line_follow_spec() -> anyhow::Result<()> {
        let accents = MirrorAccentPalette::bundled()?;
        for mode in [AppearanceMode::Light, AppearanceMode::Dark] {
            let theme = ShadcnGpuiTheme::from_bundled(mode)?;
            let glass = QuireGlassNote::new("note", "Gemini", "Inbox", &theme, &accents)
                .steps(vec![
                    QuireStep::new("archive-42", "Archived 42 newsletters"),
                    QuireStep::new("label-7", "Labeled 7 receipts").undone(true),
                ]);
            assert_eq!(glass.folio().a, accents.quire_tint_opacity);
            assert_eq!(glass.quiet_line(), "Gemini · from Inbox");
            assert!(glass.steps[1].is_undone());

            let paper = glass.economy(true).on_dismiss(|_, _| {});
            assert_eq!(paper.folio().a, accents.quire_economy_paper_opacity);
            assert_eq!(paper.quiet_line(), "Gemini · from Inbox · Esc to stop");

            let opaque_folio = paper.folio().opacity(1.0 / paper.folio().a);
            let ratio = contrast_ratio(theme.foreground, opaque_folio);
            assert!(ratio >= 4.5, "{mode:?} quire text contrast {ratio:.2} < 4.5");
        }
        Ok(())
    }

    #[test]
    fn material_matrix_covers_each_category_once() {
        let categories: BTreeSet<_> = MATERIAL_COMPONENT_MAPPINGS
            .iter()
            .map(|mapping| mapping.category)
            .collect();
        assert_eq!(categories.len(), MATERIAL_COMPONENT_MAPPINGS.len());
        for mapping in MATERIAL_COMPONENT_MAPPINGS {
            for column in [
                mapping.figma_kit,
                mapping.angular,
                mapping.flutter,
                mapping.compose,
                mapping.gpui,
            ] {
                assert!(!column.trim().is_empty(), "{} has an empty column", mapping.category);
            }
        }
    }

    #[test]
    fn glowbar_entries_keep_domain_ids() {
        let entries = vec![
            GlowbarEntry::new("gemini", "Gemini"),
            GlowbarEntry::new("files", "Files"),
        ];
        assert_eq!(entries[0].id().as_ref(), "gemini");
        assert_eq!(entries[1].label().as_ref(), "Files");
    }
}
