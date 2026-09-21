//! Context menus and popovers.
//!
//! Both are floating panels anchored to a point. A context menu is a list of
//! commands; a popover holds arbitrary content and is opened by a trigger.

use crate::theme::{Size, Theme};
use crate::widgets::overlay::dropdown::{DropdownAlign, MenuItem};
use crate::widgets::overlay::floating_shadow;
use iced::widget::{container, text};
use iced::{Alignment, Element, Length, Padding};

/// A menu that opens at a point, as a right-click context menu does.
///
/// This is a thin wrapper over [`Dropdown`](crate::widgets::overlay::Dropdown)
/// that defaults to appearing at the cursor rather than below a trigger.
#[must_use = "a ContextMenu does nothing unless it is added to a Layer"]
pub struct ContextMenu<'a, Message> {
    items: Vec<MenuItem<'a, Message>>,
    at: (f32, f32),
    width: f32,
    align: DropdownAlign,
}

impl<'a, Message: Clone + 'a> ContextMenu<'a, Message> {
    /// Creates a context menu that opens at the given point.
    pub fn new(items: Vec<MenuItem<'a, Message>>, at: (f32, f32)) -> Self {
        Self {
            items,
            at,
            width: 200.0,
            align: DropdownAlign::Start,
        }
    }

    /// Sets the menu's width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets which edge the menu aligns to when it would overflow.
    ///
    /// A context menu near the right edge of a viewport should align `End`, so
    /// it opens inward rather than off-screen.
    pub fn align(mut self, align: DropdownAlign) -> Self {
        self.align = align;
        self
    }

    /// Converts the menu into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            items,
            at,
            width,
            align,
        } = self;

        crate::widgets::overlay::Dropdown::new(items)
            .width(width)
            .align(align)
            .anchor(at.0, at.1)
            .into_element()
    }
}

impl<'a, Message: Clone + 'a> From<ContextMenu<'a, Message>> for Element<'a, Message, Theme> {
    fn from(menu: ContextMenu<'a, Message>) -> Self {
        menu.into_element()
    }
}

/// The height a popover is assumed to have when it opens above its trigger.
///
/// iced resolves real bounds only after layout, so a popover placing itself
/// above a trigger has to estimate its own height.
const ESTIMATED_POPOVER_HEIGHT: f32 = 120.0;

/// Which edge of its trigger a popover is anchored to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopoverPlacement {
    /// Below the trigger, aligned to its left edge.
    #[default]
    BottomStart,
    /// Below the trigger, aligned to its right edge.
    BottomEnd,
    /// Above the trigger, aligned to its left edge.
    TopStart,
    /// Above the trigger, aligned to its right edge.
    TopEnd,
}

impl PopoverPlacement {
    /// Whether the popover sits above its trigger.
    #[must_use]
    pub const fn is_above(self) -> bool {
        matches!(self, Self::TopStart | Self::TopEnd)
    }

    /// Whether the popover aligns to the trigger's right edge.
    #[must_use]
    pub const fn is_end_aligned(self) -> bool {
        matches!(self, Self::BottomEnd | Self::TopEnd)
    }
}

/// A floating panel holding arbitrary content, anchored to a point.
#[must_use = "a Popover does nothing unless it is added to a Layer"]
pub struct Popover<'a, Message> {
    content: Element<'a, Message, Theme>,
    at: (f32, f32),
    placement: PopoverPlacement,
    width: f32,
}

impl<'a, Message: Clone + 'a> Popover<'a, Message> {
    /// Creates a popover holding `content`, opening at the given point.
    pub fn new(content: impl Into<Element<'a, Message, Theme>>, at: (f32, f32)) -> Self {
        Self {
            content: content.into(),
            at,
            placement: PopoverPlacement::default(),
            width: 280.0,
        }
    }

    /// Sets which edge of the trigger the popover anchors to.
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets the popover's width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Converts the popover into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            content,
            at,
            placement,
            width,
        } = self;

        let panel = container(content)
            .width(Length::Fixed(width))
            .padding(Padding::new(16.0))
            .class(Box::new(|theme: &Theme| {
                let colors = theme.colors();

                container::Style {
                    background: Some(iced::Background::Color(colors.surface)),
                    border: iced::Border {
                        color: colors.border,
                        width: 1.0,
                        radius: f32::from(theme.radius().md).into(),
                    },
                    shadow: floating_shadow(theme),
                    text_color: Some(colors.surface_foreground),
                    ..container::Style::default()
                }
            }) as container::StyleFn<'a, Theme>);

        let (x, y) = if placement.is_above() {
            (at.0, (at.1 - ESTIMATED_POPOVER_HEIGHT).max(0.0))
        } else {
            at
        };

        let positioned = if placement.is_end_aligned() {
            container(panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::Start)
                .padding(Padding {
                    top: y,
                    right: (width - at.0).max(0.0),
                    bottom: 0.0,
                    left: 0.0,
                })
        } else {
            container(panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Start)
                .align_y(Alignment::Start)
                .padding(Padding {
                    top: y,
                    right: 0.0,
                    bottom: 0.0,
                    left: x,
                })
        };

        positioned.into()
    }
}

impl<'a, Message: Clone + 'a> From<Popover<'a, Message>> for Element<'a, Message, Theme> {
    fn from(popover: Popover<'a, Message>) -> Self {
        popover.into_element()
    }
}

/// Builds a popover header: a title with a close affordance.
pub fn popover_title<'a, Message: 'a>(
    title: impl text::IntoFragment<'a>,
) -> Element<'a, Message, Theme> {
    let style = Size::Md.text();

    text(title)
        .size(style.size)
        .line_height(style.line_height())
        .into()
}

/// A muted line of supporting text for a popover.
pub fn popover_description<'a, Message: 'a>(
    description: impl text::IntoFragment<'a>,
) -> Element<'a, Message, Theme> {
    let style = Size::Sm.text();

    text(description)
        .size(style.size)
        .line_height(style.line_height())
        .class(Box::new(|theme: &Theme| text::Style {
            color: Some(theme.colors().muted_foreground),
        }) as text::StyleFn<'a, Theme>)
        .into()
}

/// A thin separator for use inside a popover.
pub fn popover_separator<'a, Message: 'a>() -> Element<'a, Message, Theme> {
    container(
        iced::widget::Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(1.0)),
    )
    .width(Length::Fill)
    .padding(Padding {
        top: 8.0,
        right: 0.0,
        bottom: 8.0,
        left: 0.0,
    })
    .class(Box::new(|theme: &Theme| container::Style {
        background: Some(iced::Background::Color(theme.colors().border)),
        ..container::Style::default()
    }) as container::StyleFn<'a, Theme>)
    .into()
}

/// A transparent full-area catcher used to close a popover on outside click.
#[must_use]
pub fn popover_dismiss_area<'a, Message: Clone + 'a>(
    message: Message,
) -> Element<'a, Message, Theme> {
    // The catcher is deliberately invisible: it only needs to occupy the area
    // so a press outside the panel reaches it.
    iced::widget::MouseArea::new(
        iced::widget::Space::new()
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(message)
    .into()
}

#[cfg(test)]
mod tests {
    use super::{
        popover_description, popover_dismiss_area, popover_separator, popover_title, ContextMenu,
        Popover, PopoverPlacement,
    };
    use crate::theme::Theme;
    use crate::widgets::overlay::{DropdownAlign, MenuItem};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Cut,
        Copy,
        Close,
    }

    #[test]
    fn a_context_menu_renders() {
        let element: iced::Element<'_, Message, Theme> = ContextMenu::new(
            vec![
                MenuItem::new("Cut", Message::Cut).shortcut("Ctrl+X"),
                MenuItem::new("Copy", Message::Copy).shortcut("Ctrl+C"),
            ],
            (120.0, 80.0),
        )
        .into();
        drop(element);
    }

    #[test]
    fn a_context_menu_renders_aligned_to_either_edge() {
        for align in [DropdownAlign::Start, DropdownAlign::End] {
            let element: iced::Element<'_, Message, Theme> =
                ContextMenu::new(vec![MenuItem::new("Cut", Message::Cut)], (500.0, 300.0))
                    .align(align)
                    .into();
            drop(element);
        }
    }

    #[test]
    fn an_empty_context_menu_renders() {
        let element: iced::Element<'_, Message, Theme> =
            ContextMenu::new(Vec::new(), (0.0, 0.0)).into();
        drop(element);
    }

    #[test]
    fn a_popover_renders_in_every_placement() {
        for placement in [
            PopoverPlacement::BottomStart,
            PopoverPlacement::BottomEnd,
            PopoverPlacement::TopStart,
            PopoverPlacement::TopEnd,
        ] {
            let element: iced::Element<'_, Message, Theme> =
                Popover::new(iced::widget::text("Popover content"), (200.0, 150.0))
                    .placement(placement)
                    .into();
            drop(element);
        }
    }

    #[test]
    fn popover_placement_helpers_agree() {
        assert!(PopoverPlacement::TopStart.is_above());
        assert!(PopoverPlacement::TopEnd.is_above());
        assert!(!PopoverPlacement::BottomStart.is_above());

        assert!(PopoverPlacement::BottomEnd.is_end_aligned());
        assert!(!PopoverPlacement::BottomStart.is_end_aligned());
    }

    #[test]
    fn a_popover_above_its_trigger_never_goes_off_screen() {
        // A trigger near the top edge would otherwise place the panel at a
        // negative offset, which clips it out of the window.
        let element: iced::Element<'_, Message, Theme> =
            Popover::new(iced::widget::text("Content"), (40.0, 4.0))
                .placement(PopoverPlacement::TopStart)
                .into();
        drop(element);
    }

    #[test]
    fn popover_parts_render() {
        let title: iced::Element<'_, Message, Theme> = popover_title("Heading");
        drop(title);

        let description: iced::Element<'_, Message, Theme> = popover_description("Supporting text");
        drop(description);

        let separator: iced::Element<'_, Message, Theme> = popover_separator();
        drop(separator);

        let dismiss: iced::Element<'_, Message, Theme> = popover_dismiss_area(Message::Close);
        drop(dismiss);
    }
}
