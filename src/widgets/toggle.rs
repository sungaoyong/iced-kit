//! Selection controls: switches, checkboxes and radio buttons.

use crate::theme::{Size, Theme};
use iced::widget::{checkbox as iced_checkbox, radio as iced_radio, toggler};
use iced::Element;

/// Builds a themed switch (a sliding on/off control).
///
/// The toggle is a composed widget, so this returns an [`Element`] rather than
/// a chainable builder.
///
/// ```
/// # use iced_kit::widgets::switch;
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Toggled(bool) }
/// # fn view(enabled: bool) -> iced::Element<'static, Message, Theme> {
/// switch("Enabled", enabled, Message::Toggled)
/// # }
/// ```
pub fn switch<'a, Message: Clone + 'a>(
    label: impl iced::widget::text::IntoFragment<'a>,
    is_toggled: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message, Theme> {
    toggler(is_toggled)
        .label(label)
        .size(Size::Md.height() * 0.6)
        .on_toggle(on_toggle)
        .into()
}

/// Builds a themed checkbox.
pub fn checkbox<'a, Message: Clone + 'a>(
    label: impl iced::widget::text::IntoFragment<'a>,
    is_checked: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message, Theme> {
    iced_checkbox(is_checked)
        .label(label)
        .size(18)
        .text_size(Size::Md.text().size)
        .spacing(8)
        .on_toggle(on_toggle)
        .into()
}

/// Builds a themed radio button.
///
/// ```
/// # use iced_kit::widgets::radio;
/// # use iced_kit::Theme;
/// # use iced::Length;
/// # #[derive(Clone, Copy, Debug, PartialEq, Eq)] enum Choice { A, B }
/// # #[derive(Clone, Debug)] enum Message { Picked(Choice) }
/// # fn view(current: Choice) -> iced::Element<'static, Message, Theme> {
/// radio("Option A", Choice::A, Some(current), Message::Picked)
///     .width(Length::Fill)
///     .into()
/// # }
/// ```
pub fn radio<'a, Message: Clone + 'a, Value: Copy + Eq + 'a>(
    label: impl Into<String>,
    value: Value,
    selected: Option<Value>,
    on_click: impl Fn(Value) -> Message + 'a,
) -> iced_radio::Radio<'a, Message, Theme> {
    iced_radio(label, value, selected, on_click)
        .size(18)
        .text_size(Size::Md.text().size)
        .spacing(8)
}

/// Builds a themed slider.
///
/// A slider has no intrinsic look of its own beyond its geometry, so this is a
/// thin pass-through to iced's slider; the appearance comes from the
/// application theme, which implements `slider::Catalog`.
///
/// ```
/// # use iced_kit::widgets::slider;
/// # use iced_kit::Theme;
/// # use iced::Length;
/// # #[derive(Clone, Debug)] enum Message { Changed(f32) }
/// # fn view(volume: f32) -> iced::Element<'static, Message, Theme> {
/// slider(0.0..=100.0, volume, Message::Changed)
///     .width(Length::Fill)
///     .into()
/// # }
/// ```
pub fn slider<'a, T, Message: Clone + 'a>(
    range: std::ops::RangeInclusive<T>,
    value: T,
    on_change: impl Fn(T) -> Message + 'a,
) -> iced::widget::Slider<'a, T, Message, Theme>
where
    T: Copy + From<u8> + PartialOrd + 'a,
{
    iced::widget::slider(range, value, on_change)
}

#[cfg(test)]
mod tests {
    use super::{checkbox, radio, slider, switch};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Toggled(bool),
        Picked(u8),
        Slid(f32),
    }

    #[test]
    fn a_switch_renders() {
        let element: iced::Element<'_, Message, Theme> = switch("Enabled", true, Message::Toggled);
        drop(element);
    }

    #[test]
    fn a_checkbox_renders_both_states() {
        for checked in [true, false] {
            let element: iced::Element<'_, Message, Theme> =
                checkbox("Accept", checked, Message::Toggled);
            drop(element);
        }
    }

    #[test]
    fn a_radio_renders_checked_and_unchecked() {
        let checked: iced::Element<'_, Message, Theme> =
            radio("A", 1_u8, Some(1_u8), Message::Picked).into();
        drop(checked);

        let unchecked: iced::Element<'_, Message, Theme> =
            radio("A", 1_u8, Some(2_u8), Message::Picked).into();
        drop(unchecked);
    }

    #[test]
    fn a_slider_renders() {
        let element: iced::Element<'_, Message, Theme> =
            slider(0.0..=100.0, 50.0, Message::Slid).into();
        drop(element);
    }

    #[test]
    fn the_slider_track_thickens_when_hovered() {
        use iced::widget::slider::{Catalog, Status};

        let theme = Theme::light();
        let class = <Theme as Catalog>::default();

        let active = theme.style(&class, Status::Active);
        let hovered = theme.style(&class, Status::Hovered);

        // A track that thickens under the pointer is the affordance that makes
        // a slider feel interactive.
        assert!(hovered.rail.width > active.rail.width);
    }
}
