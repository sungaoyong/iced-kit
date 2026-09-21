# Theming

iced-kit ships its own `Theme` type rather than extending `iced::Theme`. This
is possible because iced 0.14's widgets are generic over their theme
(`Button<'a, Message, Theme = iced::Theme>`), and `Program::Theme` only has to
satisfy `theme::Base`. Carrying a real token set on the theme type is what lets
both iced-kit components and stock iced widgets read the same design tokens.

Everything below is compile-checked in the test suite; the patterns are not
aspirational.

## The token set

`Tokens` has four groups. Nothing in them names a component — a component asks
for `colors.primary`, never for "the button color" — so restyling one component
cannot silently change another.

| Group | Fields |
| --- | --- |
| `colors` | `background`, `foreground`, `surface`, `surface_foreground`, `primary`, `primary_foreground`, `secondary`, `secondary_foreground`, `muted`, `muted_foreground`, `accent`, `accent_foreground`, `destructive`, `destructive_foreground`, `warning`, `success`, `info` with their `*_foreground` pairs, `link`, `link_hover`, `border`, `input`, `ring`, `selection` |
| `radius` | `none`, `sm`, `md`, `lg`, `xl`, `full` |
| `spacing` | `xxs`, `xs`, `sm`, `md`, `lg`, `xl`, `xxl` |
| `typography` | `sans`, `mono`, and the `xs`/`sm`/`md`/`lg`/`xl` scale, each a `TextStyle { size, line_height }` |

Every `*_foreground` is meant to be legible on top of the color it is named
after. The test suite asserts WCAG contrast ratios on those pairs in both
palettes, so a custom palette that breaks them fails the build rather than
shipping unreadable text.

`Size` (`Sm`, `Md`, `Lg`) is separate from the tokens: it ties a control's
height, horizontal padding and text step together, so a size change stays
proportional.

## Starting from a preset

```rust
use iced_kit::Theme;

let theme = Theme::light();            // or Theme::dark()
```

`Theme::from_tokens(tokens)` infers the mode from the background's luminance, so
a custom palette does not have to declare whether it is dark:

```rust
use iced::Color;
use iced_kit::{Theme, Tokens};

let mut tokens = Tokens::light();
tokens.colors.background = Color::from_rgb8(0x10, 0x12, 0x16);

let theme = Theme::from_tokens(tokens);
assert!(theme.is_dark());              // inferred, not declared
```

## Changing one color

When the palette is fine and only the brand color differs, override it directly:

```rust
use iced::Color;
use iced_kit::Theme;

let theme = Theme::light().with_primary(Color::from_rgb8(0x6d, 0x28, 0xd9));
```

`with_font` swaps the sans-serif family for the whole type scale:

```rust
let theme = Theme::light().with_font(iced::Font::MONOSPACE);
assert_eq!(theme.typography().sans, iced::Font::MONOSPACE);
```

## Changing a whole role

For anything beyond a single color, edit the token set. It is plain data with
public fields, so there is no builder to learn:

```rust
use iced::Color;
use iced_kit::{Theme, Tokens};

let mut tokens = Tokens::light();
tokens.colors.surface = Color::from_rgb8(0xfa, 0xfa, 0xf9);
tokens.radius.md = 10;                 // every `md` radius in the UI
tokens.typography.md.size = 15.0;      // body text

let theme = Theme::from_tokens(tokens);
```

Changing a radius or spacing token moves every component that uses it. This is
the intended way to reshape the design language; reach for per-widget overrides
only when a single call site should differ.

## Wiring the theme into an application

Name iced-kit's `Theme` as the application's theme type and every widget —
including stock iced ones — picks up the tokens:

```rust
use iced::{Element, Task};
use iced_kit::Theme;

#[derive(Debug, Clone)]
enum Message { ToggleTheme }

#[derive(Debug, Default)]
struct App { dark: bool }

fn main() -> iced::Result {
    iced::application(App::default, update, view)
        .theme(|app: &App| if app.dark { Theme::dark() } else { Theme::light() })
        .run()
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::ToggleTheme => app.dark = !app.dark,
    }
    Task::none()
}

fn view(app: &App) -> Element<'_, Message, Theme> {
    iced::widget::text(if app.dark { "dark" } else { "light" }).into()
}
```

Returning `Theme` rather than `Option<Theme>` is what makes the token set
available to every widget in the tree. Returning `None` would fall back to
iced's built-in theme and the tokens would be unused.

## Overriding one widget

The `Catalog` implementations on `Theme` are the shared styling functions. A
call site can reuse one, or replace it entirely:

```rust
use iced::widget::button;
use iced_kit::theme::catalog;
use iced_kit::{Theme, Size};

// Reuse the crate's own style, with a variant the call site picks.
let widget = button("Ghost")
    .style(|theme: &Theme, status| {
        catalog::button_style(theme, catalog::ButtonVariant::Ghost, Size::Sm)(&status)
    });

// Or replace it outright, still reading the tokens so the widget stays
// consistent with a custom palette.
use iced::{Background, Border, Color};

let widget = button("Danger")
    .style(|theme: &Theme, _status| button::Style {
        background: Some(Background::Color(theme.colors().destructive)),
        text_color: theme.colors().destructive_foreground,
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 2.0.into() },
        ..button::Style::default()
    });
```

Read colors from the passed-in `theme` rather than closing over constants: that
is what keeps an override correct when the palette changes.

A button's appearance has more inputs than a variant and a size — outline,
selection, loading, rounding, and which corners a group member rounds. Those
live in `catalog::ButtonClass`, which resolves to a `ButtonAppearance` per
state:

```rust
use iced_kit::theme::catalog::{ButtonClass, ButtonState, Corners};

let class = ButtonClass {
    variant: catalog::ButtonVariant::Primary,
    outline: true,
    corners: Corners { top_left: true, top_right: false, bottom_right: false, bottom_left: true },
    ..ButtonClass::default()
};

// Resolve it directly when you need the colors rather than a widget style —
// to tint an icon, say.
let appearance = class.appearance(&Theme::light(), ButtonState::Normal);
```

`crate::widgets::Button` builds exactly this struct from its builder methods, so
a custom control can reuse the same palette without reimplementing it.

A field's appearance works the same way. Focus, validation and the disabled
state are independent, so they are carried as separate flags rather than folded
into one enum — a disabled field can still be showing a validation error, and
the frame has to resolve that combination rather than pick one:

```rust
use iced_kit::theme::catalog::{FieldAppearance, FieldState};

let state = FieldState { focused: true, invalid: true, ..FieldState::default() };
let appearance = FieldAppearance::resolve(&Theme::light(), state);

// The error border wins over the focus ring: focus is not new information when
// something is already wrong, and a ring would hide the signal to act on.
assert_eq!(appearance.border, Theme::light().colors().destructive);
```

`appearance.into_text_input_style()` and `into_text_editor_style()` turn it into
the style closure iced's controls expect, which is how an outer decision — the
caller marked the field invalid; a group owns the frame — reaches a control
whose own catalog can only see its own status.

Container styles are reusable the same way:

```rust
use iced_kit::theme::catalog;

let card = iced::widget::container(content)
    .class(Box::new(catalog::card) as iced::widget::container::StyleFn<'_, Theme>);
```

`catalog::card` and `catalog::muted` are the two surfaces the crate uses for
raised panels and inert regions.

## What a theme does not control

- **Component metrics.** Tab bar heights, splitter widths and pane minimums are
  functions, not tokens (`dock::tab_bar_height()`, `resizable::min_pane_size()`).
  They are set through the relevant builder.
- **Layout.** Nothing in the token set affects sizing beyond text and control
  height; containers and spacing are the application's business.
- **The dock's internal composition.** The ported dock code lays out its own
  tabs and splitters. This crate supplies the styling (see
  `widgets::dock::style`) and the metrics, not the layout.
- **The icons.** Lucide glyphs are a font, so they take their color from the
  text they sit beside rather than from a token of their own. To recolor one,
  set the color on the control containing it. The set itself is fixed: an icon
  outside Lucide is passed to `Icon::new` as an `svg::Handle`.

## Watching for regressions

The test suite covers theming from two directions:

- **Token maths.** Contrast ratios on every foreground/background pair in both
  palettes, monotonic radius and size scales, and that a control is tall enough
  for its own text.
- **Rendered output.** `tests/render.rs` renders each component through iced's
  simulator and compares against reference PNGs in `tests/snapshots/`, including
  a dark-mode pass. A palette change that breaks a layout shows up as a failed
  comparison rather than a surprise in a running application. Delete a reference
  file to regenerate it after an intentional change.
