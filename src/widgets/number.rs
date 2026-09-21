//! Numeric and one-time-code inputs.
//!
//! Both are composites over the field frame in [`crate::widgets::input`]: a spin
//! button is a field with two steppers inside its border, and a one-time-code
//! field is a row of small single-character fields. Building them on the same
//! frame is what keeps their borders, focus rings and disabled states identical
//! to a plain text field's.

use crate::theme::catalog::Corners;
use crate::theme::{Size, Theme};
use crate::widgets::input::{ControlKind, FieldFrame};
use crate::widgets::{icon_button, text_input as field};
use iced::widget::{column, row, text};
use iced::{Alignment, Element, Length, Padding};
use std::ops::RangeInclusive;

/// Builds a numeric field with increment and decrement steppers.
///
/// The steppers clamp to `range`, so the value can never leave it by clicking.
/// Typed input is passed through to `on_input` unparsed, which lets the caller
/// decide how to handle a partially-typed number such as `-` or `1.`.
///
/// ```
/// # use iced_kit::widgets::{number_input, NumberInput};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Changed(f64) }
/// # fn view(port: f64) -> Element<'static, Message, Theme> {
/// number_input("Port", port, 1.0..=65535.0, Message::Changed).into()
/// # }
/// ```
#[must_use = "a NumberInput does nothing unless it is turned into an Element"]
pub struct NumberInput<'a, Message> {
    value: f64,
    range: RangeInclusive<f64>,
    step: f64,
    size: Size,
    width: Option<Length>,
    label: Option<String>,
    disabled: bool,
    prefix: Option<Element<'a, Message, Theme>>,
    suffix: Option<Element<'a, Message, Theme>>,
    on_change: Option<Box<dyn Fn(f64) -> Message + 'a>>,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
}

/// Clamps `value` into `range`, tolerating an inverted or non-finite range.
///
/// `f64::clamp` panics when the range is inverted or contains NaN, and a range
/// assembled from user data can be exactly that, so the check lives here rather
/// than at each call site.
fn clamp_to(value: f64, range: &RangeInclusive<f64>) -> f64 {
    let start = *range.start();
    let end = *range.end();

    if start.is_nan() || end.is_nan() || start > end {
        return value;
    }

    value.clamp(start, end)
}

/// Builds a numeric field.
pub fn number_input<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    value: f64,
    range: RangeInclusive<f64>,
    on_change: impl Fn(f64) -> Message + 'a,
) -> NumberInput<'a, Message> {
    NumberInput {
        value,
        range,
        step: 1.0,
        size: Size::Md,
        width: None,
        label: Some(label.into()),
        disabled: false,
        prefix: None,
        suffix: None,
        on_change: Some(Box::new(on_change)),
        on_input: None,
    }
}

impl<'a, Message: Clone + 'a> NumberInput<'a, Message> {
    /// Sets the amount the steppers add or subtract.
    pub fn step(mut self, step: f64) -> Self {
        // A zero step would make the steppers look functional but do nothing.
        self.step = if step == 0.0 { 1.0 } else { step.abs() };
        self
    }

    /// Sets the field's size.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Sets the field's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Draws a label above the field.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Suppresses the label the constructor took.
    pub fn no_label(mut self) -> Self {
        self.label = None;
        self
    }

    /// Renders the field and its steppers as inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets content shown inside the field, before the number.
    pub fn prefix(mut self, prefix: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Sets content shown inside the field, after the number.
    ///
    /// The steppers always sit at the trailing edge, so a suffix is placed
    /// before them.
    pub fn suffix(mut self, suffix: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Reports the number as typed, before it is parsed.
    ///
    /// A stepper can only offer a parsed value, so a caller that lets the user
    /// type a partial number such as `-` or `1.` needs this as well: the text is
    /// passed through as written and the caller decides what to keep.
    pub fn on_input(mut self, f: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input = Some(Box::new(f));
        self
    }

    /// Clamps a value into the field's range.
    ///
    /// Exposed so a caller can normalise a stored value before handing it back
    /// to the field.
    #[must_use]
    pub fn clamp(&self, value: f64) -> f64 {
        clamp_to(value, &self.range)
    }

    /// Formats a value for display, without a trailing `.0` on whole numbers.
    fn format(value: f64) -> String {
        if value.fract() == 0.0 && value.abs() < 1e15 {
            format!("{value:.0}")
        } else {
            format!("{value}")
        }
    }

    /// Converts the field into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            value,
            range,
            step,
            size,
            width,
            label,
            disabled,
            prefix,
            suffix,
            on_change,
            on_input,
        } = self;

        let value = clamp_to(value, &range);
        let lower = clamp_to(value - step, &range);
        let upper = clamp_to(value + step, &range);

        // A stepper that would not change the value is disabled, so the control
        // stops looking actionable at the ends of its range.
        let can_decrease = lower < value;
        let can_increase = upper > value;

        // The field is taken without its frame: the spin button draws one
        // border around the value and both steppers.
        let mut field = field::<Message>("", &Self::format(value))
            .size(size)
            .disabled(disabled)
            .width(Length::Fill);

        if let Some(on_input) = on_input {
            field = field.on_input(on_input);
        }

        let control = field.into_control();

        let mut frame = FieldFrame::<Message>::new(
            ControlKind::Single,
            control,
            size,
            f32::from(crate::theme::Radius::DEFAULT_MD),
        )
        .disabled(disabled)
        // The steppers are as tall as the field's content, so they sit flush
        // inside the border rather than floating in the middle of it.
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: size.input_padding(),
        });

        if let Some(prefix) = prefix {
            frame = frame.leading(prefix);
        }

        if let Some(suffix) = suffix {
            frame = frame.trailing(suffix);
        }

        // The steppers take over the trailing edge, so the frame's own padding
        // is not applied there.
        // A stepper that would not move the value is left without a message,
        // which is what makes it render inert at the ends of the range.
        let decrement = can_decrease
            .then(|| on_change.as_ref().map(|f| f(lower)))
            .flatten();
        let increment = can_increase
            .then(|| on_change.as_ref().map(|f| f(upper)))
            .flatten();

        frame = frame.trailing(stepper_row(size, disabled, decrement, increment));

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

/// The two steppers, drawn as one joined control on the field's trailing edge.
fn stepper_row<'a, Message: Clone + 'a>(
    size: Size,
    disabled: bool,
    decrement: Option<Message>,
    increment: Option<Message>,
) -> Element<'a, Message, Theme> {
    // The steppers are as tall as the field's content box, so they fill the
    // border rather than sitting inside it with a gap.
    let height = Length::Fixed(size.input_inner_height());
    let width = Length::Fixed(size.height());

    let decrement = icon_button::<Message>()
        .icon("\u{2212}")
        .text()
        .size(size)
        // Only the outer corners follow the frame; the inner edge is square so
        // the two steppers read as one control.
        .corners(Corners {
            top_left: false,
            top_right: false,
            bottom_right: false,
            bottom_left: true,
        })
        .width(width)
        .height(height)
        .padding(Padding::new(0.0))
        .on_press_maybe(if disabled { None } else { decrement });

    let increment = icon_button::<Message>()
        .icon("+")
        .text()
        .size(size)
        .corners(Corners {
            top_left: false,
            top_right: true,
            bottom_right: true,
            bottom_left: false,
        })
        .width(width)
        .height(height)
        .padding(Padding::new(0.0))
        .on_press_maybe(if disabled { None } else { increment });

    row![decrement, increment]
        .spacing(0)
        .align_y(Alignment::Center)
        .into()
}

impl<'a, Message: Clone + 'a> From<NumberInput<'a, Message>> for Element<'a, Message, Theme> {
    fn from(input: NumberInput<'a, Message>) -> Self {
        input.into_element()
    }
}

/// Builds a one-time-code field: `digits` boxes that read as a single control.
///
/// The caller owns the code as a `&str`; each box edits one character of it and
/// reports the whole updated string, so the value never has to be reassembled.
/// Non-digit characters are dropped.
///
/// ```
/// # use iced_kit::widgets::otp_input;
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Changed(String) }
/// # fn view(code: &str) -> Element<'static, Message, Theme> {
/// otp_input(code, 6, Message::Changed).into()
/// # }
/// ```
#[must_use = "an OtpInput does nothing unless it is turned into an Element"]
pub struct OtpInput<'a, Message> {
    value: String,
    digits: usize,
    groups: usize,
    size: Size,
    masked: bool,
    disabled: bool,
    on_change: Option<std::rc::Rc<dyn Fn(String) -> Message + 'a>>,
}

/// Builds a one-time-code field.
pub fn otp_input<'a, Message: Clone + 'a>(
    value: &str,
    digits: usize,
    on_change: impl Fn(String) -> Message + 'a,
) -> OtpInput<'a, Message> {
    OtpInput {
        value: value.to_owned(),
        // A zero-box field would render nothing and look broken.
        digits: digits.max(1),
        groups: 1,
        size: Size::Md,
        masked: false,
        disabled: false,
        on_change: Some(std::rc::Rc::new(on_change)),
    }
}

impl<'a, Message: Clone + 'a> OtpInput<'a, Message> {
    /// Sets the size of the boxes.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Splits the boxes into `groups`, with a wider gap between them.
    ///
    /// A grouped code is easier to read back than an unbroken run, which is why
    /// a six-digit code is usually shown as two groups of three.
    pub fn groups(mut self, groups: usize) -> Self {
        self.groups = groups.max(1);
        self
    }

    /// Hides each digit behind a dot, for a code the user should not read back.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Renders every box as inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Splits `length` boxes into the requested number of groups.
    ///
    /// The count is clamped to the number of boxes: asking for more groups than
    /// digits would otherwise produce empty groups and a ragged row.
    fn resolved_groups(length: usize, requested: usize) -> usize {
        requested.max(1).min(length.max(1))
    }

    /// Converts the field into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            value,
            digits,
            groups,
            size,
            masked,
            disabled,
            on_change,
        } = self;

        let characters: Vec<char> = value.chars().filter(char::is_ascii_digit).collect();
        let groups = Self::resolved_groups(digits, groups);
        let per_group = digits.div_ceil(groups);

        let mut rendered_groups: Vec<Element<'a, Message, Theme>> = Vec::with_capacity(groups);

        for index in 0..digits {
            let character = characters.get(index).copied();
            let shown = match (masked, character) {
                // A masked box shows a dot where a digit is present, so the
                // number of entered digits stays visible.
                (true, Some(_)) => "\u{2022}".to_owned(),
                (false, Some(digit)) => digit.to_string(),
                (_, None) => String::new(),
            };

            let mut field = crate::widgets::text_input::<Message>("", &shown)
                .size(size)
                .disabled(disabled)
                .align_x(iced::alignment::Horizontal::Center)
                .padding(Padding::new(0.0))
                .width(Length::Fixed(size.height() * 1.2));

            if let Some(on_change) = on_change.as_ref().filter(|_| !disabled) {
                let on_change = std::rc::Rc::clone(on_change);
                let current = characters.clone();

                field = field.on_input(move |typed| {
                    // Only the last typed character matters: a box holds one
                    // digit, so whatever else arrived — a paste, an IME commit —
                    // is reduced to the newest digit.
                    let digit = typed.chars().rfind(char::is_ascii_digit);

                    let mut next = current.clone();

                    // The vector has to cover the box that was typed into, even
                    // when the caller's value was shorter than the code.
                    while next.len() <= index {
                        next.push(' ');
                    }

                    match digit {
                        Some(digit) => next[index] = digit,
                        None => next[index] = ' ',
                    }

                    let code: String = next.into_iter().filter(|c| *c != ' ').collect();

                    on_change(code)
                });
            }

            // The next box starts a new group, which is where the gap goes.
            if index % per_group == 0 && index != 0 {
                rendered_groups.push(row![].spacing(0).width(Length::Fixed(8.0)).into());
            }

            rendered_groups.push(field.into_element());
        }

        let fields: Element<'a, Message, Theme> = row(rendered_groups)
            .spacing(6)
            .align_y(Alignment::Center)
            .into();

        column![
            fields,
            text(format!(
                "{}/{} digits",
                characters.len().min(digits),
                digits
            ))
            .size(Size::Sm.text().size)
            .style(|theme: &Theme| text::Style {
                color: Some(theme.colors().muted_foreground),
            })
        ]
        .spacing(6)
        .into()
    }
}

impl<'a, Message: Clone + 'a> From<OtpInput<'a, Message>> for Element<'a, Message, Theme> {
    fn from(input: OtpInput<'a, Message>) -> Self {
        input.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{number_input, otp_input, NumberInput, OtpInput};
    use crate::theme::{Size, Theme};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Changed(f64),
        Code(String),
    }

    #[test]
    fn a_number_input_renders() {
        let element: iced::Element<'_, Message, Theme> =
            number_input("Port", 8080.0, 1.0..=65535.0, Message::Changed).into();
        drop(element);
    }

    #[test]
    fn a_number_input_renders_at_every_size() {
        for size in [Size::Xs, Size::Sm, Size::Md, Size::Lg] {
            let element: iced::Element<'_, Message, Theme> =
                number_input("Port", 80.0, 0.0..=100.0, Message::Changed)
                    .size(size)
                    .into();
            drop(element);
        }
    }

    #[test]
    fn a_number_input_renders_with_every_optional_part() {
        // Each of these adds or removes a child of the frame, which is where an
        // index bug in the focus lookup would show up.
        let element: iced::Element<'_, Message, Theme> =
            number_input("Price", 5.0, 0.0..=10.0, Message::Changed)
                .step(0.5)
                .prefix(iced::widget::text("$"))
                .suffix(iced::widget::text("USD"))
                .width(200.0)
                .disabled(true)
                .into();
        drop(element);
    }

    #[test]
    fn a_number_input_renders_without_a_label() {
        let element: iced::Element<'_, Message, Theme> =
            number_input("Value", 5.0, 0.0..=10.0, Message::Changed)
                .no_label()
                .into();
        drop(element);
    }

    #[test]
    fn a_number_input_clamps_its_value_into_range() {
        let input = number_input("Value", 500.0, 0.0..=100.0, Message::Changed);

        assert_eq!(input.clamp(500.0), 100.0);
        assert_eq!(input.clamp(-5.0), 0.0);
        assert_eq!(input.clamp(50.0), 50.0);
    }

    #[test]
    fn a_number_input_survives_an_inverted_or_nan_range() {
        // A range built from user data can be inverted, and `f64::clamp` panics
        // on that, so the field must not pass it straight through.
        for range in [100.0..=0.0, f64::NAN..=10.0, 0.0..=f64::NAN] {
            let input = number_input("Value", 50.0, range, Message::Changed);
            let _ = input.clamp(50.0);
        }
    }

    #[test]
    fn a_number_input_at_its_bounds_leaves_the_matching_stepper_inert() {
        let input = number_input("Value", 100.0, 0.0..=100.0, Message::Changed);

        assert_eq!(
            input.clamp(100.0 + 1.0),
            100.0,
            "increase is inert at the top"
        );
        assert!(input.clamp(100.0 - 1.0) < 100.0, "decrease still works");
    }

    #[test]
    fn a_number_input_formats_whole_numbers_without_a_decimal_point() {
        assert_eq!(NumberInput::<Message>::format(42.0), "42");
        assert_eq!(NumberInput::<Message>::format(42.5), "42.5");
        assert_eq!(NumberInput::<Message>::format(-7.0), "-7");
    }

    #[test]
    fn a_zero_step_is_replaced() {
        // A zero step would leave the steppers looking active but doing nothing.
        let input = number_input("Value", 5.0, 0.0..=10.0, Message::Changed).step(0.0);
        assert_eq!(input.step, 1.0);
    }

    #[test]
    fn a_negative_step_is_taken_as_a_magnitude() {
        // The direction is the button's job, so a negative step would silently
        // invert the two steppers.
        let input = number_input("Value", 5.0, 0.0..=10.0, Message::Changed).step(-2.0);
        assert_eq!(input.step, 2.0);
    }

    #[test]
    fn a_number_input_takes_typed_input_as_well_as_steps() {
        let element: iced::Element<'_, Message, Theme> =
            number_input("Value", 5.0, 0.0..=10.0, Message::Changed)
                .on_input(|_| Message::Changed(0.0))
                .into();
        drop(element);
    }

    #[test]
    fn an_otp_renders_with_the_requested_number_of_boxes() {
        let input = otp_input("123", 6, Message::Code);
        assert_eq!(input.digits, 6);

        let element: iced::Element<'_, Message, Theme> = input.into();
        drop(element);
    }

    #[test]
    fn an_otp_treats_zero_digits_as_one() {
        // Zero boxes would render an invisible control.
        let input = otp_input("", 0, Message::Code);
        assert_eq!(input.digits, 1);
    }

    #[test]
    fn an_otp_renders_when_the_value_is_longer_than_the_boxes() {
        // Only the leading digits are shown; the overflow must not panic.
        let element: iced::Element<'_, Message, Theme> =
            otp_input("123456789", 4, Message::Code).into();
        drop(element);
    }

    #[test]
    fn an_otp_ignores_non_digits() {
        let element: iced::Element<'_, Message, Theme> =
            otp_input("1a2b-3", 6, Message::Code).into();
        drop(element);
    }

    #[test]
    fn an_otp_renders_masked_and_disabled() {
        let element: iced::Element<'_, Message, Theme> = otp_input("123", 6, Message::Code)
            .masked(true)
            .disabled(true)
            .into();
        drop(element);
    }

    #[test]
    fn an_otp_renders_every_size() {
        for size in [Size::Xs, Size::Sm, Size::Md, Size::Lg] {
            let element: iced::Element<'_, Message, Theme> =
                otp_input("1", 4, Message::Code).size(size).into();
            drop(element);
        }
    }

    #[test]
    fn otp_groups_are_clamped_to_the_number_of_boxes() {
        // More groups than digits would otherwise leave empty groups and a
        // ragged row.
        assert_eq!(OtpInput::<Message>::resolved_groups(6, 0), 1);
        assert_eq!(OtpInput::<Message>::resolved_groups(6, 20), 6);
        assert_eq!(OtpInput::<Message>::resolved_groups(6, 2), 2);
        assert_eq!(OtpInput::<Message>::resolved_groups(0, 0), 1);
    }

    #[test]
    fn an_otp_renders_every_group_count() {
        for groups in 1..=6 {
            let element: iced::Element<'_, Message, Theme> =
                otp_input("1234", 6, Message::Code).groups(groups).into();
            drop(element);
        }
    }
}
