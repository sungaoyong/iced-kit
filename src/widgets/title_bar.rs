//! Application title bars.
//!
//! A [`TitleBar`] is the strip at the top of a window: an icon, a title, an
//! optional subtitle, and a row of window controls. It is deliberately
//! presentation-only — dragging the window, minimizing and closing are the
//! application's business, so each control emits a message and the application
//! decides what to do with it.

use crate::icons::IconName;
use crate::theme::Theme;
use crate::widgets::button as kit_button;
use crate::widgets::display::Tone;
use crate::widgets::Icon;
use iced::widget::{container, row, text, Space};
use iced::{Alignment, Color, Element, Length, Padding};

pub use crate::widgets::display::Tone as TitleTone;

/// A button on a title bar's trailing edge.
///
/// The variants cover the conventional window controls; `Custom` carries its
/// own glyph for anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowControl {
    /// Minimize the window.
    Minimize,
    /// Maximize or restore the window.
    Maximize,
    /// Close the window.
    Close,
}

impl WindowControl {
    /// The icon this control draws.
    ///
    /// These come from the bundled icon font rather than from text characters,
    /// so a window control matches every other icon in the library: one weight,
    /// one optical size, one grid. The characters they replaced — `─`, `□`, `✕`
    /// — are sized and weighted by whichever system font resolves them, so they
    /// could not be made to agree with each other, let alone with the rest of
    /// the interface.
    #[must_use]
    pub const fn icon(self) -> IconName {
        match self {
            Self::Minimize => IconName::Minus,
            Self::Maximize => IconName::Square,
            Self::Close => IconName::X,
        }
    }

    /// Whether this control destroys the window, and so is colored as a danger.
    #[must_use]
    pub const fn is_destructive(self) -> bool {
        matches!(self, Self::Close)
    }

    /// The accessibility label for this control.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Minimize => "Minimize",
            Self::Maximize => "Maximize",
            Self::Close => "Close",
        }
    }
}

/// What a trailing button draws.
#[derive(Debug, Clone)]
enum ControlGlyph {
    /// A named icon, drawn in the icon font.
    Named(IconName),
    /// A caller's own character, drawn in the ambient font.
    Text(String),
}

/// One trailing button, its glyph, and the message it emits.
#[derive(Debug, Clone)]
struct Control<Message> {
    glyph: ControlGlyph,
    message: Message,
    destructive: bool,
}

/// A window title bar.
#[must_use = "a TitleBar does nothing unless it is turned into an Element"]
pub struct TitleBar<'a, Message> {
    title: String,
    subtitle: Option<String>,
    icon: Option<ControlGlyph>,
    controls: Vec<Control<Message>>,
    height: f32,
    tone: Tone,
    on_drag: Option<Message>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, Message: Clone + 'a> Default for TitleBar<'a, Message> {
    fn default() -> Self {
        Self {
            title: String::new(),
            subtitle: None,
            icon: None,
            controls: Vec::new(),
            height: 40.0,
            tone: Tone::Neutral,
            on_drag: None,
            _lifetime: std::marker::PhantomData,
        }
    }
}

impl<'a, Message: Clone + 'a> TitleBar<'a, Message> {
    /// Creates a title bar with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    /// Adds a secondary line under the title.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Adds a leading icon, drawn in the icon font.
    ///
    /// ```
    /// # use iced_kit::widgets::TitleBar;
    /// # use iced_kit::icons::IconName;
    /// # #[derive(Clone, Debug)] enum Message {}
    /// # fn view() -> iced::Element<'static, Message, iced_kit::Theme> {
    /// TitleBar::<Message>::new("My App").icon(IconName::Settings2).into()
    /// # }
    /// ```
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(ControlGlyph::Named(icon));
        self
    }

    /// Adds a leading icon glyph: a plain character in the ambient font.
    ///
    /// For a symbol the icon set does not carry. Prefer [`icon`](Self::icon) for
    /// anything Lucide has, so the bar matches the rest of the interface.
    pub fn icon_glyph(mut self, glyph: impl Into<String>) -> Self {
        self.icon = Some(ControlGlyph::Text(glyph.into()));
        self
    }

    /// Sets the bar's height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(1.0);
        self
    }

    /// Sets the bar's background tone.
    ///
    /// `Neutral` renders the surface color; the other tones tint it, which is
    /// what a modal-looking title bar or a warning state wants.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Adds a window control that emits `message` when pressed.
    pub fn control(mut self, control: WindowControl, message: Message) -> Self {
        self.controls.push(Control {
            glyph: ControlGlyph::Named(control.icon()),
            message,
            destructive: control.is_destructive(),
        });
        self
    }

    /// Adds a custom button that emits `message` when pressed.
    ///
    /// The glyph is a plain character drawn in the ambient font, so it suits a
    /// symbol the icon set does not carry. For an icon, use
    /// [`custom_icon`](Self::custom_icon).
    pub fn custom_control(
        mut self,
        glyph: impl Into<String>,
        message: Message,
        destructive: bool,
    ) -> Self {
        self.controls.push(Control {
            glyph: ControlGlyph::Text(glyph.into()),
            message,
            destructive,
        });
        self
    }

    /// Adds a custom button drawing a named icon.
    ///
    /// This is how to add a control that matches the built-in ones, such as a
    /// restore button or a settings button on the trailing edge.
    pub fn custom_icon(
        mut self,
        icon: IconName,
        message: Message,
        destructive: bool,
    ) -> Self {
        self.controls.push(Control {
            glyph: ControlGlyph::Named(icon),
            message,
            destructive,
        });
        self
    }

    /// Emits `message` when the bar itself is pressed.
    ///
    /// This is the hook an application uses to start a window drag, since the
    /// window handle is only available from the message handler.
    pub fn on_drag(mut self, message: Message) -> Self {
        self.on_drag = Some(message);
        self
    }

    /// Builds the leading title area.
    fn title_area(&self) -> Element<'a, Message, Theme> {
        let title_style = crate::theme::Size::Sm.text();
        let subtitle_style = crate::theme::Size::Sm.text();

        let mut title = row![].spacing(8).align_y(Alignment::Center);

        if let Some(icon) = self.icon.as_ref() {
            let element: Element<'a, Message, Theme> = match icon {
                // Through the shared `Icon`, so the leading icon is sized and
                // colored exactly like the window controls beside the title.
                ControlGlyph::Named(name) => {
                    Icon::new(*name).into_element(crate::theme::Size::Md)
                }
                ControlGlyph::Text(glyph) => text(glyph.clone())
                    .size(title_style.size + 2.0)
                    .into(),
            };

            title = title.push(element);
        }

        title = title.push(
            text(self.title.clone())
                .size(title_style.size)
                .line_height(title_style.line_height()),
        );

        if let Some(subtitle) = self.subtitle.as_ref() {
            title = title.push(
                text(subtitle.clone())
                    .size(subtitle_style.size)
                    .class(Box::new(|theme: &Theme| text::Style {
                        color: Some(theme.colors().muted_foreground),
                    }) as text::StyleFn<'a, Theme>),
            );
        }

        title.into()
    }

    /// Builds the trailing control row.
    fn controls(&self) -> Element<'a, Message, Theme> {
        let mut controls = row![].spacing(2).align_y(Alignment::Center);
        let size = crate::theme::Size::Sm;

        for control in &self.controls {
            let destructive = control.destructive;

            // The glyph decides how the button is built. A named icon goes
            // through the icon slot with no label at all — `icon_button` is what
            // expresses that, and passing an empty string instead would leave a
            // label part in the content row, whose gap pushes the icon off
            // centre. A caller's character is the label, which is already a
            // single part.
            let button = match &control.glyph {
                ControlGlyph::Named(icon) => {
                    let button = if destructive {
                        kit_button::icon_button::<Message>().destructive()
                    } else {
                        kit_button::icon_button::<Message>().ghost()
                    };

                    button.icon(Icon::new(*icon))
                }
                ControlGlyph::Text(glyph) => {
                    let button = if destructive {
                        kit_button::<Message>(glyph.clone()).destructive()
                    } else {
                        kit_button::<Message>(glyph.clone()).ghost()
                    };

                    button
                }
            }
            .size(size)
            .width(Length::Fixed(32.0))
            .on_press(control.message.clone());

            controls = controls.push(button);
        }

        controls.into()
    }

    /// Converts the bar into an [`Element`].
    #[must_use]
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        // Both children are built first, while `self` is still whole.
        let title_area = self.title_area();
        let controls = self.controls();

        let Self {
            height,
            tone,
            on_drag,
            ..
        } = self;

        let content = row![title_area, Space::new().width(Length::Fill), controls,]
            .spacing(12)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        let bar = container(content)
            .width(Length::Fill)
            .height(Length::Fixed(height))
            .padding(Padding {
                top: 0.0,
                right: 6.0,
                bottom: 0.0,
                left: 12.0,
            })
            .class(Box::new(move |theme: &Theme| {
                let colors = theme.colors();
                let surface = tone.surface(theme);

                container::Style {
                    background: Some(iced::Background::Color(surface)),
                    border: iced::Border {
                        color: colors.border,
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    text_color: Some(colors.foreground),
                    ..container::Style::default()
                }
            }) as container::StyleFn<'a, Theme>);

        // A title bar is the drag handle on a borderless window, so the whole
        // strip emits the drag message rather than only the title text.
        match on_drag {
            Some(message) => iced::widget::MouseArea::new(bar).on_press(message).into(),
            None => bar.into(),
        }
    }
}

impl<'a, Message: Clone + 'a> From<TitleBar<'a, Message>> for Element<'a, Message, Theme> {
    fn from(bar: TitleBar<'a, Message>) -> Self {
        bar.into_element()
    }
}

/// The recommended title bar height, in logical pixels.
///
/// Taller than a toolbar, because a title bar is also the window's drag handle.
#[must_use]
pub fn recommended_height() -> f32 {
    40.0
}

/// The color a title bar uses for its bottom edge.
#[must_use]
pub fn separator_color(theme: &Theme) -> Color {
    theme.colors().border
}

/// A spacer that pushes later title-bar content to the trailing edge.
#[must_use]
pub fn title_spacer<'a, Message: 'a>() -> Element<'a, Message, Theme> {
    Space::new().width(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::{recommended_height, separator_color, title_spacer, TitleBar, WindowControl};
    use crate::icons::IconName;
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Minimize,
        Maximize,
        Close,
        Drag,
    }

    #[test]
    fn a_title_bar_renders_with_a_title_only() {
        let element: iced::Element<'_, Message, Theme> = TitleBar::new("My App").into();
        drop(element);
    }

    #[test]
    fn a_title_bar_renders_with_an_icon_and_subtitle() {
        let element: iced::Element<'_, Message, Theme> = TitleBar::new("My App")
            .icon(IconName::Settings2)
            .subtitle("untitled")
            .into();
        drop(element);
    }

    #[test]
    fn a_title_bar_renders_with_a_text_glyph_icon() {
        let element: iced::Element<'_, Message, Theme> = TitleBar::new("My App")
            .icon_glyph("◆")
            .into();
        drop(element);
    }

    #[test]
    fn a_custom_control_can_be_an_icon_or_a_glyph() {
        let with_icon: iced::Element<'_, Message, Theme> = TitleBar::new("My App")
            .custom_icon(IconName::Info, Message::Drag, false)
            .into();
        drop(with_icon);

        let with_glyph: iced::Element<'_, Message, Theme> = TitleBar::new("My App")
            .custom_control("⚙", Message::Drag, false)
            .into();
        drop(with_glyph);
    }

    #[test]
    fn a_title_bar_renders_with_every_window_control() {
        let element: iced::Element<'_, Message, Theme> = TitleBar::new("My App")
            .control(WindowControl::Minimize, Message::Minimize)
            .control(WindowControl::Maximize, Message::Maximize)
            .control(WindowControl::Close, Message::Close)
            .into();
        drop(element);
    }

    #[test]
    fn a_title_bar_renders_with_a_custom_control() {
        let element: iced::Element<'_, Message, Theme> = TitleBar::new("My App")
            .custom_control("⚙", Message::Drag, false)
            .into();
        drop(element);
    }

    #[test]
    fn a_draggable_title_bar_renders() {
        let element: iced::Element<'_, Message, Theme> =
            TitleBar::new("My App").on_drag(Message::Drag).into();
        drop(element);
    }

    #[test]
    fn a_title_bar_renders_in_every_tone() {
        for tone in [
            crate::widgets::Tone::Neutral,
            crate::widgets::Tone::Primary,
            crate::widgets::Tone::Warning,
        ] {
            let element: iced::Element<'_, Message, Theme> =
                TitleBar::new("My App").tone(tone).into();
            drop(element);
        }
    }

    #[test]
    fn every_window_control_renders_in_every_tone() {
        for tone in [crate::widgets::Tone::Neutral, crate::widgets::Tone::Primary] {
            let element: iced::Element<'_, Message, Theme> = TitleBar::new("My App")
                .tone(tone)
                .control(WindowControl::Minimize, Message::Minimize)
                .control(WindowControl::Maximize, Message::Maximize)
                .control(WindowControl::Close, Message::Close)
                .into();
            drop(element);
        }
    }

    #[test]
    fn a_title_bar_renders_at_several_heights() {
        for height in [28.0, 40.0, 64.0] {
            let element: iced::Element<'_, Message, Theme> =
                TitleBar::new("My App").height(height).into();
            drop(element);
        }
    }

    #[test]
    fn only_close_is_destructive() {
        assert!(!WindowControl::Minimize.is_destructive());
        assert!(!WindowControl::Maximize.is_destructive());
        assert!(WindowControl::Close.is_destructive());
    }

    #[test]
    fn every_control_has_an_icon_and_a_label() {
        for control in [
            WindowControl::Minimize,
            WindowControl::Maximize,
            WindowControl::Close,
        ] {
            // A glyph in the icon font is always a private-use character, which
            // is what keeps an icon from colliding with a letter.
            let glyph = crate::icons::glyph(control.icon());
            assert!(
                ('\u{e000}'..='\u{f8ff}').contains(&glyph),
                "{control:?} resolved to {glyph:?}, which is not an icon-font glyph"
            );
            assert!(!control.label().is_empty());
        }
    }

    #[test]
    fn control_icons_are_distinct() {
        // Two controls drawing the same icon would be indistinguishable.
        let icons = [
            WindowControl::Minimize.icon(),
            WindowControl::Maximize.icon(),
            WindowControl::Close.icon(),
        ];

        for (i, left) in icons.iter().enumerate() {
            for (j, right) in icons.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        crate::icons::glyph(*left),
                        crate::icons::glyph(*right),
                        "two window controls must not draw the same glyph"
                    );
                }
            }
        }
    }

    #[test]
    fn the_window_controls_use_the_icons_they_name() {
        // Pinned so a future icon-set change is a deliberate edit rather than a
        // silent reshuffle. The comparison is on the resolved glyph, because the
        // icon font's own enum implements neither `PartialEq` nor `Eq`.
        for (control, expected) in [
            (WindowControl::Minimize, IconName::Minus),
            (WindowControl::Maximize, IconName::Square),
            (WindowControl::Close, IconName::X),
        ] {
            assert_eq!(
                crate::icons::glyph(control.icon()),
                crate::icons::glyph(expected)
            );
        }
    }

    #[test]
    fn the_recommended_height_is_tall_enough_for_its_text() {
        let text = crate::theme::Size::Sm.text();
        assert!(recommended_height() > text.line_height);
    }

    #[test]
    fn the_separator_matches_the_border_token() {
        let theme = Theme::light();
        assert_eq!(separator_color(&theme), theme.colors().border);
    }

    #[test]
    fn a_title_spacer_renders() {
        let element: iced::Element<'_, Message, Theme> = title_spacer();
        drop(element);
    }
}
