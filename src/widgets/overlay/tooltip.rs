//! Tooltips.
//!
//! A tooltip annotates the widget it wraps. It appears on hover after a short
//! delay, so brushing the pointer across a toolbar does not flash a trail of
//! labels.

use crate::theme::{Size, Theme};
use crate::widgets::overlay::floating_shadow;
use iced::widget::{container, text, tooltip as iced_tooltip};
use iced::{Color, Element, Length, Padding};

/// Where a tooltip appears relative to its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TooltipPosition {
    /// Above the target.
    #[default]
    Top,
    /// Below the target.
    Bottom,
    /// To the left.
    Left,
    /// To the right.
    Right,
}

impl From<TooltipPosition> for iced_tooltip::Position {
    fn from(position: TooltipPosition) -> Self {
        match position {
            TooltipPosition::Top => Self::Top,
            TooltipPosition::Bottom => Self::Bottom,
            TooltipPosition::Left => Self::Left,
            TooltipPosition::Right => Self::Right,
        }
    }
}

/// Wraps `content` in a tooltip showing `label`.
///
/// ```
/// # use iced_kit::widgets::overlay::tooltip;
/// # use iced_kit::widgets::button;
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Save }
/// # fn view(content: iced::Element<'static, Message, Theme>) -> iced::Element<'static, Message, Theme> {
/// tooltip(content, "Save the document".to_owned())
/// # }
/// ```
pub fn tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message, Theme>>,
    label: String,
) -> Element<'a, Message, Theme> {
    tooltip_at(content, label, TooltipPosition::default())
}

/// Wraps `content` in a tooltip at an explicit position.
pub fn tooltip_at<'a, Message: 'a>(
    content: impl Into<Element<'a, Message, Theme>>,
    label: String,
    position: TooltipPosition,
) -> Element<'a, Message, Theme> {
    iced_tooltip(content, tooltip_bubble(label), position.into())
        .gap(6)
        // 400ms is long enough that a pointer passing over a control does not
        // trigger a tooltip, and short enough that a deliberate hover feels
        // immediate.
        .delay(std::time::Duration::from_millis(400))
        .snap_within_viewport(true)
        .into()
}

/// Builds the tooltip bubble itself, so callers can compose it directly.
#[must_use]
pub fn tooltip_bubble<'a, Message: 'a>(label: String) -> Element<'a, Message, Theme> {
    let style = Size::Sm.text();

    container(
        text(label)
            .size(style.size)
            .line_height(style.line_height()),
    )
    .padding(Padding {
        top: 6.0,
        right: 10.0,
        bottom: 6.0,
        left: 10.0,
    })
    .max_width(280.0)
    .class(Box::new(|theme: &Theme| {
        let colors = theme.colors();

        container::Style {
            // Inverted, so the bubble reads as a layer above the page in both
            // palettes rather than blending into a same-colored surface.
            background: Some(iced::Background::Color(colors.foreground)),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: f32::from(theme.radius().sm).into(),
            },
            shadow: floating_shadow(theme),
            text_color: Some(colors.background),
            ..container::Style::default()
        }
    }) as container::StyleFn<'a, Theme>)
    .width(Length::Shrink)
    .into()
}

#[cfg(test)]
mod tests {
    use super::{tooltip, tooltip_at, tooltip_bubble, TooltipPosition};
    use crate::theme::Theme;
    use crate::widgets::button;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Press,
    }

    #[test]
    fn a_tooltip_renders_around_its_target() {
        let target: iced::Element<'_, Message, Theme> =
            button("Save").primary().on_press(Message::Press).into();

        let element: iced::Element<'_, Message, Theme> =
            tooltip(target, "Save the document".to_owned());
        drop(element);
    }

    #[test]
    fn a_tooltip_renders_at_every_position() {
        for position in [
            TooltipPosition::Top,
            TooltipPosition::Bottom,
            TooltipPosition::Left,
            TooltipPosition::Right,
        ] {
            let target: iced::Element<'_, Message, Theme> = button("Hover").into();
            let element: iced::Element<'_, Message, Theme> =
                tooltip_at(target, "Hint".to_owned(), position);
            drop(element);
        }
    }

    #[test]
    fn a_bare_tooltip_bubble_renders() {
        let element: iced::Element<'_, Message, Theme> = tooltip_bubble("Hint".to_owned());
        drop(element);
    }

    #[test]
    fn tooltip_positions_map_onto_iced_positions() {
        use iced::widget::tooltip::Position;

        assert_eq!(Position::from(TooltipPosition::Top), Position::Top);
        assert_eq!(Position::from(TooltipPosition::Bottom), Position::Bottom);
        assert_eq!(Position::from(TooltipPosition::Left), Position::Left);
        assert_eq!(Position::from(TooltipPosition::Right), Position::Right);
    }
}
