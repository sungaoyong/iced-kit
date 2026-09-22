// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

//! Named dock regions: a collapsible dock on any of the three edges.
//!
//! A [`Layout`](super::Layout) is a free split tree with no notion of an edge.
//! A workspace usually wants more than that: a dock that can be collapsed to a
//! strip and reopened, remembering its own size. That is what this module adds.
//!
//! It is deliberately additive. The centre tree stays where it always was —
//! [`DockWidgetState::layout`](crate::dock::DockWidgetState::layout) — and the
//! edge docks hang beside it, each keeping its own independent `Layout` so the
//! trees stay separate values that can be normalized, compared and dumped on
//! their own. Nothing in the split/tab algebra has to know a region exists.

use std::collections::BTreeMap;

use super::{Axis, Layout, NodeId};

/// The smallest a dock may be dragged, in logical pixels.
pub const PANEL_MIN_SIZE: f32 = 100.0;

/// A closed bottom dock keeps this much height, so its tab strip stays clickable.
///
/// A closed left or right dock takes no width at all: its tab strip would be a
/// column of unreadable slivers, so reopening is left to the application — the
/// toggle button, or a drag on the handle.
pub const CLOSED_BOTTOM_STRIP: f32 = 29.0;

/// Where a dock sits relative to the centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DockPlacement {
    /// The main area. Not a dock: no size, no open state, never collapsed.
    Center,
    Left,
    Bottom,
    Right,
}

impl DockPlacement {
    /// The three placements that can hold a dock, in the order they are laid out.
    pub const DOCKS: [Self; 3] = [Self::Left, Self::Bottom, Self::Right];

    /// The axis a dock's size is measured along.
    #[must_use]
    pub fn axis(self) -> Axis {
        match self {
            Self::Bottom => Axis::Vertical,
            Self::Center | Self::Left | Self::Right => Axis::Horizontal,
        }
    }

    #[must_use]
    pub fn is_left(self) -> bool {
        matches!(self, Self::Left)
    }

    #[must_use]
    pub fn is_right(self) -> bool {
        matches!(self, Self::Right)
    }

    #[must_use]
    pub fn is_bottom(self) -> bool {
        matches!(self, Self::Bottom)
    }

    /// Whether this placement is a dock that can be shown or hidden.
    #[must_use]
    pub fn is_dock(self) -> bool {
        !matches!(self, Self::Center)
    }

    /// A stable name, for element ids and for `PaneTarget`-style lookup.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Center => "center",
            Self::Left => "left",
            Self::Bottom => "bottom",
            Self::Right => "right",
        }
    }
}

/// Every dock has a name, so an error can say which one it is about.
impl std::fmt::Display for DockPlacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// One dock's presentation state.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "dock-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dock {
    /// Whether the dock is shown. A closed dock still exists and keeps its panels.
    pub open: bool,
    /// Whether the toggle affordance may close this dock.
    ///
    /// A dock that refuses to close still accepts an explicit
    /// [`set_open`](Self::set_open), so an application can hide it itself.
    pub collapsible: bool,
    /// Size along the dock's own axis, in logical pixels.
    pub size: f32,
}

impl Dock {
    /// A dock of `size` pixels, open and collapsible.
    #[must_use]
    pub fn new(size: f32) -> Self {
        Self {
            open: true,
            collapsible: true,
            size: size.max(PANEL_MIN_SIZE),
        }
    }

    /// The size a dock gets when a layout does not state one.
    #[must_use]
    pub fn default_size() -> f32 {
        PANEL_MIN_SIZE * 2.0
    }

    pub fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    #[must_use]
    pub fn is_open(self) -> bool {
        self.open
    }

    #[must_use]
    pub fn is_collapsible(self) -> bool {
        self.collapsible
    }

    pub fn set_collapsible(&mut self, collapsible: bool) {
        self.collapsible = collapsible;
    }

    #[must_use]
    pub fn size(self) -> f32 {
        self.size
    }

    /// Set the dock's size, floored at [`PANEL_MIN_SIZE`].
    ///
    /// The ceiling belongs to the area, so it is applied by whoever knows the
    /// area bounds — see [`DockPlacement::clamp_size`].
    pub fn set_size(&mut self, size: f32) {
        self.size = size.max(PANEL_MIN_SIZE);
    }

    /// The extent this dock takes on screen, including the strip a closed
    /// bottom dock keeps.
    #[must_use]
    pub fn extent(self, placement: DockPlacement) -> f32 {
        if self.open {
            self.size
        } else if placement.is_bottom() {
            CLOSED_BOTTOM_STRIP
        } else {
            0.0
        }
    }
}

impl DockPlacement {
    /// Clamp a dock size to what the area can give it.
    ///
    /// Room is kept for the centre and for the dock opposite, so a drag cannot
    /// squeeze the middle of a workspace out of existence.
    #[must_use]
    pub fn clamp_size(self, size: f32, area: iced::Size, opposite: f32) -> f32 {
        let max = match self.axis() {
            Axis::Horizontal => (area.width - PANEL_MIN_SIZE - opposite).max(PANEL_MIN_SIZE),
            Axis::Vertical => (area.height - PANEL_MIN_SIZE).max(PANEL_MIN_SIZE),
        };
        size.clamp(PANEL_MIN_SIZE, max)
    }

    /// The dock size a pointer at `position` asks for inside `area`.
    ///
    /// The centre has no size and answers `0.0` rather than panicking, so a
    /// caller that resizes a dock generically needs no special case.
    #[must_use]
    pub fn size_from_pointer(self, position: iced::Point, area: iced::Rectangle) -> f32 {
        match self {
            Self::Left => position.x - area.x,
            Self::Right => area.x + area.width - position.x,
            Self::Bottom => area.y + area.height - position.y,
            Self::Center => 0.0,
        }
    }
}

/// One edge dock: its own layout tree, plus its presentation state.
#[derive(Debug, Clone)]
pub struct DockRegion<K> {
    pub tree: Layout<K>,
    pub dock: Dock,
}

impl<K> DockRegion<K> {
    #[must_use]
    pub fn new(dock: Dock) -> Self {
        Self {
            tree: Layout::new(),
            dock,
        }
    }
}

/// The edge docks, keyed by placement.
///
/// A placement absent from the map has no dock: nothing to show, and no
/// affordance to bring one back. That is different from a dock that is present
/// but closed, which keeps its panels and can be reopened.
#[derive(Debug, Clone)]
pub struct DockRegions<K> {
    docks: BTreeMap<DockPlacement, DockRegion<K>>,
    /// The group that fills the whole area, when a panel is zoomed.
    zoomed: Option<NodeId>,
}

impl<K> DockRegions<K> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            docks: BTreeMap::new(),
            zoomed: None,
        }
    }

    /// The docks that exist, in [`DockPlacement::DOCKS`] order.
    pub fn iter(&self) -> impl Iterator<Item = (DockPlacement, &DockRegion<K>)> {
        self.docks.iter().map(|(&p, r)| (p, r))
    }

    /// The docks that exist, mutably, in [`DockPlacement::DOCKS`] order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (DockPlacement, &mut DockRegion<K>)> {
        self.docks.iter_mut().map(|(&p, r)| (p, r))
    }

    #[must_use]
    pub fn has_dock(&self, placement: DockPlacement) -> bool {
        !matches!(placement, DockPlacement::Center) && self.docks.contains_key(&placement)
    }

    #[must_use]
    pub fn dock(&self, placement: DockPlacement) -> Option<&Dock> {
        self.docks.get(&placement).map(|r| &r.dock)
    }

    pub fn dock_mut(&mut self, placement: DockPlacement) -> Option<&mut Dock> {
        self.docks.get_mut(&placement).map(|r| &mut r.dock)
    }

    #[must_use]
    pub fn region(&self, placement: DockPlacement) -> Option<&DockRegion<K>> {
        self.docks.get(&placement)
    }

    pub fn region_mut(&mut self, placement: DockPlacement) -> Option<&mut DockRegion<K>> {
        self.docks.get_mut(&placement)
    }

    /// Whether a dock exists and is shown. `false` for a dock the area lacks.
    #[must_use]
    pub fn is_dock_open(&self, placement: DockPlacement) -> bool {
        self.dock(placement).is_some_and(|d| d.is_open())
    }

    #[must_use]
    pub fn is_dock_collapsible(&self, placement: DockPlacement) -> bool {
        self.dock(placement).is_some_and(|d| d.is_collapsible())
    }

    /// The extent a dock takes on screen.
    #[must_use]
    pub fn extent(&self, placement: DockPlacement) -> f32 {
        self.dock(placement).map_or(0.0, |d| d.extent(placement))
    }

    /// The extent of the dock on the opposite edge, which the centre shares its
    /// space with.
    #[must_use]
    pub fn opposite_extent(&self, placement: DockPlacement) -> f32 {
        match placement {
            DockPlacement::Left => self.extent(DockPlacement::Right),
            DockPlacement::Right => self.extent(DockPlacement::Left),
            DockPlacement::Center | DockPlacement::Bottom => 0.0,
        }
    }

    /// Install a dock, or replace the one already there.
    ///
    /// Replacing keeps the size the user dragged unless `size` states a new one,
    /// so re-installing a layout does not reset a resized dock.
    pub fn insert_dock(&mut self, placement: DockPlacement, region: DockRegion<K>) {
        self.docks.insert(placement, region);
    }

    /// Take a dock away entirely, panels and all. `None` when it was not there.
    pub fn remove_dock(&mut self, placement: DockPlacement) -> Option<DockRegion<K>> {
        let removed = self.docks.remove(&placement);
        if self
            .zoomed
            .is_some_and(|z| removed.as_ref().is_some_and(|r| r.tree.contains_key(z)))
        {
            self.zoomed = None;
        }
        removed
    }

    /// The zoomed group, if one is maximized.
    #[must_use]
    pub fn zoomed(&self) -> Option<NodeId> {
        self.zoomed
    }

    /// Whether `node` is the zoomed group.
    #[must_use]
    pub fn is_zoomed(&self, node: NodeId) -> bool {
        self.zoomed == Some(node)
    }

    /// Record a zoom, or clear it.
    ///
    /// A zoom is only ever recorded here after the container accepted it, so the
    /// flag and what is drawn cannot disagree.
    pub fn set_zoomed(&mut self, node: Option<NodeId>) {
        self.zoomed = node;
    }

    /// Drop a zoom whose group is no longer part of the layout.
    ///
    /// Called after a structural edit: a zoomed group that has been emptied ends
    /// the zoom silently, which is what a user closing a maximized panel expects.
    ///
    /// Reachability is what is tested, not mere presence in the arena. Collapsing an
    /// emptied root clears the root's child without removing the nodes, so a node
    /// can still be *stored* while nothing draws it — and a zoom pointing at one
    /// would render as a blank area rather than as the layout.
    pub fn prune_zoom(&mut self, center: &Layout<K>) {
        let Some(node) = self.zoomed else {
            return;
        };
        let reachable = is_reachable(center, node)
            || self
                .docks
                .values()
                .any(|r| is_reachable(&r.tree, node));
        if !reachable {
            self.zoomed = None;
        }
    }

    /// Transfer dock state — sizes and open flags — from another set of regions.
    ///
    /// Used when installing a whole layout: the incoming trees are fresh values,
    /// but a dock size is not part of a tree, so it is carried across by
    /// placement rather than being reset to the default.
    pub fn adopt_dock_state(&mut self, other: &Self) {
        for (placement, region) in &mut self.docks {
            if let Some(incoming) = other.docks.get(placement) {
                region.dock = incoming.dock;
            }
        }
    }
}

impl<K> Default for DockRegions<K> {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether `node` is reachable from a tree's root, walking the same shape the
/// renderer does.
fn is_reachable<K>(layout: &Layout<K>, node: NodeId) -> bool {
    fn walk<K>(layout: &Layout<K>, current: NodeId, target: NodeId) -> bool {
        if current == target {
            return true;
        }
        match layout.kind(current) {
            Some(super::NodeKind::Pane(pane)) => pane
                .tabs
                .iter()
                .any(|&tab| walk(layout, tab, target)),
            Some(super::NodeKind::Proportional(pg)) => pg
                .children
                .iter()
                .any(|&child| walk(layout, child, target)),
            Some(super::NodeKind::Root(_) | super::NodeKind::Panel(_)) | None => false,
        }
    }
    layout.root_child().is_some_and(|root| walk(layout, root, node))
}

/// A [`DockRegions`] carrying only the open flag and size, for tests and for
/// callers that describe a dock without building its tree.
#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> iced::Size {
        iced::Size::new(800.0, 600.0)
    }

    #[test]
    fn a_dock_is_open_and_collapsible_when_created() {
        let dock = Dock::new(240.0);
        assert!(dock.is_open());
        assert!(dock.is_collapsible());
        assert_eq!(dock.size(), 240.0);
    }

    #[test]
    fn a_dock_size_is_floored_at_the_minimum() {
        let mut dock = Dock::new(10.0);
        assert_eq!(dock.size(), PANEL_MIN_SIZE);
        dock.set_size(1.0);
        assert_eq!(dock.size(), PANEL_MIN_SIZE);
    }

    #[test]
    fn a_closed_bottom_dock_keeps_a_strip_and_the_sides_take_nothing() {
        let mut dock = Dock::new(200.0);
        dock.set_open(false);
        assert_eq!(dock.extent(DockPlacement::Bottom), CLOSED_BOTTOM_STRIP);
        assert_eq!(dock.extent(DockPlacement::Left), 0.0);
        assert_eq!(dock.extent(DockPlacement::Right), 0.0);
    }

    #[test]
    fn a_size_is_clamped_to_leave_the_centre_its_minimum() {
        // 800 wide, a 200px dock opposite: the largest this one may be is
        // 800 - 100 (centre) - 200 (opposite).
        let clamped = DockPlacement::Right.clamp_size(900.0, area(), 200.0);
        assert_eq!(clamped, 500.0);
        // And never below the dock minimum, however small the area.
        let tiny = DockPlacement::Right.clamp_size(900.0, iced::Size::new(120.0, 120.0), 100.0);
        assert_eq!(tiny, PANEL_MIN_SIZE);
    }

    #[test]
    fn a_pointer_maps_to_a_size_from_the_docks_own_edge() {
        let area = iced::Rectangle {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        assert_eq!(
            DockPlacement::Left.size_from_pointer(iced::Point::new(180.0, 300.0), area),
            180.0
        );
        assert_eq!(
            DockPlacement::Right.size_from_pointer(iced::Point::new(620.0, 300.0), area),
            180.0
        );
        assert_eq!(
            DockPlacement::Bottom.size_from_pointer(iced::Point::new(400.0, 450.0), area),
            150.0
        );
        // The centre has no size, and answers rather than panicking.
        assert_eq!(
            DockPlacement::Center.size_from_pointer(iced::Point::new(400.0, 300.0), area),
            0.0
        );
    }

    #[test]
    fn a_dock_absent_from_the_map_is_not_open() {
        let regions: DockRegions<()> = DockRegions::new();
        assert!(!regions.is_dock_open(DockPlacement::Left));
        assert!(!regions.has_dock(DockPlacement::Left));
        assert_eq!(regions.extent(DockPlacement::Left), 0.0);
    }

    #[test]
    fn removing_a_dock_takes_its_tree_with_it() {
        let mut regions: DockRegions<()> = DockRegions::new();
        regions.insert_dock(DockPlacement::Left, DockRegion::new(Dock::new(200.0)));
        assert!(regions.has_dock(DockPlacement::Left));

        let removed = regions.remove_dock(DockPlacement::Left);
        assert!(removed.is_some());
        assert!(!regions.has_dock(DockPlacement::Left));
        assert!(regions.remove_dock(DockPlacement::Left).is_none());
    }

    #[test]
    fn removing_the_dock_that_holds_the_zoom_ends_the_zoom() {
        let mut regions: DockRegions<()> = DockRegions::new();
        let region = DockRegion::new(Dock::new(200.0));
        let node = region.tree.root;
        regions.insert_dock(DockPlacement::Right, region);
        regions.set_zoomed(Some(node));
        assert!(regions.is_zoomed(node));

        regions.remove_dock(DockPlacement::Right);
        assert_eq!(regions.zoomed(), None);
    }

    #[test]
    fn adopting_dock_state_carries_the_size_across() {
        let mut regions: DockRegions<()> = DockRegions::new();
        regions.insert_dock(DockPlacement::Left, DockRegion::new(Dock::new(200.0)));

        let mut incoming: DockRegions<()> = DockRegions::new();
        incoming.insert_dock(DockPlacement::Left, DockRegion::new(Dock::new(320.0)));
        incoming.dock_mut(DockPlacement::Left).unwrap().set_open(false);

        regions.adopt_dock_state(&incoming);
        assert_eq!(regions.dock(DockPlacement::Left).unwrap().size(), 320.0);
        assert!(!regions.is_dock_open(DockPlacement::Left));
    }

    #[test]
    fn the_opposite_extent_is_the_dock_across_the_centre() {
        let mut regions: DockRegions<()> = DockRegions::new();
        regions.insert_dock(DockPlacement::Left, DockRegion::new(Dock::new(180.0)));
        regions.insert_dock(DockPlacement::Right, DockRegion::new(Dock::new(260.0)));

        assert_eq!(regions.opposite_extent(DockPlacement::Right), 180.0);
        assert_eq!(regions.opposite_extent(DockPlacement::Left), 260.0);
        assert_eq!(regions.opposite_extent(DockPlacement::Bottom), 0.0);
    }
}
