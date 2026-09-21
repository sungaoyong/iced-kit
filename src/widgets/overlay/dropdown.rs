//! Dropdown menus.
//!
//! A dropdown is an anchored popup owning a list of [`MenuItem`]s. It is
//! rendered by [`Layer`](crate::widgets::overlay::Layer), which is what lets it
//! float over the rest of the page.

use crate::theme::{Size, Theme};
use crate::widgets::overlay::floating_shadow;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Color, Element, Length, Padding};

/// Which edge of the trigger the menu aligns to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DropdownAlign {
    /// The menu's left edge lines up with the trigger's.
    #[default]
    Start,
    /// The menu's right edge lines up with the trigger's.
    End,
}

/// One entry in a dropdown.
#[must_use = "a MenuItem does nothing unless it is added to a Dropdown"]
pub struct MenuItem<'a, Message> {
    label: String,
    icon: Option<String>,
    shortcut: Option<String>,
    enabled: bool,
    destructive: bool,
    message: Message,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, Message: Clone + 'a> MenuItem<'a, Message> {
    /// Creates an enabled item.
    pub fn new(label: impl Into<String>, message: Message) -> Self {
        Self {
            label: label.into(),
            icon: None,
            shortcut: None,
            enabled: true,
            destructive: false,
            message,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Adds a leading icon glyph.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Adds a trailing keyboard shortcut hint.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Enables or disables the item. A disabled item renders but emits nothing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Renders the item in the danger color, for a destructive command.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    /// Returns the item's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether the item can be activated.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// A dropdown menu.
#[must_use = "a Dropdown does nothing unless it is added to a Layer"]
pub struct Dropdown<'a, Message> {
    items: Vec<MenuItem<'a, Message>>,
    width: f32,
    align: DropdownAlign,
    /// Placement within the layer, in logical pixels from the layer's origin.
    anchor: (f32, f32),
}

impl<'a, Message: Clone + 'a> Dropdown<'a, Message> {
    /// Creates a dropdown from its items.
    pub fn new(items: Vec<MenuItem<'a, Message>>) -> Self {
        Self {
            items,
            width: 200.0,
            align: DropdownAlign::default(),
            anchor: (0.0, 0.0),
        }
    }

    /// Sets the menu's width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets which edge of the trigger the menu aligns to.
    pub fn align(mut self, align: DropdownAlign) -> Self {
        self.align = align;
        self
    }

    /// Places the menu at a point within the layer.
    ///
    /// The application computes this from the trigger's bounds; iced does not
    /// expose a widget's resolved position to its own view function, so the
    /// anchor has to come from a layout callback in the application.
    pub fn anchor(mut self, x: f32, y: f32) -> Self {
        self.anchor = (x, y);
        self
    }

    /// How many items the menu has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the menu has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Converts the menu into an [`Element`] positioned in its layer.
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            items,
            width,
            align,
            anchor,
        } = self;

        let label_style = Size::Sm.text();
        let mut list = column![].spacing(2);

        for item in items {
            let mut content = row![].spacing(8).align_y(Alignment::Center);

            if let Some(icon) = item.icon {
                content = content.push(text(icon).size(label_style.size));
            }

            content = content.push(
                text(item.label)
                    .size(label_style.size)
                    .line_height(label_style.line_height())
                    .width(Length::Fill),
            );

            if let Some(shortcut) = item.shortcut {
                content = content.push(text(shortcut).size(label_style.size - 1.0).class(
                    Box::new(|theme: &Theme| text::Style {
                        color: Some(theme.colors().muted_foreground),
                    }) as text::StyleFn<'a, Theme>,
                ));
            }

            let enabled = item.enabled;
            let destructive = item.destructive;

            let mut widget = button(content)
                .width(Length::Fill)
                .padding(Padding {
                    top: 6.0,
                    right: 8.0,
                    bottom: 6.0,
                    left: 8.0,
                })
                .class(Box::new(move |theme: &Theme, status| {
                    item_style(theme, status, destructive, enabled)
                }) as button::StyleFn<'a, Theme>);

            if enabled {
                widget = widget.on_press(item.message);
            }

            list = list.push(widget);
        }

        let menu = container(list)
            .width(Length::Fixed(width))
            .padding(Padding::new(4.0))
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

        // The anchor is applied with padding rather than absolute positioning,
        // since iced lays overlays out in flow.
        let (x, y) = anchor;
        let placed = match align {
            DropdownAlign::Start => container(menu)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Start)
                .align_y(Alignment::Start)
                .padding(Padding {
                    top: y,
                    right: 0.0,
                    bottom: 0.0,
                    left: x,
                }),
            DropdownAlign::End => container(menu)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::Start)
                .padding(Padding {
                    top: y,
                    right: x,
                    bottom: 0.0,
                    left: 0.0,
                }),
        };

        placed.into()
    }
}

impl<'a, Message: Clone + 'a> From<Dropdown<'a, Message>> for Element<'a, Message, Theme> {
    fn from(dropdown: Dropdown<'a, Message>) -> Self {
        dropdown.into_element()
    }
}

/// The appearance of one menu item.
fn item_style(
    theme: &Theme,
    status: button::Status,
    destructive: bool,
    enabled: bool,
) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    let base = if destructive {
        colors.destructive
    } else {
        colors.foreground
    };

    button::Style {
        background: (hovered && enabled).then_some(iced::Background::Color(colors.accent)),
        text_color: if enabled {
            base
        } else {
            Color {
                a: colors.muted_foreground.a * 0.6,
                ..colors.muted_foreground
            }
        },
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: f32::from(theme.radius().sm).into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// A separator between groups of menu items.
#[must_use]
pub fn menu_separator<'a, Message: 'a>() -> Element<'a, Message, Theme> {
    container(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .padding(Padding {
            top: 4.0,
            right: 0.0,
            bottom: 4.0,
            left: 0.0,
        })
        .class(Box::new(|theme: &Theme| container::Style {
            background: Some(iced::Background::Color(theme.colors().border)),
            ..container::Style::default()
        }) as container::StyleFn<'a, Theme>)
        .into()
}

#[cfg(test)]
mod tests {
    use super::{menu_separator, Dropdown, DropdownAlign, MenuItem};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Open,
        Delete,
    }

    #[test]
    fn a_dropdown_renders_with_items() {
        let element: iced::Element<'_, Message, Theme> = Dropdown::new(vec![
            MenuItem::new("Open", Message::Open).shortcut("Ctrl+O"),
            MenuItem::new("Delete", Message::Delete)
                .destructive(true)
                .icon("🗑"),
        ])
        .anchor(100.0, 40.0)
        .into();
        drop(element);
    }

    #[test]
    fn a_dropdown_renders_aligned_to_either_edge() {
        for align in [DropdownAlign::Start, DropdownAlign::End] {
            let element: iced::Element<'_, Message, Theme> =
                Dropdown::new(vec![MenuItem::new("Open", Message::Open)])
                    .align(align)
                    .anchor(300.0, 20.0)
                    .into();
            drop(element);
        }
    }

    #[test]
    fn an_empty_dropdown_renders() {
        let dropdown: Dropdown<'_, Message> = Dropdown::new(Vec::new());
        assert!(dropdown.is_empty());
        assert_eq!(dropdown.len(), 0);

        let element: iced::Element<'_, Message, Theme> = dropdown.into();
        drop(element);
    }

    #[test]
    fn a_disabled_item_renders_but_is_not_activatable() {
        let item = MenuItem::new("Unavailable", Message::Open).enabled(false);
        assert!(!item.is_enabled());

        let element: iced::Element<'_, Message, Theme> =
            Dropdown::new(vec![item, MenuItem::new("Open", Message::Open)]).into();
        drop(element);
    }

    #[test]
    fn a_disabled_item_does_not_darken_on_hover() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let hovered = super::item_style(&theme, Status::Hovered, false, false);
        let enabled = super::item_style(&theme, Status::Hovered, false, true);

        assert!(enabled.background.is_some(), "an enabled item highlights");
        assert!(hovered.background.is_none(), "a disabled item must not");
    }

    #[test]
    fn a_destructive_item_uses_the_danger_color() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let destructive = super::item_style(&theme, Status::Active, true, true);
        let normal = super::item_style(&theme, Status::Active, false, true);

        assert_eq!(destructive.text_color, theme.colors().destructive);
        assert_ne!(destructive.text_color, normal.text_color);
    }

    #[test]
    fn a_menu_separator_renders() {
        let element: iced::Element<'_, Message, Theme> = menu_separator();
        drop(element);
    }

    #[test]
    fn a_menu_item_keeps_its_label() {
        let item = MenuItem::new("Open", Message::Open);
        assert_eq!(item.label(), "Open");
    }
}
