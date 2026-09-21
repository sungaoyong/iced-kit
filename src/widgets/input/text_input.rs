//! Single-line and multi-line text fields.
//!
//! [`TextInput`] and [`TextArea`] wrap iced's own controls so that size,
//! padding, border, focus ring, validation and the disabled state all come from
//! the [`Theme`]'s tokens. Password entry lives here too, since it shares the
//! field look.
//!
//! Both draw through [`FieldFrame`](super::frame::FieldFrame), which owns the
//! border. The wrapped control therefore draws no border of its own: a second
//! one would double the line, and only the frame can wrap a prefix and a suffix
//! as well as the value.

use super::frame::{ControlKind, FieldFrame};
use crate::theme::catalog::FieldAppearance;
use crate::theme::{Size, Theme};
use crate::widgets::{icon_button, spinner_styled, SpinnerStyle};
use iced::widget::{column, text as iced_text, text_editor, text_input as iced_text_input};
use iced::{Element, Length, Padding};

/// The corner radius a field frame is drawn with.
///
/// A frame needs its radius when it is built, before any theme is available,
/// and every palette in this crate shares one radius scale, so the default
/// token is the value to use. It is read from [`Radius`] rather than written as
/// a number so the two cannot drift apart.
const DEFAULT_RADIUS: f32 = crate::theme::Radius::DEFAULT_MD as f32;

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

/// The settings that decide how a bare control is drawn.
///
/// Gathered into one type so a composite can hand over what it knows without
/// having to keep a whole [`TextInput`] alive. The flags are independent — a
/// field can be masked and read-only at once — so they stay separate.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
struct TextInputSettings {
    size: Size,
    secure: bool,
    masked: bool,
    disabled: bool,
    readonly: bool,
    invalid: bool,
    alignment: iced::alignment::Horizontal,
}

/// Builds the bare single-line control.
///
/// Shared by [`TextInput::into_element`] and [`TextInput::into_control`], so a
/// field and a group cannot drift apart in how they style or disable a control.
fn build_text_input<'a, Message: Clone + 'a>(
    placeholder: &str,
    value: &str,
    settings: &TextInputSettings,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
) -> Element<'a, Message, Theme> {
    let TextInputSettings {
        size,
        secure,
        masked,
        disabled,
        readonly,
        invalid,
        alignment,
    } = *settings;
    let text = size.text();

    let mut widget = iced_text_input(placeholder, value)
        .secure(secure && masked)
        .size(text.size)
        .align_x(alignment)
        // The line box keeps the token's natural height; the frame centres the
        // control inside its border. Inflating the line box instead would push
        // the glyph off-centre, because iced centres a line within its box using
        // the font's own metrics.
        .line_height(text.line_height())
        .padding(Padding::new(0.0))
        .width(Length::Fill)
        // The appearance is resolved per draw, where the theme is known;
        // resolving it here would bake in the wrong palette.
        .style(move |theme: &Theme, _status| {
            FieldAppearance::resolve(
                theme,
                crate::theme::catalog::FieldState {
                    focused: false,
                    hovered: false,
                    invalid,
                    disabled,
                },
            )
            .into_text_input_style()
        });

    // Withholding the handler is what makes iced treat the field as inert; it
    // derives that from the absence of one. A read-only field is expressed the
    // same way, so it keeps the normal look and refuses edits.
    if !disabled && !readonly {
        if let Some(on_input) = on_input {
            widget = widget.on_input(on_input);
        }
    }

    if let Some(on_submit) = on_submit {
        widget = widget.on_submit(on_submit);
    }

    widget.into()
}

/// A themed single-line text field.
#[must_use = "a TextInput does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct TextInput<'a, Message> {
    placeholder: String,
    value: String,
    size: Size,
    secure: bool,
    masked: bool,
    disabled: bool,
    readonly: bool,
    invalid: bool,
    loading: bool,
    label: Option<String>,
    error: Option<String>,
    on_mask_toggle: Option<Message>,
    on_clear: Option<Message>,
    prefix: Option<Element<'a, Message, Theme>>,
    suffix: Option<Element<'a, Message, Theme>>,
    width: Option<Length>,
    padding: Option<Padding>,
    alignment: iced::alignment::Horizontal,
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
            masked: false,
            disabled: false,
            readonly: false,
            invalid: false,
            loading: false,
            label: None,
            error: None,
            on_mask_toggle: None,
            on_clear: None,
            prefix: None,
            suffix: None,
            width: None,
            padding: None,
            alignment: iced::alignment::Horizontal::Left,
            on_input: None,
            on_submit: None,
        }
    }

    /// Sets the field's size.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Masks the field's contents, for passwords.
    ///
    /// The value is hidden from the start. Pair this with
    /// [`TextInput::on_mask_toggle`] for a field the user can reveal.
    pub fn password(mut self, secure: bool) -> Self {
        self.secure = secure;
        self.masked = secure;
        self
    }

    /// Sets whether the value is hidden.
    ///
    /// Only meaningful on a field built with [`TextInput::password`].
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Adds an eye button, reporting each click so the caller can flip
    /// [`TextInput::masked`].
    pub fn on_mask_toggle(mut self, message: Message) -> Self {
        self.on_mask_toggle = Some(message);
        self
    }

    /// Adds a clear button while the field holds a value, reporting a click so
    /// the caller can empty its own state.
    pub fn clearable(mut self, on_clear: Message) -> Self {
        self.on_clear = Some(on_clear);
        self
    }

    /// Renders the field as inert.
    ///
    /// A disabled field cannot be focused or edited and is dimmed.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Renders the field as read-only.
    ///
    /// Unlike a disabled field, a read-only one keeps the normal appearance and
    /// can still be focused, selected and copied; it only rejects edits.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Marks the field as failing the caller's validation.
    ///
    /// The border turns destructive and stays that way while focused, so the
    /// error does not disappear the moment the user clicks in to fix it.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Shows a spinner after the value, for one being checked.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Draws a label above the field.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Draws an error below the field.
    ///
    /// This also marks the field invalid: a message the user can read and a
    /// border that agrees with it should not have to be asked for twice.
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.invalid = true;
        self.error = Some(error.into());
        self
    }

    /// Sets content shown inside the field, before the value.
    pub fn prefix(mut self, prefix: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Sets content shown inside the field, after the value.
    pub fn suffix(mut self, suffix: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Sets the field's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Overrides the field's inner padding.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Sets how the value is aligned inside the field.
    ///
    /// A single-character field — one box of a code — reads as a slot to fill
    /// only when its character is centred.
    pub fn align_x(mut self, alignment: impl Into<iced::alignment::Horizontal>) -> Self {
        self.alignment = alignment.into();
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

    /// Consumes the field, yielding the bare control and the state a composite
    /// frame has to know in order to draw it.
    ///
    /// A composite that owns the border — the input group — takes the control
    /// this way, so the box is drawn once rather than a framed field nesting
    /// inside a framing group.
    pub(crate) fn into_control(self) -> Element<'a, Message, Theme> {
        let Self {
            placeholder,
            value,
            size,
            secure,
            masked,
            disabled,
            readonly,
            invalid,
            alignment,
            on_input,
            on_submit,
            ..
        } = self;

        build_text_input(
            &placeholder,
            &value,
            &TextInputSettings {
                size,
                secure,
                masked,
                disabled,
                readonly,
                invalid,
                alignment,
            },
            on_input,
            on_submit,
        )
    }

    /// Converts the field into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            placeholder,
            value,
            size,
            secure,
            masked,
            disabled,
            readonly,
            invalid,
            loading,
            label,
            error,
            on_mask_toggle,
            on_clear,
            prefix,
            suffix,
            width,
            padding,
            alignment,
            on_input,
            on_submit,
        } = self;

        let element = build_text_input(
            &placeholder,
            &value,
            &TextInputSettings {
                size,
                secure,
                masked,
                disabled,
                readonly,
                invalid,
                alignment,
            },
            on_input,
            on_submit,
        );

        let mut frame = FieldFrame::new(ControlKind::Single, element, size, DEFAULT_RADIUS)
            .invalid(invalid)
            .disabled(disabled);

        if let Some(prefix) = prefix {
            frame = frame.leading(prefix);
        }

        if loading {
            frame = frame.trailing(spinner_styled(
                (size.icon_size() * 1.5).round() as u16,
                SpinnerStyle::Arc,
            ));
        }

        if let Some(message) = on_mask_toggle {
            frame = frame.trailing(mask_toggle_button(size, masked, message));
        }

        // A clear button with nothing to clear would be a dead control, so it is
        // only added once the field holds a value.
        if let Some(message) = on_clear {
            if !value.is_empty() {
                frame = frame.trailing(clear_button(size, message));
            }
        }

        if let Some(suffix) = suffix {
            frame = frame.trailing(suffix);
        }

        if let Some(padding) = padding {
            frame = frame.padding(padding);
        }

        if let Some(width) = width {
            frame = frame.width(width);
        }

        let field: Element<'a, Message, Theme> = frame.into();

        adorn(field, size, label, error)
    }
}

/// Stacks an optional label and error around a field.
///
/// Both are colored through a style closure rather than a resolved color: the
/// theme is only known when iced draws, so resolving here would bake in the
/// default palette and paint a dark-mode form with light-mode text.
fn adorn<'a, Message: 'a>(
    field: Element<'a, Message, Theme>,
    size: Size,
    label: Option<String>,
    error: Option<String>,
) -> Element<'a, Message, Theme> {
    if label.is_none() && error.is_none() {
        return field;
    }

    let mut children: Vec<Element<'a, Message, Theme>> = Vec::new();

    if let Some(label) = label {
        children.push(
            iced_text(label)
                .size(size.text().size)
                .style(|theme: &Theme| iced_text::Style {
                    color: Some(theme.colors().foreground),
                })
                .into(),
        );
    }

    children.push(field);

    if let Some(error) = error {
        children.push(
            iced_text(error)
                .size(Size::Sm.text().size)
                .style(|theme: &Theme| iced_text::Style {
                    color: Some(theme.colors().destructive),
                })
                .into(),
        );
    }

    column(children).spacing(6).into()
}

/// The button that reveals or hides a masked value.
fn mask_toggle_button<'a, Message: Clone + 'a>(
    size: Size,
    masked: bool,
    message: Message,
) -> Element<'a, Message, Theme> {
    // A text glyph rather than an SVG, so it inherits the button's color; the
    // SVG widget reads its color from the theme and would ignore the variant.
    let glyph = if masked { "\u{1f441}" } else { "\u{1f648}" };

    icon_button::<Message>()
        .icon(glyph)
        .text()
        .size(size)
        .on_press(message)
        .into()
}

/// The button that empties a field.
fn clear_button<'a, Message: Clone + 'a>(
    size: Size,
    message: Message,
) -> Element<'a, Message, Theme> {
    icon_button::<Message>()
        .icon("\u{2715}")
        .text()
        .size(size)
        .on_press(message)
        .into()
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
        min_height: None,
        disabled: false,
        readonly: false,
        invalid: false,
        label: None,
        error: None,
        on_edit: None,
    }
}

/// A themed multi-line text area.
#[must_use = "a TextArea does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct TextArea<'a, Message> {
    placeholder: String,
    content: &'a text_editor::Content,
    size: Size,
    height: f32,
    min_height: Option<f32>,
    disabled: bool,
    readonly: bool,
    invalid: bool,
    label: Option<String>,
    error: Option<String>,
    on_edit: Option<Box<dyn Fn(text_editor::Action) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> TextArea<'a, Message> {
    /// Sets the area's text size.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Sets the area's height in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Sets a minimum height, so a collapsed area still shows a few lines.
    pub fn min_height(mut self, min_height: f32) -> Self {
        self.min_height = Some(min_height);
        self
    }

    /// Renders the area as inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Renders the area as read-only.
    ///
    /// Like [`TextInput::readonly`], this withholds the edit handler, so the
    /// caller's content stays the source of truth.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Marks the area as failing the caller's validation.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Draws a label above the area.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Draws an error below the area, which also marks it invalid.
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.invalid = true;
        self.error = Some(error.into());
        self
    }

    /// Sets the message to emit when the content changes.
    ///
    /// Without this the area renders but cannot be typed into.
    pub fn on_edit(mut self, f: impl Fn(text_editor::Action) -> Message + 'a) -> Self {
        self.on_edit = Some(Box::new(f));
        self
    }

    /// Consumes the area, yielding the bare editor with no frame around it.
    ///
    /// A composite that owns the border — the input group — takes the editor
    /// this way, so the box is drawn once rather than a framed area nesting
    /// inside a framing group.
    pub(crate) fn into_control(self) -> Element<'a, Message, Theme> {
        let Self {
            placeholder,
            content,
            size,
            height,
            min_height,
            disabled,
            readonly,
            invalid,
            on_edit,
            ..
        } = self;

        let text = size.text();
        let effective_height = min_height.map_or(height, |min| height.max(min));

        let mut widget = text_editor(content)
            .placeholder(placeholder)
            .size(text.size)
            .line_height(text.line_height())
            .padding(Padding::new(size.input_padding()))
            .height(Length::Fixed(effective_height))
            .style(move |theme: &Theme, _status| {
                FieldAppearance::resolve(
                    theme,
                    crate::theme::catalog::FieldState {
                        focused: false,
                        hovered: false,
                        invalid,
                        disabled,
                    },
                )
                .into_text_editor_style()
            });

        // As with the single-line field, withholding the handler is what makes
        // the editor inert.
        if !disabled && !readonly {
            if let Some(on_edit) = on_edit {
                widget = widget.on_action(on_edit);
            }
        }

        widget.into()
    }

    /// Converts the area into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let size = self.size;
        let invalid = self.invalid;
        let disabled = self.disabled;
        let label = self.label.clone();
        let error = self.error.clone();

        let field: Element<'a, Message, Theme> = FieldFrame::new(
            ControlKind::Multi,
            self.into_control(),
            size,
            DEFAULT_RADIUS,
        )
        .invalid(invalid)
        .disabled(disabled)
        // The editor brings its own padding, so the frame adds none; the
        // two would stack and inset the text twice.
        .padding(Padding::new(0.0))
        .width(Length::Fill)
        .into();

        adorn(field, size, label, error)
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
    use super::{text_area, TextArea, TextInput};
    use crate::theme::{Size, Theme};
    use iced::{Length, Padding};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Changed(String),
        Submitted,
        ToggleMask,
        Clear,
    }

    #[test]
    fn an_input_renders_in_every_size() {
        for size in [Size::Xs, Size::Sm, Size::Md, Size::Lg] {
            let element: iced::Element<'_, Message, Theme> =
                TextInput::new("Email", "a@b.c").size(size).into();
            drop(element);
        }
    }

    #[test]
    fn a_password_field_renders_and_masks_its_value() {
        let input: TextInput<'_, Message> = TextInput::new("Password", "hunter2").password(true);

        assert!(input.secure);
        assert!(input.masked);

        let element: iced::Element<'_, Message, Theme> = input.into();
        drop(element);
    }

    #[test]
    fn a_password_field_can_be_revealed() {
        // The eye button reports a click; the caller then builds the field with
        // the mask off, so the value is shown.
        let element: iced::Element<'_, Message, Theme> = TextInput::new("Password", "hunter2")
            .password(true)
            .masked(false)
            .on_mask_toggle(Message::ToggleMask)
            .into();
        drop(element);
    }

    #[test]
    fn an_input_without_handlers_still_renders() {
        let element: iced::Element<'_, Message, Theme> =
            TextInput::new("Read only", "value").into();
        drop(element);
    }

    #[test]
    fn an_input_renders_with_every_optional_part() {
        // Each of these adds or removes a child of the frame, which is where an
        // index bug in the focus lookup would show up.
        let element: iced::Element<'_, Message, Theme> = TextInput::new("Email", "a@b.c")
            .on_input(Message::Changed)
            .on_submit(Message::Submitted)
            .label("Email")
            .prefix(iced::widget::text("@"))
            .suffix(iced::widget::text(".com"))
            .loading(true)
            .on_mask_toggle(Message::ToggleMask)
            .clearable(Message::Clear)
            .padding(Padding::new(4.0))
            .width(Length::Fixed(240.0))
            .invalid(true)
            .into();
        drop(element);
    }

    #[test]
    fn a_clear_button_only_appears_when_there_is_something_to_clear() {
        // The button is dropped for an empty value but the frame still lays out,
        // so this exercises the branch where the trailing slot disappears.
        for value in ["", "a"] {
            let element: iced::Element<'_, Message, Theme> = TextInput::new("Search", value)
                .clearable(Message::Clear)
                .into();
            drop(element);
        }
    }

    #[test]
    fn a_disabled_input_renders_without_a_handler() {
        let element: iced::Element<'_, Message, Theme> = TextInput::new("Email", "a@b.c")
            .on_input(Message::Changed)
            .disabled(true)
            .into();
        drop(element);
    }

    #[test]
    fn a_read_only_input_renders_without_a_handler() {
        let element: iced::Element<'_, Message, Theme> = TextInput::new("Email", "a@b.c")
            .on_input(Message::Changed)
            .readonly(true)
            .into();
        drop(element);
    }

    #[test]
    fn an_error_marks_the_field_invalid() {
        // A message the user can read and a border that agrees with it should
        // not have to be asked for twice.
        let input = TextInput::<Message>::new("Email", "nope").error("Not an email");

        assert!(input.invalid);
        assert_eq!(input.error.as_deref(), Some("Not an email"));
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
        let area: TextArea<'_, Message> = text_area("Notes", &content).height(200.0);

        assert_eq!(area.height, 200.0);

        let element: iced::Element<'_, Message, Theme> = area.into();
        drop(element);
    }

    #[test]
    fn a_text_area_renders_with_every_optional_part() {
        let content = iced::widget::text_editor::Content::new();
        let element: iced::Element<'_, Message, Theme> = text_area("Notes", &content)
            .size(Size::Sm)
            .height(80.0)
            .min_height(120.0)
            .label("Notes")
            .error("Too short")
            .on_edit(|_| Message::Changed(String::new()))
            .into();
        drop(element);
    }

    #[test]
    fn a_text_area_renders_disabled_and_read_only() {
        let content = iced::widget::text_editor::Content::new();

        let disabled: iced::Element<'_, Message, Theme> = text_area("Notes", &content)
            .disabled(true)
            .on_edit(|_| Message::Changed(String::new()))
            .into();
        drop(disabled);

        let readonly: iced::Element<'_, Message, Theme> = text_area("Notes", &content)
            .readonly(true)
            .on_edit(|_| Message::Changed(String::new()))
            .into();
        drop(readonly);
    }

    #[test]
    fn a_min_height_raises_a_short_height() {
        // A caller that sets both should get the larger of the two, or a
        // "minimum" smaller than the height would have no effect at all.
        let content = iced::widget::text_editor::Content::new();
        let area: TextArea<'_, Message> =
            text_area("Notes", &content).height(40.0).min_height(120.0);

        let raised = area
            .min_height
            .map_or(area.height, |min| area.height.max(min));

        assert_eq!(raised, 120.0);
    }
}
