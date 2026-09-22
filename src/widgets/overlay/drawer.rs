//! A slide-in drawer.
//!
//! A drawer is a panel that slides in from an edge and, unlike a modal, does
//! not centre itself: it occupies the full height (or width) of the window and
//! leaves the page visible beside it.

use crate::motion::DURATION_SLOW;
use crate::theme::{Size, Theme};
use crate::widgets::overlay::{floating_shadow, scrim, Enter, EnterFrom};
use crate::widgets::{button as kit_button, Button};
use iced::widget::{column, container, row, text, MouseArea, Space};
use iced::{Alignment, Element, Length, Padding};

/// The furthest a drawer slides as it arrives, in logical pixels.
///
/// A drawer wider than this still travels this far: the motion is meant to read
/// as the panel entering, and a full-width slide would take longer to complete
/// than the transition does, so it would appear to stop short and then snap.
const DRAWER_TRAVEL: f32 = 96.0;

/// Which edge a drawer slides in from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawerSide {
    /// From the left edge.
    Left,
    /// From the right edge, the usual choice for a detail panel.
    #[default]
    Right,
    /// From the bottom edge.
    Bottom,
}

impl DrawerSide {
    /// Whether the drawer stretches horizontally (bottom) or vertically.
    #[must_use]
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Bottom)
    }
}

/// How large a drawer is along its sliding axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawerSize {
    /// A narrow drawer, for a short list.
    Sm,
    /// The default size.
    #[default]
    Md,
    /// A wide drawer, for an editor.
    Lg,
}

impl DrawerSize {
    /// The extent in logical pixels.
    #[must_use]
    pub const fn extent(self) -> f32 {
        match self {
            Self::Sm => 280.0,
            Self::Md => 380.0,
            Self::Lg => 560.0,
        }
    }
}

/// A slide-in drawer.
#[must_use = "a Drawer does nothing unless it is added to a Layer"]
pub struct Drawer<'a, Message> {
    title: String,
    body: Element<'a, Message, Theme>,
    side: DrawerSide,
    size: DrawerSize,
    on_dismiss: Option<Message>,
    footer: Option<Element<'a, Message, Theme>>,
    presence: Option<crate::motion::Presence>,
}

impl<'a, Message: Clone + 'a> Drawer<'a, Message> {
    /// Creates a drawer with a title and body.
    pub fn new(title: impl Into<String>, body: impl Into<Element<'a, Message, Theme>>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            side: DrawerSide::default(),
            size: DrawerSize::default(),
            on_dismiss: None,
            footer: None,
            presence: None,
        }
    }

    /// Sets which edge the drawer slides in from.
    pub fn side(mut self, side: DrawerSide) -> Self {
        self.side = side;
        self
    }

    /// Sets the drawer's size.
    pub fn size(mut self, size: DrawerSize) -> Self {
        self.size = size;
        self
    }

    /// Enables dismissing by clicking the area beside the drawer.
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Makes the drawer slide back out as it closes.
    ///
    /// Without this the drawer slides in but is removed the moment it is closed,
    /// because by then the application has stopped building it. Pass a
    /// [`Presence`](crate::motion::Presence) the application owns and keeps
    /// supplying the drawer while it reports
    /// [`should_render`](crate::motion::Presence::should_render); the exit is
    /// drawn from that.
    pub fn presence(mut self, presence: &crate::motion::Presence) -> Self {
        self.presence = Some(presence.clone());
        self
    }

    /// Sets the footer, usually a row of buttons.
    pub fn footer(mut self, footer: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Builds the full-area layer: backdrop plus the panel against its edge.
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            title,
            body,
            side,
            size,
            on_dismiss,
            footer,
            presence,
        } = self;

        let title_style = Size::Lg.text();
        let extent = size.extent();

        let mut content = column![text(title)
            .size(title_style.size)
            .line_height(title_style.line_height())]
        .spacing(16)
        .push(body);

        if let Some(footer) = footer {
            content = content.push(
                container(footer)
                    .width(Length::Fill)
                    .align_x(Alignment::End),
            );
        }

        let mut panel = container(content)
            .padding(Padding::new(24.0))
            .class(Box::new(|theme: &Theme| {
                let colors = theme.colors();

                container::Style {
                    background: Some(iced::Background::Color(colors.surface)),
                    border: iced::Border {
                        color: colors.border,
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    shadow: floating_shadow(theme),
                    text_color: Some(colors.surface_foreground),
                    ..container::Style::default()
                }
            }) as container::StyleFn<'a, Theme>);

        // The panel stretches across its cross axis so it reads as an edge
        // panel rather than a floating card.
        panel = if side.is_horizontal() {
            panel.width(Length::Fill).height(Length::Fixed(extent))
        } else {
            panel.width(Length::Fixed(extent)).height(Length::Fill)
        };

        let (align_x, align_y) = match side {
            DrawerSide::Left => (Alignment::Start, Alignment::Start),
            DrawerSide::Right => (Alignment::End, Alignment::Start),
            DrawerSide::Bottom => (Alignment::Start, Alignment::End),
        };

        let positioned = container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(align_x)
            .align_y(align_y);

        let backdrop: Element<'a, Message, Theme> = match on_dismiss {
            Some(message) => MouseArea::new(scrim()).on_press(message).into(),
            None => scrim(),
        };

        // The panel slides in from the edge it is pinned to; the backdrop is
        // part of the same surface, so it arrives with it rather than appearing
        // under a panel that is still on its way.
        let from = match side {
            DrawerSide::Left => EnterFrom::Left,
            DrawerSide::Right => EnterFrom::Right,
            // A drawer along the bottom rises into place; the top edge is not
            // offered, so no direction travels downwards.
            DrawerSide::Bottom => EnterFrom::Below,
        };

        let surface = Enter::new(positioned, from)
            .distance(extent.min(DRAWER_TRAVEL))
            .duration(DURATION_SLOW);

        // With a presence the surface also slides back out, since the presence
        // knows how far through its exit it is.
        let surface = match &presence {
            Some(presence) => surface.presence(presence),
            None => surface,
        };

        iced::widget::stack![backdrop, surface].into()
    }
}

/// Builds a compact drawer header: a title with a close button.
#[must_use]
pub fn drawer_header<'a, Message: Clone + 'a>(
    title: impl text::IntoFragment<'a>,
    on_close: Message,
) -> Element<'a, Message, Theme> {
    let title_style = Size::Lg.text();

    row![
        text(title)
            .size(title_style.size)
            .line_height(title_style.line_height())
            .width(Length::Fill),
        kit_button("✕").ghost().size(Size::Sm).on_press(on_close),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// A row of drawer actions, right-aligned.
pub fn drawer_actions<'a, Message: Clone + 'a>(
    actions: Vec<Button<'a, Message>>,
) -> Element<'a, Message, Theme> {
    let mut footer = row![].spacing(8).align_y(Alignment::Center);

    for action in actions {
        footer = footer.push(action);
    }

    footer.into()
}

/// A spacer that pushes later content to the far side of a drawer.
#[must_use]
pub fn spacer<'a, Message: 'a>() -> Element<'a, Message, Theme> {
    Space::new().width(Length::Fill).height(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::{drawer_actions, drawer_header, spacer, Drawer, DrawerSide, DrawerSize};
    use crate::theme::Theme;
    use crate::widgets::button;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Close,
        Save,
    }

    #[test]
    fn a_drawer_renders_on_every_side_and_size() {
        for side in [DrawerSide::Left, DrawerSide::Right, DrawerSide::Bottom] {
            for size in [DrawerSize::Sm, DrawerSize::Md, DrawerSize::Lg] {
                let element: iced::Element<'_, Message, Theme> =
                    Drawer::new("Details", iced::widget::text("Body"))
                        .side(side)
                        .size(size)
                        .into_element();
                drop(element);
            }
        }
    }

    #[test]
    fn a_drawer_renders_with_a_footer_and_dismissal() {
        let element: iced::Element<'_, Message, Theme> =
            Drawer::new("Details", iced::widget::text("Body"))
                .on_dismiss(Message::Close)
                .footer(drawer_actions(vec![
                    button("Cancel").ghost().on_press(Message::Close),
                    button("Save").primary().on_press(Message::Save),
                ]))
                .into_element();
        drop(element);
    }

    #[test]
    fn a_drawer_without_dismissal_renders_a_plain_backdrop() {
        let element: iced::Element<'_, Message, Theme> =
            Drawer::new("Locked", iced::widget::text("Body")).into_element();
        drop(element);
    }

    #[test]
    fn only_a_bottom_drawer_is_horizontal() {
        assert!(DrawerSide::Bottom.is_horizontal());
        assert!(!DrawerSide::Left.is_horizontal());
        assert!(!DrawerSide::Right.is_horizontal());
    }

    #[test]
    fn drawer_sizes_increase() {
        assert!(DrawerSize::Sm.extent() < DrawerSize::Md.extent());
        assert!(DrawerSize::Md.extent() < DrawerSize::Lg.extent());
    }

    #[test]
    fn a_drawer_header_renders_with_a_close_button() {
        let element: iced::Element<'_, Message, Theme> = drawer_header("Details", Message::Close);
        drop(element);
    }

    #[test]
    fn a_drawer_spacer_renders() {
        let element: iced::Element<'_, Message, Theme> = spacer();
        drop(element);
    }
}
