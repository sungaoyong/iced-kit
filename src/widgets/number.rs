//! Numeric and one-time-code inputs.

use crate::theme::{Size, Theme};
use crate::widgets::button as kit_button;
use iced::widget::{column, row, text, text_input as iced_text_input};
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
    on_change: Option<Box<dyn Fn(f64) -> Message + 'a>>,
    label: String,
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
        on_change: Some(Box::new(on_change)),
        label: label.into(),
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
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Sets the field's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
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
            on_change,
            label,
        } = self;

        let text_style = size.text();
        let value = clamp_to(value, &range);
        let lower = clamp_to(value - step, &range);
        let upper = clamp_to(value + step, &range);

        // A stepper that would not change the value is disabled, so the control
        // stops looking actionable at the ends of its range.
        let can_decrease = lower < value;
        let can_increase = upper > value;

        let field = iced_text_input(label.as_str(), &Self::format(value))
            .size(text_style.size)
            .line_height(text_style.line_height())
            .padding(Padding {
                top: 0.0,
                right: size.padding(),
                bottom: 0.0,
                left: size.padding(),
            })
            .width(width.unwrap_or(Length::Fixed(120.0)));

        let steppers = row![
            kit_button("−")
                .variant(crate::widgets::ButtonVariant::Default)
                .size(Size::Sm)
                .on_press_maybe(
                    on_change
                        .as_ref()
                        .and_then(|f| can_decrease.then(|| f(lower)))
                ),
            kit_button("+")
                .variant(crate::widgets::ButtonVariant::Default)
                .size(Size::Sm)
                .on_press_maybe(
                    on_change
                        .as_ref()
                        .and_then(|f| can_increase.then(|| f(upper)))
                ),
        ]
        .spacing(4);

        row![field, steppers]
            .spacing(4)
            .align_y(Alignment::Center)
            .into()
    }
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
    size: Size,
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
        size: Size::Md,
        on_change: Some(std::rc::Rc::new(on_change)),
    }
}

impl<'a, Message: Clone + 'a> OtpInput<'a, Message> {
    /// Sets the size of the boxes.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Converts the field into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            value,
            digits,
            size,
            on_change,
        } = self;

        let text_style = size.text();
        let characters: Vec<char> = value.chars().filter(char::is_ascii_digit).collect();

        let mut fields = row![].spacing(8);

        for index in 0..digits {
            let character = characters.get(index).copied().unwrap_or(' ');

            let mut field = iced_text_input("", &character.to_string())
                .size(text_style.size)
                .line_height(text_style.line_height())
                .align_x(iced::alignment::Horizontal::Center)
                .width(Length::Fixed(size.height() * 1.2))
                .padding(Padding::new(0.0));

            if let Some(on_change) = on_change.as_ref() {
                let on_change = std::rc::Rc::clone(on_change);
                let current = characters.clone();

                field = field.on_input(move |typed| {
                    // Only the last typed character matters: a box holds one
                    // digit, so whatever else arrived (a paste, an IME commit)
                    // is reduced to the newest digit.
                    let digit = typed.chars().rfind(|c: &char| c.is_ascii_digit());

                    let mut next: Vec<char> = current.clone();

                    while next.len() < current.len().max(1) {
                        next.push(' ');
                    }

                    match digit {
                        Some(digit) => {
                            if index < next.len() {
                                next[index] = digit;
                            } else {
                                next.push(digit);
                            }
                        }
                        None => {
                            if index < next.len() {
                                next[index] = ' ';
                            }
                        }
                    }

                    let code: String = next.into_iter().filter(|c| *c != ' ').collect();

                    on_change(code)
                });
            }

            fields = fields.push(field);
        }

        column![
            fields,
            text(format!(
                "{}/{} digits",
                characters.len().min(digits),
                digits
            ))
            .size(Size::Sm.text().size)
            .class(Box::new(|theme: &Theme| text::Style {
                color: Some(theme.colors().muted_foreground),
            }) as text::StyleFn<'a, Theme>)
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
    use super::{number_input, otp_input, NumberInput};
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
        for size in [Size::Sm, Size::Md, Size::Lg] {
            let element: iced::Element<'_, Message, Theme> =
                number_input("Port", 80.0, 0.0..=100.0, Message::Changed)
                    .size(size)
                    .into();
            drop(element);
        }
    }

    #[test]
    fn a_number_input_clamps_its_value_into_range() {
        let input = NumberInput {
            value: 500.0,
            range: 0.0..=100.0,
            step: 1.0,
            size: Size::Md,
            width: None,
            on_change: Some(Box::new(Message::Changed)),
            label: "Value".to_owned(),
        };

        assert_eq!(input.clamp(500.0), 100.0);
        assert_eq!(input.clamp(-5.0), 0.0);
        assert_eq!(input.clamp(50.0), 50.0);
    }

    #[test]
    fn a_number_input_survives_an_inverted_or_nan_range() {
        // A range built from user data can be inverted; `f64::clamp` panics on
        // that, so the field must not pass it straight through.
        for range in [100.0..=0.0, f64::NAN..=10.0, 0.0..=f64::NAN] {
            let input = NumberInput {
                value: 50.0,
                range,
                step: 1.0,
                size: Size::Md,
                width: None,
                on_change: Some(Box::new(Message::Changed)),
                label: "Value".to_owned(),
            };

            let _ = input.clamp(50.0);
        }
    }

    #[test]
    fn a_number_input_at_its_bounds_disables_the_matching_stepper() {
        let input = NumberInput {
            value: 100.0,
            range: 0.0..=100.0,
            step: 1.0,
            size: Size::Md,
            width: None,
            on_change: Some(Box::new(Message::Changed)),
            label: "Value".to_owned(),
        };

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
}
