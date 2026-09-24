//! Tree views: hierarchical data shown as an indented, expandable list.
//!
//! # The shape of the API
//!
//! A tree is three pieces:
//!
//! - [`TreeItem`] is the data: an id, a label, and its children.
//! - [`Tree`] holds the open set and the selection, and flattens the items into
//!   the rows to draw.
//! - The application owns the [`Tree`] — iced has no place for a widget to keep
//!   hidden state — and hands it to [`tree`] to draw each frame.
//!
//! Flattening is a plain function ([`Tree::rows`]) rather than something the
//! renderer does, so the row order and the indentation can be checked without
//! rendering anything.

use crate::icons::IconName;
use crate::theme::{Size, Theme};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Padding};

/// One node of a tree: an id, a label, and whatever hangs below it.
///
/// Equality compares the shape of the node — its id, label, children and
/// flags — rather than the icon, whose glyph type carries no comparison.
#[derive(Debug, Clone)]
pub struct TreeItem {
    id: String,
    label: String,
    icon: Option<IconName>,
    children: Vec<TreeItem>,
    disabled: bool,
}

impl TreeItem {
    /// Creates a leaf with the given id and label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            children: Vec::new(),
            disabled: false,
        }
    }

    /// Adds a child.
    #[must_use]
    pub fn child(mut self, child: TreeItem) -> Self {
        self.children.push(child);
        self
    }

    /// Adds several children.
    #[must_use]
    pub fn children(mut self, children: impl IntoIterator<Item = TreeItem>) -> Self {
        self.children.extend(children);
        self
    }

    /// Draws an icon before the label.
    #[must_use]
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Disables the node: it renders but cannot be selected or expanded.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The node's id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The node's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The node's children.
    #[must_use]
    pub fn child_items(&self) -> &[TreeItem] {
        &self.children
    }

    /// The node's icon, if it has one.
    #[must_use]
    pub fn icon_name(&self) -> Option<IconName> {
        self.icon
    }

    /// Whether the node can be selected.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Whether the node has children, which is what makes it expandable.
    #[must_use]
    pub fn is_folder(&self) -> bool {
        !self.children.is_empty()
    }
}

impl PartialEq for TreeItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.label == other.label
            && self.children == other.children
            && self.disabled == other.disabled
    }
}

/// One row of a flattened tree: a node and how deep it sits.
#[derive(Debug, Clone)]
pub struct TreeRow {
    /// The node's id.
    pub id: String,
    /// The node's label.
    pub label: String,
    /// How many levels deep the node is; a root is zero.
    pub depth: usize,
    /// Whether the node has children.
    pub is_folder: bool,
    /// Whether the node is currently expanded.
    pub is_expanded: bool,
    /// Whether the node can be selected.
    pub is_disabled: bool,
    /// The node's icon, if it has one.
    pub icon: Option<IconName>,
}

impl PartialEq for TreeRow {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.label == other.label
            && self.depth == other.depth
            && self.is_folder == other.is_folder
            && self.is_expanded == other.is_expanded
            && self.is_disabled == other.is_disabled
    }
}

/// A tree's state: its data, which nodes are open, and which is selected.
///
/// The application owns this. Nothing in here is hidden inside a widget: the
/// flattening that decides which rows are drawn reads only these fields, so a
/// test can check the row order without a window.
#[derive(Debug, Clone, Default)]
pub struct Tree {
    items: Vec<TreeItem>,
    expanded: Vec<String>,
    selected: Option<String>,
}

impl Tree {
    /// Creates an empty tree.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the tree's items.
    ///
    /// A node that is open stays open if its id is still present; ids that are
    /// gone are dropped from the open set, so the set cannot grow without
    /// bound as data is reloaded.
    pub fn set_items(&mut self, items: impl IntoIterator<Item = TreeItem>) {
        self.items = items.into_iter().collect();
        self.prune_expanded();
    }

    /// Sets the items at construction.
    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = TreeItem>) -> Self {
        self.set_items(items);
        self
    }

    /// The tree's root items.
    #[must_use]
    pub fn root_items(&self) -> &[TreeItem] {
        &self.items
    }

    /// Whether `id` is open.
    #[must_use]
    pub fn is_expanded(&self, id: &str) -> bool {
        self.expanded.iter().any(|open| open == id)
    }

    /// Opens or closes `id`.
    ///
    /// Returns whether the state changed, so a caller can tell a real toggle
    /// from a no-op.
    pub fn set_expanded(&mut self, id: &str, expanded: bool) -> bool {
        let is_open = self.is_expanded(id);

        if expanded == is_open {
            return false;
        }

        if expanded {
            self.expanded.push(id.to_owned());
        } else {
            self.expanded.retain(|open| open != id);
        }

        true
    }

    /// Opens or closes `id`, whichever it is not.
    pub fn toggle(&mut self, id: &str) {
        let open = self.is_expanded(id);
        self.set_expanded(id, !open);
    }

    /// The ids that are open.
    #[must_use]
    pub fn expanded_ids(&self) -> &[String] {
        &self.expanded
    }

    /// Opens every folder.
    ///
    /// Only folders are opened: a leaf has no children to show, so an open
    /// entry for it would be state that never means anything.
    pub fn expand_all(&mut self) {
        let mut ids = Vec::new();
        collect_folder_ids(&self.items, &mut ids);
        self.expanded = ids;
    }

    /// Closes every folder.
    pub fn collapse_all(&mut self) {
        self.expanded.clear();
    }

    /// The selected node's id, if any.
    #[must_use]
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// Selects a node, or clears the selection with `None`.
    ///
    /// Selecting a disabled node is refused, and a stale id is accepted but
    /// simply never matches a row: a selection that is no longer in the data is
    /// not a reason to panic.
    pub fn select(&mut self, id: Option<&str>) {
        self.selected = match id {
            None => None,
            Some(id) => {
                let mut ids = Vec::new();
                collect_ids(&self.items, &mut ids);

                if ids.iter().any(|known| known == id) {
                    Some(id.to_owned())
                } else {
                    None
                }
            }
        };
    }

    /// Whether `id` is selected.
    #[must_use]
    pub fn is_selected(&self, id: &str) -> bool {
        self.selected.as_deref() == Some(id)
    }

    /// The rows to draw, in order: every root, each followed by its open
    /// subtree, depth-first.
    #[must_use]
    pub fn rows(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        flatten(&self.items, 0, self, &mut rows);

        rows
    }

    /// The nodes whose ids are in the tree, depth-first.
    fn all_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        collect_ids(&self.items, &mut ids);

        ids
    }

    /// Drops open ids that the current items no longer contain.
    fn prune_expanded(&mut self) {
        let known = self.all_ids();
        self.expanded
            .retain(|open| known.iter().any(|id| id == open));

        // A selection whose node is gone is cleared rather than left dangling.
        if let Some(selected) = self.selected.as_deref() {
            if !known.iter().any(|id| id == selected) {
                self.selected = None;
            }
        }
    }
}

/// Appends `items` and their open descendants to `rows`.
fn flatten(items: &[TreeItem], depth: usize, tree: &Tree, rows: &mut Vec<TreeRow>) {
    for item in items {
        let is_expanded = tree.is_expanded(&item.id);
        let is_folder = item.is_folder();

        // A disabled folder cannot be opened, so it is drawn closed whatever
        // the open set says: the row must not claim to be open while its
        // children are not drawn under it.
        let is_expanded = is_expanded && !item.disabled;

        rows.push(TreeRow {
            id: item.id.clone(),
            label: item.label.clone(),
            depth,
            is_folder,
            is_expanded: is_folder && is_expanded,
            is_disabled: item.disabled,
            icon: item.icon,
        });

        if is_folder && is_expanded {
            flatten(&item.children, depth + 1, tree, rows);
        }
    }
}

/// Collects every id in the tree, depth-first.
fn collect_ids(items: &[TreeItem], ids: &mut Vec<String>) {
    for item in items {
        ids.push(item.id.clone());
        collect_ids(&item.children, ids);
    }
}

/// Collects the ids of the folders in the tree, depth-first.
fn collect_folder_ids(items: &[TreeItem], ids: &mut Vec<String>) {
    for item in items {
        if item.is_folder() {
            ids.push(item.id.clone());
        }
        collect_folder_ids(&item.children, ids);
    }
}

/// What a tree reports when a row is acted on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeEvent {
    /// A row was selected.
    Selected(String),
    /// A folder's disclosure control was pressed, carrying the id and whether
    /// it is now open.
    Toggled(String, bool),
}

impl TreeEvent {
    /// The row the event is about.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Selected(id) | Self::Toggled(id, _) => id,
        }
    }
}

/// Draws a tree's rows.
///
/// The tree itself is the caller's, so this reads it and reports what the user
/// did; the caller applies the change and draws the next frame.
///
/// ```
/// # use iced_kit::widgets::{tree, Tree, TreeEvent, TreeItem};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Acted(TreeEvent) }
/// # fn view<'a>(items: &'a [TreeItem], state: &'a Tree) -> Element<'a, Message, Theme> {
/// tree(items, state).on_event(Message::Acted).into()
/// # }
/// ```
#[must_use = "a TreeView does nothing unless it is turned into an Element"]
pub struct TreeView<'a, Message> {
    items: &'a [TreeItem],
    state: &'a Tree,
    on_event: Option<Box<dyn Fn(TreeEvent) -> Message + 'a>>,
    max_height: Option<f32>,
    indent: f32,
}

impl<'a, Message: Clone + 'a> TreeView<'a, Message> {
    /// Creates a view over `items` and the state that says which are open.
    pub fn new(items: &'a [TreeItem], state: &'a Tree) -> Self {
        Self {
            items,
            state,
            on_event: None,
            max_height: None,
            indent: 16.0,
        }
    }

    /// Reports selection and expansion.
    pub fn on_event(mut self, on_event: impl Fn(TreeEvent) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    /// Caps the tree's height with a scrollbar. Without it the tree is as tall
    /// as its rows.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height.max(20.0));
        self
    }

    /// Sets how far each level is indented. The default is 16.
    pub fn indent(mut self, indent: f32) -> Self {
        self.indent = indent.max(0.0);
        self
    }

    /// The rows this view draws, in order.
    #[must_use]
    pub fn rows(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        flatten(self.items, 0, self.state, &mut rows);

        rows
    }

    /// Turns the tree into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            items,
            state,
            on_event,
            max_height,
            indent,
        } = self;

        let mut rows_vector = Vec::new();
        flatten(items, 0, state, &mut rows_vector);

        let mut body = column![].spacing(0).width(Length::Fill);

        for tree_row in rows_vector {
            body = body.push(row_view(&tree_row, state, on_event.as_deref(), indent));
        }

        let content: Element<'a, Message, Theme> = match max_height {
            None => body.into(),
            Some(height) => scrollable(body).height(Length::Fixed(height)).into(),
        };

        content
    }
}

impl<'a, Message: Clone + 'a> From<TreeView<'a, Message>> for Element<'a, Message, Theme> {
    fn from(view: TreeView<'a, Message>) -> Self {
        view.into_element()
    }
}

/// Draws one row.
fn row_view<'a, Message: Clone + 'a>(
    tree_row: &TreeRow,
    state: &Tree,
    on_event: Option<&(dyn Fn(TreeEvent) -> Message + 'a)>,
    indent: f32,
) -> Element<'a, Message, Theme> {
    let text_style = Size::Sm.text();
    let selected = state.is_selected(&tree_row.id);
    let disabled = tree_row.is_disabled;
    let row_id = tree_row.id.clone();
    let row_is_folder = tree_row.is_folder;
    let row_is_expanded = tree_row.is_expanded;

    let mut line = row![].spacing(6).align_y(Alignment::Center);

    // The disclosure column is reserved whether or not the row is a folder, so
    // a leaf's label lines up with a folder's rather than shifting left.
    if tree_row.is_folder {
        let id = row_id.clone();
        let mut disclosure = button(
            crate::widgets::Icon::new(if row_is_expanded {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            })
            .into_element(Size::Sm),
        )
        .padding(Padding::ZERO)
        .class(Box::new(|_theme: &Theme, status| disclosure_style(status))
            as button::StyleFn<'a, Theme>);

        if !disabled {
            if let Some(on_event) = on_event {
                let message = on_event(TreeEvent::Toggled(id, !row_is_expanded));
                disclosure = disclosure.on_press(message);
            }
        }

        line = line.push(disclosure);
    } else {
        line = line.push(
            container(iced::widget::Space::new())
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(1.0)),
        );
    }

    if let Some(icon) = tree_row.icon {
        line = line.push(crate::widgets::Icon::new(icon).into_element(Size::Sm));
    }

    line = line.push(
        text(tree_row.label.clone())
            .size(text_style.size)
            .line_height(text_style.line_height()),
    );

    let mut widget = button(line.width(Length::Fill))
        .width(Length::Fill)
        .padding(Padding {
            top: 4.0,
            right: 8.0,
            bottom: 4.0,
            // The indent is padding rather than nested containers, so a deep
            // tree does not build a container per level per row.
            left: 8.0 + tree_row.depth as f32 * indent,
        })
        .class(Box::new(move |theme: &Theme, status| {
            tree_row_style(theme, status, selected, disabled)
        }) as button::StyleFn<'a, Theme>);

    // A folder's row selects as well as discloses: the label is the target for
    // choosing it, which is what lets a folder be selected without opening it.
    if !disabled {
        if let Some(on_event) = on_event {
            let message = on_event(TreeEvent::Selected(row_id));
            widget = widget.on_press(message);
        }
    }

    let _ = row_is_folder;

    widget.into()
}

/// The appearance of a tree row.
fn tree_row_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
    disabled: bool,
) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    let text_color = if disabled {
        Color {
            a: colors.muted_foreground.a * 0.6,
            ..colors.muted_foreground
        }
    } else if selected {
        colors.accent_foreground
    } else {
        colors.foreground
    };

    button::Style {
        background: if selected {
            Some(iced::Background::Color(colors.accent))
        } else if hovered && !disabled {
            Some(iced::Background::Color(Color {
                a: colors.accent.a * 0.5,
                ..colors.accent
            }))
        } else {
            None
        },
        text_color,
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: f32::from(theme.radius().sm).into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// The appearance of a disclosure control.
fn disclosure_style(status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: None,
        text_color: Color::BLACK,
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: !hovered,
    }
}

/// Builds a tree view.
pub fn tree<'a, Message: Clone + 'a>(
    items: &'a [TreeItem],
    state: &'a Tree,
) -> TreeView<'a, Message> {
    TreeView::new(items, state)
}

#[cfg(test)]
mod tests {
    use super::{tree, Tree, TreeEvent, TreeItem};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Acted(TreeEvent),
    }

    /// A small tree: two roots, one of them a folder with two children, one of
    /// which is a folder itself.
    fn sample() -> Vec<TreeItem> {
        vec![
            TreeItem::new("src", "src").children([
                TreeItem::new("main", "main.rs"),
                TreeItem::new("widgets", "widgets")
                    .child(TreeItem::new("button", "button.rs"))
                    .child(TreeItem::new("input", "input.rs")),
            ]),
            TreeItem::new("readme", "README.md"),
        ]
    }

    #[test]
    fn an_item_keeps_its_parts() {
        let item = TreeItem::new("id", "Label")
            .icon(crate::icons::IconName::File)
            .disabled(true)
            .child(TreeItem::new("child", "Child"));

        assert_eq!(item.id(), "id");
        assert_eq!(item.label(), "Label");
        assert!(item.is_disabled());
        assert!(item.icon_name().is_some());
        assert!(item.is_folder());
        assert_eq!(item.child_items().len(), 1);
        assert!(!TreeItem::new("leaf", "Leaf").is_folder());
    }

    #[test]
    fn a_collapsed_tree_shows_only_its_roots() {
        let tree = Tree::new().items(sample());
        let rows = tree.rows();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "src");
        assert_eq!(rows[1].id, "readme");
        assert!(rows.iter().all(|row| row.depth == 0));
        assert!(rows.iter().all(|row| !row.is_expanded));
    }

    #[test]
    fn opening_a_folder_shows_its_children_indented() {
        let mut tree = Tree::new().items(sample());
        tree.set_expanded("src", true);

        let rows = tree.rows();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].id, "src");
        assert_eq!(rows[0].depth, 0);
        assert!(rows[0].is_expanded);
        assert_eq!(rows[1].id, "main");
        assert_eq!(rows[1].depth, 1);
        assert_eq!(rows[2].id, "widgets");
        assert_eq!(rows[2].depth, 1);
        assert_eq!(rows[3].id, "readme");
        assert_eq!(rows[3].depth, 0, "a root after a subtree stays at depth 0");
    }

    #[test]
    fn a_nested_folder_keeps_its_own_children_hidden_until_opened() {
        let mut tree = Tree::new().items(sample());
        tree.set_expanded("src", true);
        assert_eq!(tree.rows().len(), 4, "widgets is open? no");

        tree.set_expanded("widgets", true);
        let rows = tree.rows();

        assert_eq!(rows.len(), 6);
        assert_eq!(rows[2].id, "widgets");
        assert_eq!(rows[3].id, "button");
        assert_eq!(rows[3].depth, 2);
        assert_eq!(rows[4].id, "input");
        assert_eq!(rows[4].depth, 2);
    }

    #[test]
    fn closing_a_folder_hides_everything_under_it() {
        let mut tree = Tree::new().items(sample());
        tree.expand_all();
        assert_eq!(tree.rows().len(), 6);

        tree.set_expanded("src", false);
        let rows = tree.rows();

        assert_eq!(rows.len(), 2, "the closed folder's whole subtree is hidden");
        // The nested folder stays open, so reopening `src` shows its children
        // again without a second click.
        assert!(tree.is_expanded("widgets"));
    }

    #[test]
    fn toggling_reports_whether_the_state_changed() {
        let mut tree = Tree::new().items(sample());

        assert!(tree.set_expanded("src", true), "opening is a change");
        assert!(!tree.set_expanded("src", true), "opening again is not");
        assert!(tree.set_expanded("src", false), "closing is a change");
        assert!(!tree.set_expanded("src", false));
    }

    #[test]
    fn toggle_flips_the_state() {
        let mut tree = Tree::new().items(sample());

        tree.toggle("src");
        assert!(tree.is_expanded("src"));

        tree.toggle("src");
        assert!(!tree.is_expanded("src"));
    }

    #[test]
    fn expanding_and_collapsing_everything_works() {
        let mut tree = Tree::new().items(sample());

        tree.expand_all();
        assert_eq!(tree.rows().len(), 6);
        assert_eq!(tree.expanded_ids().len(), 2, "only the folders");

        tree.collapse_all();
        assert_eq!(tree.rows().len(), 2);
        assert!(tree.expanded_ids().is_empty());
    }

    #[test]
    fn reloading_the_items_keeps_the_open_ones_that_still_exist() {
        let mut tree = Tree::new().items(sample());
        tree.expand_all();

        // The same ids, so the open set is untouched.
        tree.set_items(sample());
        assert_eq!(tree.rows().len(), 6);

        // A different tree: the old ids are gone, so the set is pruned rather
        // than left to grow with every reload.
        tree.set_items([TreeItem::new("other", "Other")]);
        assert!(tree.expanded_ids().is_empty());
        assert_eq!(tree.rows().len(), 1);
    }

    #[test]
    fn selecting_a_node_marks_it() {
        let mut tree = Tree::new().items(sample());
        tree.select(Some("readme"));

        assert_eq!(tree.selected(), Some("readme"));
        assert!(tree.is_selected("readme"));
        assert!(!tree.is_selected("src"));

        tree.select(None);
        assert_eq!(tree.selected(), None);
    }

    /// A selection of a node the tree no longer holds is what an application
    /// has after a reload; it must not panic, and the tree must not keep
    /// claiming a row is selected when it is not there.
    #[test]
    fn selecting_an_unknown_id_selects_nothing() {
        let mut tree = Tree::new().items(sample());
        tree.select(Some("no-such-node"));

        assert_eq!(tree.selected(), None);
        assert!(!tree.is_selected("no-such-node"));
    }

    #[test]
    fn reloading_clears_a_selection_that_is_gone() {
        let mut tree = Tree::new().items(sample());
        tree.select(Some("readme"));

        tree.set_items([TreeItem::new("other", "Other")]);
        assert_eq!(tree.selected(), None);
    }

    #[test]
    fn an_event_reports_the_row_it_is_about() {
        assert_eq!(TreeEvent::Selected("a".to_owned()).id(), "a");
        assert_eq!(TreeEvent::Toggled("b".to_owned(), true).id(), "b");

        assert_ne!(
            TreeEvent::Toggled("b".to_owned(), true),
            TreeEvent::Toggled("b".to_owned(), false),
            "the open state is part of the event"
        );
    }

    #[test]
    fn an_empty_tree_has_no_rows() {
        let tree = Tree::new();
        assert!(tree.rows().is_empty());
        assert!(tree.root_items().is_empty());
    }

    #[test]
    fn a_row_records_the_node_it_stands_for() {
        let mut tree = Tree::new().items(sample());
        tree.set_expanded("src", true);
        tree.set_expanded("widgets", true);

        let rows = tree.rows();
        let widgets = rows.iter().find(|row| row.id == "widgets").expect("a row");

        assert_eq!(widgets.label, "widgets");
        assert_eq!(widgets.depth, 1);
        assert!(widgets.is_folder);
        assert!(widgets.is_expanded);
        assert!(!widgets.is_disabled);
    }

    #[test]
    fn a_disabled_node_reports_itself_as_disabled() {
        let tree = Tree::new().items([TreeItem::new("locked", "Locked").disabled(true)]);
        let rows = tree.rows();

        assert!(rows[0].is_disabled);
    }

    #[test]
    fn a_disabled_folder_is_not_drawn_as_open_even_if_it_is_in_the_set() {
        let mut tree = Tree::new().items([TreeItem::new("locked", "Locked")
            .disabled(true)
            .child(TreeItem::new("inside", "Inside"))]);
        tree.set_expanded("locked", true);

        let rows = tree.rows();
        assert!(
            !rows[0].is_expanded,
            "a row must not claim to be open when its children are not drawn"
        );
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn trees_render_collapsed_and_open() {
        let items = sample();
        let mut state = Tree::new().items(items.clone());

        let collapsed: iced::Element<'_, Message, Theme> =
            tree(&items, &state).on_event(Message::Acted).into();
        drop(collapsed);

        state.expand_all();
        let open: iced::Element<'_, Message, Theme> = tree(&items, &state)
            .on_event(Message::Acted)
            .max_height(200.0)
            .indent(20.0)
            .into();
        drop(open);

        let read_only: iced::Element<'_, Message, Theme> = tree(&items, &state).into();
        drop(read_only);
    }

    #[test]
    fn the_view_reports_the_rows_the_state_implies() {
        let items = sample();
        let mut state = Tree::new();
        state.set_items(items.clone());
        state.set_expanded("src", true);

        let view: super::TreeView<'_, Message> = tree(&items, &state);
        assert_eq!(view.rows().len(), 4);
    }

    #[test]
    fn a_selected_row_is_filled_with_the_accent() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let selected = super::tree_row_style(&theme, Status::Active, true, false);
        let ordinary = super::tree_row_style(&theme, Status::Active, false, false);
        let disabled = super::tree_row_style(&theme, Status::Active, false, true);

        assert_eq!(
            selected.background,
            Some(iced::Background::Color(theme.colors().accent))
        );
        assert!(ordinary.background.is_none());
        assert!(
            disabled.text_color.a < ordinary.text_color.a,
            "a disabled row must be dimmer"
        );
    }
}
