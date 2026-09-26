//! The ribbon's plain-data model.
//!
//! A ribbon is described entirely as data: [`RibbonTab`]s hold [`RibbonGroup`]s,
//! a group holds [`RibbonItem`]s, and an item wraps one or more [`RibbonTool`]s.
//! Nothing here touches iced's widget tree — [`super::Ribbon`] turns this
//! description into elements, and the application rebuilds it every frame from
//! the values it owns, exactly as it does for the rest of the library.
//!
//! The vocabulary follows the reference ribbon (a tab strip over a multi-row
//! tool area) but drops its CAD-specific payloads: a tool carries an
//! [`on_press`](RibbonTool::on_press) message rather than a command id, so the
//! component is generic over any application's `Message`.

use super::collapse::CollapseMode;
use super::theme::RibbonTheme;
use crate::icons::IconName;
use crate::widgets::button::{Icon, IconSource};
use iced::Color;

/// One clickable tool button in a ribbon panel.
///
/// A tool is an icon, an optional label, and the message it sends when pressed.
/// Whether it is drawn large or small, with or without a label, is decided by
/// the [`RibbonItem`] that wraps it rather than by the tool itself.
#[must_use = "a RibbonTool does nothing unless it is placed in a RibbonItem"]
pub struct RibbonTool<Message> {
    pub(crate) icon: Icon,
    pub(crate) label: Option<String>,
    pub(crate) on_press: Option<Message>,
    pub(crate) selected: bool,
    pub(crate) disabled: bool,
    pub(crate) tooltip: Option<String>,
}

impl<Message> RibbonTool<Message> {
    /// Creates a tool from an icon — a [`IconName`] variant, an SVG handle, or a
    /// raw glyph.
    pub fn new(icon: impl Into<IconSource>) -> Self {
        Self {
            icon: Icon::new(icon),
            label: None,
            on_press: None,
            selected: false,
            disabled: false,
            tooltip: None,
        }
    }

    /// Creates a tool from a named [Lucide](https://lucide.dev) icon.
    pub fn named(icon: IconName) -> Self {
        Self::new(icon)
    }

    /// Sets the label drawn beside (small) or under (large) the icon.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the message sent when the tool is pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the message sent when pressed, if there is one; `None` leaves the
    /// tool inert.
    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    /// Marks the tool as active, drawing its selected ring.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Greys the tool out and stops it emitting its message.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets a tooltip shown when the pointer rests on the tool.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// One item in a ribbon group: a button, a dropdown, or a grid of buttons, in a
/// small (single-row) or large (full-height) footprint.
///
/// The variants mirror the reference's button sizes. A *large* item fills the
/// whole three-row tool area with its icon over a label; a *small* item takes a
/// single row, and three small items stack into one column.
// The dropdown and grid variants carry a `Vec` beside their icon; boxing every
// payload would obscure the model for a few bytes, so the size gap is accepted.
#[allow(clippy::large_enum_variant)]
#[must_use = "a RibbonItem does nothing unless it is added to a RibbonGroup"]
pub enum RibbonItem<Message> {
    /// A single-row, icon-only button.
    Tool(RibbonTool<Message>),
    /// A single-row button with its label beside the icon.
    LabeledTool(RibbonTool<Message>),
    /// A full-height button with its label under the icon.
    LargeTool(RibbonTool<Message>),
    /// A single-row button with a trailing ▾ that toggles `items`.
    Dropdown {
        id: String,
        icon: Icon,
        items: Vec<RibbonTool<Message>>,
    },
    /// A single-row dropdown that also shows a fixed label.
    LabeledDropdown {
        id: String,
        label: Option<String>,
        icon: Icon,
        items: Vec<RibbonTool<Message>>,
    },
    /// A single-row split button: pressing the body runs `on_press`, and the
    /// trailing ▾ toggles `items` — the reference's action-menu button.
    ActionDropdown {
        id: String,
        icon: Icon,
        on_press: Option<Message>,
        items: Vec<RibbonTool<Message>>,
    },
    /// A full-height dropdown: icon over label, with a ▾ beneath.
    LargeDropdown {
        id: String,
        icon: Icon,
        label: Option<String>,
        items: Vec<RibbonTool<Message>>,
    },
    /// Explicit columns of small icon-only buttons.
    ToolGrid {
        columns: Vec<Vec<RibbonTool<Message>>>,
    },
    /// An Office-style gallery: a labeled grid of visual items that can be
    /// embedded in a group or hosted as a standalone panel.
    Gallery(RibbonGallery<Message>),
}

impl<Message> RibbonItem<Message> {
    /// A small icon-only button.
    pub fn tool(tool: RibbonTool<Message>) -> Self {
        Self::Tool(tool)
    }

    /// A small button with a label beside the icon.
    pub fn labeled(tool: RibbonTool<Message>) -> Self {
        Self::LabeledTool(tool)
    }

    /// A large, full-height button with a label under the icon.
    pub fn large(tool: RibbonTool<Message>) -> Self {
        Self::LargeTool(tool)
    }

    /// A small dropdown: an icon button whose ▾ opens `items`.
    pub fn dropdown(
        id: impl Into<String>,
        icon: impl Into<IconSource>,
        items: Vec<RibbonTool<Message>>,
    ) -> Self {
        Self::Dropdown {
            id: id.into(),
            icon: Icon::new(icon),
            items,
        }
    }

    /// A small dropdown that also shows a fixed label beside the icon.
    pub fn labeled_dropdown(
        id: impl Into<String>,
        label: impl Into<String>,
        icon: impl Into<IconSource>,
        items: Vec<RibbonTool<Message>>,
    ) -> Self {
        Self::LabeledDropdown {
            id: id.into(),
            label: Some(label.into()),
            icon: Icon::new(icon),
            items,
        }
    }

    /// A split action button: pressing the body runs `on_press` (reported as
    /// `Some` through `message`) while the trailing ▾ toggles `items` — the
    /// reference's action-menu button.
    pub fn action_dropdown(
        id: impl Into<String>,
        icon: impl Into<IconSource>,
        message: Message,
        items: Vec<RibbonTool<Message>>,
    ) -> Self {
        Self::ActionDropdown {
            id: id.into(),
            icon: Icon::new(icon),
            on_press: Some(message),
            items,
        }
    }

    /// A large dropdown: a full-height icon-over-label button with a ▾.
    pub fn large_dropdown(
        id: impl Into<String>,
        icon: impl Into<IconSource>,
        label: impl Into<String>,
        items: Vec<RibbonTool<Message>>,
    ) -> Self {
        Self::LargeDropdown {
            id: id.into(),
            icon: Icon::new(icon),
            label: Some(label.into()),
            items,
        }
    }

    /// A grid of small buttons laid out in explicit columns.
    pub fn grid(columns: Vec<Vec<RibbonTool<Message>>>) -> Self {
        Self::ToolGrid { columns }
    }

    /// An Office-style gallery embedded in a group.
    pub fn gallery(gallery: RibbonGallery<Message>) -> Self {
        Self::Gallery(gallery)
    }

    /// Whether this item fills the whole tool-area height rather than one row.
    pub(crate) fn is_large(&self) -> bool {
        matches!(
            self,
            Self::LargeTool(_)
                | Self::LargeDropdown { .. }
                | Self::ToolGrid { .. }
                | Self::Gallery(_)
        )
    }
}

/// An Office-style gallery: a labeled grid of visual items (style swatches,
/// chart type previews, etc.) embedded in a ribbon group.
///
/// The gallery renders as a grid of small buttons, each an icon over a label.
/// If the grid exceeds the available height it scrolls vertically.
#[must_use = "a RibbonGallery does nothing unless it is placed in a RibbonItem"]
pub struct RibbonGallery<Message> {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) items: Vec<RibbonGalleryItem<Message>>,
    pub(crate) columns: usize,
}

impl<Message> RibbonGallery<Message> {
    /// Creates an empty gallery with the given id and title.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            items: Vec::new(),
            columns: 4,
        }
    }

    /// Appends an item to the gallery.
    pub fn item(mut self, item: RibbonGalleryItem<Message>) -> Self {
        self.items.push(item);
        self
    }

    /// Sets the number of columns in the grid.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }
}

/// One visual item in a [`RibbonGallery`]: an icon over a label.
#[derive(Clone)]
#[must_use = "a RibbonGalleryItem does nothing unless it is added to a RibbonGallery"]
pub struct RibbonGalleryItem<Message> {
    pub(crate) icon: Icon,
    pub(crate) label: Option<String>,
    pub(crate) on_press: Option<Message>,
    pub(crate) selected: bool,
}

impl<Message> RibbonGalleryItem<Message> {
    /// Creates a gallery item from an icon.
    pub fn new(icon: impl Into<IconSource>) -> Self {
        Self {
            icon: Icon::new(icon),
            label: None,
            on_press: None,
            selected: false,
        }
    }

    /// Sets the label drawn under the icon.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the message sent when the item is pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Marks the item as active, drawing its selected ring.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// A Word-style quick access bar item: an icon, an optional label, and the
/// message it sends when pressed.
#[derive(Clone)]
#[must_use = "a QuickAccessItem does nothing unless it is added to a QuickAccessBar"]
pub struct QuickAccessItem<Message> {
    pub(crate) icon: Icon,
    pub(crate) label: Option<String>,
    pub(crate) on_press: Option<Message>,
    pub(crate) tooltip: Option<String>,
}

impl<Message> QuickAccessItem<Message> {
    /// Creates a quick-access item from an icon.
    pub fn new(icon: impl Into<IconSource>) -> Self {
        Self {
            icon: Icon::new(icon),
            label: None,
            on_press: None,
            tooltip: None,
        }
    }

    /// Sets the label drawn beside the icon.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the message sent when the item is pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets a tooltip shown when the pointer rests on the item.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// A Word-style quick access bar: a row of small icon buttons above the tab
/// strip, for the most-used commands (save, undo, redo).
#[must_use = "a QuickAccessBar does nothing unless it is added to a Ribbon"]
pub struct QuickAccessBar<Message> {
    pub(crate) items: Vec<QuickAccessItem<Message>>,
}

impl<Message> Default for QuickAccessBar<Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> QuickAccessBar<Message> {
    /// Creates an empty quick access bar.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Appends an item to the bar.
    pub fn item(mut self, item: QuickAccessItem<Message>) -> Self {
        self.items.push(item);
        self
    }

    /// Appends several items to the bar.
    pub fn items(mut self, items: impl IntoIterator<Item = QuickAccessItem<Message>>) -> Self {
        self.items.extend(items);
        self
    }
}

/// A contextual tab's identity: its display name and optional accent color.
///
/// Contextual tabs appear only under certain conditions (e.g., a "Picture
/// Tools" tab appears when an image is selected). They render to the right of
/// the regular tabs, visually distinguished by their accent color.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextualTab {
    /// The display name shown on the tab.
    pub name: String,
    /// The accent color for the tab label. `None` uses the default accent.
    pub color: Option<Color>,
}

impl ContextualTab {
    /// Creates a contextual tab with the given name and no custom color.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            color: None,
        }
    }

    /// Sets a custom accent color for the tab label.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A named panel of [`RibbonItem`]s, the boxed group with a label along the
/// bottom edge of the tool area.
#[must_use = "a RibbonGroup does nothing unless it is added to a RibbonTab"]
pub struct RibbonGroup<Message> {
    pub(crate) title: String,
    pub(crate) items: Vec<RibbonItem<Message>>,
}

impl<Message> RibbonGroup<Message> {
    /// Creates an empty group with the given bottom label.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
        }
    }

    /// Appends an item to the group.
    pub fn item(mut self, item: RibbonItem<Message>) -> Self {
        self.items.push(item);
        self
    }

    /// Appends several items to the group.
    pub fn items(mut self, items: impl IntoIterator<Item = RibbonItem<Message>>) -> Self {
        self.items.extend(items);
        self
    }
}

/// A tab: a title in the strip and the groups shown while it is active.
#[must_use = "a RibbonTab does nothing unless it is added to a Ribbon"]
pub struct RibbonTab<Message> {
    pub(crate) title: String,
    pub(crate) groups: Vec<RibbonGroup<Message>>,
}

impl<Message> RibbonTab<Message> {
    /// Creates an empty tab with the given strip label.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            groups: Vec::new(),
        }
    }

    /// Appends a group to the tab.
    pub fn group(mut self, group: RibbonGroup<Message>) -> Self {
        self.groups.push(group);
        self
    }

    /// Appends several groups to the tab.
    pub fn groups(mut self, groups: impl IntoIterator<Item = RibbonGroup<Message>>) -> Self {
        self.groups.extend(groups);
        self
    }
}

/// A ribbon group's layout style, matching the reference's six panel styles.
///
/// The styles vary two axes — how many rows a column stacks, and whether
/// *loose* keeps large items at full height or *compact* forces every item to
/// its small face — so a caller can match the density of the screen it is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RibbonLayout {
    /// Adapt to the window: degrade groups from the right as space runs out.
    #[default]
    Auto,
    /// Loose three-row: large items full-height, small items stack three.
    LooseThreeRow,
    /// Compact three-row: every item small, stacked three.
    CompactThreeRow,
    /// Loose two-row: large items full-height, small items stack two.
    LooseTwoRow,
    /// Compact two-row: every item small, stacked two.
    CompactTwoRow,
    /// Single-row: every item in one row, large items reduced to it.
    LooseSingleRow,
    /// Compact single-row: every item in one row at a tighter spacing.
    CompactSingleRow,
}

impl RibbonLayout {
    /// Every style, in the order a selector would list them.
    pub const ALL: &'static [RibbonLayout] = &[
        Self::Auto,
        Self::LooseThreeRow,
        Self::CompactThreeRow,
        Self::LooseTwoRow,
        Self::CompactTwoRow,
        Self::LooseSingleRow,
        Self::CompactSingleRow,
    ];

    /// The label for a style, for a settings control.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::LooseThreeRow => "Loose 3-row",
            Self::CompactThreeRow => "Compact 3-row",
            Self::LooseTwoRow => "Loose 2-row",
            Self::CompactTwoRow => "Compact 2-row",
            Self::LooseSingleRow => "Loose 1-row",
            Self::CompactSingleRow => "Compact 1-row",
        }
    }

    /// Whether the style stacks columns of three, two, or one — the axis that
    /// decides the group's height.
    #[must_use]
    pub fn rows_per_column(self) -> usize {
        match self {
            Self::LooseThreeRow | Self::CompactThreeRow => 3,
            Self::LooseTwoRow | Self::CompactTwoRow => 2,
            Self::LooseSingleRow | Self::CompactSingleRow | Self::Auto => 1,
        }
    }

    /// Whether *loose* keeps large items at full height; a *compact* style
    /// forces every item to its small face so the group narrows.
    #[must_use]
    pub fn keeps_large_items(self) -> bool {
        matches!(self, Self::LooseThreeRow | Self::LooseTwoRow)
    }

    /// Whether the style lays its items out in a single row.
    #[must_use]
    pub fn is_single_row(self) -> bool {
        matches!(self, Self::LooseSingleRow | Self::CompactSingleRow)
    }
}

impl std::fmt::Display for RibbonLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// The view state a ribbon reads: which tab is active, which dropdown (by
/// id) is open, whether the ribbon is minimized, and which contextual tab
/// is shown.
///
/// The application owns a `RibbonState` as it owns every other value, mutating
/// it from the messages the ribbon emits. It is deliberately plain data so a
/// screen can persist or drive it from anywhere.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RibbonState {
    /// The index of the active tab.
    pub active: usize,
    /// The id of the open dropdown, if any.
    pub open_dropdown: Option<String>,
    /// How the groups are sized when the row runs out of width.
    pub collapse_mode: CollapseMode,
    /// The groups' layout style (one of the six panel styles).
    pub layout: RibbonLayout,
    /// Whether the ribbon is minimized (only the tab strip is shown).
    pub minimized: bool,
    /// The currently visible contextual tabs — the reference shows more than
    /// one, each appearing only under its own condition.
    pub contextual_tabs: Vec<ContextualTab>,
    /// The ribbon's visual theme (one of the 10 built-in SARibbon styles).
    pub ribbon_theme: RibbonTheme,
}

impl RibbonState {
    /// A fresh state with the first tab active, no dropdown open, adaptive
    /// ([`Auto`](CollapseMode::Auto)) density, and the default ribbon theme.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the ribbon's visual theme.
    pub fn set_ribbon_theme(&mut self, theme: RibbonTheme) {
        self.ribbon_theme = theme;
    }

    /// Sets the groups' layout style, closing any open dropdown.
    pub fn set_layout(&mut self, layout: RibbonLayout) {
        self.layout = layout;
        self.close_dropdown();
    }

    /// Selects a tab by index, closing any open dropdown.
    pub fn select(&mut self, index: usize) {
        self.active = index;
        self.close_dropdown();
    }

    /// Sets the density mode, closing any open dropdown.
    pub fn set_collapse_mode(&mut self, mode: CollapseMode) {
        self.collapse_mode = mode;
        self.close_dropdown();
    }

    /// Toggles the minimized state, closing any open dropdown.
    pub fn toggle_minimize(&mut self) {
        self.minimized = !self.minimized;
        self.close_dropdown();
    }

    /// Sets the minimized state, closing any open dropdown.
    pub fn set_minimize(&mut self, minimized: bool) {
        self.minimized = minimized;
        self.close_dropdown();
    }

    /// Adds a contextual tab with the given name, unless one with that name is
    /// already shown.
    pub fn show_contextual_tab(&mut self, name: impl Into<String>) {
        self.show_contextual_tab_color(name, Color::from_rgb(0.8, 0.2, 0.2));
    }

    /// Adds a contextual tab with a custom accent color.
    pub fn show_contextual_tab_color(&mut self, name: impl Into<String>, color: Color) {
        let name = name.into();
        if !self.contextual_tabs.iter().any(|ct| ct.name == name) {
            self.contextual_tabs
                .push(ContextualTab::new(name).color(color));
        }
    }

    /// Removes the contextual tab with the given name, if it is shown.
    pub fn hide_contextual_tab(&mut self, name: &str) {
        self.contextual_tabs.retain(|ct| ct.name != name);
    }

    /// Hides every contextual tab.
    pub fn hide_all_contextual_tabs(&mut self) {
        self.contextual_tabs.clear();
    }

    /// Whether any contextual tab is currently shown.
    #[must_use]
    pub fn has_contextual_tabs(&self) -> bool {
        !self.contextual_tabs.is_empty()
    }

    /// Whether the contextual tab named `name` is shown.
    #[must_use]
    pub fn is_contextual_tab_shown(&self, name: &str) -> bool {
        self.contextual_tabs.iter().any(|ct| ct.name == name)
    }

    /// Opens `id` if it was closed, or closes it if it was already open.
    pub fn toggle_dropdown(&mut self, id: &str) {
        if self.open_dropdown.as_deref() == Some(id) {
            self.close_dropdown();
        } else {
            self.open_dropdown = Some(id.to_owned());
        }
    }

    /// Closes the open dropdown, if any.
    pub fn close_dropdown(&mut self) {
        self.open_dropdown = None;
    }

    /// Whether the dropdown named `id` is the one currently open.
    #[must_use]
    pub fn is_dropdown_open(&self, id: &str) -> bool {
        self.open_dropdown.as_deref() == Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        QuickAccessBar, QuickAccessItem, RibbonGallery, RibbonGalleryItem, RibbonGroup, RibbonItem,
        RibbonState, RibbonTab, RibbonTool,
    };
    use iced::Color;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Press,
    }

    fn tool() -> RibbonTool<Msg> {
        RibbonTool::new("x").on_press(Msg::Press)
    }

    #[test]
    fn a_state_starts_on_the_first_tab_with_nothing_open() {
        let state = RibbonState::new();
        assert_eq!(state.active, 0);
        assert_eq!(state.open_dropdown, None);
        assert!(!state.minimized);
        assert!(state.contextual_tabs.is_empty());
    }

    #[test]
    fn selecting_a_tab_moves_the_active_index_and_closes_the_dropdown() {
        let mut state = RibbonState::new();
        state.toggle_dropdown("paste");
        state.select(2);
        assert_eq!(state.active, 2);
        assert_eq!(state.open_dropdown, None);
    }

    #[test]
    fn toggling_a_dropdown_opens_then_closes_it() {
        let mut state = RibbonState::new();
        state.toggle_dropdown("paste");
        assert!(state.is_dropdown_open("paste"));
        assert!(!state.is_dropdown_open("layers"));
        // Toggling the same id again closes it; toggling another swaps open.
        state.toggle_dropdown("paste");
        assert_eq!(state.open_dropdown, None);
        state.toggle_dropdown("a");
        state.toggle_dropdown("b");
        assert!(state.is_dropdown_open("b"));
    }

    #[test]
    fn builders_accumulate_in_order() {
        let tab: RibbonTab<Msg> = RibbonTab::new("Home")
            .group(RibbonGroup::new("Draw").item(RibbonItem::tool(tool())))
            .group(RibbonGroup::new("Edit"));
        assert_eq!(tab.title, "Home");
        assert_eq!(tab.groups.len(), 2);
        assert_eq!(tab.groups[0].items.len(), 1);
        assert_eq!(tab.groups[1].items.len(), 0);
    }

    #[test]
    fn only_the_full_height_variants_are_large() {
        assert!(RibbonItem::large(tool()).is_large());
        assert!(RibbonItem::large_dropdown("d", "x", "L", vec![tool()]).is_large());
        assert!(RibbonItem::grid(vec![vec![tool()]]).is_large());
        assert!(RibbonItem::gallery(RibbonGallery::<Msg>::new("g", "Gallery")).is_large());
        assert!(!RibbonItem::tool(tool()).is_large());
        assert!(!RibbonItem::dropdown("d", "x", vec![tool()]).is_large());
    }

    #[test]
    fn minimize_toggles_and_closes_dropdown() {
        let mut state = RibbonState::new();
        assert!(!state.minimized);
        state.toggle_minimize();
        assert!(state.minimized);
        state.toggle_dropdown("paste");
        state.set_minimize(false);
        assert!(!state.minimized);
        assert_eq!(state.open_dropdown, None);
    }

    #[test]
    fn contextual_tab_shows_and_hides() {
        let mut state = RibbonState::new();
        assert!(!state.has_contextual_tabs());
        state.show_contextual_tab("Picture Tools");
        let ct = state.contextual_tabs.first().unwrap();
        assert_eq!(ct.name, "Picture Tools");
        assert!(ct.color.is_some());
        // A second context category can sit beside the first.
        state.show_contextual_tab_color("Table Tools", Color::from_rgb(0.2, 0.6, 0.3));
        assert_eq!(state.contextual_tabs.len(), 2);
        assert!(state.is_contextual_tab_shown("Table Tools"));
        // Hiding one leaves the other shown.
        state.hide_contextual_tab("Picture Tools");
        assert!(state.is_contextual_tab_shown("Table Tools"));
        state.hide_all_contextual_tabs();
        assert!(!state.has_contextual_tabs());
    }

    #[test]
    fn contextual_tab_with_color() {
        let mut state = RibbonState::new();
        state.show_contextual_tab_color("Picture Tools", Color::from_rgb(1.0, 0.0, 0.0));
        let ct = state.contextual_tabs.first().unwrap();
        assert_eq!(ct.name, "Picture Tools");
        assert!(ct.color.is_some());
        // Showing the same name again does not duplicate it.
        state.show_contextual_tab_color("Picture Tools", Color::from_rgb(0.0, 0.0, 1.0));
        assert_eq!(state.contextual_tabs.len(), 1);
    }

    #[test]
    fn quick_access_bar_accumulates_items() {
        let bar: QuickAccessBar<Msg> = QuickAccessBar::new()
            .item(QuickAccessItem::new("S").label("Save").on_press(Msg::Press))
            .item(QuickAccessItem::new("U").label("Undo").on_press(Msg::Press));
        assert_eq!(bar.items.len(), 2);
        assert_eq!(bar.items[0].label.as_deref(), Some("Save"));
        assert_eq!(bar.items[1].label.as_deref(), Some("Undo"));
    }

    #[test]
    fn gallery_accumulates_items_and_sets_columns() {
        let gallery: RibbonGallery<Msg> = RibbonGallery::new("styles", "Styles")
            .item(
                RibbonGalleryItem::new("A")
                    .label("Normal")
                    .on_press(Msg::Press),
            )
            .item(RibbonGalleryItem::new("B").label("Heading").selected(true))
            .columns(3);
        assert_eq!(gallery.items.len(), 2);
        assert_eq!(gallery.columns, 3);
        assert_eq!(gallery.items[0].label.as_deref(), Some("Normal"));
        assert!(gallery.items[1].selected);
    }
}
