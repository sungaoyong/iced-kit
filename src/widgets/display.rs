//! Presentational components: cards, dividers, badges, progress and alerts.

use crate::theme::{catalog, Size, Theme};
use iced::widget::{container, rule, text};
use iced::{Color, Element, Length, Padding};

/// The emphasis of a [`badge`] or [`alert`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    /// A neutral, informational tone.
    #[default]
    Neutral,
    /// A positive tone.
    Success,
    /// A cautionary tone.
    Warning,
    /// A destructive or erroneous tone.
    Danger,
    /// The brand tone.
    Primary,
}

impl Tone {
    /// The accent color this tone uses for text and icons.
    pub(crate) fn accent(self, theme: &Theme) -> Color {
        let colors = theme.colors();

        match self {
            Self::Neutral => colors.muted_foreground,
            Self::Success => Color::from_rgb8(0x16, 0xa3, 0x4a),
            Self::Warning => Color::from_rgb8(0xd9, 0x77, 0x06),
            Self::Danger => colors.destructive,
            Self::Primary => colors.primary,
        }
    }

    /// The tinted background this tone uses on a filled surface.
    ///
    /// The tint is derived from [`Tone::accent`] so that a custom palette's
    /// colors flow through instead of hard-coded pastels.
    pub(crate) fn surface(self, theme: &Theme) -> Color {
        let colors = theme.colors();
        let accent = self.accent(theme);
        let base = if theme.is_dark() {
            colors.background
        } else {
            colors.surface
        };

        // Blend the accent into the surface at low alpha. `Neutral` is already
        // a surface color, so it needs its own treatment.
        let tint_alpha = if self == Self::Neutral { 0.0 } else { 0.12 };

        Color {
            r: accent.r * tint_alpha + base.r * (1.0 - tint_alpha),
            g: accent.g * tint_alpha + base.g * (1.0 - tint_alpha),
            b: accent.b * tint_alpha + base.b * (1.0 - tint_alpha),
            a: 1.0,
        }
    }

    /// The border color this tone uses.
    ///
    /// A neutral tone draws no border, so it returns the accent at zero alpha
    /// and callers blend it themselves.
    pub(crate) fn border(self, theme: &Theme) -> Color {
        let alpha = if self == Self::Neutral { 0.0 } else { 0.4 };

        Color {
            a: alpha,
            ..self.accent(theme)
        }
    }
}

/// Builds a card: a raised surface with a border and rounded corners.
///
/// ```
/// # use iced_kit::widgets::{card, label};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// card(label("Account")).into()
/// # }
/// ```
pub fn card<'a, Message: 'a>(
    content: impl Into<Element<'a, Message, Theme>>,
) -> container::Container<'a, Message, Theme> {
    container(content)
        .padding(16)
        .class(Box::new(catalog::card) as container::StyleFn<'a, Theme>)
}

/// Builds a horizontal divider.
///
/// A divider carries no message type, so it can be dropped into any view.
pub fn divider<'a>() -> rule::Rule<'a, Theme> {
    rule::horizontal(1)
}

/// Builds a vertical divider.
pub fn vertical_divider<'a>() -> rule::Rule<'a, Theme> {
    rule::vertical(1)
}

/// Builds a small, pill-shaped status label.
///
/// ```
/// # use iced_kit::widgets::{badge, Tone};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// badge("Active", Tone::Success).into()
/// # }
/// ```
pub fn badge<'a, Message: 'a>(
    label: impl text::IntoFragment<'a>,
    tone: Tone,
) -> container::Container<'a, Message, Theme> {
    let text_size = Size::Sm.text();

    container(
        text(label)
            .size(text_size.size)
            .line_height(text_size.line_height()),
    )
    .padding(Padding {
        top: 2.0,
        right: 8.0,
        bottom: 2.0,
        left: 8.0,
    })
    .class(Box::new(move |theme: &Theme| {
        let accent = tone.accent(theme);

        container::Style {
            background: Some(iced::Background::Color(tone.surface(theme))),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: f32::from(theme.radius().full.min(999)).into(),
            },
            text_color: Some(accent),
            ..container::Style::default()
        }
    }) as container::StyleFn<'a, Theme>)
}

/// Builds a labelled progress bar.
///
/// The value is clamped to the `0.0..=1.0` range; passing a percentage is a
/// common mistake that would otherwise silently overflow the track.
///
/// The bar eases to a new value rather than jumping to it. The value is driven
/// by a spring, so it carries its momentum across a change: a bar that is
/// retargeted mid-travel — a download whose total is revised, a step that
/// completes faster than the last — turns around from where it is instead of
/// restarting from a standstill.
pub fn progress<'a, Message: 'a>(value: f32, tone: Tone) -> Element<'a, Message, Theme> {
    Progress::new(value.clamp(0.0, 1.0), tone).into()
}

/// A progress bar that eases towards its value.
struct Progress {
    value: f32,
    tone: Tone,
}

impl Progress {
    /// Creates a bar showing `value`, which is already clamped.
    const fn new(value: f32, tone: Tone) -> Self {
        Self { value, tone }
    }
}

/// The height of a progress bar, in logical pixels.
const BAR_HEIGHT: f32 = 8.0;

/// The spring a progress bar eases towards its value with.
///
/// Critically damped, so the bar never overshoots the value it reports — a bar
/// that passed 100% and came back would be claiming work that had not happened.
/// The tolerance is coarser than the default because the value is measured in
/// pixels: a hundredth of a pixel is not worth a frame.
const PROGRESS_SPRING: crate::motion::Spring =
    crate::motion::Spring::new(std::time::Duration::from_millis(200)).with_epsilon(0.01);

/// The state a [`Progress`] keeps between frames.
#[derive(Debug, Clone, Copy)]
struct ProgressState {
    spring: crate::motion::SpringState,
    /// The frame the bar was last advanced to.
    last: Option<iced::time::Instant>,
    /// Whether the bar has ever been laid out, so the first frame places it
    /// rather than easing in from zero.
    primed: bool,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self {
            spring: crate::motion::SpringState::new(0.0),
            last: None,
            primed: false,
        }
    }
}

impl<Message, Renderer> iced::advanced::Widget<Message, Theme, Renderer> for Progress
where
    Renderer: iced::advanced::Renderer,
{
    fn size(&self) -> iced::Size<Length> {
        iced::Size::new(Length::Fill, Length::Fixed(BAR_HEIGHT))
    }

    fn layout(
        &mut self,
        _tree: &mut iced::advanced::widget::tree::Tree,
        _renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        iced::advanced::layout::atomic(limits, Length::Fill, Length::Fixed(BAR_HEIGHT))
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::of::<ProgressState>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(ProgressState::default())
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::tree::Tree,
        event: &iced::Event,
        _layout: iced::advanced::layout::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &iced::Rectangle,
    ) {
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<ProgressState>();

            // The first frame places the bar at its value instead of sliding to
            // it from empty, which would read as the work having just started.
            // A reduced-motion application is placed on every frame, so the bar
            // always reports the value it was given.
            if !state.primed || crate::motion::reduce_motion() {
                state.spring.set(self.value);
                state.primed = true;
                return;
            }

            let elapsed = state
                .last
                .map_or(std::time::Duration::ZERO, |last| now.duration_since(last));
            state.last = Some(*now);

            let spring = PROGRESS_SPRING;

            state.spring.step(self.value, spring, elapsed);

            if !state.spring.is_settled(self.value, spring) {
                shell.request_redraw();
            }
        }
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::tree::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        layout: iced::advanced::layout::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        let bounds = layout.bounds();

        if !bounds.intersects(viewport) {
            return;
        }

        let state = tree.state.downcast_ref::<ProgressState>();
        let colors = theme.colors();
        let radius: iced::border::Radius = f32::from(theme.radius().full.min(8)).into();

        renderer.fill_quad(
            iced::advanced::renderer::Quad {
                bounds,
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius,
                },
                shadow: iced::Shadow::default(),
                snap: true,
            },
            colors.secondary,
        );

        // A fill of zero width is skipped rather than drawn, so a bar at rest
        // at the bottom of its range leaves no sliver of color.
        let filled = state.spring.value().clamp(0.0, 1.0);
        if filled > 0.0 {
            renderer.fill_quad(
                iced::advanced::renderer::Quad {
                    bounds: iced::Rectangle {
                        width: bounds.width * filled,
                        ..bounds
                    },
                    border: iced::Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius,
                    },
                    shadow: iced::Shadow::default(),
                    snap: true,
                },
                match self.tone {
                    Tone::Neutral => colors.primary,
                    other => other.accent(theme),
                },
            );
        }
    }
}

impl<'a, Message, Renderer> From<Progress> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(progress: Progress) -> Self {
        Element::new(progress)
    }
}

/// Builds an alert: a full-width message with a tinted background.
///
/// `title` and `body` are separate so the caller can give the alert a heading;
/// pass the same string to both for a single-line alert.
///
/// ```
/// # use iced_kit::widgets::{alert, Tone};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # use iced::widget::column;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// alert("Heads up", "Your trial ends in 3 days.", Tone::Warning).into()
/// # }
/// ```
pub fn alert<'a, Message: 'a>(
    title: impl text::IntoFragment<'a>,
    body: impl text::IntoFragment<'a>,
    tone: Tone,
) -> Element<'a, Message, Theme> {
    let title_size = Size::Md.text();
    let body_size = Size::Sm.text();

    container(
        iced::widget::column![
            text(title)
                .size(title_size.size)
                .line_height(title_size.line_height()),
            text(body)
                .size(body_size.size)
                .line_height(body_size.line_height()),
        ]
        .spacing(4),
    )
    .padding(16)
    .width(Length::Fill)
    .class(Box::new(move |theme: &Theme| {
        let colors = theme.colors();
        let surface = tone.surface(theme);

        container::Style {
            background: Some(iced::Background::Color(surface)),
            border: iced::Border {
                // The accent is translucent, so it is composited onto the
                // tinted surface to stay visible against it.
                color: catalog::blend(tone.border(theme), surface),
                width: 1.0,
                radius: f32::from(theme.radius().md).into(),
            },
            // The tinted surface is the signal; the text stays the theme's
            // foreground so it keeps its contrast on a custom palette.
            text_color: Some(colors.foreground),
            ..container::Style::default()
        }
    }) as container::StyleFn<'a, Theme>)
    .into()
}

/// Builds an empty-state placeholder: a centered, muted message.
pub fn empty_state<'a, Message: 'a>(
    title: impl text::IntoFragment<'a>,
    description: impl text::IntoFragment<'a>,
) -> Element<'a, Message, Theme> {
    let title_size = Size::Md.text();
    let body_size = Size::Sm.text();

    container(
        iced::widget::column![
            text(title)
                .size(title_size.size)
                .line_height(title_size.line_height()),
            text(description)
                .size(body_size.size)
                .line_height(body_size.line_height()),
        ]
        .spacing(4)
        .align_x(iced::Alignment::Center),
    )
    .padding(32)
    .width(Length::Fill)
    .center_x(Length::Fill)
    .class(Box::new(catalog::muted) as container::StyleFn<'a, Theme>)
    .into()
}

#[cfg(test)]
mod tests {
    use super::{alert, badge, card, divider, empty_state, progress, Tone};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    #[test]
    fn a_card_renders() {
        let element: iced::Element<'_, Message, Theme> = card("Content").into();
        drop(element);
    }

    #[test]
    fn dividers_render_in_both_orientations() {
        let horizontal: iced::Element<'_, Message, Theme> = divider().into();
        let vertical: iced::Element<'_, Message, Theme> = super::vertical_divider().into();
        drop(horizontal);
        drop(vertical);
    }

    #[test]
    fn a_badge_renders_in_every_tone() {
        for tone in [
            Tone::Neutral,
            Tone::Success,
            Tone::Warning,
            Tone::Danger,
            Tone::Primary,
        ] {
            let element: iced::Element<'_, Message, Theme> = badge("Active", tone).into();
            drop(element);
        }
    }

    #[test]
    fn progress_clamps_out_of_range_values() {
        // Values outside 0..=1 are clamped rather than panicking, because a
        // caller passing a percentage should not crash the UI.
        for value in [-1.0, 0.0, 0.5, 1.0, 2.0, f32::NAN] {
            let element: iced::Element<'_, Message, Theme> = progress(value, Tone::Primary);
            drop(element);
        }
    }

    #[test]
    fn an_alert_renders_in_every_tone() {
        for tone in [Tone::Neutral, Tone::Success, Tone::Warning, Tone::Danger] {
            let element: iced::Element<'_, Message, Theme> = alert("Title", "Body", tone);
            drop(element);
        }
    }

    #[test]
    fn an_empty_state_renders() {
        let element: iced::Element<'_, Message, Theme> = empty_state("Nothing here", "Add an item");
        drop(element);
    }

    #[test]
    fn tone_accents_are_distinct_where_they_should_be() {
        let theme = Theme::light();
        assert_ne!(Tone::Success.accent(&theme), Tone::Danger.accent(&theme));
        assert_ne!(Tone::Warning.accent(&theme), Tone::Success.accent(&theme));
    }

    #[test]
    fn a_neutral_tone_adds_no_tint() {
        let theme = Theme::light();
        // A neutral surface must not be tinted, or a plain card would look
        // like an alert.
        let surface = Tone::Neutral.surface(&theme);
        let base = theme.colors().surface;
        assert_eq!(surface, base);
    }
}
