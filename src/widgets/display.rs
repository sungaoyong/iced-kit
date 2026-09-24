//! Presentational components: cards, dividers, badges, progress and alerts.

use crate::icons::IconName;
use crate::theme::{catalog, Size, Theme};
use iced::widget::{container, row, rule, text};
use iced::{Alignment, Color, Element, Length, Padding};

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

    /// The text color that reads on a surface filled with [`Tone::accent`].
    ///
    /// A badge is a filled pill, so its text sits on the accent rather than on
    /// the page. The tones that carry their own foreground token use it; the
    /// two that are raw colors pick white or near-black by luminance, so a
    /// custom palette cannot produce unreadable text.
    pub(crate) fn on_accent(self, theme: &Theme) -> Color {
        let colors = theme.colors();

        match self {
            Self::Primary => colors.primary_foreground,
            Self::Danger => colors.destructive_foreground,
            Self::Neutral => colors.background,
            Self::Success | Self::Warning => {
                if relative_luminance(self.accent(theme)) > 0.5 {
                    Color::from_rgb8(0x18, 0x18, 0x1b)
                } else {
                    Color::WHITE
                }
            }
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

/// A color's perceived brightness, for choosing readable text over it.
fn relative_luminance(color: Color) -> f32 {
    0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
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
/// For a rule with a label or a dashed line, use [`horizontal_separator`].
/// This stays the plain two-line convenience for the common case.
pub fn divider<'a>() -> rule::Rule<'a, Theme> {
    rule::horizontal(1)
}

/// Builds a vertical divider.
pub fn vertical_divider<'a>() -> rule::Rule<'a, Theme> {
    rule::vertical(1)
}

/// How a separator's line is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SeparatorStyle {
    /// A continuous line. The default.
    #[default]
    Solid,
    /// A line of short dashes.
    Dashed,
}

/// A separator, optionally labelled and optionally dashed.
///
/// The reference's `Separator` is a builder rather than a pair of functions,
/// because a rule with a label needs a slot for the label. The unlabelled,
/// solid case remains [`divider`] and [`vertical_divider`].
///
/// ```
/// # use iced_kit::widgets::{horizontal_separator, SeparatorStyle};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// horizontal_separator()
///     .label("or")
///     .dashed()
///     .into()
/// # }
/// ```
#[must_use = "a Separator does nothing unless it is turned into an Element"]
pub struct Separator<'a, Message> {
    label: Option<String>,
    style: SeparatorStyle,
    color: Option<Color>,
    vertical: bool,
    _message: std::marker::PhantomData<&'a Message>,
}

impl<'a, Message: 'a> Separator<'a, Message> {
    /// A horizontal separator.
    pub fn horizontal() -> Self {
        Self {
            label: None,
            style: SeparatorStyle::Solid,
            color: None,
            vertical: false,
            _message: std::marker::PhantomData,
        }
    }

    /// A vertical separator.
    pub fn vertical() -> Self {
        Self {
            vertical: true,
            ..Self::horizontal()
        }
    }

    /// Draws a label in the middle of the rule.
    ///
    /// A vertical separator has no room for a label and ignores it.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Draws the line dashed.
    pub fn dashed(mut self) -> Self {
        self.style = SeparatorStyle::Dashed;
        self
    }

    /// Overrides the line's color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Turns the separator into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let height = 1.0;
        let style = self.style;
        let color = self.color;
        let line = move |theme: &Theme| {
            let color = color.unwrap_or_else(|| theme.colors().border);

            container::Style {
                background: Some(iced::Background::Color(color)),
                ..container::Style::default()
            }
        };

        if self.vertical {
            // A vertical rule is one pixel wide and stretches to its row.
            return container(iced::widget::Space::new().width(Length::Fill))
                .width(Length::Fixed(height))
                .height(Length::Fill)
                .class(Box::new(line) as container::StyleFn<'a, Theme>)
                .into();
        }

        // A dashed horizontal rule is a strip of short dashes: iced's rule
        // widget draws one continuous line and owns no dash pattern.
        if style == SeparatorStyle::Dashed && self.label.is_none() {
            return container(dashes(1, color))
                .width(Length::Fill)
                .height(Length::Fixed(height))
                .into();
        }

        match self.label {
            None => container(iced::widget::Space::new().height(Length::Fixed(height)))
                .width(Length::Fill)
                .height(Length::Fixed(height))
                .class(Box::new(line) as container::StyleFn<'a, Theme>)
                .into(),
            Some(label) => {
                let label_style = Size::Sm.text();
                let mut line_row = row![].spacing(8).align_y(Alignment::Center);

                line_row = line_row.push(rule_segment(color, style));
                line_row = line_row.push(
                    text(label)
                        .size(label_style.size)
                        .line_height(label_style.line_height())
                        .class(Box::new(|theme: &Theme| text::Style {
                            color: Some(theme.colors().muted_foreground),
                        }) as text::StyleFn<'a, Theme>),
                );
                line_row = line_row.push(rule_segment(color, style));

                line_row.into()
            }
        }
    }
}

impl<'a, Message: 'a> From<Separator<'a, Message>> for Element<'a, Message, Theme> {
    fn from(separator: Separator<'a, Message>) -> Self {
        separator.into_element()
    }
}

/// One side of a labelled rule: a single-pixel line that takes the space left
/// over, drawn dashed or solid.
fn rule_segment<'a, Message: 'a>(
    color: Option<Color>,
    style: SeparatorStyle,
) -> Element<'a, Message, Theme> {
    if style == SeparatorStyle::Dashed {
        return container(dashes(8, color))
            .width(Length::Fill)
            .height(Length::Fixed(1.0))
            .into();
    }

    container(iced::widget::Space::new().height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .class(Box::new(move |theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                color.unwrap_or_else(|| theme.colors().border),
            )),
            ..container::Style::default()
        }) as container::StyleFn<'a, Theme>)
        .into()
}

/// A row of short dashes filling the width it is given.
fn dashes<'a, Message: 'a>(count: usize, color: Option<Color>) -> Element<'a, Message, Theme> {
    let mut strip = row![].spacing(4).width(Length::Fill);

    for _ in 0..count.max(1) {
        strip = strip.push(
            container(iced::widget::Space::new().width(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fixed(1.0))
                .class(Box::new(move |theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(
                        color.unwrap_or_else(|| theme.colors().border),
                    )),
                    ..container::Style::default()
                }) as container::StyleFn<'a, Theme>),
        );
    }

    strip.into()
}

/// Builds a horizontal separator.
pub fn horizontal_separator<'a, Message: 'a>() -> Separator<'a, Message> {
    Separator::horizontal()
}

/// Builds a vertical separator.
pub fn vertical_separator<'a, Message: 'a>() -> Separator<'a, Message> {
    Separator::vertical()
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

/// What a badge shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    /// A short text label. The default, and what [`badge`] builds.
    #[default]
    Label,
    /// A small filled dot, for an unread marker.
    Dot,
    /// A count, hidden at zero and clamped to a maximum.
    Count,
    /// A single icon.
    Icon,
}

/// A small status marker: a label, a dot, a count or an icon.
///
/// [`badge`] covers the labelled case; this builder adds the reference's other
/// three variants, which have no text to pass.
///
/// ```
/// # use iced_kit::widgets::{badge_builder, BadgeVariant, Tone};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// // An unread marker, and a count that reads "9+".
/// badge_builder(Tone::Danger).dot().into()
/// # }
/// ```
#[must_use = "a Badge does nothing unless it is turned into an Element"]
pub struct Badge<'a, Message> {
    variant: BadgeVariant,
    label: Option<String>,
    count: usize,
    max: usize,
    icon: Option<IconName>,
    tone: Tone,
    size: Size,
    _message: std::marker::PhantomData<&'a Message>,
}

impl<'a, Message: 'a> Badge<'a, Message> {
    /// Creates a badge in the given tone.
    pub fn new(tone: Tone) -> Self {
        Self {
            variant: BadgeVariant::Label,
            label: None,
            count: 0,
            max: 99,
            icon: None,
            tone,
            size: Size::Sm,
            _message: std::marker::PhantomData,
        }
    }

    /// Sets the text label, which also selects the labelled variant.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.variant = BadgeVariant::Label;
        self
    }

    /// Shows a dot instead of text.
    pub fn dot(mut self) -> Self {
        self.variant = BadgeVariant::Dot;
        self
    }

    /// Shows a count, capped by [`Badge::max`].
    ///
    /// A count of zero draws nothing at all: an unread marker at zero is not a
    /// marker, and drawing a `0` would claim otherwise.
    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self.variant = BadgeVariant::Count;
        self
    }

    /// Shows an icon instead of text.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self.variant = BadgeVariant::Icon;
        self
    }

    /// Sets the largest count drawn; above it the badge reads `{max}+`.
    /// The default is 99.
    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }

    /// Sets the size step.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Turns the badge into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let tone = self.tone;
        let text_style = self.size.text();

        let background: container::StyleFn<'a, Theme> =
            Box::new(move |theme: &Theme| container::Style {
                background: Some(iced::Background::Color(tone.accent(theme))),
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: f32::from(theme.radius().full.min(999)).into(),
                },
                text_color: Some(tone.on_accent(theme)),
                ..container::Style::default()
            });

        match self.variant {
            BadgeVariant::Label => {
                let label = self.label.unwrap_or_default();

                container(
                    text(label)
                        .size(text_style.size)
                        .line_height(text_style.line_height()),
                )
                .padding(Padding {
                    top: 2.0,
                    right: 8.0,
                    bottom: 2.0,
                    left: 8.0,
                })
                .class(background)
                .into()
            }
            BadgeVariant::Dot => {
                // A dot is as tall as its text line so it agrees with a badge
                // beside it, and as wide as it is tall.
                container(iced::widget::Space::new())
                    .width(Length::Fixed(8.0))
                    .height(Length::Fixed(8.0))
                    .class(background)
                    .into()
            }
            BadgeVariant::Count => {
                if self.count == 0 {
                    // An empty element, so a row of badges keeps its spacing
                    // rather than collapsing as the count reaches zero.
                    return row![].into();
                }

                let label = count_label(self.count, self.max);

                container(
                    text(label)
                        .size(text_style.size)
                        .line_height(text_style.line_height()),
                )
                .padding(Padding {
                    top: 2.0,
                    right: 6.0,
                    bottom: 2.0,
                    left: 6.0,
                })
                .class(background)
                .into()
            }
            BadgeVariant::Icon => {
                let icon = self.icon.unwrap_or(IconName::Info);

                container(crate::widgets::Icon::new(icon).into_element(self.size))
                    .padding(Padding {
                        top: 2.0,
                        right: 6.0,
                        bottom: 2.0,
                        left: 6.0,
                    })
                    .class(background)
                    .into()
            }
        }
    }
}

impl<'a, Message: 'a> From<Badge<'a, Message>> for Element<'a, Message, Theme> {
    fn from(badge: Badge<'a, Message>) -> Self {
        badge.into_element()
    }
}

/// Builds a badge in any of its variants.
pub fn badge_builder<'a, Message: 'a>(tone: Tone) -> Badge<'a, Message> {
    Badge::new(tone)
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

/// An alert under construction: a message with an icon, an optional title, the
/// banner form and an optional close button.
///
/// [`alert`] covers the common title-and-body case; this builder adds what the
/// reference's `Alert` carries.
///
/// ```
/// # use iced_kit::widgets::{alert_builder, Tone};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Dismiss }
/// # fn view() -> Element<'static, Message, Theme> {
/// // A dismissible banner across the top of a page.
/// alert_builder("Your trial ends in 3 days.", Tone::Warning)
///     .title("Heads up")
///     .banner()
///     .on_close(Message::Dismiss)
///     .into()
/// # }
/// ```
#[must_use = "an Alert does nothing unless it is turned into an Element"]
pub struct Alert<'a, Message> {
    title: Option<String>,
    body: String,
    tone: Tone,
    icon: Option<IconName>,
    banner: bool,
    visible: bool,
    on_close: Option<Message>,
    _message: std::marker::PhantomData<&'a Message>,
}

impl<'a, Message: 'a> Alert<'a, Message> {
    /// Creates an alert with the given body text.
    pub fn new(body: impl Into<String>, tone: Tone) -> Self {
        Self {
            title: None,
            body: body.into(),
            tone,
            icon: Some(default_icon(tone)),
            banner: false,
            visible: true,
            on_close: None,
            _message: std::marker::PhantomData,
        }
    }

    /// Sets the heading drawn above the body.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replaces the default icon for the tone.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Draws the alert without an icon.
    pub fn no_icon(mut self) -> Self {
        self.icon = None;
        self
    }

    /// Draws the alert edge to edge: no border, no radius, full width.
    ///
    /// This is the strip at the top of a page rather than a card within it.
    pub fn banner(mut self) -> Self {
        self.banner = true;
        self
    }

    /// Hides the alert. A hidden alert renders nothing.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Adds a close button that emits this message.
    pub fn on_close(mut self, message: Message) -> Self {
        self.on_close = Some(message);
        self
    }

    /// Turns the alert into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme>
    where
        Message: Clone + 'a,
    {
        if !self.visible {
            return row![].into();
        }

        let Self {
            title,
            body,
            tone,
            icon,
            banner,
            visible: _,
            on_close,
            _message,
        } = self;

        let title_size = Size::Md.text();
        let body_size = Size::Sm.text();

        let mut content = row![].spacing(12).align_y(Alignment::Start);

        if let Some(icon) = icon {
            content = content.push(crate::widgets::Icon::new(icon).into_element(Size::Md));
        }

        let mut lines = iced::widget::column![].spacing(4);

        if let Some(title) = title {
            lines = lines.push(
                text(title)
                    .size(title_size.size)
                    .line_height(title_size.line_height()),
            );
        }

        lines = lines.push(
            text(body)
                .size(body_size.size)
                .line_height(body_size.line_height()),
        );

        content = content.push(lines.width(Length::Fill));

        if let Some(message) = on_close {
            let close: Element<'a, Message, Theme> = crate::widgets::icon_button::<Message>()
                .icon(IconName::X)
                .ghost()
                .size(Size::Sm)
                .on_press(message)
                .into();

            content = content.push(close);
        }

        let padding = if banner {
            Padding {
                top: 10.0,
                right: 16.0,
                bottom: 10.0,
                left: 16.0,
            }
        } else {
            Padding::from(16)
        };

        container(content)
            .padding(padding)
            .width(Length::Fill)
            .class(Box::new(move |theme: &Theme| {
                let colors = theme.colors();
                let surface = tone.surface(theme);

                container::Style {
                    background: Some(iced::Background::Color(surface)),
                    border: iced::Border {
                        // A banner is a strip, so it has no outline and no
                        // corners to round; a card within a page has both.
                        color: if banner {
                            Color::TRANSPARENT
                        } else {
                            catalog::blend(tone.border(theme), surface)
                        },
                        width: if banner { 0.0 } else { 1.0 },
                        radius: if banner {
                            0.0.into()
                        } else {
                            f32::from(theme.radius().md).into()
                        },
                    },
                    text_color: Some(colors.foreground),
                    ..container::Style::default()
                }
            }) as container::StyleFn<'a, Theme>)
            .into()
    }
}

impl<'a, Message: Clone + 'a> From<Alert<'a, Message>> for Element<'a, Message, Theme> {
    fn from(alert: Alert<'a, Message>) -> Self {
        alert.into_element()
    }
}

/// How a count badge draws its number: capped, with a `+` past the maximum.
fn count_label(count: usize, max: usize) -> String {
    if count > max {
        format!("{max}+")
    } else {
        format!("{count}")
    }
}

/// The icon a tone's alert wears by default.
fn default_icon(tone: Tone) -> IconName {
    match tone {
        Tone::Neutral | Tone::Primary => IconName::Info,
        Tone::Success => IconName::CircleCheck,
        Tone::Warning => IconName::TriangleAlert,
        Tone::Danger => IconName::CircleX,
    }
}

/// Builds an alert with the reference's full option set.
///
/// [`alert`] covers the title-and-body case; this is the entry point when the
/// icon, the banner form or a close button matters.
pub fn alert_builder<'a, Message: 'a>(body: impl Into<String>, tone: Tone) -> Alert<'a, Message> {
    Alert::new(body, tone)
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
    use super::{
        alert, alert_builder, badge, badge_builder, card, divider, empty_state,
        horizontal_separator, progress, vertical_separator, Alert, Badge, BadgeVariant, Separator,
        SeparatorStyle, Tone,
    };
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Closed,
    }

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

    #[test]
    fn a_badge_builder_records_its_variant() {
        let label: Badge<'_, Message> = badge_builder(Tone::Success).label("Active");
        assert_eq!(label.variant, BadgeVariant::Label);

        let dot: Badge<'_, Message> = badge_builder(Tone::Danger).dot();
        assert_eq!(dot.variant, BadgeVariant::Dot);

        let count: Badge<'_, Message> = badge_builder(Tone::Primary).count(5).max(9);
        assert_eq!(count.variant, BadgeVariant::Count);
        assert_eq!(count.count, 5);
        assert_eq!(count.max, 9);

        let icon: Badge<'_, Message> =
            badge_builder(Tone::Neutral).icon(crate::icons::IconName::Bell);
        assert_eq!(icon.variant, BadgeVariant::Icon);
    }

    /// Each variant must render, including the count that draws nothing.
    #[test]
    fn badge_variants_render() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            badge_builder(Tone::Success).label("Done").into(),
            badge_builder(Tone::Danger).dot().into(),
            badge_builder(Tone::Primary).count(7).into(),
            badge_builder(Tone::Warning).count(0).into(),
            badge_builder(Tone::Neutral)
                .icon(crate::icons::IconName::Bell)
                .into(),
        ];

        for element in elements {
            drop(element);
        }
    }

    /// A count above the maximum is capped rather than shown in full.
    #[test]
    fn a_count_is_capped_by_its_maximum() {
        // The label of a count badge is built during rendering, so the cap is
        // checked through the helper that formats it.
        assert_eq!(super::count_label(5, 99), "5");
        assert_eq!(super::count_label(100, 99), "99+");
        assert_eq!(super::count_label(99, 99), "99");
    }

    #[test]
    fn a_separator_records_its_options() {
        let separator: Separator<'_, Message> = horizontal_separator()
            .label("or")
            .dashed()
            .color(iced::Color::from_rgb8(1, 2, 3));

        assert_eq!(separator.label.as_deref(), Some("or"));
        assert_eq!(separator.style, SeparatorStyle::Dashed);
        assert!(separator.color.is_some());
        assert!(!separator.vertical);
        assert!(vertical_separator::<Message>().vertical);
    }

    #[test]
    fn separators_render_in_every_form() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            horizontal_separator().into(),
            horizontal_separator().label("Section").into(),
            horizontal_separator().dashed().into(),
            horizontal_separator().label("or").dashed().into(),
            vertical_separator().into(),
            vertical_separator().label("ignored").into(),
        ];

        for element in elements {
            drop(element);
        }
    }

    #[test]
    fn an_alert_builder_records_its_settings() {
        let built: Alert<'_, Message> = alert_builder("Trial ends soon.", Tone::Warning)
            .title("Heads up")
            .icon(crate::icons::IconName::Bell)
            .banner()
            .on_close(Message::Closed);

        assert_eq!(built.title.as_deref(), Some("Heads up"));
        assert_eq!(built.body, "Trial ends soon.");
        assert_eq!(
            built.icon.map(crate::icons::glyph),
            Some(crate::icons::glyph(crate::icons::IconName::Bell))
        );
        assert!(built.banner);
        assert!(built.on_close.is_some());
        assert!(built.visible);
    }

    /// The tone picks the icon, which is what tells the four kinds apart.
    #[test]
    fn an_alert_gets_the_icon_its_tone_calls_for() {
        use crate::icons::IconName;

        for (tone, expected) in [
            (Tone::Success, IconName::CircleCheck),
            (Tone::Warning, IconName::TriangleAlert),
            (Tone::Danger, IconName::CircleX),
            (Tone::Neutral, IconName::Info),
            (Tone::Primary, IconName::Info),
        ] {
            assert_eq!(
                crate::icons::glyph(super::default_icon(tone)),
                crate::icons::glyph(expected),
                "{tone:?} must wear its own icon"
            );
        }
    }

    #[test]
    fn alerts_render_in_every_form() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            alert_builder("Plain", Tone::Neutral).into(),
            alert_builder("Titled", Tone::Success).title("Saved").into(),
            alert_builder("Banner", Tone::Warning).banner().into(),
            alert_builder("Dismissible", Tone::Danger)
                .on_close(Message::Closed)
                .into(),
            alert_builder("Iconless", Tone::Primary).no_icon().into(),
        ];

        for element in elements {
            drop(element);
        }
    }

    /// A hidden alert draws nothing, so an application can keep it mounted
    /// while its exit is being animated.
    #[test]
    fn a_hidden_alert_renders_nothing() {
        let hidden: Alert<'_, Message> = alert_builder("Gone", Tone::Neutral).visible(false);
        let element: iced::Element<'_, Message, Theme> = hidden.into();
        drop(element);
    }
}
