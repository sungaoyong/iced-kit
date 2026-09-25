//! Checks that a [`trigger`](iced_kit::widgets::overlay::trigger) reports
//! where the pressed trigger sits: the anchor arrives before whatever the
//! content publishes, the reported rectangle is the trigger's own rather than
//! its container's, a press outside the trigger reports nothing, and a right
//! press reports the anchor without activating the content.
//!
//! # Why the order is asserted
//!
//! The whole point of the wrapper is the handshake with the toggle: the
//! application records the anchor message, then processes the toggle, so a
//! panel opening is anchored to the press that opened it. A wrapper that
//! published after its content would be useless in exactly the way that is
//! hardest to notice — the panel would open at the previous press's place.

use iced::mouse::{self, Button};
use iced::widget::container;
use iced::{Alignment, Element, Event, Length, Point, Size};
use iced_kit::widgets::{button, trigger};
use iced_kit::Theme;
use iced_test::Simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    /// Where the pressed trigger sits.
    Anchored(iced::Rectangle),
    /// The trigger's own intent.
    Toggled,
}

/// A trigger hugging the padding origin, clear of the edges.
fn view<'a>() -> Element<'a, Message, Theme> {
    container(trigger(
        button("Press me").on_press(Message::Toggled),
        Message::Anchored,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(40)
    .align_x(Alignment::Start)
    .align_y(Alignment::Start)
    .into()
}

fn simulator() -> Simulator<'static, Message, Theme> {
    Simulator::with_size(iced::Settings::default(), Size::new(400.0, 300.0), view())
}

/// Presses and releases the left button where the cursor points.
fn click_at(simulator: &mut Simulator<'static, Message, Theme>, at: Point) {
    simulator.point_at(at);
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonPressed(Button::Left))]);
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonReleased(Button::Left))]);
}

/// The anchor the simulator recorded, if the sequence of messages is one a
/// press inside the trigger can produce.
fn anchored_rect(messages: &[Message]) -> iced::Rectangle {
    match messages {
        [Message::Anchored(rect), ..] => *rect,
        other => panic!("the press should anchor first, and anchor once: {other:?}"),
    }
}

#[test]
fn a_press_reports_the_trigger_bounds_before_the_toggle() {
    let mut simulator = simulator();

    // A point inside the trigger: the trigger starts at the padding origin,
    // so a few pixels in is its body whatever the label's width.
    let press_at = Point::new(48.0, 48.0);
    click_at(&mut simulator, press_at);

    let messages: Vec<_> = simulator.into_messages().collect();
    let rect = anchored_rect(&messages);

    assert_eq!(messages.len(), 2, "one press, one anchor, one toggle");
    assert_eq!(messages[1], Message::Toggled, "the toggle follows");
    assert!(
        rect.contains(press_at),
        "the anchor should cover the press: {rect:?}"
    );
    assert!(
        rect.width < 400.0 && rect.height < 300.0,
        "the anchor is the trigger's rectangle, not its container's: {rect:?}"
    );
}

#[test]
fn a_press_outside_the_trigger_reports_nothing() {
    let mut simulator = simulator();

    // The far corner of the window, well clear of the 40px padding origin.
    click_at(&mut simulator, Point::new(360.0, 260.0));

    let messages: Vec<_> = simulator.into_messages().collect();
    assert!(
        messages.is_empty(),
        "a press that missed the trigger must anchor nothing: got {messages:?}"
    );
}

#[test]
fn a_right_press_anchors_without_activating() {
    let mut simulator = simulator();

    // A context menu opens on a right press like a panel opens on a left one,
    // so the anchor reports — but a button does not act on a right press.
    simulator.point_at(Point::new(48.0, 48.0));
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonPressed(Button::Right))]);
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonReleased(Button::Right))]);

    let messages: Vec<_> = simulator.into_messages().collect();
    let rect = anchored_rect(&messages);

    assert_eq!(messages.len(), 1, "a right press anchors and nothing more");
    assert!(
        rect.contains(Point::new(48.0, 48.0)),
        "the anchor should cover the press: {rect:?}"
    );
}
