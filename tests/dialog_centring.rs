//! Checks where a dialog is placed: centred in its window, with its close
//! button inset equally from both edges of its corner.
//!
//! # Why these are measured rather than asserted on the layout
//!
//! Neither defect they pin is something the type system catches. The layout
//! stays valid in both cases and the dialog is simply drawn in the wrong place:
//!
//! - The dialog sat against the window's left edge. Its card asked for a
//!   filling width, and `Stack::push` adopts the first child's size hint, so the
//!   stack around the card spanned the window and the centring box had no room
//!   left to centre in.
//! - The close button sat 25px down from the top against 9px in from the right,
//!   because the overlay that positions it reused the surface's vertical-only
//!   padding. Each margin was individually reasonable; together they read as the
//!   button being low rather than in the corner.
//!
//! Both are therefore checked against rendered pixels.
//!
//! # Why every case is in one test
//!
//! Rendering in `iced_test` is offscreen, but it shares a device and a scratch
//! directory. Cases running in parallel read each other's half-written
//! snapshots, which showed up as decode failures and as dialogs that had not
//! been drawn yet. Running them in sequence is what makes this deterministic.

use iced::widget::{column, container};
use iced::{Element, Length};
use iced_kit::widgets::button;
use iced_kit::widgets::overlay::{layer, AlertDialog, AlertTone, DialogWidth, Layer, Modal};
use iced_kit::Theme;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Close,
}

/// A page for the dialog to sit over, dimmed behind it by the scrim.
fn page<'a>() -> Element<'a, Message, Theme> {
    container(
        column![
            button("Open").primary().on_press(Message::Close),
            button("Another").on_press(Message::Close),
        ]
        .spacing(12),
    )
    .padding(24)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// A rendered frame: the dialog card's bounds, in device pixels, and the pixels
/// themselves.
struct Frame {
    /// The card's left edge.
    left: usize,
    /// The card's right edge.
    right: usize,
    /// The card's top edge.
    top: usize,
    /// The card's bottom edge.
    bottom: usize,
    /// The frame's width.
    width: usize,
    /// The frame's height.
    height: usize,
    /// Device pixels per logical pixel.
    scale: f32,
    /// The frame's RGBA pixels.
    pixels: Vec<u8>,
}

/// Renders `element` offscreen and finds the dialog card in the pixels.
///
/// `label` names the scratch PNG. It has to be unique per call, because the
/// snapshot is written in order to be read back and two cases sharing a name
/// would remove each other's file mid-read.
fn render(element: Element<'_, Message, Theme>, width: f32, height: f32, label: &str) -> Frame {
    // Reduced motion, so a case is measured where it settles rather than part
    // way through its entrance.
    iced_kit::motion::set_reduce_motion(true);

    let mut simulator = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(width, height),
        element,
    );

    let name = format!("target/dialog_placement_probe-{label}");
    let path = format!("{name}-wgpu.png");

    // `matches_image` writes the PNG when the file is absent, which is how the
    // rendered pixels are read back. The file is scratch, not a reference.
    let _ = std::fs::remove_file(&path);
    simulator
        .snapshot(&Theme::light())
        .expect("the dialog should render")
        .matches_image(&name)
        .expect("the snapshot should be written");

    let bytes = std::fs::read(&path).expect("the snapshot should exist");
    let pixels = decode_png(&bytes);
    let _ = std::fs::remove_file(&path);

    // The window is `width` by `height` in logical pixels, and the frame is
    // larger than that on a scaled device. The ratio is derived from the frame
    // rather than assumed, so these measurements hold either way.
    let logical = (width * height) as usize;
    let device = pixels.len() / 4;
    let scale = ((device as f32 / logical as f32).sqrt()).round().max(1.0);

    let frame_width = (width * scale) as usize;
    let frame_height = (height * scale) as usize;

    assert_eq!(
        frame_width * frame_height * 4,
        pixels.len(),
        "the frame should be {frame_width}x{frame_height} at {scale}x"
    );

    // The card is the one large region that stays bright: the page behind the
    // scrim, and the scrim itself, are both darker.
    let mid = frame_height / 2;
    let mut left = None;
    let mut right = None;

    for x in 0..frame_width {
        if is_card(&pixels, frame_width, x, mid) {
            if left.is_none() {
                left = Some(x);
            }

            right = Some(x);
        }
    }

    let left = left.unwrap_or_else(|| {
        panic!("{label}: no dialog card on the middle row of a {frame_width}x{frame_height} frame")
    });
    let right = right.unwrap();
    let centre = left.midpoint(right);

    let mut top = None;
    let mut bottom = None;

    for y in 0..frame_height {
        if is_card(&pixels, frame_width, centre, y) {
            if top.is_none() {
                top = Some(y);
            }

            bottom = Some(y);
        }
    }

    Frame {
        left,
        right,
        top: top.expect("the card should have a top edge"),
        bottom: bottom.expect("the card should have a bottom edge"),
        width: frame_width,
        height: frame_height,
        scale,
        pixels,
    }
}

/// Whether the pixel at `x, y` is part of the dialog card.
///
/// A card is drawn on the surface color, which is the brightest thing in the
/// frame: everything else is either the page dimmed by the scrim or the scrim
/// itself.
fn is_card(pixels: &[u8], width: usize, x: usize, y: usize) -> bool {
    let i = (y * width + x) * 4;

    pixels[i] > 245 && pixels[i + 1] > 245 && pixels[i + 2] > 245
}

/// Whether the pixel at `x, y` is part of a dark glyph.
fn is_dark(pixels: &[u8], width: usize, x: usize, y: usize) -> bool {
    let i = (y * width + x) * 4;

    pixels[i] < 140 && pixels[i + 1] < 140 && pixels[i + 2] < 140
}

/// Decodes an RGBA PNG into a flat buffer.
fn decode_png(bytes: &[u8]) -> Vec<u8> {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder.read_info().expect("the snapshot should be a PNG");
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .expect("the snapshot should decode");

    assert_eq!(
        info.color_type,
        png::ColorType::Rgba,
        "a pixel should be four bytes"
    );
    assert_eq!(info.bit_depth, png::BitDepth::Eight, "eight bits a channel");

    buffer.truncate(info.buffer_size());
    buffer
}

/// Asserts that the card sits in the middle of its frame on both axes.
fn assert_centred(frame: &Frame, what: &str) {
    let left = frame.left;
    let right = frame.width - 1 - frame.right;
    let top = frame.top;
    let bottom = frame.height - 1 - frame.bottom;

    // One device pixel of slack, for rounding to whole pixels.
    assert!(
        left.abs_diff(right) <= 1,
        "{what}: the dialog is not centred horizontally — \
         {left} device px of margin on the left against {right} on the right"
    );

    assert!(
        top.abs_diff(bottom) <= 1,
        "{what}: the dialog is not centred vertically — \
         {top} device px of margin above against {bottom} below"
    );
}

/// How far the close button's glyph sits from the corner's two edges, in
/// logical pixels.
fn close_button_inset(frame: &Frame) -> (f32, f32) {
    let mut right = None;
    let mut top = None;

    // Only the card's top-right corner, where nothing but the button sits. The
    // title is kept out by starting past the card's last tenth.
    let from_x = frame.left + (frame.right - frame.left) * 9 / 10;
    let to_y = frame.top + (frame.bottom - frame.top) / 4;

    for y in frame.top..=to_y {
        for x in from_x..=frame.right {
            if !is_dark(&frame.pixels, frame.width, x, y) {
                continue;
            }

            right = Some(right.map_or(x, |r: usize| r.max(x)));
            top = Some(top.map_or(y, |t: usize| t.min(y)));
        }
    }

    let (Some(right), Some(top)) = (right, top) else {
        panic!("no close button found in the card's top-right corner");
    };

    (
        (frame.right - right) as f32 / frame.scale,
        (top - frame.top) as f32 / frame.scale,
    )
}

/// A dialog is centred in its window, its close button is inset equally from
/// both edges of its corner, and a dialog wider than its window shrinks to fit
/// rather than running off an edge.
#[test]
fn dialogs_are_placed_where_they_belong() {
    // A dialog at three window sizes, so the centring holds both for a card
    // narrower than the window and for one that is the same width.
    for (width, height) in [(1280.0, 1000.0), (900.0, 700.0), (2048.0, 1200.0)] {
        let open = Modal::new(
            "Delete project",
            iced::widget::text("This permanently removes the project."),
            Message::Close,
        )
        .description("Every file in it goes with it.")
        .cancel("Cancel", Message::Close)
        .destructive("Delete", Message::Close);

        let label = format!("modal-{}x{}", width as u32, height as u32);
        let frame = render(
            layer(page(), Layer::new().modal(open)),
            width,
            height,
            &label,
        );

        assert_centred(&frame, &label);
    }

    // An alert, which has its own layer path.
    let alert = AlertDialog::new()
        .tone(AlertTone::Danger)
        .title("Delete project?")
        .description("Every file in it goes with it.")
        .confirm()
        .on_confirm(Message::Close)
        .on_cancel(Message::Close);

    let frame = render(
        layer(page(), Layer::new().modal(alert)),
        1280.0,
        1000.0,
        "alert",
    );

    assert_centred(&frame, "an alert");

    // The close button, inset from both edges of its corner by the same amount.
    let open = Modal::new("Delete project", iced::widget::text("Body"), Message::Close)
        .cancel("Cancel", Message::Close);

    let frame = render(
        layer(page(), Layer::new().modal(open)),
        1280.0,
        1000.0,
        "close-inset",
    );

    let (from_right, from_top) = close_button_inset(&frame);

    assert!(
        (from_right - from_top).abs() <= 1.0,
        "the close button is {from_top}px from the top but {from_right}px from the right"
    );

    // A dialog wider than its window shrinks to fit and stays centred, rather
    // than running off both edges. Its card asks for 720 logical pixels in a
    // 500-pixel window, so it should come out at the window's width — a small
    // inset either side is the page's own padding showing through.
    let open = Modal::new(
        "A long title that will not fit in this window",
        iced::widget::text("Body"),
        Message::Close,
    )
    .width(DialogWidth::Lg);

    let frame = render(
        layer(page(), Layer::new().modal(open)),
        500.0,
        700.0,
        "narrow",
    );

    let card_width = frame.right - frame.left + 1;
    let expected = 500.0 * frame.scale;

    assert!(
        (card_width as f32) >= expected - 8.0 * frame.scale,
        "a dialog wider than its window should shrink to about its width: \
         it came out {card_width} device px against a window of {expected}"
    );

    assert_centred(&frame, "a shrunk dialog");
}
