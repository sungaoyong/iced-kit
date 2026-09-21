//! A shared frame around one field and its addons.
//!
//! The parts follow `shadcn`'s input group, which is also what `gpui-kit` ports:
//! [`InputGroup`] is the frame, and [`InputGroupAddon`] holds text, icons or
//! buttons on one of its four sides. Inline addons sit beside the control, in the
//! same row; block addons sit above or below it and span the full width.
//!
//! The caller keeps its own value and handler and supplies the control through
//! [`InputGroup::input`]; the group owns only the frame and the addon layout.

use super::frame::{ControlKind, FieldFrame};
use super::{TextArea, TextInput};
use crate::theme::{Size, Theme};
use crate::widgets::{button, icon_button, Button, ButtonVariant, Icon};
use iced::widget::{column, row, text};

/// Multiplies a color's alpha, matching how a disabled control is dimmed.
fn fade(color: iced::Color, factor: f32) -> iced::Color {
    iced::Color {
        a: color.a * factor,
        ..color
    }
}
use iced::{Alignment, Element, Length, Padding};

/// A shared frame around one text control and any number of addons.
///
/// ```
/// # use iced_kit::widgets::{addon, group_button, input_group, text_input, AddonAlignment};
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Changed(String), Search }
/// # fn view(value: &str) -> iced::Element<'_, Message, Theme> {
/// input_group()
///     .input(text_input::<Message>("Query", value).on_input(Message::Changed))
///     .addon(
///         addon()
///             .align(AddonAlignment::InlineEnd)
///             .push(group_button::<Message>("Go").on_press(Message::Search)),
///     )
///     .into()
/// # }
/// ```
#[must_use = "an InputGroup does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct InputGroup<'a, Message> {
    size: Size,
    invalid: bool,
    disabled: bool,
    /// The control and the frame it needs. Held as an enum because a group takes
    /// either a single-line field or a text area, and each has its own builder.
    control: Option<GroupControl<'a, Message>>,
    addons: Vec<InputGroupAddon<'a, Message>>,
    label: Option<String>,
    width: Option<Length>,
}

/// The control a group frames.
///
/// A group takes either a single-line field or a text area; this is the type
/// that lets [`InputGroup::input`] accept both without the caller naming it.
pub enum GroupControl<'a, Message> {
    /// A single-line field.
    Single(TextInput<'a, Message>),
    /// A multi-line field.
    Multi(TextArea<'a, Message>),
}

/// Builds a group frame around the control it is given.
///
/// A group holds no state of its own — the caller keeps the value and the
/// handler — so it needs no identity to key that state on.
pub fn input_group<'a, Message: Clone + 'a>() -> InputGroup<'a, Message> {
    InputGroup {
        size: Size::Md,
        invalid: false,
        disabled: false,
        control: None,
        addons: Vec::new(),
        label: None,
        width: None,
    }
}

impl<'a, Message: Clone + 'a> InputGroup<'a, Message> {
    /// Sets the size every part of the group uses.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Sets the control the group frames.
    pub fn input(mut self, input: impl Into<GroupControl<'a, Message>>) -> Self {
        self.control = Some(input.into());
        self
    }

    /// Appends an addon.
    ///
    /// Addons sharing a side keep the order they were added in.
    pub fn addon(mut self, addon: InputGroupAddon<'a, Message>) -> Self {
        self.addons.push(addon);
        self
    }

    /// Prevents every part of the group from being interacted with.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Marks the group as failing the caller's validation.
    ///
    /// This does not reject any edit; it only reports what the caller decided.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Draws a label above the group.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the group's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Converts the group into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            size,
            invalid,
            disabled,
            control,
            addons,
            label,
            width,
        } = self;

        // A group is assembled by a builder, so a caller can forget the control.
        // Drawing an empty frame would read as a rendering fault, so nothing is
        // rendered at all.
        let Some(control) = control else {
            return text("").into();
        };

        // Resolved before the addons are consumed, since rendering one moves it.
        let has_inline_start = addons
            .iter()
            .any(|a| a.alignment == AddonAlignment::InlineStart);
        let has_inline_end = addons
            .iter()
            .any(|a| a.alignment == AddonAlignment::InlineEnd);

        let mut inline_leading: Vec<Element<'a, Message, Theme>> = Vec::new();
        let mut inline_trailing: Vec<Element<'a, Message, Theme>> = Vec::new();
        let mut block_leading: Vec<Element<'a, Message, Theme>> = Vec::new();
        let mut block_trailing: Vec<Element<'a, Message, Theme>> = Vec::new();

        for addon in addons {
            let alignment = addon.alignment;
            let rendered = addon.into_element(size, disabled);

            match alignment {
                AddonAlignment::InlineStart => inline_leading.push(rendered),
                AddonAlignment::InlineEnd => inline_trailing.push(rendered),
                AddonAlignment::BlockStart => block_leading.push(rendered),
                AddonAlignment::BlockEnd => block_trailing.push(rendered),
            }
        }

        // The control is taken without its own frame: the group draws the one
        // border, and a framed field inside a framing group would draw two.
        let (kind, element) = match control {
            GroupControl::Single(input) => (ControlKind::Single, input.into_control()),
            GroupControl::Multi(area) => (ControlKind::Multi, area.into_control()),
        };

        let mut frame = FieldFrame::new(
            kind,
            element,
            size,
            f32::from(crate::theme::Radius::DEFAULT_MD),
        )
        .invalid(invalid)
        .disabled(disabled)
        // An inline addon takes over the horizontal inset on its side, the way
        // `shadcn` does, so the addon's own padding is not doubled.
        .padding(Padding {
            top: 0.0,
            right: if has_inline_end {
                0.0
            } else {
                size.input_padding()
            },
            bottom: 0.0,
            left: if has_inline_start {
                0.0
            } else {
                size.input_padding()
            },
        });

        for addon in inline_leading {
            frame = frame.leading(addon);
        }

        for addon in inline_trailing {
            frame = frame.trailing(addon);
        }

        for addon in block_leading {
            frame = frame.block_leading(addon);
        }

        for addon in block_trailing {
            frame = frame.block_trailing(addon);
        }

        if let Some(width) = width {
            frame = frame.width(width);
        }

        let field: Element<'a, Message, Theme> = frame.into();

        match label {
            None => field,
            Some(label) => column![
                text(label).size(size.text().size).style(|theme: &Theme| {
                    text::Style {
                        color: Some(theme.colors().foreground),
                    }
                }),
                field,
            ]
            .spacing(6)
            .into(),
        }
    }
}

impl<'a, Message: Clone + 'a> From<TextInput<'a, Message>> for GroupControl<'a, Message> {
    fn from(input: TextInput<'a, Message>) -> Self {
        Self::Single(input)
    }
}

impl<'a, Message: Clone + 'a> From<TextArea<'a, Message>> for GroupControl<'a, Message> {
    fn from(area: TextArea<'a, Message>) -> Self {
        Self::Multi(area)
    }
}

impl<'a, Message: Clone + 'a> From<InputGroup<'a, Message>> for Element<'a, Message, Theme> {
    fn from(group: InputGroup<'a, Message>) -> Self {
        group.into_element()
    }
}

/// The side of the control an addon sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddonAlignment {
    /// Before the control, in the same row.
    #[default]
    InlineStart,
    /// After the control, in the same row.
    InlineEnd,
    /// Above the control, spanning the group's width.
    BlockStart,
    /// Below the control, spanning the group's width.
    BlockEnd,
}

/// Text, an icon, a button or custom content on one side of a group.
#[must_use = "an InputGroupAddon does nothing unless it is given to an InputGroup"]
pub struct InputGroupAddon<'a, Message> {
    alignment: AddonAlignment,
    children: Vec<Element<'a, Message, Theme>>,
}

/// Builds an addon, which defaults to the leading side.
pub fn addon<'a, Message: Clone + 'a>() -> InputGroupAddon<'a, Message> {
    InputGroupAddon {
        alignment: AddonAlignment::InlineStart,
        children: Vec::new(),
    }
}

impl<'a, Message: Clone + 'a> InputGroupAddon<'a, Message> {
    /// Sets which side of the control the addon sits on.
    pub fn align(mut self, alignment: AddonAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Appends content.
    pub fn push(mut self, child: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Appends an icon, sized to the group.
    pub fn icon(self, icon: impl Into<Icon>, size: Size) -> Self {
        self.push(icon.into().into_element(size))
    }

    /// Renders the addon.
    ///
    /// An addon is muted text on the control's own inset, so an icon or a short
    /// unit reads as part of the field rather than as a second control.
    fn into_element(self, size: Size, disabled: bool) -> Element<'a, Message, Theme> {
        let gap = match size {
            Size::Xs => 4.0,
            _ => 6.0,
        };

        // An inline addon has to be as tall as the control or the row would grow
        // around it; a block addon takes the height its content needs.
        let height = match self.alignment {
            AddonAlignment::InlineStart | AddonAlignment::InlineEnd => {
                Length::Fixed(size.input_inner_height())
            }
            AddonAlignment::BlockStart | AddonAlignment::BlockEnd => Length::Shrink,
        };

        let padding = match self.alignment {
            // An inline addon sits flush against the border, since the frame
            // stopped applying its own inset on that side.
            AddonAlignment::InlineStart => Padding {
                top: 0.0,
                right: gap,
                bottom: 0.0,
                left: size.input_padding(),
            },
            AddonAlignment::InlineEnd => Padding {
                top: 0.0,
                right: size.input_padding(),
                bottom: 0.0,
                left: gap,
            },
            // A block addon spans the width, so it lines up with the value.
            AddonAlignment::BlockStart => Padding {
                top: gap,
                right: size.input_padding(),
                bottom: 0.0,
                left: size.input_padding(),
            },
            AddonAlignment::BlockEnd => Padding {
                top: 0.0,
                right: size.input_padding(),
                bottom: gap,
                left: size.input_padding(),
            },
        };

        let mut content = row(self.children)
            .spacing(gap)
            .align_y(Alignment::Center)
            .height(height)
            .padding(padding);

        if matches!(
            self.alignment,
            AddonAlignment::BlockStart | AddonAlignment::BlockEnd
        ) {
            content = content.width(Length::Fill);
        }

        // A disabled group dims its addons along with its control, so the whole
        // box reads as one inert unit instead of a live label on a dead field.
        if disabled {
            return iced::widget::container(content)
                .style(|theme: &Theme| iced::widget::container::Style {
                    text_color: Some(fade(theme.colors().muted_foreground, 0.5)),
                    ..iced::widget::container::Style::default()
                })
                .into();
        }

        content.into()
    }
}

/// A compact button drawn inside a group.
///
/// A group's button is not a field control, so it is ghost-styled and sized to
/// the control's inset rather than to the button scale.
pub fn group_button<'a, Message: Clone + 'a>(
    label: impl iced::widget::text::IntoFragment<'a>,
) -> Button<'a, Message> {
    button(label)
        .variant(ButtonVariant::Ghost)
        .padding(Padding::new(0.0))
}

/// An icon-only button drawn inside a group.
pub fn group_icon_button<'a, Message: Clone + 'a>() -> Button<'a, Message> {
    icon_button::<Message>()
        .variant(ButtonVariant::Ghost)
        .padding(Padding::new(0.0))
}

#[cfg(test)]
mod tests {
    use super::{
        addon, group_button, group_icon_button, input_group, AddonAlignment, GroupControl,
        InputGroup,
    };
    use crate::theme::{Size, Theme};
    use crate::widgets::{text_area, text_input};
    use iced::Length;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Changed(String),
        Go,
    }

    fn field(value: &str) -> crate::widgets::TextInput<'_, Message> {
        text_input::<Message>("Query", value).on_input(Message::Changed)
    }

    #[test]
    fn a_group_renders_with_a_control() {
        let element: iced::Element<'_, Message, Theme> = input_group().input(field("")).into();
        drop(element);
    }

    #[test]
    fn a_group_renders_with_addons_on_every_side() {
        // Each position adds children to the frame's body, which is where the
        // layout would break if one were placed wrongly.
        for alignment in [
            AddonAlignment::InlineStart,
            AddonAlignment::InlineEnd,
            AddonAlignment::BlockStart,
            AddonAlignment::BlockEnd,
        ] {
            let element: iced::Element<'_, Message, Theme> = input_group()
                .input(field("x"))
                .addon(addon().align(alignment).push(iced::widget::text("part")))
                .into();
            drop(element);
        }
    }

    #[test]
    fn a_group_renders_with_addons_on_all_four_sides_at_once() {
        // Inline and block addons together change the frame from a row into a
        // column with the row nested inside, which is the structure the focus
        // lookup has to follow.
        let element: iced::Element<'_, Message, Theme> = input_group()
            .input(field("x"))
            .addon(addon().push(iced::widget::text("$")))
            .addon(
                addon()
                    .align(AddonAlignment::InlineEnd)
                    .push(group_button::<Message>("Go").on_press(Message::Go)),
            )
            .addon(
                addon()
                    .align(AddonAlignment::BlockStart)
                    .push(iced::widget::text("above")),
            )
            .addon(
                addon()
                    .align(AddonAlignment::BlockEnd)
                    .push(iced::widget::text("below")),
            )
            .width(Length::Fixed(320.0))
            .into();
        drop(element);
    }

    #[test]
    fn a_group_renders_around_a_text_area() {
        let content = iced::widget::text_editor::Content::new();
        let element: iced::Element<'_, Message, Theme> = input_group()
            .input(text_area::<Message>("Notes", &content))
            .addon(addon().push(iced::widget::text("Note")))
            .into();
        drop(element);
    }

    #[test]
    fn a_group_renders_at_every_size() {
        for size in [Size::Xs, Size::Sm, Size::Md, Size::Lg] {
            let element: iced::Element<'_, Message, Theme> = input_group()
                .input(field("x"))
                .addon(addon().push(iced::widget::text("$")))
                .size(size)
                .into();
            drop(element);
        }
    }

    #[test]
    fn a_group_renders_disabled_and_invalid() {
        let element: iced::Element<'_, Message, Theme> = input_group()
            .input(field("x"))
            .addon(addon().push(iced::widget::text("$")))
            .addon(
                addon()
                    .align(AddonAlignment::InlineEnd)
                    .push(group_icon_button::<Message>().icon("\u{1f50d}")),
            )
            .disabled(true)
            .invalid(true)
            .into();
        drop(element);
    }

    #[test]
    fn a_group_renders_a_label() {
        let element: iced::Element<'_, Message, Theme> =
            input_group().input(field("x")).label("Search").into();
        drop(element);
    }

    #[test]
    fn a_group_without_a_control_renders_nothing_rather_than_panicking() {
        // A group is assembled by a builder, so a caller can forget the control;
        // an empty box would read as a rendering bug.
        let element: iced::Element<'_, Message, Theme> = input_group().into();
        drop(element);
    }

    #[test]
    fn both_control_kinds_convert_into_the_group_control_type() {
        let content = iced::widget::text_editor::Content::new();

        let _: GroupControl<'_, Message> = field("x").into();
        let _: GroupControl<'_, Message> = text_area::<Message>("Notes", &content).into();
    }

    #[test]
    fn the_builder_is_reachable_from_a_typed_binding() {
        let group: InputGroup<'_, Message> = input_group().input(field("x")).size(Size::Lg);

        assert_eq!(group.size, Size::Lg);
    }
}
