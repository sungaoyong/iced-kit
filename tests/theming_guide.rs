//! Compile-checks the patterns documented in THEMING.md.
//!
//! A theming guide whose examples do not compile is worse than none, so each
//! pattern in the guide has a counterpart here.

use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Element, Task};
use iced_kit::theme::catalog;
use iced_kit::{Size, Theme, Tokens};

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
}

#[derive(Debug, Default)]
struct App {
    dark: bool,
}

#[test]
fn starting_from_a_preset() {
    let light = Theme::light();
    let dark = Theme::dark();

    assert!(!light.is_dark());
    assert!(dark.is_dark());
}

#[test]
fn changing_one_color() {
    let theme = Theme::light().with_primary(Color::from_rgb8(0x6d, 0x28, 0xd9));

    assert_eq!(theme.colors().primary, Color::from_rgb8(0x6d, 0x28, 0xd9));
    // The rest of the palette is untouched.
    assert_eq!(
        theme.colors().background,
        Theme::light().colors().background
    );
}

#[test]
fn changing_the_font() {
    let theme = Theme::light().with_font(iced::Font::MONOSPACE);

    assert_eq!(theme.typography().sans, iced::Font::MONOSPACE);
}

#[test]
fn changing_a_whole_role() {
    let mut tokens = Tokens::light();
    tokens.colors.surface = Color::from_rgb8(0xfa, 0xfa, 0xf9);
    tokens.radius.md = 10;
    tokens.typography.md.size = 15.0;

    let theme = Theme::from_tokens(tokens);

    assert_eq!(theme.colors().surface, Color::from_rgb8(0xfa, 0xfa, 0xf9));
    assert_eq!(theme.radius().md, 10);
    assert_eq!(theme.typography().md.size, 15.0);
}

#[test]
fn the_mode_is_inferred_from_the_background() {
    let mut tokens = Tokens::light();
    tokens.colors.background = Color::from_rgb8(0x10, 0x12, 0x16);

    let theme = Theme::from_tokens(tokens);
    assert!(theme.is_dark(), "a dark background must infer dark mode");
}

/// The application wiring the guide shows, compile-checked end to end.
fn main_app() -> iced::Result {
    iced::application(App::default, update, view)
        .theme(|app: &App| {
            if app.dark {
                Theme::dark()
            } else {
                Theme::light()
            }
        })
        .run()
}

// The by-value signature is iced's, so the argument shape is not this
// function's choice.
#[allow(clippy::needless_pass_by_value)]
fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::ToggleTheme => app.dark = !app.dark,
    }
    Task::none()
}

fn view(app: &App) -> Element<'_, Message, Theme> {
    text(if app.dark { "dark" } else { "light" }).into()
}

#[test]
fn the_application_wiring_compiles() {
    // Taking the function pointer proves the signatures line up without
    // actually opening a window.
    let _ = main_app as fn() -> iced::Result;
    let _ = update as fn(&mut App, Message) -> Task<Message>;
    let _ = view as fn(&App) -> Element<'_, Message, Theme>;
}

#[test]
fn reusing_a_catalog_style() {
    let element: Element<'_, Message, Theme> = button("Ghost")
        .style(|theme: &Theme, status| {
            catalog::button_style(theme, catalog::ButtonVariant::Ghost, Size::Sm)(&status)
        })
        .on_press(Message::ToggleTheme)
        .into();

    drop(element);
}

/// The guide shows a `ButtonClass` as the way to reach a button's resolved
/// colors; the pattern has to compile and hand back usable values.
#[test]
fn resolving_a_button_class() {
    use iced_kit::theme::catalog::{ButtonClass, ButtonState, ButtonVariant, Corners};

    let class = ButtonClass {
        variant: ButtonVariant::Primary,
        outline: true,
        corners: Corners {
            top_left: true,
            top_right: false,
            bottom_right: false,
            bottom_left: true,
        },
        ..ButtonClass::default()
    };

    let appearance = class.appearance(&Theme::light(), ButtonState::Normal);

    // An outline button washes its accent onto the surface rather than filling
    // with it, so it must have both a background and a border color.
    assert!(appearance.background.is_some());
    assert!(appearance.border.color.a > 0.0);
    // The corner mask is honoured, which is what lets a group member square off
    // its inner edges.
    assert_eq!(appearance.border.radius.top_right, 0.0);
    assert!(appearance.border.radius.top_left > 0.0);
}

/// A custom palette flows through the button styles without a call site having
/// to restate them.
#[test]
fn a_custom_size_and_rounding_are_honoured() {
    use iced_kit::theme::catalog::{ButtonClass, ButtonRounded, ButtonState};

    let class = ButtonClass {
        size: Size::Custom(52.0),
        rounded: ButtonRounded::Custom(13.0),
        ..ButtonClass::default()
    };

    let appearance = class.appearance(&Theme::light(), ButtonState::Normal);
    assert_eq!(appearance.border.radius.top_left, 13.0);

    // A custom size carries geometry but no typographic intent of its own.
    assert_eq!(Size::Custom(52.0).text(), Size::Md.text());
    assert_eq!(Size::Custom(52.0).height(), 52.0);
}

#[test]
fn replacing_a_widget_style() {
    let element: Element<'_, Message, Theme> = button("Danger")
        .style(|theme: &Theme, _status| button::Style {
            background: Some(Background::Color(theme.colors().destructive)),
            text_color: theme.colors().destructive_foreground,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 2.0.into(),
            },
            ..button::Style::default()
        })
        .on_press(Message::ToggleTheme)
        .into();

    drop(element);
}

#[test]
fn reusing_a_container_style() {
    let element: Element<'_, Message, Theme> = container(text("In a card"))
        .class(Box::new(catalog::card) as iced::widget::container::StyleFn<'_, Theme>)
        .into();
    drop(element);

    let element: Element<'_, Message, Theme> = container(text("Muted"))
        .class(Box::new(catalog::muted) as iced::widget::container::StyleFn<'_, Theme>)
        .into();
    drop(element);
}

#[test]
fn component_metrics_are_functions_not_tokens() {
    // The guide states these live outside the token set; keep that true.
    #[cfg(feature = "dock")]
    {
        assert!(iced_kit::widgets::dock::tab_bar_height() > 0.0);
    }

    assert!(iced_kit::widgets::resizable::min_pane_size() > 0.0);
    assert!(iced_kit::widgets::resizable::splitter_width() > 0.0);
}
