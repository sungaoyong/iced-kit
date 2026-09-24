//! Color pickers: choosing a color from a saturation/value square, a hue strip
//! and a set of swatches.
//!
//! # How it is put together
//!
//! A color picker is the one component where a popup is genuinely required: the
//! square, the hue strip and the swatches do not fit in a form column. Following
//! the same division of labour as every other overlay in this crate,
//! [`color_picker`] builds the trigger — a swatch showing the current color —
//! and [`ColorPickerPanel`] builds the choosing surface, which the application
//! positions and draws through [`Layer`](crate::widgets::overlay::Layer).
//!
//! # Color space
//!
//! The picker works in HSV, converted to and from iced's RGB. HSV is what makes
//! the surface a rectangle: hue runs along one edge, saturation and value across
//! the other, so every point in the square is a color and every color is a point
//! in the square.

use crate::theme::{Size, Theme};
use crate::widgets::overlay::floating_shadow;
use iced::widget::canvas::{Frame, Geometry, Path, Stroke};
use iced::widget::{button, canvas, column, container, row, text};
use iced::{
    Alignment, Color, Element, Length, Padding, Point, Rectangle, Renderer, Size as IcedSize,
};

/// A color in hue, saturation and value.
///
/// Hue is in degrees, `0.0..360.0`; saturation and value are `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsv {
    /// The hue, in degrees.
    pub hue: f32,
    /// How saturated the color is.
    pub saturation: f32,
    /// How bright the color is.
    pub value: f32,
}

impl Hsv {
    /// Creates a color from its parts, clamping each to its range.
    #[must_use]
    pub fn new(hue: f32, saturation: f32, value: f32) -> Self {
        Self {
            hue: hue.rem_euclid(360.0),
            saturation: saturation.clamp(0.0, 1.0),
            value: value.clamp(0.0, 1.0),
        }
    }

    /// The color as RGB.
    #[must_use]
    pub fn to_color(self) -> Color {
        let hue = self.hue.rem_euclid(360.0) / 60.0;
        let saturation = self.saturation.clamp(0.0, 1.0);
        let value = self.value.clamp(0.0, 1.0);

        let chroma = value * saturation;
        let second = chroma * (1.0 - (hue % 2.0 - 1.0).abs());
        let offset = value - chroma;

        let (r, g, b) = match hue as u32 {
            0 => (chroma, second, 0.0),
            1 => (second, chroma, 0.0),
            2 => (0.0, chroma, second),
            3 => (0.0, second, chroma),
            4 => (second, 0.0, chroma),
            _ => (chroma, 0.0, second),
        };

        Color::from_rgb(r + offset, g + offset, b + offset)
    }

    /// The color read as HSV.
    #[must_use]
    pub fn from_color(color: Color) -> Self {
        let max = color.r.max(color.g).max(color.b);
        let min = color.r.min(color.g).min(color.b);
        let chroma = max - min;

        let hue = if chroma == 0.0 {
            0.0
        } else if max == color.r {
            60.0 * ((color.g - color.b) / chroma).rem_euclid(6.0)
        } else if max == color.g {
            60.0 * ((color.b - color.r) / chroma + 2.0)
        } else {
            60.0 * ((color.r - color.g) / chroma + 4.0)
        };

        let saturation = if max == 0.0 { 0.0 } else { chroma / max };

        Self {
            hue: hue.rem_euclid(360.0),
            saturation,
            value: max,
        }
    }

    /// The color as a `#rrggbb` string, which is what a field shows and what a
    /// config file carries.
    #[must_use]
    pub fn to_hex(self) -> String {
        let color = self.to_color();
        format!(
            "#{:02x}{:02x}{:02x}",
            (color.r * 255.0).round() as u8,
            (color.g * 255.0).round() as u8,
            (color.b * 255.0).round() as u8
        )
    }
}

/// Parses a color from `#rrggbb`, `#rgb` or the same without the hash.
///
/// Returns `None` for anything else, so a half-typed value does not silently
/// become black.
///
/// ```
/// # use iced_kit::widgets::color_picker::Hsv;
/// assert_eq!(Hsv::parse_hex("#ff8800").unwrap().to_hex(), "#ff8800");
/// assert_eq!(Hsv::parse_hex("#f80").unwrap().to_hex(), "#ff8800");
/// assert_eq!(Hsv::parse_hex("ff8800").unwrap().to_hex(), "#ff8800");
/// assert!(Hsv::parse_hex("#ff88").is_none());
/// assert!(Hsv::parse_hex("not a color").is_none());
/// ```
#[must_use]
pub fn parse_hex(text: &str) -> Option<Hsv> {
    let digits = text.trim().trim_start_matches('#');

    // `#rgb` is shorthand: each digit is doubled, so `f` means `ff`.
    let expanded = match digits.len() {
        3 => digits
            .chars()
            .flat_map(|digit| [digit, digit])
            .collect::<String>(),
        6 => digits.to_owned(),
        _ => return None,
    };

    let value = u32::from_str_radix(&expanded, 16).ok()?;
    let r = ((value >> 16) & 0xff) as f32 / 255.0;
    let g = ((value >> 8) & 0xff) as f32 / 255.0;
    let b = (value & 0xff) as f32 / 255.0;

    Some(Hsv::from_color(Color::from_rgb(r, g, b)))
}

/// The default swatches a picker offers, following the palette's own steps.
///
/// A picker with no caller-supplied colors still needs something to click, so
/// the default set is a greyscale ramp: it is useful in every application and
/// makes no claim about the brand.
#[must_use]
pub fn default_swatches() -> Vec<Color> {
    [
        "#000000", "#171717", "#404040", "#737373", "#a3a3a3", "#d4d4d4", "#f5f5f5", "#ffffff",
    ]
    .iter()
    .filter_map(|hex| parse_hex(hex).map(Hsv::to_color))
    .collect()
}

/// A trigger showing the current color as a swatch.
///
/// See the module documentation for how this pairs with [`ColorPickerPanel`].
///
/// ```
/// # use iced_kit::widgets::{color_picker, color_picker::Hsv};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # use iced::Color;
/// # #[derive(Clone, Debug)] enum Message { Toggled }
/// # fn view(current: Color) -> Element<'static, Message, Theme> {
/// color_picker(current).on_toggle(Message::Toggled).into()
/// # }
/// ```
#[must_use = "a ColorPicker does nothing unless it is turned into an Element"]
pub struct ColorPicker<'a, Message> {
    color: Color,
    label: Option<String>,
    open: bool,
    on_toggle: Option<Message>,
    show_hex: bool,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, Message: Clone + 'a> ColorPicker<'a, Message> {
    /// Creates a trigger showing `color`.
    pub fn new(color: Color) -> Self {
        Self {
            color,
            label: None,
            open: false,
            on_toggle: None,
            show_hex: true,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Adds a text label after the swatch.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Marks the trigger as open, which keeps it outlined while the panel
    /// shows.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Reports a press on the trigger.
    pub fn on_toggle(mut self, message: Message) -> Self {
        self.on_toggle = Some(message);
        self
    }

    /// Shows the color's hex code beside the swatch. On by default.
    pub fn show_hex(mut self, show_hex: bool) -> Self {
        self.show_hex = show_hex;
        self
    }

    /// The color's hex code, as the trigger shows it.
    #[must_use]
    pub fn hex(&self) -> String {
        Hsv::from_color(self.color).to_hex()
    }

    /// Turns the trigger into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let text_style = Size::Sm.text();
        // Read the hex before the label is moved out of `self`.
        let hex = self.hex();

        let mut content = row![].spacing(8).align_y(Alignment::Center);
        content = content.push(swatch(self.color, 18.0));

        if let Some(label) = self.label {
            content = content.push(
                text(label)
                    .size(text_style.size)
                    .line_height(text_style.line_height()),
            );
        }

        if self.show_hex {
            content = content.push(
                text(hex)
                    .size(text_style.size)
                    .line_height(text_style.line_height())
                    .font(iced::Font::MONOSPACE)
                    .class(Box::new(|theme: &Theme| text::Style {
                        color: Some(theme.colors().muted_foreground),
                    }) as text::StyleFn<'a, Theme>),
            );
        }

        let open = self.open;
        let mut trigger = button(content)
            .padding(Padding {
                top: 4.0,
                right: 8.0,
                bottom: 4.0,
                left: 8.0,
            })
            .class(
                Box::new(move |theme: &Theme, status| trigger_style(theme, status, open))
                    as button::StyleFn<'a, Theme>,
            );

        if let Some(message) = self.on_toggle {
            trigger = trigger.on_press(message);
        }

        trigger.into()
    }
}

impl<'a, Message: Clone + 'a> From<ColorPicker<'a, Message>> for Element<'a, Message, Theme> {
    fn from(picker: ColorPicker<'a, Message>) -> Self {
        picker.into_element()
    }
}

/// Builds a color picker trigger.
pub fn color_picker<'a, Message: Clone + 'a>(color: Color) -> ColorPicker<'a, Message> {
    ColorPicker::new(color)
}

/// A square swatch of a color.
fn swatch<'a, Message: 'a>(color: Color, side: f32) -> Element<'a, Message, Theme> {
    container(iced::widget::Space::new())
        .width(Length::Fixed(side))
        .height(Length::Fixed(side))
        .class(Box::new(move |theme: &Theme| container::Style {
            background: Some(iced::Background::Color(color)),
            border: iced::Border {
                // A swatch needs an outline of its own: a white or transparent
                // color would otherwise be invisible against the surface.
                color: theme.colors().border,
                width: 1.0,
                radius: f32::from(theme.radius().sm).into(),
            },
            ..container::Style::default()
        }) as container::StyleFn<'a, Theme>)
        .into()
}

/// The appearance of a color picker trigger.
fn trigger_style(theme: &Theme, status: button::Status, open: bool) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: Some(iced::Background::Color(if hovered || open {
            colors.accent
        } else {
            colors.surface
        })),
        text_color: colors.foreground,
        border: iced::Border {
            color: if open { colors.primary } else { colors.border },
            width: 1.0,
            radius: f32::from(theme.radius().md).into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// The choosing surface: the saturation/value square, the hue strip and the
/// swatches.
///
/// The application positions this and draws it through
/// [`Layer`](crate::widgets::overlay::Layer).
#[must_use = "a ColorPickerPanel does nothing unless it is turned into an Element"]
pub struct ColorPickerPanel<'a, Message> {
    color: Color,
    swatches: Vec<Color>,
    on_change: Option<std::rc::Rc<dyn Fn(Color) -> Message + 'a>>,
    hex_on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    width: f32,
    show_hex: bool,
}

impl<'a, Message: Clone + 'a> ColorPickerPanel<'a, Message> {
    /// Creates a panel editing `color`.
    pub fn new(color: Color) -> Self {
        Self {
            color,
            swatches: default_swatches(),
            on_change: None,
            hex_on_input: None,
            width: 240.0,
            show_hex: true,
        }
    }

    /// Reports a new color as the user drags or clicks.
    pub fn on_change(mut self, on_change: impl Fn(Color) -> Message + 'a) -> Self {
        self.on_change = Some(std::rc::Rc::new(on_change));
        self
    }

    /// Makes the hex field editable, reporting each change.
    ///
    /// Without this the hex code is shown but cannot be typed.
    pub fn on_hex_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.hex_on_input = Some(Box::new(on_input));
        self
    }

    /// Replaces the swatch set. Passing an empty set hides the swatch row.
    pub fn swatches(mut self, swatches: impl IntoIterator<Item = Color>) -> Self {
        self.swatches = swatches.into_iter().collect();
        self
    }

    /// Sets the panel's width. The default is 240.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(120.0);
        self
    }

    /// Hides the hex code row.
    pub fn show_hex(mut self, show_hex: bool) -> Self {
        self.show_hex = show_hex;
        self
    }

    /// The color the panel is editing.
    #[must_use]
    pub fn color(&self) -> Color {
        self.color
    }

    /// Turns the panel into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            color,
            swatches,
            on_change,
            hex_on_input,
            width,
            show_hex,
        } = self;

        let hsv = Hsv::from_color(color);
        let text_style = Size::Sm.text();

        let mut body: iced::widget::Column<'a, Message, Theme> = column![].spacing(10);

        // The saturation/value square. Its hue comes from the strip below, and
        // its marker from the current saturation and value.
        body = body.push(
            canvas(SvSquare {
                hue: hsv.hue,
                saturation: hsv.saturation,
                value: hsv.value,
            })
            .width(Length::Fill)
            .height(Length::Fixed(width * 0.6)),
        );

        if let Some(on_change) = on_change.as_ref() {
            // The hue strip is a canvas rather than a slider: iced's slider
            // draws a flat track, and the whole point of this control is to
            // show the hue under the pointer.
            let report = std::rc::Rc::clone(on_change);

            body = body.push(
                row![
                    text("Hue").size(text_style.size).width(Length::Fixed(34.0)),
                    canvas(HueStrip {
                        hue: hsv.hue,
                        saturation: hsv.saturation,
                        value: hsv.value,
                        on_change: Some(report),
                    })
                    .width(Length::Fill)
                    .height(Length::Fixed(14.0)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        } else {
            body = body.push(
                canvas(HueStrip {
                    hue: hsv.hue,
                    saturation: hsv.saturation,
                    value: hsv.value,
                    on_change: None,
                })
                .width(Length::Fill)
                .height(Length::Fixed(14.0)),
            );
        }

        if show_hex {
            let hex = hsv.to_hex();

            let field: Element<'a, Message, Theme> = if let Some(on_input) = hex_on_input {
                crate::widgets::input::text_input::<Message>("#000000", &hex)
                    .on_input(on_input)
                    .into()
            } else {
                text(hex)
                    .size(text_style.size)
                    .line_height(text_style.line_height())
                    .font(iced::Font::MONOSPACE)
                    .into()
            };

            body = body.push(field);
        }

        if !swatches.is_empty() {
            let mut grid = row![].spacing(4);

            for swatch_color in swatches {
                let mut cell = button(swatch(swatch_color, 20.0))
                    .padding(Padding::ZERO)
                    .class(
                        Box::new(|_theme: &Theme, status| swatch_button_style(status))
                            as button::StyleFn<'a, Theme>,
                    );

                if let Some(on_change) = on_change.as_ref() {
                    cell = cell.on_press(on_change(swatch_color));
                }

                grid = grid.push(cell);
            }

            body = body.push(grid);
        }

        container(body)
            .width(Length::Fixed(width))
            .padding(12)
            .class(Box::new(|theme: &Theme| {
                let colors = theme.colors();

                container::Style {
                    background: Some(iced::Background::Color(colors.surface)),
                    border: iced::Border {
                        color: colors.border,
                        width: 1.0,
                        radius: f32::from(theme.radius().md).into(),
                    },
                    shadow: floating_shadow(theme),
                    text_color: Some(colors.foreground),
                    ..container::Style::default()
                }
            }) as container::StyleFn<'a, Theme>)
            .into()
    }
}

impl<'a, Message: Clone + 'a> From<ColorPickerPanel<'a, Message>> for Element<'a, Message, Theme> {
    fn from(panel: ColorPickerPanel<'a, Message>) -> Self {
        panel.into_element()
    }
}

/// Builds a color picker's choosing surface.
pub fn color_picker_panel<'a, Message: Clone + 'a>(color: Color) -> ColorPickerPanel<'a, Message> {
    ColorPickerPanel::new(color)
}

/// The saturation/value square for one hue.
struct SvSquare {
    hue: f32,
    saturation: f32,
    value: f32,
}

impl<Message> canvas::Program<Message, Theme> for SvSquare {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        // The square is a grid of cells: the horizontal axis is saturation and
        // the vertical one is value. Cells rather than a gradient because iced
        // fills quads, and forty steps are smooth enough to read as one.
        const STEPS: usize = 40;
        let mut frame = Frame::new(renderer, bounds.size());
        let cell = IcedSize::new(bounds.width / STEPS as f32, bounds.height / STEPS as f32);

        for column in 0..STEPS {
            for row in 0..STEPS {
                let saturation = column as f32 / (STEPS - 1) as f32;
                let value = 1.0 - row as f32 / (STEPS - 1) as f32;
                let color = Hsv::new(self.hue, saturation, value).to_color();

                frame.fill_rectangle(
                    Point::new(column as f32 * cell.width, row as f32 * cell.height),
                    // A hair wider, so rounding between cells leaves no seams.
                    IcedSize::new(cell.width + 0.5, cell.height + 0.5),
                    color,
                );
            }
        }

        // The marker: a ring rather than a filled dot, so the color under it
        // stays visible while dragging.
        let marker = Point::new(
            self.saturation * bounds.width,
            (1.0 - self.value) * bounds.height,
        );
        let ring = Path::circle(marker, 6.0);

        frame.stroke(
            &ring,
            Stroke::default().with_width(2.0).with_color(Color::WHITE),
        );
        frame.stroke(
            &ring,
            Stroke::default().with_width(1.0).with_color(Color::BLACK),
        );

        vec![frame.into_geometry()]
    }
}

/// A horizontal strip of every hue, reporting the hue under the pointer.
struct HueStrip<'a, Message> {
    hue: f32,
    saturation: f32,
    value: f32,
    on_change: Option<std::rc::Rc<dyn Fn(Color) -> Message + 'a>>,
}

impl<Message: Clone> canvas::Program<Message, Theme> for HueStrip<'_, Message> {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let report = self.on_change.as_ref()?;

        // The strip follows a press and a drag, which is what lets a hue be
        // found by eye rather than by clicking and looking away.
        let dragging = matches!(
            event,
            iced::Event::Mouse(
                iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)
                    | iced::mouse::Event::CursorMoved { .. }
            )
        );

        if !dragging {
            return None;
        }

        let position = cursor.position_over(bounds)?;
        let fraction = (position.x / bounds.width).clamp(0.0, 1.0);

        let mut next = Hsv::new(self.hue, self.saturation, self.value);
        next.hue = fraction * 360.0;

        Some(canvas::Action::publish(report(next.to_color())))
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        const STEPS: usize = 60;
        let mut frame = Frame::new(renderer, bounds.size());
        let width = bounds.width / STEPS as f32;

        for step in 0..STEPS {
            let hue = step as f32 / STEPS as f32 * 360.0;
            let color = Hsv::new(hue, 1.0, 1.0).to_color();

            frame.fill_rectangle(
                Point::new(step as f32 * width, 0.0),
                IcedSize::new(width + 0.5, bounds.height),
                color,
            );
        }

        // The marker sits at the current hue.
        let x = self.hue / 360.0 * bounds.width;
        let line = Path::line(Point::new(x, 0.0), Point::new(x, bounds.height));

        frame.stroke(
            &line,
            Stroke::default().with_width(3.0).with_color(Color::WHITE),
        );
        frame.stroke(
            &line,
            Stroke::default().with_width(1.0).with_color(Color::BLACK),
        );

        vec![frame.into_geometry()]
    }
}

/// The appearance of a swatch button in the panel.
fn swatch_button_style(status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        // The swatch itself draws the color; the button only shows that it is
        // clickable.
        background: None,
        text_color: Color::BLACK,
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: !hovered,
    }
}

#[cfg(test)]
mod tests {
    use super::{color_picker, color_picker_panel, default_swatches, parse_hex, Hsv};
    use crate::theme::Theme;
    use iced::Color;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Changed(Color),
        Typed(String),
        Toggled,
    }

    #[test]
    fn hsv_round_trips_through_rgb() {
        for (hue, saturation, value) in [
            (0.0, 1.0, 1.0),
            (120.0, 1.0, 1.0),
            (240.0, 1.0, 1.0),
            (60.0, 0.5, 0.5),
            (300.0, 0.25, 0.9),
        ] {
            let hsv = Hsv::new(hue, saturation, value);
            let back = Hsv::from_color(hsv.to_color());

            assert!(
                (back.hue - hsv.hue).abs() < 1.0 || (back.hue - hsv.hue).abs() > 359.0,
                "hue {hue} came back as {}",
                back.hue
            );
            assert!((back.saturation - saturation).abs() < 0.01);
            assert!((back.value - value).abs() < 0.01);
        }
    }

    #[test]
    fn grey_has_no_hue() {
        // A grey has no chroma, so there is no hue to report and the picker
        // must not invent one.
        for grey in [Color::BLACK, Color::WHITE, Color::from_rgb(0.5, 0.5, 0.5)] {
            let hsv = Hsv::from_color(grey);

            assert_eq!(hsv.hue, 0.0);
            assert_eq!(hsv.saturation, 0.0);
        }

        assert_eq!(Hsv::from_color(Color::WHITE).value, 1.0);
        assert_eq!(Hsv::from_color(Color::BLACK).value, 0.0);
    }

    #[test]
    fn hsv_parts_are_clamped() {
        let clamped = Hsv::new(400.0, 2.0, -1.0);

        assert_eq!(clamped.hue, 40.0, "a hue past the circle wraps");
        assert_eq!(clamped.saturation, 1.0);
        assert_eq!(clamped.value, 0.0);

        assert_eq!(Hsv::new(-10.0, 0.5, 0.5).hue, 350.0);
    }

    #[test]
    fn a_color_serializes_to_hex_and_back() {
        for hex in ["#ff8800", "#000000", "#ffffff", "#1a2b3c"] {
            let hsv = parse_hex(hex).expect("a valid color");
            assert_eq!(hsv.to_hex(), hex, "{hex} did not survive the round trip");
        }
    }

    #[test]
    fn hex_parsing_takes_the_shorthand_and_the_bare_form() {
        let expected = parse_hex("#ff8800").expect("a valid color");

        assert_eq!(parse_hex("#f80"), Some(expected));
        assert_eq!(parse_hex("ff8800"), Some(expected));
        assert_eq!(parse_hex("  #FF8800  "), Some(expected));
    }

    #[test]
    fn hex_parsing_refuses_anything_else() {
        for text in [
            "",
            "#",
            "#f",
            "#ff",
            "#ff88",
            "#ff88000",
            "gggggg",
            "not a color",
        ] {
            assert_eq!(parse_hex(text), None, "{text} must not parse");
        }
    }

    #[test]
    fn the_default_swatches_cover_grey_from_end_to_end() {
        let swatches = default_swatches();
        assert_eq!(swatches.len(), 8);

        let first = Hsv::from_color(swatches[0]);
        let last = Hsv::from_color(swatches[swatches.len() - 1]);

        assert_eq!(first.value, 0.0, "the ramp starts at black");
        assert_eq!(last.value, 1.0, "and ends at white");
        // Every default swatch is neutral: no hue, no saturation.
        assert!(swatches
            .iter()
            .all(|color| Hsv::from_color(*color).saturation == 0.0));
    }

    #[test]
    fn a_trigger_reports_the_color_it_shows() {
        let color = parse_hex("#336699").expect("a valid color").to_color();
        let picker: super::ColorPicker<'_, Message> = color_picker(color);

        assert_eq!(picker.hex(), "#336699");
        assert_eq!(picker.color, color);
    }

    #[test]
    fn color_pickers_render_in_every_form() {
        let color = parse_hex("#336699").expect("a valid color").to_color();

        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            color_picker::<Message>(color).into(),
            color_picker::<Message>(color)
                .label("Accent")
                .open(true)
                .on_toggle(Message::Toggled)
                .into(),
            color_picker::<Message>(color).show_hex(false).into(),
        ];

        for element in elements {
            drop(element);
        }
    }

    #[test]
    fn a_panel_reports_the_color_it_edits() {
        let color = parse_hex("#336699").expect("a valid color").to_color();
        let panel: super::ColorPickerPanel<'_, Message> = color_picker_panel(color);

        assert_eq!(panel.color(), color);
        assert_eq!(panel.swatches.len(), 8);
    }

    #[test]
    fn panels_render_in_every_form() {
        let color = parse_hex("#336699").expect("a valid color").to_color();

        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            color_picker_panel::<Message>(color).into(),
            color_picker_panel::<Message>(color)
                .on_change(Message::Changed)
                .on_hex_input(Message::Typed)
                .width(300.0)
                .into(),
            color_picker_panel::<Message>(color)
                .swatches([])
                .show_hex(false)
                .into(),
        ];

        for element in elements {
            drop(element);
        }
    }

    #[test]
    fn a_panel_without_a_change_callback_still_renders_its_square() {
        // The sat/value square is drawn whatever the callbacks, but the hue
        // strip is only interactive when there is somewhere to report to. In
        // the read-only case the panel draws a static strip instead.
        let color = parse_hex("#ff8800").expect("a valid color").to_color();
        let element: iced::Element<'_, Message, Theme> =
            color_picker_panel::<Message>(color).into();
        drop(element);
    }
}
