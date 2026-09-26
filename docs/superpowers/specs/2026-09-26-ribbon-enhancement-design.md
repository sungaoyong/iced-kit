# Ribbon Enhancement Design — Align with SARibbon

## Date: 2026-09-26

## Goal

Enhance the existing `iced-kit` ribbon component to align with the feature set of [SARibbon](https://github.com/czyt1988/SARibbon), the reference Qt ribbon. The current ribbon already has tabs, groups, tool buttons (large/small/dropdown/grid), adaptive collapse, tooltips, and selected/disabled states. This design adds the four highest-value missing features.

## Design Principles

- **Data-driven, as the rest of the library**: every new feature is plain data the application owns in its state, rebuilt every frame.
- **No new iced abstractions**: build on the existing `Layer`/`trigger`/`Dropdown` overlay system, `tabs` strip, and `container` styles.
- **Incremental**: each feature is independently testable and could be merged on its own.
- **Clippy-clean**: matches the crate's `pedantic` lint profile.

## Feature 1: Minimized Mode

SARibbon lets the user double-click a tab to collapse the ribbon to just the tab strip. This is the ribbon's signature space-saving feature.

### Model

`RibbonState` gains a `minimized: bool` field.

```rust
pub struct RibbonState {
    pub active: usize,
    pub open_dropdown: Option<String>,
    pub collapse_mode: CollapseMode,
    pub minimized: bool,          // NEW
    pub contextual_tab: Option<ContextualTab>, // NEW (Feature 4)
}
```

New methods on `RibbonState`:

```rust
pub fn toggle_minimize(&mut self)      // flips minimized
pub fn set_minimize(&mut self, bool)   // sets minimized, closes dropdown
```

### Builder

`Ribbon` gains `.minimized(bool)` to override the state's value (same pattern as `.active(usize)`).

### View

When `minimized` is true, the ribbon renders only the tab strip, plus a small "expand" affordance (a `ChevronUp` icon button at the right end of the strip) that sends the same `on_select`-style toggle. The group band is hidden entirely.

The application toggles `minimized` from its own message handler — the ribbon reports intent, never mutates state.

### Double-click detection

iced has no native double-click event. We track it at the application level: the `on_select` callback fires on every tab press; the application can compare timestamps. The ribbon itself just needs the `minimized` flag. (We do NOT add internal timestamp tracking to the ribbon — that would be a concern of the application.)

## Feature 2: Quick Access Bar

Word-style quick access toolbar: a row of small icon buttons above the tab strip, for the most-used commands (save, undo, redo).

### Model

New type in `model.rs`:

```rust
pub struct QuickAccessBar<Message> {
    pub(crate) items: Vec<QuickAccessItem<Message>>,
}

pub struct QuickAccessItem<Message> {
    pub(crate) icon: Icon,
    pub(crate) label: Option<String>,
    pub(crate) on_press: Option<Message>,
    pub(crate) tooltip: Option<String>,
}

impl<Message> QuickAccessBar<Message> {
    pub fn new() -> Self;
    pub fn item(mut self, item: QuickAccessItem<Message>) -> Self;
    pub fn items(mut self, items: impl IntoIterator<Item = QuickAccessItem<Message>>) -> Self;
}

impl<Message> QuickAccessItem<Message> {
    pub fn new(icon: impl Into<IconSource>) -> Self;
    pub fn label(mut self, label: impl Into<String>) -> Self;
    pub fn on_press(mut self, message: Message) -> Self;
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self;
}
```

### Builder

`Ribbon` gains `.quick_access_bar(QuickAccessBar<Message>)`.

### View

The quick access bar renders as a `row` of small icon buttons (same `SMALL_ICON`/`ROW_H` metrics as ribbon small tools), positioned above the tab strip, right-aligned. It uses the same `tool_style` for visual consistency. Items with a label show it beside the icon.

The QAB is always visible regardless of minimized state — it sits above the tabs and is never collapsed.

## Feature 3: Gallery Widget

Office-style gallery: a labeled grid of visual items (style swatches, chart type previews, etc.) that can be embedded inside a ribbon group.

### Model

New type in `model.rs`:

```rust
pub struct RibbonGallery<Message> {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) items: Vec<RibbonGalleryItem<Message>>,
    pub(crate) columns: usize,
}

pub struct RibbonGalleryItem<Message> {
    pub(crate) icon: Icon,
    pub(crate) label: Option<String>,
    pub(crate) on_press: Option<Message>,
    pub(crate) selected: bool,
}

impl<Message> RibbonGallery<Message> {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self;
    pub fn item(mut self, item: RibbonGalleryItem<Message>) -> Self;
    pub fn columns(mut self, columns: usize) -> Self;
}

impl<Message> RibbonGalleryItem<Message> {
    pub fn new(icon: impl Into<IconSource>) -> Self;
    pub fn label(mut self, label: impl Into<String>) -> Self;
    pub fn on_press(mut self, message: Message) -> Self;
    pub fn selected(mut self, selected: bool) -> Self;
}
```

### Builder

New `RibbonItem` variant:

```rust
pub enum RibbonItem<Message> {
    // ... existing variants ...
    Gallery(RibbonGallery<Message>),
}
```

Constructor: `RibbonItem::gallery(gallery)`.

### View

The gallery renders inside its group at full density as a labeled grid. Each item is a small button (icon over label, similar to `Face::Large` but smaller). The gallery scrolls vertically if it exceeds the group height. At compact/collapsed/tight densities, the gallery collapses to a single dropdown button (same pattern as other items).

## Feature 4: Contextual Tabs

SARibbon supports contextual tabs that appear only under certain conditions (e.g., a "Picture Tools" tab appears when an image is selected).

### Model

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextualTab {
    pub name: String,
    pub color: Option<Color>,  // accent color for the tab label
}
```

`RibbonState` gains:

```rust
pub contextual_tab: Option<ContextualTab>,
```

New methods:

```rust
pub fn show_contextual_tab(&mut self, name: impl Into<String>);
pub fn hide_contextual_tab(&mut self);
```

### Builder

`Ribbon` gains `.contextual_tab(Option<ContextualTab>)`.

### View

When a contextual tab is active, it renders to the right of the regular tabs, visually distinguished by its accent color (or the default accent if none is set). Selecting it activates its groups just like a regular tab. The contextual tab disappears when `hide_contextual_tab` is called.

## File Changes

| File | Change |
|------|--------|
| `src/widgets/ribbon/model.rs` | Add `minimized`, `contextual_tab` to `RibbonState`; add `QuickAccessBar`, `QuickAccessItem`, `RibbonGallery`, `RibbonGalleryItem`, `ContextualTab`; add `Gallery` variant to `RibbonItem` |
| `src/widgets/ribbon/builder.rs` | Add `.minimized()`, `.quick_access_bar()`, `.contextual_tab()` builder methods; update `into_element` to render QAB, minimized mode, and contextual tab |
| `src/widgets/ribbon/buttons.rs` | Add rendering for `RibbonItem::Gallery`; add QAB item rendering; add contextual tab strip rendering |
| `src/widgets/ribbon/style.rs` | Add styles for gallery grid, QAB items, contextual tab accent |
| `src/widgets/ribbon/collapse.rs` | No changes (gallery collapses like other large items) |
| `src/widgets/ribbon/mod.rs` | Update module docs |
| `src/widgets/mod.rs` | Re-export new types |
| `examples/ribbon.rs` | Update to demonstrate new features |
| `tests/render.rs` | Add render tests for new features |

## Testing

- Unit tests for `RibbonState::toggle_minimize`, `set_minimize`, `show_contextual_tab`, `hide_contextual_tab`.
- Unit tests for builder accumulation on `QuickAccessBar`, `RibbonGallery`.
- Render snapshot tests for: minimized ribbon, QAB, gallery in a group, contextual tab.
- The existing collapse/decision tests continue to pass unchanged.

## Out of Scope

- **Horizontal scrolling**: the adaptive collapse already handles narrow windows; scrolling would be a different UX paradigm.
- **Delayed popup menu buttons**: an edge case with limited applicability in iced.
- **6 group layout styles**: the current 4-level collapse ladder (full → compact → collapsed → tight) already covers the practical cases.
- **QSS theming**: the theme system already handles light/dark and custom tokens.
- **4K/multi-monitor**: iced handles this at the window level.
