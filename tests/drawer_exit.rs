//! Drives a drawer's full life at normal motion — entrance frames, a backdrop
//! press, exit frames — and checks the panel is actually gone when the exit
//! has run, rather than stuck half-way waiting for another click.

use iced::mouse::{self, Button};
use iced::widget::{column, container, text};
use iced::{Element, Event, Length, Point, Size};
use iced_kit::motion::Presence;
use iced_kit::widgets::overlay::{layer, Drawer, DrawerSide, Layer};
use iced_kit::Theme;
use iced_test::Simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Dismiss,
    /// The exit has finished drawing; the view rebuild around it is what
    /// removes the drawer.
    ExitDrawn,
}

fn page<'a>() -> Element<'a, Message, Theme> {
    container(text("The page"))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn click(simulator: &mut Simulator<'static, Message, Theme>, at: Point) {
    simulator.point_at(at);
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonPressed(Button::Left))]);
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonReleased(Button::Left))]);
}

/// Feeds the redraw loop the way the runtime does: one `RedrawRequested` per
/// frame, at the real walking clock — never a timestamp in the future, the
/// way a live window produces them.
fn frames(simulator: &mut Simulator<'static, Message, Theme>, count: usize) {
    for _ in 0..count {
        std::thread::sleep(std::time::Duration::from_millis(16));
        let _ = simulator.simulate([Event::Window(iced::window::Event::RedrawRequested(
            std::time::Instant::now(),
        ))]);
    }
}

/// Renders a snapshot and reads its pixels back, the way `dialog_drag` does.
fn pixels(simulator: &mut Simulator<'static, Message, Theme>, label: &str) -> Vec<u8> {
    let name = format!("target/drawer_exit_probe-{label}");
    let path = format!("{name}-wgpu.png");

    let _ = std::fs::remove_file(&path);
    simulator
        .snapshot(&Theme::light())
        .expect("the frame should render")
        .matches_image(&name)
        .expect("the snapshot should be written");

    let bytes = std::fs::read(&path).expect("the snapshot should exist");
    let _ = std::fs::remove_file(&path);
    decode_png(&bytes)
}

fn decode_png(bytes: &[u8]) -> Vec<u8> {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder.read_info().expect("the snapshot should be a PNG");
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .expect("the snapshot should decode");

    buffer.truncate(info.buffer_size());
    buffer
}

#[test]
fn an_exit_undrawn_by_clicks_still_completes() {
    let presence: &'static mut Presence = Box::leak(Box::new(Presence::new()));
    presence.show(true, std::time::Instant::now());
    let shared_presence: &'static Presence = presence;

    let mut simulator = Simulator::with_size(
        iced::Settings::default(),
        Size::new(900.0, 600.0),
        layer(
            page(),
            Layer::new().drawer(
                Drawer::new(
                    "Details",
                    column![text("A drawer slides in from an edge.")].spacing(12),
                )
                .side(DrawerSide::Right)
                .presence(shared_presence)
                .on_dismiss(Message::Dismiss)
                .on_closed(Message::ExitDrawn),
            ),
        ),
    );

    // Drive the entrance to completion: twenty frames at ~16ms is past the
    // normal motion duration.
    frames(&mut simulator, 20);

    let open = pixels(&mut simulator, "open");

    // A press beside the panel; the application answers it by hiding.
    click(&mut simulator, Point::new(80.0, 300.0));

    presence.show(false, std::time::Instant::now());

    // Drive the exit the same way: twenty frames is past the 140ms exit.
    frames(&mut simulator, 20);

    let now = std::time::Instant::now();
    let settled = presence.progress(now);
    assert!(
        settled < 0.05,
        "the presence should have finished its exit: progress {settled}"
    );

    let closed = pixels(&mut simulator, "closed");

    let messages: Vec<_> = simulator.into_messages().collect();
    assert_eq!(
        messages,
        vec![Message::Dismiss, Message::ExitDrawn],
        "one press, one dismissal, and the exit-finished cue to rebuild"
    );

    // The view here is fixed — there is no application rebuilding it — so the
    // backdrop stays after the exit. What must be gone is the panel: its
    // mid-body sat at logical (750, 300), twice that in device pixels.
    let sample = |frame: &[u8]| {
        let (x, y) = (1500usize, 600usize);
        let at = (y * 1800 + x) * 4;
        frame[at]
    };

    assert!(
        sample(&open) > 200,
        "while open the panel should be light at its body: {}",
        sample(&open)
    );
    assert!(
        sample(&closed) < 200,
        "after the exit has run the panel must be gone, not stuck half-way: {}",
        sample(&closed)
    );
}
