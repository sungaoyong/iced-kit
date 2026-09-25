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
use crate::icons::IconName;
use crate::widgets::button::{Icon, IconSource};

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

    /// Whether this item fills the whole tool-area height rather than one row.
    pub(crate) fn is_large(&self) -> bool {
        matches!(
            self,
            Self::LargeTool(_) | Self::LargeDropdown { .. } | Self::ToolGrid { .. }
        )
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

/// The view state a ribbon reads: which tab is active and which dropdown (by
/// id) is open.
///
/// The application owns a `RibbonState` as it owns every other value, mutating
/// it from the messages the ribbon emits. It is deliberately plain data so a
/// screen can persist or drive it from anywhere.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RibbonState {
    /// The index of the active tab.
    pub active: usize,
    /// The id of the open dropdown, if any.
    pub open_dropdown: Option<String>,
    /// How the groups are sized when the row runs out of width.
    pub collapse_mode: CollapseMode,
}

impl RibbonState {
    /// A fresh state with the first tab active, no dropdown open, and adaptive
    /// ([`Auto`](CollapseMode::Auto)) density.
    pub fn new() -> Self {
        Self::default()
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
    use super::{RibbonGroup, RibbonItem, RibbonState, RibbonTab, RibbonTool};

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
        assert!(!RibbonItem::tool(tool()).is_large());
        assert!(!RibbonItem::dropdown("d", "x", vec![tool()]).is_large());
    }
}
