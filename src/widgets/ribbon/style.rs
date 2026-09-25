//! Ribbon metrics and theme-derived styling.
//!
//! The geometry (row height, icon sizes, button widths) is fixed in logical
//! pixels — a ribbon is a dense, predictable band and its layout does not flex
//! with the palette. The *colors*, by contrast, are resolved from the [`Theme`]
//! inside iced's style closures, so a ribbon recolours with light/dark and with
//! any custom token set, with no ribbon-specific palette code.

use crate::theme::Theme;
use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Element, Padding, Shadow};

/// The height of one tool row, in logical pixels.
pub(crate) const ROW_H: f32 = 24.0;

/// The side of the icon in a large, full-height button.
pub(crate) const LARGE_ICON: f32 = 32.0;

/// The side of the icon in a single-row button.
pub(crate) const SMALL_ICON: f32 = 16.0;

/// The width of a large button.
pub(crate) const LARGE_W: f32 = 56.0;

/// The width of a small, icon-only button — square, so a column lines up.
pub(crate) const SMALL_W: f32 = ROW_H;

/// The type size of a group's bottom label.
const GROUP_LABEL_SIZE: f32 = 11.0;

/// The total height of the tool area: three rows, their vertical padding, and
/// the group-label line beneath them.
pub(crate) const TOOL_BAR_H: f32 = 3.0 * ROW_H + 20.0;

/// The padding around a single group.
pub(crate) const GROUP_PADDING: Padding = Padding {
    top: 3.0,
    right: 4.0,
    bottom: 1.0,
    left: 4.0,
};

/// The gap between buttons within a group's row or column.
pub(crate) const BUTTON_GAP: f32 = 2.0;

/// The style of a ribbon tool button: transparent at rest, an accent wash on
/// hover, and a ring while selected.
///
/// A tool is deliberately quieter than a form [`Button`](crate::widgets::Button):
/// a ribbon packs dozens of them, so the resting face is bare and only the
/// states a pointer or a toggle produces are painted.
pub(crate) fn tool_style<'a>(selected: bool) -> button::StyleFn<'a, Theme> {
    Box::new(move |theme: &Theme, status| {
        let colors = theme.colors();
        let radius = f32::from(theme.radius().sm).into();

        let (background, text_color) = match status {
            button::Status::Disabled => (
                Color::TRANSPARENT,
                Color {
                    a: colors.muted_foreground.a * 0.5,
                    ..colors.muted_foreground
                },
            ),
            button::Status::Hovered => (colors.accent, colors.accent_foreground),
            button::Status::Pressed => (colors.primary, colors.primary_foreground),
            button::Status::Active => {
                if selected {
                    (colors.accent, colors.accent_foreground)
                } else {
                    (Color::TRANSPARENT, colors.foreground)
                }
            }
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                // A selected tool draws a ring rather than only a fill: a
                // background shift alone is too subtle to read as "on" in a
                // dense band of identical buttons.
                color: if selected {
                    colors.ring
                } else {
                    Color::TRANSPARENT
                },
                width: if selected { 1.0 } else { 0.0 },
                radius,
            },
            shadow: Shadow::default(),
            snap: false,
        }
    })
}

/// The style of the ribbon's tool-area band: the sidebar surface with a rule
/// along its bottom edge separating it from the page.
pub(crate) fn band_style<'a>() -> container::StyleFn<'a, Theme> {
    Box::new(|theme: &Theme| container::Style {
        background: Some(Background::Color(theme.colors().sidebar)),
        text_color: Some(theme.colors().sidebar_foreground),
        border: Border {
            color: theme.colors().sidebar_border,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    })
}

/// The muted, small label centred under a group.
pub(crate) fn group_label<'a, Message: 'a>(title: &str) -> Element<'a, Message, Theme> {
    text(title.to_string())
        .size(GROUP_LABEL_SIZE)
        .class(Box::new(|theme: &Theme| text::Style {
            color: Some(theme.colors().muted_foreground),
        }) as text::StyleFn<'a, Theme>)
        .into()
}
