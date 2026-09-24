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
    top_inset: f32,
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
            top_inset: 0.0,
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

    /// Insets the panel from the top edge, so it does not cover a title bar.
    ///
    /// This is what makes the panel a sheet rather than a drawer: a sheet is
    /// subordinate to the window's chrome and sits below it, which is how an
    /// editor's side panel behaves.
    pub fn top_inset(mut self, inset: f32) -> Self {
        self.top_inset = inset.max(0.0);
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
            top_inset,
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
            .align_y(align_y)
            .padding(Padding {
                // A sheet stops below the window's chrome; a drawer covers the
                // whole edge because that is what a modal panel does.
                top: top_inset,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            });

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

impl<'a, Message: Clone + 'a> From<Drawer<'a, Message>> for Element<'a, Message, Theme> {
    fn from(drawer: Drawer<'a, Message>) -> Self {
        drawer.into_element()
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

/// The inset a sheet leaves above itself by default, matching the title bar's
/// height.
pub const SHEET_TOP_INSET: f32 = 34.0;

/// Builds a sheet: a panel that slides in from an edge and stops below the
/// window's title bar.
///
/// A sheet is a [`Drawer`] with a top inset, which is the difference between a
/// panel that belongs to a window with chrome and one that covers the whole
/// edge.
///
/// ```
/// # use iced_kit::widgets::overlay::{DrawerSide, DrawerSize};
/// # use iced_kit::widgets::{sheet, muted_text};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Closed }
/// # fn view() -> Element<'static, Message, Theme> {
/// sheet("Settings", muted_text("Panel body"))
///     .side(DrawerSide::Right)
///     .on_dismiss(Message::Closed)
///     .into()
/// # }
/// ```
pub fn sheet<'a, Message: Clone + 'a>(
    title: impl Into<String>,
    body: impl Into<Element<'a, Message, Theme>>,
) -> Drawer<'a, Message> {
    Drawer::new(title, body).top_inset(SHEET_TOP_INSET)
}

/// A popup that opens when the pointer rests on its trigger.
///
/// # How it is put together
///
/// Following the same division of labour as every overlay in this crate, the
/// trigger is built here and the card itself is positioned and drawn by the
/// application through [`Layer`](crate::widgets::overlay::Layer). What makes a
/// hover card different from a popover is the trigger's behaviour: it opens on
/// hover rather than on click.
///
/// The delays are expressed as durations rather than as a clock the component
/// owns: iced components keep no timers, so the application schedules the
/// message. `open_delay` is how long the pointer must rest before the card
/// opens, and `close_delay` how long it may be away before the card closes —
/// the second is what lets the pointer cross the gap to the card itself.
///
/// ```
/// # use iced_kit::widgets::overlay::{HoverCard, HoverCardPlacement};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Opened, Closed }
/// # fn view<'a>(trigger: Element<'a, Message, Theme>) -> Element<'a, Message, Theme> {
/// HoverCard::new(trigger)
///     .on_open(Message::Opened)
///     .on_close(Message::Closed)
///     .into()
/// # }
/// ```
#[must_use = "a HoverCard does nothing unless it is turned into an Element"]
pub struct HoverCard<'a, Message> {
    trigger: Element<'a, Message, Theme>,
    on_open: Option<Message>,
    on_close: Option<Message>,
    open_delay: std::time::Duration,
    close_delay: std::time::Duration,
}

/// Where a hover card's popup is placed relative to its trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HoverCardPlacement {
    /// Below the trigger. The default.
    #[default]
    Bottom,
    /// Above the trigger.
    Top,
    /// To the trigger's left.
    Left,
    /// To the trigger's right.
    Right,
}

impl HoverCardPlacement {
    /// The name this placement is read from a config by.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bottom => "bottom",
            Self::Top => "top",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    /// The placement a name refers to.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "top" => Self::Top,
            "left" => Self::Left,
            "right" => Self::Right,
            _ => Self::Bottom,
        }
    }
}

impl<'a, Message: Clone + 'a> HoverCard<'a, Message> {
    /// Wraps `trigger`.
    pub fn new(trigger: impl Into<Element<'a, Message, Theme>>) -> Self {
        Self {
            trigger: trigger.into(),
            on_open: None,
            on_close: None,
            // Long enough that a pointer passing over the trigger does not open
            // the card, short enough that a deliberate hover feels prompt.
            open_delay: std::time::Duration::from_millis(400),
            close_delay: std::time::Duration::from_millis(150),
        }
    }

    /// Reports that the pointer has rested on the trigger.
    pub fn on_open(mut self, message: Message) -> Self {
        self.on_open = Some(message);
        self
    }

    /// Reports that the pointer has left.
    pub fn on_close(mut self, message: Message) -> Self {
        self.on_close = Some(message);
        self
    }

    /// Sets how long the pointer must rest before the card opens.
    pub fn open_delay(mut self, delay: std::time::Duration) -> Self {
        self.open_delay = delay;
        self
    }

    /// Sets how long the pointer may be away before the card closes.
    pub fn close_delay(mut self, delay: std::time::Duration) -> Self {
        self.close_delay = delay;
        self
    }

    /// The delay before the card opens.
    #[must_use]
    pub fn open_duration(&self) -> std::time::Duration {
        self.open_delay
    }

    /// The delay before the card closes.
    #[must_use]
    pub fn close_duration(&self) -> std::time::Duration {
        self.close_delay
    }

    /// Turns the trigger into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            trigger,
            on_open,
            on_close,
            open_delay: _,
            close_delay: _,
        } = self;

        let area = MouseArea::new(trigger);
        let area = match on_open {
            Some(open) => area.on_enter(open),
            None => area,
        };
        let area = match on_close {
            Some(close) => area.on_exit(close),
            None => area,
        };

        area.into()
    }
}

impl<'a, Message: Clone + 'a> From<HoverCard<'a, Message>> for Element<'a, Message, Theme> {
    fn from(card: HoverCard<'a, Message>) -> Self {
        card.into_element()
    }
}

#[cfg(test)]
mod hover_tests {
    use super::{
        sheet, Drawer, DrawerSide, DrawerSize, HoverCard, HoverCardPlacement, SHEET_TOP_INSET,
    };
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Opened,
        Closed,
    }

    #[test]
    fn a_sheet_is_a_drawer_with_a_top_inset() {
        let sheet: Drawer<'_, Message> = sheet("Settings", iced::widget::text("Body"));
        assert_eq!(sheet.top_inset, SHEET_TOP_INSET);

        // A drawer covers its whole edge by default.
        let drawer: Drawer<'_, Message> = Drawer::new("Panel", iced::widget::text("Body"));
        assert_eq!(drawer.top_inset, 0.0);
    }

    #[test]
    fn sheets_render_on_every_side() {
        for side in [DrawerSide::Left, DrawerSide::Right, DrawerSide::Bottom] {
            let element: iced::Element<'_, Message, Theme> =
                sheet::<Message>("Settings", iced::widget::text("Body"))
                    .side(side)
                    .size(DrawerSize::Lg)
                    .on_dismiss(Message::Closed)
                    .into();
            drop(element);
        }

        let pinned: iced::Element<'_, Message, Theme> =
            sheet::<Message>("Panel", iced::widget::text("Body"))
                .top_inset(60.0)
                .into();
        drop(pinned);
    }

    #[test]
    fn a_hover_card_records_its_delays_and_callbacks() {
        let card: HoverCard<'_, Message> = HoverCard::new(iced::widget::text("Trigger"))
            .on_open(Message::Opened)
            .on_close(Message::Closed)
            .open_delay(std::time::Duration::from_millis(200))
            .close_delay(std::time::Duration::from_millis(50));

        assert!(card.on_open.is_some());
        assert!(card.on_close.is_some());
        assert_eq!(card.open_duration().as_millis(), 200);
        assert_eq!(card.close_duration().as_millis(), 50);
    }

    #[test]
    fn a_hover_card_waits_before_opening_by_default() {
        let card: HoverCard<'_, Message> = HoverCard::new(iced::widget::text("Trigger"));

        assert!(
            card.open_duration() >= std::time::Duration::from_millis(200),
            "a card that opened at once would flash as the pointer crossed it"
        );
        assert!(
            card.close_duration() < card.open_duration(),
            "closing must be quicker than opening, so the pointer can cross the gap"
        );
    }

    #[test]
    fn hover_cards_render_with_any_subset_of_their_callbacks() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            HoverCard::new(iced::widget::text("Trigger")).into(),
            HoverCard::new(iced::widget::text("Trigger"))
                .on_open(Message::Opened)
                .into(),
            HoverCard::new(iced::widget::text("Trigger"))
                .on_close(Message::Closed)
                .into(),
            HoverCard::new(iced::widget::text("Trigger"))
                .on_open(Message::Opened)
                .on_close(Message::Closed)
                .into(),
        ];

        for element in elements {
            drop(element);
        }
    }

    #[test]
    fn a_hover_card_placement_round_trips_through_its_name() {
        for placement in [
            HoverCardPlacement::Bottom,
            HoverCardPlacement::Top,
            HoverCardPlacement::Left,
            HoverCardPlacement::Right,
        ] {
            assert_eq!(HoverCardPlacement::from_name(placement.as_str()), placement);
        }

        assert_eq!(HoverCardPlacement::default(), HoverCardPlacement::Bottom);
        assert_eq!(
            HoverCardPlacement::from_name("nope"),
            HoverCardPlacement::Bottom
        );
        assert_eq!(
            HoverCardPlacement::from_name("TOP"),
            HoverCardPlacement::Top
        );
    }
}
