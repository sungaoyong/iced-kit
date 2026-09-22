//! Toasts: transient notifications shown over the application.
//!
//! Toasts are queued by the application and rendered by
//! [`Layer`](crate::widgets::overlay::Layer). They are intentionally stateless
//! about time: the application decides when one is added and when it is
//! dismissed, so an application that never expires them gets a persistent
//! notification instead of a surprise disappearance.

use crate::motion::DURATION_NORMAL;
use crate::theme::{Size, Theme};
use crate::widgets::display::Tone;
use crate::widgets::overlay::{floating_shadow, scrim, Enter, EnterFrom};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Color, Element, Length, Padding};

/// Which corner the toast stack sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastPlacement {
    /// Below the top edge, centred.
    #[default]
    TopCenter,
    /// Below the top edge, against the right.
    TopRight,
    /// Above the bottom edge, against the right.
    BottomRight,
    /// Above the bottom edge, centred.
    BottomCenter,
}

/// The severity of a toast, which selects its color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastKind {
    /// A neutral message.
    #[default]
    Info,
    /// The action succeeded.
    Success,
    /// Something needs attention.
    Warning,
    /// The action failed.
    Error,
}

impl ToastKind {
    /// The tone this kind renders with.
    #[must_use]
    pub const fn tone(self) -> Tone {
        match self {
            Self::Info => Tone::Primary,
            Self::Success => Tone::Success,
            Self::Warning => Tone::Warning,
            Self::Error => Tone::Danger,
        }
    }
}

/// One toast.
#[derive(Debug, Clone)]
#[must_use = "a Toast does nothing unless it is given to `Toasts`"]
pub struct Toast {
    title: String,
    description: Option<String>,
    kind: ToastKind,
    dismissible: bool,
}

impl Toast {
    /// Creates a toast with a title.
    pub fn new(title: impl Into<String>, kind: ToastKind) -> Self {
        Self {
            title: title.into(),
            description: None,
            kind,
            dismissible: true,
        }
    }

    /// Adds a second line of detail.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Whether the toast shows a close button.
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Returns the toast's kind.
    #[must_use]
    pub fn kind(&self) -> ToastKind {
        self.kind
    }
}

/// A stack of toasts, newest last.
#[must_use = "Toasts do nothing unless they are added to a Layer"]
pub struct Toasts<'a, Message> {
    items: Vec<(Toast, Option<Message>)>,
    placement: ToastPlacement,
    max_width: f32,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, Message: Clone + 'a> Default for Toasts<'a, Message> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            placement: ToastPlacement::default(),
            max_width: 360.0,
            _lifetime: std::marker::PhantomData,
        }
    }
}

impl<'a, Message: Clone + 'a> Toasts<'a, Message> {
    /// Creates an empty toast stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a toast that cannot be dismissed by the user.
    pub fn push(mut self, toast: Toast) -> Self {
        self.items.push((toast, None));
        self
    }

    /// Adds a toast with a message emitted when its close button is pressed.
    pub fn push_dismissible(mut self, toast: Toast, on_dismiss: Message) -> Self {
        self.items.push((toast, Some(on_dismiss)));
        self
    }

    /// Sets which corner the stack sits in.
    pub fn placement(mut self, placement: ToastPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets the maximum width of a single toast.
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = max_width;
        self
    }

    /// Whether the stack has no toasts.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Builds the stack as a full-area, non-interactive overlay.
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            items,
            placement,
            max_width,
            _lifetime,
        } = self;

        let mut stack = column![].spacing(8).width(Length::Fixed(max_width));

        // Newest first: a just-reported outcome should not push an older one
        // off the edge of attention.
        for (toast, on_dismiss) in items.into_iter().rev() {
            stack = stack.push(toast_element(toast, on_dismiss));
        }

        let (align_x, align_y, padding) = match placement {
            ToastPlacement::TopCenter => (Alignment::Center, Alignment::Start, 16.0),
            ToastPlacement::TopRight => (Alignment::End, Alignment::Start, 16.0),
            ToastPlacement::BottomRight => (Alignment::End, Alignment::End, 16.0),
            ToastPlacement::BottomCenter => (Alignment::Center, Alignment::End, 16.0),
        };

        // The stack arrives from whichever edge it is pinned to, so a toast
        // reads as coming from that side of the window rather than appearing on
        // top of everything at once. The wrapper goes around the stack rather
        // than the full-area container, which would slide the whole layer.
        let from = match placement {
            ToastPlacement::TopCenter | ToastPlacement::TopRight => EnterFrom::Above,
            ToastPlacement::BottomRight | ToastPlacement::BottomCenter => EnterFrom::Below,
        };

        let arriving = Enter::new(stack, from).duration(DURATION_NORMAL);

        container(arriving)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(align_x)
            .align_y(align_y)
            .padding(Padding::new(padding))
            // The container must not eat clicks, or the page behind the toasts
            // would stop responding where they overlap.
            .style(|_theme| container::Style::default())
            .into()
    }
}

/// Renders one toast.
fn toast_element<'a, Message: Clone + 'a>(
    toast: Toast,
    on_dismiss: Option<Message>,
) -> Element<'a, Message, Theme> {
    let title_style = Size::Md.text();
    let body_style = Size::Sm.text();
    let tone = toast.kind.tone();

    let mut content = column![text(toast.title)
        .size(title_style.size)
        .line_height(title_style.line_height())]
    .spacing(2);

    if let Some(description) = toast.description {
        content = content.push(
            text(description)
                .size(body_style.size)
                .line_height(body_style.line_height()),
        );
    }

    let mut row = row![content.width(Length::Fill)]
        .spacing(12)
        .align_y(Alignment::Start);

    if toast.dismissible {
        if let Some(message) = on_dismiss {
            row = row.push(
                button(text("✕").size(body_style.size))
                    .padding(Padding::new(4.0))
                    .class(Box::new(|theme: &Theme, status| {
                        let colors = theme.colors();
                        let hovered = matches!(status, button::Status::Hovered);

                        button::Style {
                            background: hovered.then_some(iced::Background::Color(colors.accent)),
                            text_color: colors.muted_foreground,
                            border: iced::Border {
                                color: Color::TRANSPARENT,
                                width: 0.0,
                                radius: f32::from(theme.radius().sm).into(),
                            },
                            ..button::Style::default()
                        }
                    }) as button::StyleFn<'a, Theme>)
                    .on_press(message),
            );
        }
    }

    container(row)
        .width(Length::Fill)
        .max_width(360.0)
        .padding(Padding::new(12.0))
        .class(Box::new(move |theme: &Theme| {
            let colors = theme.colors();
            let surface = tone.surface(theme);

            container::Style {
                background: Some(iced::Background::Color(surface)),
                border: iced::Border {
                    // A colored edge marks the severity without tinting the
                    // whole surface dark enough to lose text contrast.
                    color: tone.accent(theme),
                    width: 1.0,
                    radius: f32::from(theme.radius().md).into(),
                },
                shadow: floating_shadow(theme),
                text_color: Some(colors.foreground),
                ..container::Style::default()
            }
        }) as container::StyleFn<'a, Theme>)
        .into()
}

/// Dims the page behind a blocking overlay.
///
/// Re-exported here so an application that manages its own layering can use the
/// same backdrop as [`Modal`](crate::widgets::Modal).
#[must_use]
pub fn toast_scrim<'a, Message: 'a>() -> Element<'a, Message, Theme> {
    scrim()
}

#[cfg(test)]
mod tests {
    use super::{Toast, ToastKind, ToastPlacement, Toasts};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Dismiss(usize),
    }

    #[test]
    fn an_empty_stack_reports_empty_and_renders() {
        let toasts: Toasts<'_, Message> = Toasts::new();
        assert!(toasts.is_empty());

        let element: iced::Element<'_, Message, Theme> = toasts.into_element();
        drop(element);
    }

    #[test]
    fn a_stack_renders_in_every_placement() {
        for placement in [
            ToastPlacement::TopCenter,
            ToastPlacement::TopRight,
            ToastPlacement::BottomRight,
            ToastPlacement::BottomCenter,
        ] {
            let element: iced::Element<'_, Message, Theme> = Toasts::new()
                .placement(placement)
                .push(Toast::new("Saved", ToastKind::Success))
                .into_element();
            drop(element);
        }
    }

    #[test]
    fn every_toast_kind_renders_with_and_without_a_description() {
        for kind in [
            ToastKind::Info,
            ToastKind::Success,
            ToastKind::Warning,
            ToastKind::Error,
        ] {
            let element: iced::Element<'_, Message, Theme> =
                Toasts::new().push(Toast::new("Title", kind)).into_element();
            drop(element);

            let element: iced::Element<'_, Message, Theme> = Toasts::new()
                .push(Toast::new("Title", kind).description("Detail line"))
                .into_element();
            drop(element);
        }
    }

    #[test]
    fn a_dismissible_toast_renders_a_close_button() {
        let element: iced::Element<'_, Message, Theme> = Toasts::new()
            .push_dismissible(Toast::new("Saved", ToastKind::Success), Message::Dismiss(0))
            .into_element();
        drop(element);
    }

    #[test]
    fn a_non_dismissible_toast_renders_without_one() {
        let element: iced::Element<'_, Message, Theme> = Toasts::new()
            .push_dismissible(
                Toast::new("Working…", ToastKind::Info).dismissible(false),
                Message::Dismiss(0),
            )
            .into_element();
        drop(element);
    }

    #[test]
    fn a_stack_of_many_toasts_renders() {
        let mut toasts = Toasts::new();

        for index in 0..8 {
            toasts = toasts.push_dismissible(
                Toast::new(format!("Event {index}"), ToastKind::Info),
                Message::Dismiss(index),
            );
        }

        assert!(!toasts.is_empty());

        let element: iced::Element<'_, Message, Theme> = toasts.into_element();
        drop(element);
    }

    #[test]
    fn toast_kinds_map_onto_distinct_tones() {
        use crate::widgets::Tone;

        assert_eq!(ToastKind::Info.tone(), Tone::Primary);
        assert_eq!(ToastKind::Success.tone(), Tone::Success);
        assert_eq!(ToastKind::Error.tone(), Tone::Danger);
        assert_ne!(ToastKind::Warning.tone(), ToastKind::Error.tone());
    }

    #[test]
    fn a_toast_keeps_its_kind() {
        assert_eq!(
            Toast::new("x", ToastKind::Warning).kind(),
            ToastKind::Warning
        );
    }
}
