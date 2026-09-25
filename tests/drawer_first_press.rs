//! Checks that an open drawer dismisses on the first backdrop press, in the
//! shape the gallery uses: a `Presence` already showing, and a text field in
//! the body.

use iced::mouse::{self, Button};
use iced::widget::{column, container, text, text_input};
use iced::{Element, Event, Length, Point, Size};
use iced_kit::motion::Presence;
use iced_kit::widgets::overlay::{layer, Drawer, DrawerSide, Layer};
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

/// An open drawer in the gallery's shape: presence showing, a text field in
/// the body, dismissal wired to the backdrop.
fn open_drawer(presence: &Presence) -> Drawer<'_, Message> {
    Drawer::new(
        "Details",
        column![
            text("A drawer slides in from an edge."),
            text_input("Name", ""),
        ]
        .spacing(12),
    )
    .side(DrawerSide::Right)
    .presence(presence)
    .on_dismiss(Message::Dismiss)
}

fn click(simulator: &mut Simulator<'static, Message, Theme>, at: Point) {
    simulator.point_at(at);
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonPressed(Button::Left))]);
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonReleased(Button::Left))]);
}

#[test]
fn the_first_backdrop_press_dismisses_an_open_drawer() {
    iced_kit::motion::set_reduce_motion(true);

    let presence = Presence::new();
    presence.show(true, std::time::Instant::now());
    let presence: &'static Presence = Box::leak(Box::new(presence));

    let mut simulator = Simulator::with_size(
        iced::Settings::default(),
        Size::new(900.0, 600.0),
        layer(page(), Layer::new().drawer(open_drawer(presence))),
    );

    simulator
        .snapshot(&Theme::light())
        .expect("the drawer should render");

    // The far left, well clear of a right-pinned panel.
    click(&mut simulator, Point::new(80.0, 300.0));

    let messages: Vec<_> = simulator.into_messages().collect();
    assert_eq!(
        messages,
        vec![Message::Dismiss],
        "one press beside the panel should be one dismissal: got {messages:?}"
    );
}
