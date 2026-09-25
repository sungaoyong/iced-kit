//! Checks that a floating surface claims the presses that land on it: a press
//! beside a drawer or a dialog reaches the backdrop and dismisses it, while a
//! press on the surface's own inert areas — its blank half, its heading text —
//! stays with the surface instead of falling through to the backdrop.
//!
//! # Why this is simulated rather than asserted on the layout
//!
//! The claim is an event-routing question — whether a press on the surface
//! stops at the surface or falls through the stack of layers to the backdrop's
//! `MouseArea` — and only a simulated press answers it.

use iced::mouse::{self, Button};
use iced::widget::{column, container, text};
use iced::{Element, Event, Length, Point, Size};
use iced_kit::widgets::overlay::{layer, Drawer, DrawerSide, Layer, Modal};
use iced_kit::Theme;
use iced_test::Simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Dismiss,
}

fn page<'a>() -> Element<'a, Message, Theme> {
    container(text("The page"))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Presses and releases the left button where the cursor points.
fn click(simulator: &mut Simulator<'static, Message, Theme>, at: Point) {
    simulator.point_at(at);
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonPressed(Button::Left))]);
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonReleased(Button::Left))]);
}

fn messages_after_click(element: Element<'static, Message, Theme>, at: Point) -> Vec<Message> {
    let mut simulator =
        Simulator::with_size(iced::Settings::default(), Size::new(900.0, 600.0), element);

    simulator
        .snapshot(&Theme::light())
        .expect("the overlay should render");

    click(&mut simulator, at);

    simulator.into_messages().collect()
}

#[test]
fn a_press_beside_the_drawer_dismisses_it() {
    let drawer = Drawer::new(
        "Details",
        column![text("A drawer slides in from an edge.")].spacing(12),
    )
    .side(DrawerSide::Right)
    .on_dismiss(Message::Dismiss);

    // A right-pinned drawer covers roughly the right third; the far left is
    // backdrop.
    let messages = messages_after_click(
        layer(page(), Layer::new().drawer(drawer)),
        Point::new(80.0, 300.0),
    );

    assert_eq!(
        messages,
        vec![Message::Dismiss],
        "a press beside the panel should dismiss"
    );
}

#[test]
fn a_press_on_the_drawer_panel_does_not_dismiss() {
    let drawer = Drawer::new(
        "Details",
        column![text("A drawer slides in from an edge.")].spacing(12),
    )
    .side(DrawerSide::Right)
    .on_dismiss(Message::Dismiss);

    // Inside the panel, clear of the body text: an inert part of the surface.
    let messages = messages_after_click(
        layer(page(), Layer::new().drawer(drawer)),
        Point::new(800.0, 450.0),
    );

    assert!(
        messages.is_empty(),
        "a press on the panel must stay: got {messages:?}"
    );
}

#[test]
fn a_press_beside_a_modal_dismisses_it() {
    let modal = Modal::new(
        "Delete project",
        text("This permanently removes the project."),
        Message::Dismiss,
    )
    .cancel("Cancel", Message::Dismiss);

    let messages = messages_after_click(
        layer(page(), Layer::new().modal(modal)),
        Point::new(60.0, 300.0),
    );

    assert_eq!(
        messages,
        vec![Message::Dismiss],
        "a press beside the card should dismiss"
    );
}

#[test]
fn a_press_on_a_modal_card_does_not_dismiss() {
    let modal = Modal::new(
        "Delete project",
        text("This permanently removes the project."),
        Message::Dismiss,
    )
    .cancel("Cancel", Message::Dismiss);

    // The card's centre, on the body text: inert, and formerly a fall-through.
    let messages = messages_after_click(
        layer(page(), Layer::new().modal(modal)),
        Point::new(450.0, 310.0),
    );

    assert!(
        messages.is_empty(),
        "a press on the card must stay: got {messages:?}"
    );
}
