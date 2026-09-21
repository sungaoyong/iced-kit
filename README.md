# iced-kit

A shadcn/ui-flavored component library and design system for
[iced](https://iced.rs/) 0.14, porting the visual language of
[gpui-kit](https://github.com/longbridge/gpui-kit).

The gpui-kit component set cannot be wrapped for iced — gpui's `IntoElement`
tree and iced's `Widget` layout/draw pipeline share no layer, so a gpui element
cannot be drawn into an iced window. This crate instead **reimplements** the
gpui-kit token model and component API on iced's own widget system.

## How it works

iced 0.14 widgets are generic over their theme type (`Button<'a, Message,
Theme = iced::Theme>`), and `Program::Theme` only has to satisfy `theme::Base`.
iced-kit ships its own `Theme` carrying the full semantic token set and
implements the upstream widget `Catalog` traits for it. The payoff: stock iced
widgets *and* iced-kit components render with one consistent look.

```rust
use iced_kit::Theme;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(App::theme)   // returns iced_kit::Theme
        .run()
}
```

## Design tokens

Token names follow shadcn/ui so they map one-to-one onto gpui-kit's
`SemanticThemeTokens`:

| Group | Tokens |
| --- | --- |
| Colors | `background`, `foreground`, `surface`, `primary`, `secondary`, `muted`, `accent`, `destructive`, `warning`, `success`, `info`, `link`, `border`, `input`, `ring`, `selection`, each with its `*_foreground` pair where applicable |
| Radius | `none`, `sm`, `md`, `lg`, `xl`, `full` |
| Spacing | `xxs` … `xxl` |
| Typography | `sans`, `mono`, and the `xs`/`sm`/`md`/`lg`/`xl` scale |
| Size | `Xs`, `Sm`, `Md`, `Lg`, or `Custom(px)` — control height, padding, icon and text step together |

Build a theme from tokens when you need a custom palette:

```rust
use iced_kit::{Theme, Tokens};

let mut tokens = Tokens::light();
tokens.colors.primary = iced::Color::from_rgb8(0x6d, 0x28, 0xd9);
let theme = Theme::from_tokens(tokens);  // mode is inferred from the background
```

## Components

**Form** — `button`, `icon_button`, `ButtonGroup`, `DropdownButton`, `Toggle`,
`ToggleGroup`, `text_input`, `password`, `text_area`, `select`, `checkbox`,
`radio`, `switch`, `slider`, `number_input`, `otp_input`

**Display** — `card`, `divider`, `vertical_divider`, `badge`, `progress`,
`alert`, `empty_state`, `label`, `tag`

**Feedback** — `spinner` (arc and dots), `ring_progress`, `skeleton`,
`skeleton_list_item`, `skeleton_table`

**Data** — `list`, `ListItem`, `VirtualList` (variable-height virtualization),
`DataTable` (sortable, virtualized), `Markdown`

**Charts** — `LineChart`, `AreaChart` (overlaid or stacked), `BarChart`
(grouped/stacked, four orientations), `PieChart` (pie or donut), `RadarChart`,
`CandlestickChart`, `SankeyChart`

**Typography** — `heading`, `paragraph`, `muted_text`, `code`, `kbd`,
`shortcut`, `avatar`, `avatar_with_name`

**Navigation** — `tabs`, `accordion`, `pagination`

**Shell** — `TitleBar` (with window controls), `Resizable` (draggable split
panes), and — behind the `dock` feature — a full docking layout with draggable
tabs, nested splits and drop targets

**Overlays** — `Modal`, `Dialog`, `Drawer`, `Toast`/`Toasts`,
`Dropdown`/`MenuItem`, `ContextMenu`, `Popover`, `tooltip`

## Buttons

`Button` wraps iced's own button so that variant, size, outline, selection and
loading state come from the theme rather than from per-call styling. Eleven
variants cover the design system's signals:

| Variant | Reads as |
| --- | --- |
| `Default` | A neutral action: surface fill, input-colored border |
| `Primary` / `Secondary` | The main call to action, and a lower-emphasis one |
| `Danger` | A soft destructive action: a red wash with red text |
| `Destructive` | A filled destructive action, for irreversible operations |
| `Warning` / `Success` / `Info` | The remaining tones |
| `Ghost` | Transparent until hovered |
| `Link` | Underlined in every state |
| `Text` | Plain inline text, with no padding or border |
| `Custom(ButtonCustomVariant)` | A caller-supplied palette |

```rust
// A labelled action, a themed variant, and a size all at once.
button("Save").primary().size(Size::Lg).on_press(Message::Save);

// An icon-only button is square, so a toolbar of them lines up.
icon_button::<Message>().icon("✕").ghost().on_press(Message::Close);
```

Three composite controls build on it:

- **`ButtonGroup`** joins several buttons into one segmented control, rounding
  only the outer edges. `on_select` receives the indices of the selected
  members, and the application owns that selection — the group is a view of the
  state, like every other iced component.
- **`DropdownButton`** is a split button: an action half plus a menu trigger.
  The trigger reports a press rather than holding a menu, because iced has no
  window-level z-order; the application owns the open flag and the menu's
  placement through `overlay::Layer`.
- **`Toggle`** and **`ToggleGroup`** are the latched controls. A group can be
  `segmented()`, which joins its members' borders.

Two implementation notes worth knowing:

- **Icons inherit their color.** iced's `svg` widget reads its color from the
  theme rather than from the enclosing button, so an icon inside a primary
  button would be drawn in the wrong color. `Icon` is drawn through a small
  wrapper that takes the color the button set instead, which is what keeps an
  icon and its label matching in every variant.
- **Selection draws a ring.** Shifting the background alone is not enough
  signal: darkening an already near-black `Primary` fill is imperceptible. A
  selected button therefore draws a ring, which is visible on every variant in
  both palettes.

## Virtualized data

A list of ten thousand rows costs ten thousand widget trees per frame with
iced's `scrollable`. `VirtualList` renders only the rows intersecting the
viewport and reserves the rest as space, so the cost tracks what is visible
rather than the size of the data.

```rust
use iced_kit::widgets::{VirtualList, VirtualListState};

fn view<'a>(rows: &'a [Row], state: &'a VirtualListState) -> Element<'a, Message, Theme> {
    VirtualList::new(rows, state, |row, _index| {
        iced_kit::widgets::virtual_list::text_row(row.name.clone(), None)
    })
    // Rows may differ in height; `fixed_row_height` is the cheaper uniform case.
    .row_height(|_row: &Row, index| if index % 3 == 0 { 46.0 } else { 28.0 })
    .height(400.0)
    .on_scroll(Message::Scrolled)
    .into_element()
}
```

`VirtualListState` carries the scroll offset and the measured row heights. The
application owns it and replaces it from `on_scroll`, which is what keeps the
widget stateless and testable; heights are measured as rows come into view and
estimates are corrected as measurements arrive.

`DataTable` builds on the same idea. Its sort keys carry a numeric value
separately from the display text, so a column showing `"1,234"` sorts by
magnitude rather than by string order:

```rust
Column::new("Size", |row: &Row, _index| number_cell(format_bytes(row.size)))
    .width(Width::Fixed(120.0))
    .sortable(|row| SortKey::number(row.size as f64)),
```

## Charts

Ported from gpui-kit's `plot` layer (Apache-2.0) and rebuilt on iced's canvas.
Seven chart types share one infrastructure, so they behave the same way:

```rust
use iced_kit::widgets::plot::{LineChart, LineSeries};

LineChart::new(
    labels,
    vec![
        LineSeries::new("Reads", reads).tone(Tone::Primary),
        LineSeries::new("Writes", writes).tone(Tone::Success),
    ],
)
.height(220.0)
.into_element::<Message>()
```

What the shared layer gives every chart:

- **Hover** — a crosshair (or highlight band, or lifted slice) plus a tooltip
  showing the datum under the cursor. The hit test runs in the canvas program's
  `update`, where the state is mutable, so `draw` only reads the result.
- **Scales** — `ScaleLinear`, `ScaleBand` and `ScalePoint` map data to pixels
  *and back*, which is what makes the hover work. A chart with a fixed axis
  (`force_zero(false)`, `max_value`, `range`) keeps two charts comparable.
- **Axes and grid** — labels are placed before the plot area is drawn, so the
  area reserves exactly the space they need. Large magnitudes abbreviate
  (`1.5k`, `2.5M`).
- **Theme tokens** — every color, including the series palette, comes from the
  theme, so light/dark needs no chart-specific code.

Two design points worth knowing:

- **Line and area charts anchor their axis at zero** by default, so a small
  change in a large value does not read as a dramatic one. Pass
  `force_zero(false)` when the trend itself is the point.
- **Sankey and candlestick colors are settable.** A rising candle is green by
  default, but much of Asia reads a rise as red: `.bullish(..)` and
  `.bearish(..)` invert it.

`Markdown` wraps iced's Markdown widget and derives its settings from the
theme's type scale; it parses once and renders per frame.

## Overlays

iced has no window-level z-order, so overlays are assembled into a `Stack` by
one function, in paint order: page, dropdowns, modal, toasts.

```rust
use iced_kit::widgets::overlay::{self, Layer, Toast, ToastKind, Toasts};

fn view(&self) -> Element<'_, Message, Theme> {
    let page = /* the application's own view */;

    let mut open = Layer::new();

    if self.menu_open {
        open = open.dropdown(Dropdown::new(items).anchor(x, y));
    }

    if self.drawer_open {
        open = open.drawer(
            Drawer::new("Details", body).side(DrawerSide::Right)
                .on_dismiss(Message::CloseDrawer),
        );
    }

    if self.confirm_open {
        open = open.modal(
            Modal::new("Delete project", body, Message::Close)
                .cancel("Cancel", Message::Close)
                .destructive("Delete", Message::Confirm),
        );
    }

    open = open.toasts(
        Toasts::new().push(Toast::new("Saved", ToastKind::Success)),
    );

    overlay::layer(page, open)
}
```

The application owns the state of each overlay; `Layer` only decides how open
ones are painted. A modal's backdrop dims the page and dismisses on click, and
a `Modal` can be made non-dismissible for a dialog that must be answered.

Every component is available both as a module and as a constructor function,
matching how `iced::widget` exposes its own widgets:

```rust
use iced_kit::prelude::*;

button("Save").primary().on_press(Message::Save)
text_input::<Message>("Email", &self.email).on_input(Message::EmailChanged)
badge("Active", Tone::Success)
alert("Saved", "Your changes were written.", Tone::Success)
```

## Documentation

- [`THEMING.md`](THEMING.md) — customizing the token set, per-widget overrides,
  and what a theme deliberately does not control.
- [`PUBLISHING.md`](PUBLISHING.md) — release audit: metadata, licensing,
  attribution, docs.rs, and the one open blocker.

## Running the gallery

```sh
cargo run --example gallery

# or start in dark mode
GALLERY_DARK=1 cargo run --example gallery
```

## Docking (`dock` feature)

Docking is opt-in, because it is a large subsystem:

```toml
iced-kit = { version = "0.1", features = ["dock"] }
```

The docking machinery is a code-level port of `iced_dock` (MIT), inlined under
`src/dock/` so the crate ships without a second dependency — upstream is not
published to crates.io. See [`NOTICE`](NOTICE) for the attribution.

`src/widgets/dock.rs` is the design-system layer over it: the layout logic,
dragging and focus tracking are upstream's, while the styling comes from this
crate's tokens, so a dock's tabs, panes, splitters and drop highlights match
everything else:

```rust
use iced_kit::widgets::dock;

dock::dock::<Panel, Message, iced_kit::Theme, iced::Renderer>()
    .state(session.state())
    .on_event(Message::Dock)
    .style(dock::style)          // tokens, not hard-coded colors
    .content(|panel| view_panel(panel))
    .build()
    .into()
```

The dock types are generic over the theme and renderer, so both are named
explicitly. The theme also implements the dock's own `Catalog`, so a dock picks
up the token-derived style even without the `.style(...)` call. Switching
light/dark recolors the dock with no dock-specific code.

`Resizable` covers the simpler case — a handful of panes separated by draggable
splitters — without requiring a layout tree:

```rust
use iced_kit::widgets::resizable::{Resizable, SplitAxis};

let (state, _panes) = Resizable::<Message>::split_state(2, SplitAxis::Horizontal, 0.3);

Resizable::new(&state)
    .min_size(80.0)   // panes × min_size must fit the area
    .on_resize(Message::Resized)
    .pane(|_pane, index: &usize| view_pane(*index))
    .into_element()
```

## Development

The crate name `iced-kit` is free on crates.io and `cargo package` succeeds;
see [`PUBLISHING.md`](PUBLISHING.md) for the release audit and the remaining
checklist.


```sh
# Features are gated, so every configuration is checked.
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features dock -- -D warnings
cargo clippy --all-targets --features dock-serde -- -D warnings
cargo test
cargo test --features dock
cargo test --features dock-serde
```

The suite has three parts. Unit tests cover theme token maths — including WCAG
contrast checks on every foreground/background pair — and each component's
render path, plus the specific bugs that are easy to reintroduce (for example,
that a `line_height` token reaches iced as absolute pixels rather than as a
multiple of the font size).

`tests/render.rs` renders every component offscreen through iced's simulator and
compares the pixels against the reference PNGs in `tests/snapshots/`. These are
real renderings, so they catch layout faults a widget-tree assertion cannot —
the modal centring bug and the ring-label overflow were both found this way.
Delete a reference file to regenerate it after an intentional visual change.

## Attribution

The design language, token model and component naming come from
[gpui-kit](https://github.com/longbridge/gpui-kit) (Apache-2.0) and
[shadcn/ui](https://ui.shadcn.com/). The token values are the gpui-kit defaults
converted from HSL to sRGB. Implementation patterns were informed by
[iced-astraui](https://github.com/AstraBrew-Labs/iced-astraui) and
[iced-shadcn](https://github.com/FerrisMind/shadcn-rs) (both MIT).

## License

MIT OR Apache-2.0
