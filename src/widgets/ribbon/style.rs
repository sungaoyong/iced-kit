//! Ribbon metrics and theme-derived styling.
//!
//! The geometry (row height, icon sizes, button widths) is fixed in logical
//! pixels — a ribbon is a dense, predictable band and its layout does not flex
//! with the palette. The *colors*, by contrast, are resolved from the
//! [`RibbonTheme`] inside iced's style closures, so a ribbon recolours with
//! each of the 10 built-in themes.

use super::theme::RibbonTheme;
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

// ── Quick access bar ──────────────────────────────────────────────────────

/// The height of the quick access bar row.
pub(crate) const QAB_H: f32 = 28.0;

/// The gap between items in the quick access bar.
pub(crate) const QAB_GAP: f32 = 2.0;

/// The icon size in a quick access bar item.
pub(crate) const QAB_ICON: f32 = 16.0;

// ── Gallery ────────────────────────────────────────────────────────────────

/// The gap between items in a gallery grid.
pub(crate) const GALLERY_GAP: f32 = 4.0;

/// The icon size in a gallery item.
pub(crate) const GALLERY_ICON: f32 = 28.0;

/// The width of a gallery item.
pub(crate) const GALLERY_ITEM_W: f32 = 52.0;

/// The height of a gallery item.
pub(crate) const GALLERY_ITEM_H: f32 = 52.0;

/// The type size of a gallery item's label.
pub(crate) const GALLERY_LABEL_SIZE: f32 = 10.0;

/// The style of a ribbon tool button: transparent at rest, a themed hover wash,
/// and a ring while selected.
///
/// The colors come from the [`RibbonTheme`], not the global [`Theme`], so the
/// ribbon can adopt a different visual language (Office 2013, Windows 7, etc.)
/// without affecting the rest of the UI.
pub(crate) fn tool_style<'a>(
    selected: bool,
    ribbon_theme: RibbonTheme,
) -> button::StyleFn<'a, Theme> {
    Box::new(move |theme: &Theme, status| {
        let radius = f32::from(theme.radius().sm).into();
        let rt = ribbon_theme;

        let (background, text_color) = match status {
            button::Status::Disabled => (
                Color::TRANSPARENT,
                Color {
                    a: rt.muted_foreground().a * 0.5,
                    ..rt.muted_foreground()
                },
            ),
            button::Status::Hovered => (rt.button_hover(), rt.tool_text()),
            button::Status::Pressed => (rt.accent(), Color::WHITE),
            button::Status::Active => {
                if selected {
                    (rt.accent(), Color::WHITE)
                } else {
                    (Color::TRANSPARENT, rt.tool_text())
                }
            }
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                color: if selected {
                    rt.accent()
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

/// The style of a gallery item button.
pub(crate) fn gallery_item_style<'a>(
    selected: bool,
    ribbon_theme: RibbonTheme,
) -> button::StyleFn<'a, Theme> {
    Box::new(move |theme: &Theme, status| {
        let radius = f32::from(theme.radius().sm).into();
        let rt = ribbon_theme;

        let (background, text_color) = match status {
            button::Status::Disabled => (
                Color::TRANSPARENT,
                Color {
                    a: rt.muted_foreground().a * 0.5,
                    ..rt.muted_foreground()
                },
            ),
            button::Status::Hovered => (rt.button_hover(), rt.tool_text()),
            button::Status::Pressed => (rt.accent(), Color::WHITE),
            button::Status::Active => {
                if selected {
                    (rt.accent(), Color::WHITE)
                } else {
                    (Color::TRANSPARENT, rt.tool_text())
                }
            }
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                color: if selected {
                    rt.accent()
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

/// The style of the quick access bar: themed background with a bottom rule.
pub(crate) fn qab_style<'a>(ribbon_theme: RibbonTheme) -> container::StyleFn<'a, Theme> {
    Box::new(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(ribbon_theme.tab_bar_background())),
        text_color: Some(ribbon_theme.tool_text()),
        border: Border {
            color: ribbon_theme.border(),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    })
}

/// The style of the ribbon's tool-area band: themed group background with a
/// rule along its bottom edge.
pub(crate) fn band_style<'a>(ribbon_theme: RibbonTheme) -> container::StyleFn<'a, Theme> {
    Box::new(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(ribbon_theme.group_background())),
        text_color: Some(ribbon_theme.tool_text()),
        border: Border {
            color: ribbon_theme.border(),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    })
}

/// The label for a gallery or contextual section.
pub(crate) fn section_label<'a, Message: 'a>(
    title: &str,
    ribbon_theme: RibbonTheme,
) -> Element<'a, Message, Theme> {
    text(title.to_string())
        .size(GROUP_LABEL_SIZE)
        .class(Box::new(move |_theme: &Theme| text::Style {
            color: Some(ribbon_theme.muted_foreground()),
        }) as text::StyleFn<'a, Theme>)
        .into()
}
