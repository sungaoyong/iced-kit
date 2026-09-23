//! The supporting line under a dialog title.

use crate::theme::Theme;
use iced::widget::{text, Text};
use iced::{Element, Font};

use super::{DESCRIPTION_LINE_HEIGHT, DESCRIPTION_SIZE};

/// Builds a dialog description.
///
/// The reference's `DialogDescription`: `text_sm` in the theme's muted
/// foreground. A description explains what the title names — which action is
/// about to be taken, what it will affect — so it is deliberately quieter than
/// the title rather than the same weight in a smaller size.
///
/// ```
/// # use iced_kit::widgets::overlay::dialog_description;
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// dialog_description("This action cannot be undone.").into()
/// # }
/// ```
#[must_use]
pub fn dialog_description<'a>(content: impl text::IntoFragment<'a>) -> Text<'a, Theme> {
    text(content)
        .size(DESCRIPTION_SIZE)
        .line_height(iced::Pixels(DESCRIPTION_LINE_HEIGHT))
        .font(Font::DEFAULT)
        .class(Box::new(|theme: &Theme| text::Style {
            color: Some(theme.colors().muted_foreground),
        }) as text::StyleFn<'a, Theme>)
}

/// A dialog description as an element, for a header that mixes it with others.
#[must_use]
pub fn description_element<'a, Message: 'a>(
    content: impl text::IntoFragment<'a>,
) -> Element<'a, Message, Theme> {
    dialog_description(content).into()
}

#[cfg(test)]
mod tests {
    use super::{description_element, dialog_description};
    use crate::theme::Theme;
    use iced::Element;

    #[test]
    fn a_description_renders() {
        let element: Element<'_, (), Theme> = description_element("This cannot be undone.");
        drop(element);
    }

    #[test]
    fn a_description_renders_on_both_palettes() {
        // The muted foreground is the one color the description names itself,
        // so both palettes have to supply it.
        for theme in [Theme::light(), Theme::dark()] {
            let muted = theme.colors().muted_foreground;
            let foreground = theme.colors().foreground;
            assert_ne!(
                muted, foreground,
                "a description must not read as body text"
            );
        }
        drop(dialog_description::<'_>("Body"));
    }
}
