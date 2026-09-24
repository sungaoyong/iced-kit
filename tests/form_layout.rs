//! Measures where a form actually puts its fields, by reading back pixels.
//!
//! `iced_test` renders through the real pipeline into an offscreen buffer, so
//! these checks are deterministic and see what the user would see. Each test
//! field carries a solid-color control; the test finds each color's bounding
//! box in the frame and asserts the relationships the reference guarantees.

use iced::widget::{column, container};
use iced::{Color, Element, Length};
use iced_kit::widgets::{field, form, Field, FormLabelLayout};
use iced_kit::Theme;

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Message {
    Noop,
}

/// A control block's bounding box, in device pixels.
#[derive(Debug)]
struct Box2D {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl Box2D {
    fn center_y(&self) -> usize {
        self.top.midpoint(self.bottom)
    }
}

/// The RGB a test control is painted with.
const RED: [u8; 3] = [0xde, 0x10, 0x10];
const GREEN: [u8; 3] = [0x10, 0xde, 0x10];
const BLUE: [u8; 3] = [0x10, 0x10, 0xde];

/// Renders `element` at `width` × `height` and finds each color's box.
///
/// The frame is written only to be read back, so the file is scratch: unique
/// per call, deleted after reading.
fn render(
    element: Element<'_, Message, Theme>,
    width: f32,
    height: f32,
    label: &str,
) -> (Vec<Option<Box2D>>, f32) {
    iced_kit::motion::set_reduce_motion(true);

    let framed: Element<'_, Message, Theme> = container(element).padding(PADDING).into();

    let mut simulator = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(width, height),
        framed,
    );

    let name = format!("target/form_placement_probe-{label}");
    let path = format!("{name}-wgpu.png");
    let _ = std::fs::remove_file(&path);
    simulator
        .snapshot(&Theme::light())
        .expect("the form should render")
        .matches_image(&name)
        .expect("the snapshot should be written");

    let bytes = std::fs::read(&path).expect("the snapshot should exist");
    let _ = std::fs::remove_file(&path);
    let (pixels, frame_width, frame_height) = decode_png(&bytes);

    let logical = (width * height) as usize;
    let device = pixels.len() / 4;
    let scale = ((device as f32 / logical as f32).sqrt()).round().max(1.0);

    let boxes = [RED, GREEN, BLUE]
        .into_iter()
        .map(|target| find_box(&pixels, frame_width, frame_height, target))
        .collect();

    (boxes, scale)
}

fn decode_png(bytes: &[u8]) -> (Vec<u8>, usize, usize) {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("a readable PNG");
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).expect("a decoded frame");
    assert_eq!(
        info.color_type,
        png::ColorType::Rgba,
        "eight bits a channel"
    );
    (
        pixels[..info.buffer_size()].to_vec(),
        info.width as usize,
        info.height as usize,
    )
}

fn find_box(pixels: &[u8], width: usize, height: usize, target: [u8; 3]) -> Option<Box2D> {
    let mut left = usize::MAX;
    let mut top = usize::MAX;
    let mut right = 0;
    let mut bottom = 0;

    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) * 4;
            let close = |channel: usize, value: u8| {
                (i16::from(pixels[index + channel]) - i16::from(value)).abs() <= 6
            };
            if close(0, target[0]) && close(1, target[1]) && close(2, target[2]) {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
        }
    }

    (left != usize::MAX).then_some(Box2D {
        left,
        top,
        right,
        bottom,
    })
}

/// The tolerance of an assertion about an edge, in device pixels: rounding
/// and the block's own antialiased border.
fn near(actual: usize, expected: f32, scale: f32) -> bool {
    (actual as f32 - expected * scale).abs() <= 2.0 * scale
}

/// A solid-color control block a test can find in the frame.
fn block<'a>(color: [u8; 3]) -> Element<'a, Message, Theme> {
    let style: iced::widget::container::StyleFn<'static, Theme> = Box::new(move |_| {
        let (r, g, b) = (
            f32::from(color[0]) / 255.0,
            f32::from(color[1]) / 255.0,
            f32::from(color[2]) / 255.0,
        );
        container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(r, g, b))),
            ..container::Style::default()
        }
    });
    container(iced::widget::text(""))
        .width(Length::Fill)
        .height(Length::Fixed(20.0))
        .class(style)
        .into()
}

/// A labelled field whose control is a findable block.
fn marked(color: [u8; 3], label: &str) -> Field<'_, Message> {
    field().label(label).push(block(color))
}

const PAGE_WIDTH: f32 = 400.0;
const PADDING: f32 = 16.0;
// The form fills the page between the padding, so its content width is:
const CONTENT: f32 = PAGE_WIDTH - 2.0 * PADDING;

#[test]
fn two_columns_place_fields_side_by_side_then_row_by_row() {
    let element = column![form()
        .columns(2)
        .child(marked(RED, "One"))
        .child(marked(GREEN, "Two"))
        .child(marked(BLUE, "Three")),];
    let (boxes, scale) = render(element.into(), PAGE_WIDTH, 300.0, "columns");
    let [red, green, blue] = boxes.try_into().expect("three boxes");
    let red = red.expect("the first field's block");
    let green = green.expect("the second field's block");
    let blue = blue.expect("the third field's block");

    // Fields 0 and 1 share the first row; field 2 starts the second.
    assert_eq!(red.center_y(), green.center_y());
    assert!(green.left > red.right);
    assert!(blue.top > red.bottom);
    assert!(near(blue.left, PADDING, scale));

    // Each field is one column wide: (content - one gap) / 2.
    let row_spacing = 8.0;
    let column_spacing = row_spacing * 3.0;
    let column_width = (CONTENT - column_spacing) / 2.0;
    assert!(near(red.right - red.left, column_width, scale));
}

#[test]
fn the_label_direction_moves_the_control_across_or_below_the_label() {
    // Vertical: the control takes the full row, its top pushed down by the
    // label sitting above it.
    let vertical = column![form().child(marked(RED, "Name"))];
    let (boxes, scale) = render(vertical.into(), PAGE_WIDTH, 200.0, "vertical");
    let red = boxes[0].as_ref().expect("the control block");
    assert!(near(red.left, PADDING, scale));
    assert!(
        (red.top as f32) > (PADDING + 10.0) * scale,
        "a label must sit above the control"
    );

    // Horizontal: the control starts after the 100-wide label column and the
    // inner gap, level with the label.
    let horizontal = column![form()
        .label_layout(FormLabelLayout::Horizontal)
        .label_width(100.0)
        .child(marked(RED, "Name"))];
    let (boxes, scale) = render(horizontal.into(), PAGE_WIDTH, 200.0, "horizontal");
    let red = boxes[0].as_ref().expect("the control block");
    let inner_gap = 4.0;
    assert!(near(red.left, PADDING + 100.0 + inner_gap, scale));
    assert!(
        (red.top as f32) < (PADDING + 10.0) * scale,
        "the label must sit beside the control"
    );
}

#[test]
fn the_footer_spans_the_columns_and_aligns_to_the_trailing_edge() {
    let footer_style: iced::widget::container::StyleFn<'static, Theme> =
        Box::new(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(
                BLUE[0], BLUE[1], BLUE[2],
            ))),
            ..container::Style::default()
        });
    let element = column![form()
        .columns(2)
        .child(marked(RED, "One"))
        .child(marked(GREEN, "Two"))
        .footer(
            container(iced::widget::text(""))
                .width(Length::Fixed(80.0))
                .height(Length::Fixed(20.0))
                .class(footer_style),
        ),];
    let (boxes, scale) = render(element.into(), PAGE_WIDTH, 300.0, "footer");
    let [red, green, blue] = boxes.try_into().expect("three boxes");
    let blue = blue.expect("the footer block");

    assert!(blue.top > red.expect("the first field's block").bottom);
    assert!(
        near(blue.right, PADDING + CONTENT, scale),
        "the footer's trailing edge is the form's"
    );
    assert!(near(blue.right - blue.left, 80.0, scale));
    assert!(green.is_some(), "both fields rendered");
}
