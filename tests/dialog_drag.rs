//! Checks that a draggable dialog is carried by its surface: the card moves
//! with the drag, keeps where it was put, opens from the centre again, never
//! dismisses by being grabbed, and stays reachable however far it is dragged.
//!
//! # Why these are measured rather than asserted on the layout
//!
//! A drag's outcome is a displacement, and a displacement only shows in where
//! the card is drawn: the same rendered-pixel read-back
//! `dialog_centring.rs` uses, before and after a simulated grab.
//!
//! # Why every case is in one test
//!
//! Rendering in `iced_test` is offscreen but shares a device and a scratch
//! directory; parallel cases read each other's half-written snapshots. The
//! cases here run in sequence for the same reason their centring counterparts
//! do.

use iced::mouse::{self, Button};
use iced::widget::{column, container};
use iced::{Element, Event, Length, Point};
use iced_test::Simulator;
use iced_kit::widgets::button;
use iced_kit::widgets::overlay::{layer, AlertDialog, AlertTone, Layer, Modal};
use iced_kit::Theme;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Close,
}

/// A page for the dialog to sit over.
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

/// A modal to grab, by its header, left of the floating close button.
fn modal() -> Modal<'static, Message> {
    Modal::new(
        "Delete project",
        iced::widget::text("This permanently removes the project."),
        Message::Close,
    )
    .description("Every file in it goes with it.")
    .cancel("Cancel", Message::Close)
    .destructive("Delete", Message::Close)
    .draggable(true)
}

/// The card's edges, in device pixels, as `dialog_centring` measures them.
struct Frame {
    left: usize,
    right: usize,
    top: usize,
    bottom: usize,
    #[allow(dead_code)]
    width: usize,
    #[allow(dead_code)]
    height: usize,
    scale: f32,
}

/// One window, and the dialog in it, that can be grabbed and measured.
struct Probe {
    simulator: Simulator<'static, Message, Theme>,
    width: f32,
    height: f32,
}

impl Probe {
    fn new(element: Element<'static, Message, Theme>, width: f32, height: f32) -> Self {
        iced_kit::motion::set_reduce_motion(true);

        Self {
            simulator: Simulator::with_size(
                iced::Settings::default(),
                iced::Size::new(width, height),
                element,
            ),
            width,
            height,
        }
    }

    /// Renders the current frame and finds the card in the pixels.
    fn measure(&mut self, label: &str) -> Frame {
        let name = format!("target/dialog_drag_probe-{label}");
        let path = format!("{name}-wgpu.png");

        // `matches_image` writes the PNG when the file is absent, which is how
        // the rendered pixels are read back. The file is scratch, not a
        // reference.
        let _ = std::fs::remove_file(&path);
        self.simulator
            .snapshot(&Theme::light())
            .expect("the dialog should render")
            .matches_image(&name)
            .expect("the snapshot should be written");

        let bytes = std::fs::read(&path).expect("the snapshot should exist");
        let pixels = decode_png(&bytes);
        let _ = std::fs::remove_file(&path);

        let logical = (self.width * self.height) as usize;
        let device = pixels.len() / 4;
        let scale = ((device as f32 / logical as f32).sqrt()).round().max(1.0);

        let frame_width = (self.width * scale) as usize;
        let frame_height = (self.height * scale) as usize;

        assert_eq!(
            frame_width * frame_height * 4,
            pixels.len(),
            "the frame should be {frame_width}x{frame_height} at {scale}x"
        );

        // The card is the one large bright region: the page behind the scrim,
        // and the scrim itself, are both darker.
        let mid = frame_height / 2;
        let mut left = None;
        let mut right = None;

        for x in 0..frame_width {
            if is_card(&pixels, frame_width, x, mid) {
                left = left.or(Some(x));
                right = Some(x);
            }
        }

        let left = left.unwrap_or_else(|| {
            panic!("{label}: no dialog card on the middle row of a {frame_width}x{frame_height} frame")
        });
        let right = right.expect("the card should have a right edge");
        let centre = left.midpoint(right);

        let mut top = None;
        let mut bottom = None;

        for y in 0..frame_height {
            if is_card(&pixels, frame_width, centre, y) {
                top = top.or(Some(y));
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
        }
    }

    /// Presses at `from`, drags to `to`, and releases.
    ///
    /// The simulator's cursor only moves through `point_at`, so each leg of
    /// the gesture is pointed at before its event batch is sent — the way the
    /// dock's drag tests drive a grab past the press threshold.
    fn drag(&mut self, from: Point, to: Point) {
        self.simulator.point_at(from);
        let _ = self.simulator.simulate([Event::Mouse(mouse::Event::ButtonPressed(
            Button::Left,
        ))]);

        self.simulator.point_at(to);
        let _ = self.simulator.simulate([Event::Mouse(mouse::Event::CursorMoved {
            position: to,
        })]);

        let _ = self.simulator.simulate([Event::Mouse(mouse::Event::ButtonReleased(
            Button::Left,
        ))]);
    }

    /// A point on the card's header, left of the floating close button, in
    /// logical window coordinates.
    #[allow(clippy::unused_self)]
    fn grab_point(&self, frame: &Frame) -> Point {
        Point::new(
            (frame.left + 40 * frame.scale as usize) as f32 / frame.scale,
            (frame.top + 12 * frame.scale as usize) as f32 / frame.scale,
        )
    }

    /// The card's centre, in logical window coordinates.
    #[allow(clippy::unused_self)]
    fn centre(&self, frame: &Frame) -> Point {
        Point::new(
            frame.left.midpoint(frame.right) as f32 / frame.scale,
            frame.top.midpoint(frame.bottom) as f32 / frame.scale,
        )
    }
}

/// Whether the pixel at `x, y` is part of the dialog card: the surface color
/// is the brightest thing in the frame.
fn is_card(pixels: &[u8], width: usize, x: usize, y: usize) -> bool {
    let i = (y * width + x) * 4;

    pixels[i] > 245 && pixels[i + 1] > 245 && pixels[i + 2] > 245
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

/// A drag carries the card, the card keeps where it was put, the grab never
/// dismisses, and the close button answers at the displaced corner.
#[test]
fn a_dialog_is_carried_by_its_surface() {
    let mut probe = Probe::new(
        layer(page(), Layer::new().modal(modal())),
        1280.0,
        1000.0,
    );

    let before = probe.measure("before");
    let grab = probe.grab_point(&before);
    let centre = probe.centre(&before);

    // A diagonal drag, well past the press threshold.
    let (dx, dy) = (120.0, 60.0);
    probe.drag(grab, Point::new(grab.x + dx, grab.y + dy));

    let after = probe.measure("after");
    let moved = probe.centre(&after);

    assert!(
        ((moved.x - centre.x) - dx).abs() <= 2.0,
        "the card should move {dx}px right: it moved {} px",
        moved.x - centre.x
    );
    assert!(
        ((moved.y - centre.y) - dy).abs() <= 2.0,
        "the card should move {dy}px down: it moved {} px",
        moved.y - centre.y
    );

    // Idle frames do not move a released dialog: the displacement lives in
    // the widget's state, not in the drag.
    let settled = probe.measure("settled");
    let settled_centre = probe.centre(&settled);
    assert!(
        (settled_centre.x - moved.x).abs() <= 1.0
            && (settled_centre.y - moved.y).abs() <= 1.0,
        "the card should stay where it was dragged"
    );

    // The grab never dismissed the dialog: the backdrop never saw a press the
    // surface had claimed.
    let close_at = Point::new(
        after.right as f32 / after.scale - 18.0,
        after.top as f32 / after.scale + 18.0,
    );
    probe.simulator.point_at(close_at);
    let _ = probe
        .simulator
        .simulate([Event::Mouse(mouse::Event::ButtonPressed(Button::Left))]);
    let _ = probe.simulator.simulate([Event::Mouse(
        mouse::Event::ButtonReleased(Button::Left),
    )]);

    let messages: Vec<_> = probe.simulator.into_messages().collect();
    assert_eq!(
        messages,
        vec![Message::Close],
        "the only message should be the displaced close button's"
    );
}

/// A dialog dragged past the window's edge is held back by its corner, rather
/// than vanishing off the glass.
#[test]
fn a_dialog_dragged_off_the_glass_is_clamped() {
    let mut probe = Probe::new(
        layer(page(), Layer::new().modal(modal())),
        1280.0,
        1000.0,
    );

    let before = probe.measure("clamp-before");
    let grab = probe.grab_point(&before);
    let card_width = (before.right - before.left) as f32 / before.scale;

    // A thousand pixels of pull to the left: far enough that the clamp, and
    // not the pull, decides where the card stops.
    probe.drag(grab, Point::new(grab.x - 1000.0, grab.y));

    let after = probe.measure("clamp-after");
    let visible = after.right as f32 / after.scale;

    assert!(
        visible >= 45.0,
        "the dragged card should keep a corner on the glass: \
         its right edge sits at {visible} logical px"
    );
    assert!(
        (visible - 48.0).abs() <= 4.0,
        "the clamp should hold the card at the sliver's edge: \
         its right edge sits at {visible} against a card {card_width} px wide"
    );
}

/// A dialog built without `draggable` ignores a grab, as it always has.
#[test]
fn a_plain_modal_stays_put() {
    let mut probe = Probe::new(
        layer(
            page(),
            Layer::new().modal(Modal::new(
                "Delete project",
                iced::widget::text("Body"),
                Message::Close,
            )),
        ),
        1280.0,
        1000.0,
    );

    let before = probe.measure("plain-before");
    let centre = probe.centre(&before);
    let grab = probe.grab_point(&before);

    probe.drag(grab, Point::new(grab.x + 120.0, grab.y + 60.0));

    let after = probe.measure("plain-after");
    let moved = probe.centre(&after);

    assert!(
        (moved.x - centre.x).abs() <= 1.0 && (moved.y - centre.y).abs() <= 1.0,
        "a modal without `draggable` should not move: it moved to {moved:?}"
    );
}

/// The alert, which assembles its own layer, drags the same way.
#[test]
fn an_alert_is_carried_by_its_surface() {
    let mut probe = Probe::new(
        layer(
            page(),
            Layer::new().modal(
                AlertDialog::new()
                    .tone(AlertTone::Danger)
                    .title("Delete project?")
                    .description("Every file in it goes with it.")
                    .confirm()
                    .on_confirm(Message::Close)
                    .on_cancel(Message::Close)
                    .draggable(true),
            ),
        ),
        1280.0,
        1000.0,
    );

    let before = probe.measure("alert-before");
    let centre = probe.centre(&before);
    let grab = probe.grab_point(&before);

    probe.drag(grab, Point::new(grab.x + 80.0, grab.y + 40.0));

    let after = probe.measure("alert-after");
    let moved = probe.centre(&after);

    assert!(
        ((moved.x - centre.x) - 80.0).abs() <= 2.0
            && ((moved.y - centre.y) - 40.0).abs() <= 2.0,
        "the alert should follow the drag: it moved to {moved:?}"
    );
}
