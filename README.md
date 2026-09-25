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
| Motion | `instant`, `fast`, `normal`, `slow`, and the `enter`/`exit`/`move` curves |

Build a theme from tokens when you need a custom palette:

```rust
use iced_kit::{Theme, Tokens};

let mut tokens = Tokens::light();
tokens.colors.primary = iced::Color::from_rgb8(0x6d, 0x28, 0xd9);
let theme = Theme::from_tokens(tokens);  // mode is inferred from the background
```

## Components

> 中文使用文档（含浮层托管、`trigger` 锚点、`Presence` 离场等核心概念的讲解与逐组件
> 示例）：[docs/COMPONENTS.zh-CN.md](docs/COMPONENTS.zh-CN.md)

**Form** — `button`, `icon_button`, `ButtonGroup`, `DropdownButton`, `Toggle`,
`ToggleGroup`, `text_input`, `password`, `text_area`, `input_group`, `select`,
`searchable_select`, `combobox` (searchable, multi-select, with a filtered
panel), `checkbox`, `radio`, `switch`, `slider`, `number_input`, `otp_input`,
`calendar`, `date_picker` (a month grid with ranges, presets and a chosen first
day of the week), `color_picker` (a saturation/value square, a hue strip and
swatches)

**Form layout** — `form`/`field` arrange labelled controls into a form:
vertical or horizontal label placement, a `columns` grid with
`col_span`/`col_start`/`col_end` placement, `required` markers, muted
`description` lines, and a trailing `footer` for the form's actions

**Display** — `card`, `group_box`, `divider`, `vertical_divider`,
`horizontal_separator` (labelled and dashed rules), `badge` (label, dot, count
with a maximum, and icon forms), `progress`, `alert` (with a banner form and a
close button), `empty_state`, `label`, `tag`, `avatar_group` (overlapping
avatars with an overflow chip), `label_builder` (masked values and search
highlights), `description_list` (label/value rows in one or several columns),
`link`, `clipboard_button`, `rating`, `tree` (an expandable hierarchy over
caller-owned state)

**Feedback** — `spinner` (arc and dots), `ring_progress`, `skeleton`,
`skeleton_list_item`, `skeleton_table`, `collapsible`, `shimmer` (text, blocks
and any element, with a configurable sweep)

**Data** — `list`, `ListItem`, `VirtualList` (variable-height virtualization),
`DataTable` (sortable, virtualized, with heading groups, pinned columns and a
skeleton loading state), `Markdown`

**Charts** — `LineChart`, `AreaChart` (overlaid or stacked), `BarChart`
(grouped/stacked, four orientations), `PieChart` (pie or donut), `RadarChart`,
`CandlestickChart`, `SankeyChart`

**Typography** — `heading`, `paragraph`, `muted_text`, `code`, `kbd`,
`shortcut`, `avatar`, `avatar_with_name`

**Chat** — `bubble` (seven surfaces: filled, secondary, muted, tinted, outline,
destructive and ghost, with reactions in a pill that rides over the bubble's
edge), `message` (an avatar or a numbered slot, a header, a content surface and a
footer, aligned to either edge), `message_group`, `marker` (plain, separator and
bordered forms, loading with a spinner or a shimmer), `attachment` (five upload
statuses, a media preview, a title and description, an action row, at five size
steps) and `message_scroller` (virtualized, tail-following, with a bottom fade
and a jump-to-latest control)

**Navigation** — `tabs` (underline, tab, outline, pill and segmented variants,
with sizes and icon slots), `accordion` (multiple-open, bordered, sized, with
per-section icons), `breadcrumb`, `stepper` (horizontal or vertical, marking
what is done), `app_menu_bar`, `pagination`, `carousel` (horizontal or
vertical, with previous/next controls, a dot indicator, looping, arrow keys and
drag-to-snap), `Sidebar` (a collapsible panel with a header, grouped menus and
a footer; `Icon`, `Offcanvas` and `None` collapsing modes, an animated width
transition, a draggable width, submenus, badges and collapsed-state tooltips)

**Shell** — `TitleBar` (with window controls), `Resizable` (draggable split
panes), and — behind the `dock` feature — a full docking layout with draggable
tabs, nested splits, drop targets, collapsible edge docks and panel zoom

**Settings** — `Settings` (a panel with a searchable sidebar, user-draggable to
resize), `SettingPage`, `SettingGroup`, `SettingItem` and `SettingField`

**Icons** — the whole [Lucide](https://lucide.dev) set as `IconName`, drawn
through the bundled icon font. Components draw from it throughout, including a
title bar's window controls (`─`/`□`/`✕` are `IconName::Minus`/`Square`/`X`)

**Overlays** — `Modal`, `Dialog`, `AlertDialog`, `Drawer`, `sheet` (a drawer
that stops below the window's title bar), `Toast`/`Toasts`, `Dropdown`/`MenuItem`,
`ContextMenu`, `Popover`, `HoverCard` (opens on hover), `tooltip` (optionally
naming a keyboard shortcut). A dialog body can also be assembled by hand from
`DialogHeader`, `dialog_title`, `dialog_description`, `DialogContent` and
`DialogFooter`, and a status strip from `status_bar`. `trigger` wraps any
trigger so the application can learn where the pressed trigger sits — the
anchor a hosted panel needs, reported ahead of the toggle message

**Motion** — surfaces animate as they arrive: a drawer slides in from its edge, a
dialog rises, a dropdown drops, a toast comes in from the corner it sits in. The
tabs indicator glides between tabs, an accordion panel grows open, a progress bar
eases to its value, and a skeleton breathes while it waits. `Presence` is the
piece an application owns to keep a surface mounted while it leaves, since iced
cannot animate something that has stopped being built. See
[THEMING.md](THEMING.md#motion) for the tokens, and for what iced's renderer does
and does not allow.

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

An anchor like the `x, y` above has to come from somewhere, and iced has no
API that tells a widget where it sits. `trigger` fills that in: it wraps any
trigger and publishes the trigger's window-space `Rectangle` the moment a
press lands inside it — before the toggle message, so a panel opens anchored
to the press that opened it. The gallery's combobox, date picker and color
picker panels are hosted exactly this way.

`Modal` carries the common case: a title, an optional line of description under
it, and action buttons. `AlertDialog` is the same surface with the defaults an
interrupting message wants — a tone that picks its icon, centred buttons, and
no close button:

```rust
use iced_kit::widgets::{AlertDialog, AlertTone};

AlertDialog::new()
    .tone(AlertTone::Danger)
    .title("Delete project")
    .description("This permanently removes every file in it.")
    .confirm()
    .on_confirm(Message::Delete)
    .on_cancel(Message::Close)
```

For a body the two of them cannot express, the dialog's parts are public:
`DialogHeader` holds a title and description, `DialogContent` is the body, and
`DialogFooter` the actions. Pass an empty title to `Modal::new` so it draws no
header of its own, and lay the card out yourself.

Note that Escape does not close a dialog: an iced widget sees no keystroke on
its own, so the application listens for the key in its `subscription` and emits
whatever message it gave `Modal::new` as `on_dismiss`.

Every component is available both as a module and as a constructor function,
matching how `iced::widget` exposes its own widgets:

```rust
use iced_kit::prelude::*;

button("Save").primary().on_press(Message::Save)
text_input::<Message>("Email", &self.email).on_input(Message::EmailChanged)
badge("Active", Tone::Success)
alert("Saved", "Your changes were written.", Tone::Success)
```

## Text inputs

Every field draws one border around its whole box, so a prefix, a suffix, a
clear button and a spinner all sit inside it rather than beside it. The frame
owns the border, the validation state and the focus ring; the control inside it
draws none of its own.

```rust
text_input::<Message>("you@example.com", &self.email)
    .label("Email")
    .on_input(Message::EmailChanged)
```

A field is one of four shapes:

- **`text_input`** — a single-line field. `prefix`, `suffix`, `label`, `error`,
  `invalid`, `disabled`, `readonly`, `loading`, `clearable`, `password` with an
  `on_mask_toggle` eye button, and four size steps.
- **`text_area`** — a multi-line one, with `height`, `min_height`, `label`,
  `error` and the same state flags.
- **`number_input`** — a spin button: the value and both steppers share one
  border, and a stepper is left inert at the end of the range it would leave.
- **`otp_input`** — a row of one-character boxes, optionally `masked` and split
  into `groups`.

`input_group` composes a field with addons on any of four sides. Inline addons
sit in the row beside the value; block addons span the width above or below it:

```rust
input_group()
    .input(text_input::<Message>("Query", &self.query).on_input(Message::QueryChanged))
    .addon(addon().push(label("https://")))
    .addon(
        addon()
            .align(AddonAlignment::InlineEnd)
            .push(group_button::<Message>("Go").on_press(Message::Search)),
    )
```

Two things are worth knowing about the implementation. An invalid field keeps
its error border while focused, because focus is not new information and a focus
ring would hide the one signal the user has to act on. And icons and the spinner
inside a field inherit the field's text color rather than reading it from the
theme's SVG catalog, which is why they stay legible on every background.

iced 0.14 has no accessibility tree and no read-only text input, so `label`
draws a visible label rather than an ARIA name, and `readonly` withholds the
edit handler — the field keeps its normal look and value but loses the caret.

## Icons

Icons come from the bundled [Lucide](https://lucide.dev) font, so every icon is
a name rather than a file:

```rust
use iced_kit::icons::IconName;
use iced_kit::widgets::{button, Icon};

button::<Message>("Find")
    .icon(Icon::new(IconName::Search))
    .on_press(Message::Search)
```

`IconName` is the font's own enum, so a variant is the Lucide name in upper
camel case and can be looked up directly on lucide.dev. `Icon::new` also takes
an `svg::Handle`, for art the font does not carry.

The font is registered on first use, so nothing is required of the application.
Passing `iced_kit::icons::LUCIDE_FONT_BYTES` to `iced::application(..).font(..)`
instead loads it at startup; both work together, since the font system ignores a
font it already has.

## Group boxes

`group_box` is a titled surface for grouping related content, in three variants:
`Normal` (a plain surface), `Fill` (a muted one, for a group inset inside
another) and `Outline` (a border with no fill).

```rust
group_box::<Message>()
    .title(label("Appearance"))
    .description("How the app looks.")
    .push(text("Theme"))
```

## Settings

`Settings` builds an application's preferences screen: a searchable sidebar of
pages, each a column of titled groups, each group a list of labelled controls.
It follows the reference implementation's structure — page, group, item, field —
closely enough that a screen built for `gpui-kit` maps over one level at a time.

```rust
use iced_kit::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings, SettingsState,
};

fn view(app: &App) -> Element<'_, Message, Theme> {
    Settings::new(&app.settings)
        .on_event(Message::Settings)
        .on_reset(Message::SettingsReset)
        .page(
            SettingPage::new("General").group(
                SettingGroup::new()
                    .title("Updates")
                    .item(
                        SettingItem::new("Automatic updates")
                            .description("Install new versions in the background.")
                            .keywords(["auto", "upgrade"])
                            .field(SettingField::switch(
                                app.auto_update,
                                Message::AutoUpdate,
                            )),
                    ),
            ),
        )
        .into()
}
```

A field is one of six shapes: `switch`, `checkbox`, `text`, `number` (a stepper
over an inclusive range), `select` (a dropdown of value/label pairs) and
`custom` (any element). A field with a `default_value` can be reset, and the
reset control appears only while something actually differs from its default.

The panel's own view state — the selected page, the search query, the last
chosen group — lives in `SettingsState`, which the caller owns as it owns every
other value. `SettingsState::apply` does the bookkeeping, so an application
forwards the event and handles the one case that needs a task:

```rust
Message::Settings(event) => {
    app.settings.apply(event);

    // A group click asks to scroll to it. Its position is only known once iced
    // has laid the page out, so it comes back as a task.
    match app.settings.take_pending_scroll() {
        Some((page, group)) => Settings::<Message>::scroll_to_group(page, group),
        None => Task::none(),
    }
}
```

Searching matches titles, descriptions and keywords, case-insensitively.
Groups and pages with no match drop out of both the content and the sidebar, so
a query that matches nothing leaves nothing to click; clearing it restores the
full panel.

Three things differ from the reference, all for reasons iced makes unavoidable:

- **Nothing is bound to a global store.** `gpui-kit` reads and writes setting
  values through the app's global state and downcasts the type at render time.
  iced has no such store, and its `Element` is already type-erased, so a field
  is built from the current value and a message constructor instead. As a
  benefit, the runtime type erasure and the `TypeId` dispatch disappear.
- **The sidebar is not resizable.** iced's split panes resize proportionally
  rather than within a pixel range, so `Settings::sidebar_width` is fixed.
- **Stacking is the application's decision.** The reference picks between
  side-by-side and stacked from the panel's own width. Rebuilding the pages
  inside iced's width-aware widget is not possible, because a rendered element
  borrows the pages rather than owning them — so the application reports its
  window width with `Settings::stacked`, against
  `iced_kit::setting::STACKED_LAYOUT_MAX_WIDTH`.

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

The docking machinery starts as a code-level port of `iced_dock` (MIT), inlined
under `src/dock/` so the crate ships without a second dependency — upstream is not
published to crates.io. See [`NOTICE`](NOTICE) for the attribution. On top of it,
the region model, panel zoom, panel chrome and workspace persistence follow the
dock in gpui-kit, so a workspace built here behaves the way the reference does.

`src/widgets/dock.rs` is the design-system layer over it: the layout logic,
dragging and focus tracking are the port's, while the styling comes from this
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

### Regions, zoom and persistence

A workspace is described as a **centre plus edge docks**, not only as a split tree,
so the three regions a user expects — a sidebar, a panel across the bottom — can be
collapsed to a strip and reopened, each remembering its own size:

```rust
use iced_kit::widgets::dock::{self, DockPlacement, LayoutArea, PanelDef};

let area = LayoutArea::new(dock::tabs([PanelDef::new("editor", "main.rs", Panel::Editor)]))
    .dock(DockPlacement::Left, 240.0, dock::tabs([PanelDef::new("files", "Files", Panel::Files)]))
    .dock(DockPlacement::Bottom, 200.0, dock::tabs([PanelDef::new("term", "Terminal", Panel::Terminal)]));

let session = dock::DockSession::from_area(area)?;
```

Each dock has a resize handle on its inner edge, a toggle in the tab bar of the
group beside it, and an optional zoom on the panel it shows. `LayoutArea` compiles
to the same centre tree the simpler `LayoutTree` did, so a layout written against
one keeps working.

Save and restore the whole workspace with `DockSession::capture` and
`restore`. Panels are keyed by the string in their `PanelDef`, so a saved file
outlives a Rust type being renamed, and a key nothing rebuilds still restores as an
"unknown panel" that keeps its own payload — saving again preserves a layout written
by a build that knew more than this one.

### Panel chrome

A panel's title, toolbar and menu come from a `PanelPresentation` the application
implements. Every method has a default that draws nothing, so a dock whose panels
are plain implements none of them:

```rust
impl dock::PanelPresentation<Panel, Message> for Panels {
    fn title(&self, panel: Panel) -> Option<Element<'static, Message>> {
        Some(row![icon(panel.icon()), text(panel.title())].into())
    }

    fn toolbar(&self, panel: Panel) -> Vec<Element<'static, Message>> {
        vec![icon_button(IconName::Save).into()]
    }
}
```

`PanelStyle::Auto`, the default, draws a plain title bar for a group holding one
panel and a tab strip for more; `PanelStyle::TabBar` always draws the strip.

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
