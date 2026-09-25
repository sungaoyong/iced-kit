//! Checks that the color panel's saturation/value square and hue strip report
//! colors when pressed, in the shape the gallery hosts them: the panel over a
//! dismissal catcher.

use iced::mouse::{self, Button};
use iced::widget::{container, stack};
use iced::{Color, Element, Event, Length, Point, Size};
use iced_kit::widgets::color_picker_panel;
use iced_kit::widgets::overlay::popover_dismiss_area;
use iced_kit::Theme;
use iced_test::Simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Close,
    ColorChanged(Color),
}

fn hosted_panel<'a>() -> Element<'a, Message, Theme> {
    stack![
        popover_dismiss_area(Message::Close),
        container(
            color_picker_panel::<Message>(Color::from_rgb8(0x33, 0x66, 0x99))
                .on_change(Message::ColorChanged)
                .width(240.0)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::Alignment::Start)
        .align_y(iced::Alignment::Start)
    ]
    .into()
}

fn click(simulator: &mut Simulator<'static, Message, Theme>, at: Point) {
    simulator.point_at(at);
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonPressed(Button::Left))]);
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonReleased(Button::Left))]);
}

#[test]
fn a_press_on_the_sv_square_reports_a_color() {
    let mut simulator = Simulator::with_size(
        iced::Settings::default(),
        Size::new(600.0, 500.0),
        hosted_panel(),
    );

    simulator
        .snapshot(&Theme::light())
        .expect("the panel should render");

    // The square is the panel's first element: 240 wide, 0.6 * 240 tall,
    // inset by the panel's 12px padding. Its bright middle is around (100, 80).
    click(&mut simulator, Point::new(100.0, 80.0));

    let messages: Vec<_> = simulator.into_messages().collect();
    assert!(
        matches!(
            messages.as_slice(),
            [Message::ColorChanged(color), ..] if *color != Color::from_rgb8(0x33, 0x66, 0x99)
        ),
        "a press on the square should report a different color: got {messages:?}"
    );
}

/// The gallery's exact hosting: the panel anchored mid-window over a catcher,
/// padded into place from the trigger's rectangle.
fn anchored_like_gallery<'a>(anchor: Point) -> Element<'a, Message, Theme> {
    stack![
        popover_dismiss_area(Message::Close),
        container(
            color_picker_panel::<Message>(Color::from_rgb8(0x33, 0x66, 0x99))
                .on_change(Message::ColorChanged)
                .width(240.0)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::Alignment::Start)
        .align_y(iced::Alignment::Start)
        .padding(iced::Padding {
            top: anchor.y + 8.0,
            left: anchor.x,
            ..iced::Padding::ZERO
        })
    ]
    .into()
}

#[test]
fn a_press_on_an_anchored_square_reports_a_color() {
    let anchor = Point::new(260.0, 150.0);
    let mut simulator = Simulator::with_size(
        iced::Settings::default(),
        Size::new(900.0, 700.0),
        anchored_like_gallery(anchor),
    );

    simulator
        .snapshot(&Theme::light())
        .expect("the panel should render");

    // The panel body starts at (anchor + 12px padding); the square's bright
    // middle is a third of the way in.
    let square_x = anchor.x + 12.0 + 80.0;
    let square_y = anchor.y + 12.0 + 50.0;

    click(&mut simulator, Point::new(square_x, square_y));

    let messages: Vec<_> = simulator.into_messages().collect();
    assert!(
        matches!(
            messages.as_slice(),
            [Message::ColorChanged(color), ..] if *color != Color::from_rgb8(0x33, 0x66, 0x99)
        ),
        "a press on the anchored square should report a color: got {messages:?}"
    );
}

#[test]
fn a_press_on_the_hue_strip_reports_a_color() {
    let mut simulator = Simulator::with_size(
        iced::Settings::default(),
        Size::new(600.0, 500.0),
        hosted_panel(),
    );

    simulator
        .snapshot(&Theme::light())
        .expect("the panel should render");

    // The strip sits below the square, next to the "Hue" caption.
    click(&mut simulator, Point::new(150.0, 170.0));

    let messages: Vec<_> = simulator.into_messages().collect();
    assert!(
        matches!(messages.as_slice(), [Message::ColorChanged(_), ..]),
        "a press on the strip should report a color: got {messages:?}"
    );
}

#[test]
fn dragging_the_square_reports_colors_along_the_way() {
    let mut simulator = Simulator::with_size(
        iced::Settings::default(),
        Size::new(600.0, 500.0),
        hosted_panel(),
    );

    simulator
        .snapshot(&Theme::light())
        .expect("the panel should render");

    // A drag: press, then a stream of cursor moves, the way a real pointer
    // produces them.
    simulator.point_at(Point::new(40.0, 40.0));
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonPressed(Button::Left))]);

    for (x, y) in [(80.0, 60.0), (140.0, 80.0), (200.0, 100.0)] {
        simulator.point_at(Point::new(x, y));
        let _ = simulator.simulate([Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(x, y),
        })]);
    }

    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonReleased(Button::Left))]);

    let messages: Vec<_> = simulator.into_messages().collect();
    let colors: Vec<_> = messages
        .iter()
        .filter_map(|message| match message {
            Message::ColorChanged(color) => Some(*color),
            Message::Close => None,
        })
        .collect();

    assert!(
        colors.len() >= 4,
        "a drag should report a color on the press and on every move: got {messages:?}"
    );
    assert!(
        colors.windows(2).any(|pair| pair[0] != pair[1]),
        "the colors should differ along the drag: {colors:?}"
    );
}

#[test]
fn a_drag_past_the_square_keeps_reporting_at_the_edge() {
    let mut simulator = Simulator::with_size(
        iced::Settings::default(),
        Size::new(600.0, 500.0),
        hosted_panel(),
    );

    simulator
        .snapshot(&Theme::light())
        .expect("the panel should render");

    // The square spans roughly (12..252, 12..156). Press inside, then drag
    // well past its bottom edge — the pointer leaves the square halfway.
    simulator.point_at(Point::new(120.0, 100.0));
    let _ = simulator.simulate([Event::Mouse(mouse::Event::ButtonPressed(Button::Left))]);

    for (x, y) in [(120.0, 200.0), (120.0, 300.0), (120.0, 400.0)] {
        simulator.point_at(Point::new(x, y));
        let _ = simulator.simulate([Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(x, y),
        })]);
    }

    let messages: Vec<_> = simulator.into_messages().collect();
    let colors: Vec<_> = messages
        .iter()
        .filter_map(|message| match message {
            Message::ColorChanged(color) => Some(*color),
            Message::Close => None,
        })
        .collect();

    assert!(
        colors.len() >= 3,
        "a drag that leaves the square should keep reporting at the edge: got {messages:?}"
    );

    // Every reported color past the bottom edge is the clamped one: full
    // darkness at the square's own hue.
    for color in &colors[1..] {
        let hsv = iced_kit::widgets::color_picker::Hsv::from_color(*color);
        assert!(
            hsv.value <= 0.05,
            "a drag past the bottom edge should clamp to its darkest row: {hsv:?}"
        );
    }
}
