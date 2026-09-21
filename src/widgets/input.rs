//! Text inputs.
//!
//! [`TextInput`] wraps iced's text input so that size, padding and styling come
//! from the [`Theme`]'s tokens. Password entry and text areas live here too,
//! since all three share the same field look.

use crate::theme::{Size, Theme};
use iced::widget::{text as iced_text, text_editor};
use iced::{Element, Length, Padding};

/// Builds a themed single-line text field.
///
/// ```
/// # use iced_kit::widgets::text_input;
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Changed(String) }
/// # fn view(value: &str) -> iced::Element<'_, Message, Theme> {
/// text_input::<Message>("Email", value)
///     .on_input(Message::Changed)
///     .into()
/// # }
/// ```
pub fn text_input<'a, Message: Clone + 'a>(
    placeholder: &str,
    value: &str,
) -> TextInput<'a, Message> {
    TextInput::new(placeholder, value)
}

/// A themed single-line text field.
#[must_use = "a TextInput does nothing unless it is turned into an Element"]
pub struct TextInput<'a, Message> {
    placeholder: String,
    value: String,
    size: Size,
    secure: bool,
    width: Option<Length>,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
}

impl<'a, Message: Clone + 'a> TextInput<'a, Message> {
    /// Creates an empty field with the given placeholder.
    pub fn new(placeholder: &str, value: &str) -> Self {
        Self {
            placeholder: placeholder.to_owned(),
            value: value.to_owned(),
            size: Size::Md,
            secure: false,
            width: None,
            on_input: None,
            on_submit: None,
        }
    }

    /// Sets the field's size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Masks the field's contents, for passwords.
    pub fn password(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Sets the field's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the message to emit on every edit.
    pub fn on_input(mut self, f: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input = Some(Box::new(f));
        self
    }

    /// Sets the message to emit when the field is submitted.
    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    /// Converts the field into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            placeholder,
            value,
            size,
            secure,
            width,
            on_input,
            on_submit,
        } = self;

        let text = size.text();
        let padding = Padding {
            top: 0.0,
            right: size.padding(),
            bottom: 0.0,
            left: size.padding(),
        };

        let mut widget = iced::widget::text_input(placeholder.as_str(), value.as_str())
            .secure(secure)
            .size(text.size)
            .line_height(text.line_height())
            .padding(padding);

        if let Some(width) = width {
            widget = widget.width(width);
        }

        if let Some(on_input) = on_input {
            widget = widget.on_input(on_input);
        }

        if let Some(on_submit) = on_submit {
            widget = widget.on_submit(on_submit);
        }

        widget.into()
    }
}

impl<'a, Message: Clone + 'a> From<TextInput<'a, Message>> for Element<'a, Message, Theme> {
    fn from(input: TextInput<'a, Message>) -> Self {
        input.into_element()
    }
}

/// Builds a themed password field.
pub fn password<'a, Message: Clone + 'a>(placeholder: &str, value: &str) -> TextInput<'a, Message> {
    TextInput::new(placeholder, value).password(true)
}

/// Builds a themed multi-line text area.
///
/// The area is sized by the caller through [`TextArea::height`]; it does not
/// grow with its content.
pub fn text_area<'a, Message: Clone + 'a>(
    placeholder: &str,
    content: &'a text_editor::Content,
) -> TextArea<'a, Message> {
    TextArea {
        placeholder: placeholder.to_owned(),
        content,
        size: Size::Md,
        height: 120.0,
        on_edit: None,
    }
}

/// A themed multi-line text area.
#[must_use = "a TextArea does nothing unless it is turned into an Element"]
pub struct TextArea<'a, Message> {
    placeholder: String,
    content: &'a text_editor::Content,
    size: Size,
    height: f32,
    on_edit: Option<Box<dyn Fn(text_editor::Action) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> TextArea<'a, Message> {
    /// Sets the area's text size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Sets the area's height in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Sets the message to emit when the content changes.
    ///
    /// Without this the area renders but cannot be typed into.
    pub fn on_edit(mut self, f: impl Fn(text_editor::Action) -> Message + 'a) -> Self {
        self.on_edit = Some(Box::new(f));
        self
    }

    /// Converts the area into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            placeholder,
            content,
            size,
            height,
            on_edit,
        } = self;

        let text = size.text();
        let padding = Padding {
            top: size.padding(),
            right: size.padding(),
            bottom: size.padding(),
            left: size.padding(),
        };

        let mut widget = text_editor(content)
            .placeholder(placeholder)
            .size(text.size)
            .padding(padding)
            .height(Length::Fixed(height));

        if let Some(on_edit) = on_edit {
            widget = widget.on_action(on_edit);
        }

        widget.into()
    }
}

impl<'a, Message: Clone + 'a> From<TextArea<'a, Message>> for Element<'a, Message, Theme> {
    fn from(area: TextArea<'a, Message>) -> Self {
        area.into_element()
    }
}

/// A themed, non-editable line of text.
///
/// This is a thin convenience wrapper so callers do not have to import both
/// `iced::widget::text` and this crate's widgets.
pub fn label<'a>(content: impl iced_text::IntoFragment<'a>) -> iced_text::Text<'a, Theme> {
    iced_text(content)
}

#[cfg(test)]
mod tests {
    use super::{TextArea, TextInput};
    use crate::theme::{Size, Theme};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Changed(String),
        Submitted,
    }

    #[test]
    fn an_input_renders_in_every_size() {
        for size in [Size::Sm, Size::Md, Size::Lg] {
            let element: iced::Element<'_, Message, Theme> =
                TextInput::new("Email", "a@b.c").size(size).into();
            drop(element);
        }
    }

    #[test]
    fn a_password_field_renders_and_masks_its_value() {
        let input: TextInput<'_, Message> = TextInput::new("Password", "hunter2").password(true);

        assert!(input.secure);

        let element: iced::Element<'_, Message, Theme> = input.into();
        drop(element);
    }

    #[test]
    fn an_input_without_handlers_still_renders() {
        let element: iced::Element<'_, Message, Theme> =
            TextInput::new("Read only", "value").into();
        drop(element);
    }

    #[test]
    fn builder_methods_are_chainable() {
        let input = TextInput::new("Email", "")
            .size(Size::Lg)
            .width(240)
            .on_input(Message::Changed)
            .on_submit(Message::Submitted);

        assert_eq!(input.size, Size::Lg);
        assert_eq!(input.width, Some(iced::Length::Fixed(240.0)));
        assert!(input.on_input.is_some());
        assert_eq!(input.on_submit, Some(Message::Submitted));
    }

    #[test]
    fn a_text_area_renders_and_accepts_a_custom_height() {
        let content = iced::widget::text_editor::Content::new();
        let area: TextArea<'_, Message> = super::text_area("Notes", &content).height(200.0);

        assert_eq!(area.height, 200.0);

        let element: iced::Element<'_, Message, Theme> = area.into();
        drop(element);
    }
}
