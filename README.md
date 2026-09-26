# iced-kit

A shadcn/ui-flavored component library and design system for
[iced](https://iced.rs/) 0.14, porting the visual language of
[gpui-kit](https://github.com/longbridge/gpui-kit).

The gpui-kit component set cannot be wrapped for iced — gpui's `IntoElement`
tree and iced's `Widget` layout/draw pipeline share no layer, so a gpui element
cannot be drawn into an iced window. This crate instead **reimplements** the
gpui-kit token model and component API on iced's own widget system.

## Preview

Every screenshot below is rendered offscreen by the test suite
(`tests/render.rs`) from the same code an application would run, so it is the
real component, not a mock-up.

**Buttons & toggles**

| Variants | Groups & split buttons | Toggle buttons |
| --- | --- | --- |
| ![Button variants](tests/snapshots/button_variants-wgpu.png) | ![Button groups](tests/snapshots/button_groups-wgpu.png) | ![Toggles](tests/snapshots/toggle_buttons-wgpu.png) |

**Text inputs & pickers**

| Fields | Input groups | Numbers & OTP | Passwords |
| --- | --- | --- | --- |
| ![Text fields](tests/snapshots/text_fields-wgpu.png) | ![Input groups](tests/snapshots/input_groups-wgpu.png) | ![Number and OTP](tests/snapshots/number_and_otp_fields-wgpu.png) | ![Passwords](tests/snapshots/password_fields-wgpu.png) |

| Select | Combobox | Calendar | Color picker |
| --- | --- | --- | --- |
| ![Select](tests/snapshots/select-wgpu.png) | ![Combobox](tests/snapshots/combobox-wgpu.png) | ![Calendar](tests/snapshots/calendar-wgpu.png) | ![Color picker](tests/snapshots/color_picker-wgpu.png) |

**Forms**

| Vertical | Multi-column |
| --- | --- |
| ![Vertical form](tests/snapshots/form_vertical-wgpu.png) | ![Column form](tests/snapshots/form_columns-wgpu.png) |

**Display**

| Display | Group boxes | Tree |
| --- | --- | --- |
| ![Display](tests/snapshots/display-wgpu.png) | ![Group boxes](tests/snapshots/group_boxes-wgpu.png) | ![Tree](tests/snapshots/tree-wgpu.png) |

**Feedback**

| Feedback | Loading overlays |
| --- | --- |
| ![Feedback](tests/snapshots/feedback-wgpu.png) | ![Loading overlays](tests/snapshots/loading_overlays-wgpu.png) |

**Data**

| Virtual list | Data table |
| --- | --- |
| ![Virtual list](tests/snapshots/virtual_list-wgpu.png) | ![Data table](tests/snapshots/data_table-wgpu.png) |

**Charts**

| Line | Area | Bar | Pie |
| --- | --- | --- | --- |
| ![Line chart](tests/snapshots/line_chart-wgpu.png) | ![Area chart](tests/snapshots/area_chart-wgpu.png) | ![Bar chart](tests/snapshots/bar_chart-wgpu.png) | ![Pie chart](tests/snapshots/pie_chart-wgpu.png) |

| Radar | Candlestick | Sankey |
| --- | --- | --- |
| ![Radar chart](tests/snapshots/radar_chart-wgpu.png) | ![Candlestick chart](tests/snapshots/candlestick_chart-wgpu.png) | ![Sankey chart](tests/snapshots/sankey_chart-wgpu.png) |

**Chat**

| Bubbles | Message rail | Attachments |
| --- | --- | --- |
| ![Chat bubbles](tests/snapshots/chat_bubbles-wgpu.png) | ![Message rail](tests/snapshots/chat_message_rail-wgpu.png) | ![Attachments](tests/snapshots/chat_attachments-wgpu.png) |

**Navigation**

| Tabs, accordions & more | Carousel | Sidebar |
| --- | --- | --- |
| ![Navigation](tests/snapshots/navigation-wgpu.png) | ![Carousel](tests/snapshots/carousel-wgpu.png) | ![Sidebar](tests/snapshots/sidebar-wgpu.png) |

**Ribbon** — the same command band pinned to three densities, and `Auto`
degrading a narrow row from the right:

| Full | Compact | Collapsed | Auto (narrow) |
| --- | --- | --- | --- |
| ![Ribbon](tests/snapshots/ribbon-wgpu.png) | ![Ribbon compact](tests/snapshots/ribbon_compact-wgpu.png) | ![Ribbon collapsed](tests/snapshots/ribbon_collapsed-wgpu.png) | ![Ribbon auto narrow](tests/snapshots/ribbon_auto_narrow-wgpu.png) |

**Overlays**

| Modal | Drawer | Toasts | Dropdown |
| --- | --- | --- | --- |
| ![Modal](tests/snapshots/modal-wgpu.png) | ![Drawer](tests/snapshots/drawer-wgpu.png) | ![Toasts](tests/snapshots/toasts-wgpu.png) | ![Dropdown](tests/snapshots/dropdown-wgpu.png) |

| Popover | Context menu | Alert dialog | Tooltip |
| --- | --- | --- | --- |
| ![Popover](tests/snapshots/popover-wgpu.png) | ![Context menu](tests/snapshots/context_menu-wgpu.png) | ![Alert dialog](tests/snapshots/alert_dialog-wgpu.png) | ![Tooltip](tests/snapshots/tooltip-wgpu.png) |

**Shell & docking**

| Dock layout | Resizable | Title bar |
| --- | --- | --- |
| ![Dock](tests/snapshots/dock-wgpu.png) | ![Resizable](tests/snapshots/resizable-wgpu.png) | ![Title bar](tests/snapshots/title_bar-wgpu.png) |

**Settings, typography & icons**

| Settings panel | Typography | Lucide icons |
| --- | --- | --- |
| ![Settings](tests/snapshots/settings_panel-wgpu.png) | ![Typography](tests/snapshots/typography-wgpu.png) | ![Icons](tests/snapshots/named_icons-wgpu.png) |

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
transition, a draggable width, submenus, badges and collapsed-state tooltips),
and `Ribbon` (a tabbed command bar of grouped, multi-size tool buttons that
degrades from the right as the window narrows — see [Ribbon](#ribbon))

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

## Ribbon

A ribbon is a command palette in two bands: a strip of tabs along the top, and
beneath the active tab a row of *groups*, each a boxed cluster of tool buttons
with a small label on its bottom edge. Tools come in a large (full-height,
icon over label) and a small (single-row, icon only or icon beside label)
footprint, and either can carry a ▾ that opens a dropdown of related commands.

Beyond the core tab/group/tool model, the ribbon carries a full set of
SARibbon-style features:

- **Ten built-in themes** — Office 2013, Office 2016 Blue/Green/Dark, Office
  2021 Blue/Green/Dark, Windows 7, and two dark variants. Each defines its own
  accent, tab bar, group background, button hover, and border colors, and is
  switchable at runtime through `RibbonTheme`.
- **Six panel layouts** — Loose/Compact × three-row/two-row/single-row, plus an
  adaptive `Auto` mode. The layout is plain data in `RibbonState`, so a picker
  can switch it live.
- **Quick access bar** — a row of small icon buttons above the tab strip for
  the most-used commands.
- **Galleries** — Office-style labeled grids of visual items (style swatches,
  chart previews) embedded in a group.
- **Contextual tabs** — conditionally shown tabs with their own accent colors,
  rendered beside the regular tabs.
- **Minimized mode** — collapse the ribbon to just the tab strip.
- **Dynamic categories** — add and remove tabs and panels at runtime from the
  application.
- **Split action buttons** — a button whose body runs an action while its ▾
  opens a menu, the reference's action-menu button.

Like every iced-kit component, a ribbon is described entirely as data —
`RibbonTab`s of `RibbonGroup`s of `RibbonItem`s wrapping `RibbonTool`s — and
rebuilt every frame. Which tab is active, which dropdown is open, and the
current theme and layout live in a `RibbonState` the caller holds; the ribbon
reports intent through `on_select` and `on_dropdown_toggle` and never mutates
anything itself.

### Full-window demo

The `main_window` example shows the ribbon in a complete SARibbon-style
application — a tinted title bar hosting the application button, quick access
bar, tabs, theme and layout pickers, and window controls; a central log area;
and a status bar. Run it with `cargo run --example main_window`.

![Main window demo](tests/snapshots/main_window.png)

### Minimal example

```rust
use iced_kit::widgets::ribbon::{
    Ribbon, RibbonGroup, RibbonItem, RibbonState, RibbonTab, RibbonTool,
};

fn view(state: &RibbonState) -> Element<'static, Message, Theme> {
    Ribbon::new()
        .tab(RibbonTab::new("Home").group(
            RibbonGroup::new("Draw")
                .item(RibbonItem::large(RibbonTool::new("／").label("Line")))
                .item(RibbonItem::tool(RibbonTool::new("▢").label("Rectangle"))),
        ))
        .state(state)
        .on_select(Message::Selected)
        .into()
}
```

Two things set it apart:

- **Dropdowns are hosted, not painted.** iced has no window-level z-order, so a
  ribbon draws no floating panel of its own. A ▾ reports the id it wants
  opened; the application builds that panel and hosts it through
  [`overlay::Layer`](#overlays), anchored with [`trigger`](#overlays) exactly as
  it does for the combobox, date-picker and colour-picker panels. Each dropdown
  button wraps itself in a `trigger` so the panel drops beneath the button that
  opened it rather than the whole ribbon.
- **The band degrades instead of overflowing.** When a tab's groups outrun the
  width, they collapse *from the right*, one group at a time, down a ladder of
  densities — full to compact icon columns, then to a title button, then to a
  tight small-icon button — and the row's height shrinks with the shortest
  visible group. That is `CollapseMode::Auto`, the default; pin every group to
  one density instead with `Ribbon::collapse_mode(mode)`, and the active mode
  rides along in `RibbonState` so a density picker can drive it from the app.

## Documentation

- [`THEMING.md`](THEMING.md) — customizing the token set, per-widget overrides,
  and what a theme deliberately does not control.
- [`PUBLISHING.md`](PUBLISHING.md) — release audit: metadata, licensing,
  attribution, docs.rs, and the one open blocker.

## Internationalization (`i18n` feature)

iced-kit provides a lightweight translation layer that plugs into any widget
constructor accepting `impl text::IntoFragment`. The core `i18n` feature adds
the `Translator` trait, the `I18n` context, and the `tr!` macro. Enable
`i18n-fluent` for a ready-made backend using `.ftl` files:

```toml
iced-kit = { version = "0.1", features = ["i18n-fluent"] }
```

```rust
use iced_kit::i18n::{FluentTranslator, I18n};
use iced_kit::prelude::*;

struct App {
    i18n: I18n<FluentTranslator>,
}

impl App {
    fn new() -> Self {
        let translator = FluentTranslator::from_str("en", r#"
save = Save
cancel = Cancel
hello-user = Hello, { $name }!
items-count = { $count ->
    [one] { $count } item
   *[other] { $count } items
}
        "#).expect("valid ftl");
        Self { i18n: I18n::new(translator) }
    }

    fn view(&self) -> Element<'_, Message, Theme> {
        let i18n = &self.i18n;
        column![
            heading(i18n.tr("app-title"), Heading::H2),
            button(tr!("save")).on_press(Message::Save),
            button(tr!("cancel")).on_press(Message::Cancel),
            text(tr!("hello-user", "name" => "Alice")),
            text(tr!("items-count", "count" => 42)),
        ]
        .into()
    }
}
```

Or load from `.ftl` files on disk (one directory per locale, each containing
a `main.ftl`):

```rust
let translator = FluentTranslator::load_from_dir("en", "locales/")?;
```

Reference translations live in [`locales/en/main.ftl`](locales/en/main.ftl)
and [`locales/zh-CN/main.ftl`](locales/zh-CN/main.ftl). Missing keys fall
back to the key string itself, so untranslated text stays visible rather
than blank.

## Running the gallery

```sh
cargo run --example gallery

# or start in dark mode
GALLERY_DARK=1 cargo run --example gallery

# the ribbon command bar on its own
cargo run --example ribbon
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
