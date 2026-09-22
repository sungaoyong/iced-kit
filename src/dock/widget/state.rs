// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

use std::collections::HashSet;

use iced::Rectangle;

use crate::dock::builder::compile::{first_pane, first_pane_where, BuiltLayout};
use crate::dock::builder::DockIndex;
use crate::dock::factory::Factory;
use crate::dock::manager::{DockManager, DragSession, TabBarTarget};
use crate::dock::model::{
    DockPlacement, DockRegion, DockRegions, Layout as DockLayout, NodeEntry, NodeId, NodeKind, Pane,
};
use crate::dock::widget::action::{DockAction, TabAction};

/// Persistent docking state shared between the [`Dock`](crate::dock::Dock) widget
/// and the application via `Rc<RefCell<DockWidgetState<K>>>`.
///
/// Obtain one from [`DockSession::state`](crate::dock::DockSession::state) or
/// [`DockWidgetState::from_tree`].
#[derive(Debug, Clone)]
pub struct DockWidgetState<K> {
    /// The centre split/tab layout graph.
    ///
    /// The centre is the main area and always present; edge docks live beside it
    /// in [`Self::regions`]. Keeping the centre where it always was means code
    /// written against a single tree keeps working unchanged.
    pub layout: DockLayout<K>,
    /// The edge docks, each with its own tree, plus any zoom.
    pub regions: DockRegions<K>,
    /// String-id lookup index, rebuilt automatically when the layout changes.
    pub index: DockIndex,
    /// Active tab-drag session, if a drag is in progress.
    pub drag: Option<DragSession>,
    /// Per-frame pane content drop targets, rebuilt by every layout pass.
    pub drop_targets: Vec<(NodeId, Rectangle)>,
    /// Per-frame tab-bar drop targets, rebuilt by every layout pass.
    pub tab_bar_targets: Vec<TabBarTarget>,
    /// Absolute bounds of each visible pane, collected each draw pass.
    pub pane_bounds: Vec<(NodeId, Rectangle)>,
    /// Absolute bounds of the whole area, recorded during layout. Dock resizing
    /// needs the area's extent to clamp against.
    pub area_bounds: Option<Rectangle>,
    /// The dock whose resize handle is being dragged, if any. Only one can be.
    pub resizing_dock: Option<DockPlacement>,
    /// Pane that last received user focus (tab click or content click).
    pub focused_pane: Option<NodeId>,
    /// Pane that currently draws the focus frame.
    ///
    /// Sticky: only follows [`Self::focused_pane`] when the newly focused pane is eligible
    /// under [`Self::focus_frame_groups`]. Equal to `focused_pane` when no filter is set.
    pub focus_frame_pane: Option<NodeId>,
    /// Tab groups eligible for the focus frame. `None` means every pane is eligible.
    ///
    /// Set through [`DockBuilder::focus_frame_groups`](crate::dock::widget::DockBuilder::focus_frame_groups).
    pub focus_frame_groups: Option<HashSet<String>>,
    /// Panes that belong to a closed dock, and so draw collapsed.
    ///
    /// Recomputed by the widget each time it builds, because which region owns a
    /// pane is not something a pane can answer on its own.
    pub collapsed_panes: HashSet<NodeId>,
    /// Panels the application has hidden at runtime.
    ///
    /// A set of ids rather than a flag on the panel node, because visibility is
    /// application state that changes without the layout changing: a hidden
    /// panel must not force a rebuild of the tree around it. Declared visibility
    /// ([`PanelDef::visible`](crate::dock::PanelDef::visible)) is folded in when
    /// the layout is compiled.
    pub hidden: HashSet<String>,
    /// Set when focus changed without a layout rebuild; triggers a redraw.
    pub focus_dirty: bool,
    /// Set when the layout tree changes and the cached widget root must rebuild.
    pub layout_dirty: bool,
}

impl<K> DockWidgetState<K> {
    /// Rebuild string-id index from the current layout graph.
    pub fn sync_index(&mut self) {
        let mut index = DockIndex::rebuild_from_layout(&self.layout);
        for (_, region) in self.regions.iter() {
            for (id, node) in DockIndex::rebuild_from_layout(&region.tree).panels {
                index.panels.insert(id, node);
            }
            for (name, node) in DockIndex::rebuild_from_layout(&region.tree).panes {
                index.panes.insert(name, node);
            }
        }
        self.index = index;
        self.resync_focus_frame();
    }

    /// Whether `pane` may draw the focus frame under the current group filter.
    #[must_use]
    pub fn frame_eligible(&self, pane: NodeId) -> bool {
        matches!(
            self.layout.kind(pane),
            Some(NodeKind::Pane(p))
                if pane_frame_eligible(self.focus_frame_groups.as_ref(), p)
        )
    }

    /// Move logical focus to `pane`, dragging the focus frame along when eligible.
    ///
    /// Returns `true` when either field changed.
    pub(crate) fn focus(&mut self, pane: NodeId) -> bool {
        let mut changed = false;
        if self.focused_pane != Some(pane) {
            self.focused_pane = Some(pane);
            changed = true;
        }
        if self.focus_frame_pane != Some(pane) && self.frame_eligible(pane) {
            self.focus_frame_pane = Some(pane);
            changed = true;
        }
        if changed {
            self.focus_dirty = true;
        }
        changed
    }

    /// Restrict the focus frame to panes tagged with one of `groups`.
    ///
    /// `None` restores the default, where every pane shows the frame while focused.
    pub fn set_focus_frame_groups(&mut self, groups: Option<HashSet<String>>) {
        if self.focus_frame_groups == groups {
            return;
        }
        self.focus_frame_groups = groups;
        self.resync_focus_frame();
        self.focus_dirty = true;
    }

    /// Re-resolve [`Self::focus_frame_pane`] after a policy or structural change.
    ///
    /// Repairs a frame that became dangling or ineligible; never resurrects one that was
    /// deliberately cleared.
    fn resync_focus_frame(&mut self) {
        if self.focused_pane.is_some_and(|p| self.frame_eligible(p)) {
            self.focus_frame_pane = self.focused_pane;
            return;
        }
        let Some(current) = self.focus_frame_pane else {
            return;
        };
        if self.frame_eligible(current) {
            return;
        }
        let groups = self.focus_frame_groups.as_ref();
        self.focus_frame_pane =
            first_pane_where(&self.layout, |_, pane| pane_frame_eligible(groups, pane));
    }

    pub(crate) fn commit_layout(&mut self) {
        if self.layout_dirty {
            self.sync_index();
            self.layout_dirty = false;
        }
    }

    /// Build widget state from a declarative [`LayoutTree`](crate::dock::LayoutTree).
    ///
    /// The tree becomes the centre; the area has no edge docks. Use
    /// [`from_area`](Self::from_area) to describe docks as well.
    pub fn from_tree(tree: crate::dock::LayoutTree<K>) -> crate::dock::Result<Self>
    where
        K: Copy,
    {
        Self::from_area(crate::dock::LayoutArea::new(tree))
    }

    /// Build widget state from a declarative [`LayoutArea`](crate::dock::LayoutArea).
    pub fn from_area(area: crate::dock::LayoutArea<K>) -> crate::dock::Result<Self>
    where
        K: Copy,
    {
        let (built, regions) = crate::dock::builder::compile::build_area(&area)?;
        let focused_pane = first_pane(&built.layout);
        let mut state = Self::from_built(built, focused_pane);
        state.regions = regions;
        state.layout_dirty = true;
        Ok(state)
    }

    /// Build widget state from a compiled layout.
    #[must_use]
    pub fn from_built(built: BuiltLayout<K>, focused_pane: Option<NodeId>) -> Self {
        Self {
            layout: built.layout,
            regions: DockRegions::new(),
            index: built.index,
            drag: None,
            drop_targets: Vec::new(),
            tab_bar_targets: Vec::new(),
            pane_bounds: Vec::new(),
            area_bounds: None,
            resizing_dock: None,
            collapsed_panes: HashSet::new(),
            focused_pane,
            focus_frame_pane: focused_pane,
            focus_frame_groups: None,
            hidden: HashSet::new(),
            focus_dirty: false,
            layout_dirty: true,
        }
    }

    /// Install a whole layout: a centre tree plus its edge docks.
    ///
    /// The dock sizes and open flags the user dragged are carried across by
    /// placement, so re-installing a layout does not reset a resized or collapsed
    /// dock.
    pub fn set_area(&mut self, area: crate::dock::LayoutArea<K>) -> crate::dock::Result
    where
        K: Copy,
    {
        let (built, mut regions) = crate::dock::builder::compile::build_area(&area)?;
        regions.adopt_dock_state(&self.regions);
        self.layout = built.layout;
        self.regions = regions;
        self.index = built.index;
        self.focused_pane = first_pane(&self.layout);
        self.focus_frame_pane = self.focused_pane;
        self.drag = None;
        self.layout_dirty = true;
        Ok(())
    }

    /// The tree a node lives in: the centre, or one of the docks.
    ///
    /// A `NodeId` is unique across the whole area, so at most one tree can hold it —
    /// but *finding* that tree needs every region to be consulted, which only the
    /// state can do. Everything that takes an id from the application and needs to
    /// read or edit it goes through here, because a panel in a dock is not reachable
    /// from the centre alone.
    #[must_use]
    pub fn tree_of(&self, node: NodeId) -> Option<&DockLayout<K>> {
        if self.layout.contains_key(node) {
            return Some(&self.layout);
        }
        self.regions
            .iter()
            .find(|(_, region)| region.tree.contains_key(node))
            .map(|(_, region)| &region.tree)
    }

    /// The tree a node lives in, mutably.
    pub fn tree_of_mut(&mut self, node: NodeId) -> Option<&mut DockLayout<K>> {
        if self.layout.contains_key(node) {
            return Some(&mut self.layout);
        }
        self.regions
            .iter_mut()
            .find(|(_, region)| region.tree.contains_key(node))
            .map(|(_, region)| &mut region.tree)
    }

    /// The pane holding a panel, wherever it is in the area.
    #[must_use]
    pub fn pane_of_panel(&self, panel: NodeId) -> Option<NodeId> {
        self.layout
            .get(panel)
            .and_then(|entry| entry.owner)
            .or_else(|| {
                self.regions
                    .iter()
                    .find_map(|(_, region)| region.tree.get(panel).and_then(|e| e.owner))
            })
    }

    /// The pane holding a panel, by the panel's string id.
    #[must_use]
    pub fn pane_for_panel_id(&self, id: &str) -> Option<NodeId> {
        let panel = self.index.panels.get(id).copied()?;
        self.pane_of_panel(panel)
    }

    /// A pane's state, wherever it is in the area.
    #[must_use]
    pub fn pane(&self, pane: NodeId) -> Option<&Pane> {
        match self.tree_of(pane).and_then(|tree| tree.kind(pane)) {
            Some(NodeKind::Pane(pane)) => Some(pane),
            _ => None,
        }
    }

    /// The panel a pane is displaying: the first visible one, since a hidden panel is
    /// passed over rather than shown.
    ///
    /// This is what the widget draws, so it — not the stored `active` — is the answer
    /// to "which panel is on screen".
    #[must_use]
    pub fn displayed_panel(&self, pane: NodeId) -> Option<NodeId> {
        let pane_state = self.pane(pane)?;
        let tree = self.tree_of(pane)?;

        // Both halves of visibility are consulted: the declared flag on the node, and
        // the runtime set. `compile::displayed_panel` only knows the first, because a
        // tree is a value and the hidden set is session state — so the stored active
        // tab is checked against the second here before falling back.
        if let Some(active) = pane_state.active {
            if pane_state.tabs.contains(&active) && self.is_node_visible(tree, active) {
                return Some(active);
            }
        }
        pane_state
            .tabs
            .iter()
            .copied()
            .find(|&tab| self.is_node_visible(tree, tab))
    }

    /// Whether a panel node is offered, by both its declared flag and the runtime set.
    #[must_use]
    pub fn is_node_visible(&self, tree: &DockLayout<K>, panel: NodeId) -> bool {
        crate::dock::builder::compile::panel_is_visible(tree, panel)
            && tree
                .kind(panel)
                .and_then(|kind| match kind {
                    NodeKind::Panel(panel) => Some(panel.id.as_str()),
                    _ => None,
                })
                .is_none_or(|id| self.is_panel_visible(id))
    }

    /// Whether a region's tree would draw anything.
    ///
    /// A region whose every panel is hidden is empty, whatever its nodes say:
    /// the same question [`compile::panel_is_visible`] answers per panel,
    /// asked of the whole tree, with the runtime hidden set folded in. The
    /// area uses this to give an empty region's extent back to the centre —
    /// a dock with nothing on screen holding 240 pixels reads as "hide did
    /// not work" just as much as the panel staying visible would.
    #[must_use]
    pub fn region_is_empty(&self, tree: &DockLayout<K>) -> bool {
        fn walk<K>(state: &DockWidgetState<K>, tree: &DockLayout<K>, node: NodeId) -> bool {
            match tree.kind(node) {
                Some(NodeKind::Panel(_)) => !state.is_node_visible(tree, node),
                Some(NodeKind::Pane(pane)) => pane
                    .tabs
                    .iter()
                    .all(|&tab| walk(state, tree, tab)),
                Some(NodeKind::Proportional(pg)) => pg
                    .children
                    .iter()
                    .all(|&child| walk(state, tree, child)),
                // A root with a child is answered by the child; an empty root
                // draws nothing, so it counts as empty.
                Some(NodeKind::Root(root)) => root
                    .child
                    .is_none_or(|child| walk(state, tree, child)),
                None => true,
            }
        }
        let Some(root) = tree.root_child() else {
            return true;
        };
        walk(self, tree, root)
    }

    /// Whether the tab bar should offer panel `id`.
    #[must_use]
    pub fn is_panel_visible(&self, id: &str) -> bool {
        !self.hidden.contains(id)
    }

    /// Hide or show a panel at runtime, without touching the layout.
    ///
    /// A hidden panel keeps its node and its tab slot; it is left out of the tab
    /// strip and passed over when the displayed tab is resolved. Returns whether
    /// anything changed.
    pub fn set_panel_visible(&mut self, id: &str, visible: bool) -> bool {
        let changed = if visible {
            self.hidden.remove(id)
        } else {
            self.hidden.insert(id.to_owned())
        };
        if changed {
            self.layout_dirty = true;
        }
        changed
    }
}

/// Which of the area's trees a node lives in, as a value.
///
/// A reference from `tree_of` cannot be held across the edits a cross-tree move
/// makes — detaching in one tree while borrowing the other — so the tree is named
/// instead and re-borrowed per step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegionKey {
    Center,
    Dock(DockPlacement),
}

impl RegionKey {}

/// Whether a pane's group tag passes the focus-frame filter (`None` accepts every pane).
fn pane_frame_eligible(groups: Option<&HashSet<String>>, pane: &Pane) -> bool {
    groups.is_none_or(|groups| pane.group.as_deref().is_some_and(|g| groups.contains(g)))
}

impl<K> Default for DockWidgetState<K> {
    fn default() -> Self {
        let layout = DockLayout::new();
        let index = DockIndex::rebuild_from_layout(&layout);
        Self {
            layout,
            regions: DockRegions::new(),
            index,
            drag: None,
            drop_targets: Vec::new(),
            tab_bar_targets: Vec::new(),
            pane_bounds: Vec::new(),
            area_bounds: None,
            resizing_dock: None,
            collapsed_panes: HashSet::new(),
            focused_pane: None,
            focus_frame_pane: None,
            focus_frame_groups: None,
            hidden: HashSet::new(),
            focus_dirty: false,
            layout_dirty: false,
        }
    }
}

impl<K> DockWidgetState<K> {
    /// Install or replace an edge dock's tree.
    pub fn set_dock(
        &mut self,
        placement: DockPlacement,
        built: BuiltLayout<K>,
        dock: crate::dock::model::Dock,
    ) {
        for (id, node) in &built.index.panels {
            self.index.panels.insert(id.clone(), *node);
        }
        for (name, node) in &built.index.panes {
            self.index.panes.insert(name.clone(), *node);
        }
        self.regions.insert_dock(
            placement,
            DockRegion {
                tree: built.layout,
                dock,
            },
        );
        self.sync_index();
        self.layout_dirty = true;
    }

    /// Take an edge dock away entirely, panels and all.
    ///
    /// Returns whether a dock was there to remove.
    pub fn remove_dock(&mut self, placement: DockPlacement) -> bool {
        let removed = self.regions.remove_dock(placement);
        if let Some(region) = &removed {
            for (id, node) in DockIndex::rebuild_from_layout(&region.tree).panels {
                // A panel that leaves for good must not keep an index entry, or a
                // later open-by-id would resolve to a node no longer in any tree.
                if self.index.panels.get(&id) == Some(&node) {
                    self.index.panels.remove(&id);
                }
            }
        }
        if removed.is_some() {
            self.sync_index();
            self.layout_dirty = true;
        }
        removed.is_some()
    }

    /// Show or hide a dock.
    ///
    /// Closing never refuses: a non-collapsible dock only refuses the *toggle
    /// affordance* — see [`toggle_dock`](Self::toggle_dock) — so an application
    /// can still hide a dock it marked as fixed.
    pub fn set_dock_open(&mut self, placement: DockPlacement, open: bool) -> bool {
        let Some(dock) = self.regions.dock_mut(placement) else {
            return false;
        };
        if dock.is_open() == open {
            return false;
        }
        dock.set_open(open);
        self.layout_dirty = true;
        true
    }

    /// Toggle a dock, refusing to close one that is not collapsible.
    ///
    /// Opening is never refused. Returns whether the state changed.
    pub fn toggle_dock(&mut self, placement: DockPlacement) -> bool {
        let Some(dock) = self.regions.dock(placement) else {
            return false;
        };
        if dock.is_open() && !dock.is_collapsible() {
            return false;
        }
        self.set_dock_open(placement, !dock.is_open())
    }

    /// Resize a dock from a pointer position, clamping to the area.
    ///
    /// A closed dock is reopened by the drag, matching the behaviour of dragging
    /// its handle: the pointer is asking to see the dock, and a resize that
    /// changed an invisible extent would look like nothing happened.
    pub fn resize_dock_to(&mut self, placement: DockPlacement, position: iced::Point) -> bool {
        let area = self.area_bounds.unwrap_or(Rectangle {
            x: 0.0,
            y: 0.0,
            width: f32::MAX,
            height: f32::MAX,
        });
        let mut changed = false;
        if !self.regions.is_dock_open(placement) && self.regions.has_dock(placement) {
            let Some(dock) = self.regions.dock_mut(placement) else {
                return false;
            };
            dock.set_open(true);
            changed = true;
        }
        let opposite = self.regions.opposite_extent(placement);
        let requested = placement.size_from_pointer(position, area);
        let size = placement.clamp_size(requested, area.size(), opposite);
        if let Some(dock) = self.regions.dock_mut(placement) {
            if dock.size() != size {
                dock.set_size(size);
                changed = true;
            }
        }
        if changed {
            self.layout_dirty = true;
        }
        changed
    }

    /// Whether a panel node may be zoomed.
    #[must_use]
    pub fn panel_can_zoom(&self, panel: NodeId) -> bool {
        self.tree_of(panel)
            .and_then(|tree| tree.kind(panel))
            .and_then(|k| match k {
                NodeKind::Panel(p) => Some(p.can_zoom),
                _ => None,
            })
            .unwrap_or(false)
    }

    /// Whether the pane `node` belongs to is the zoomed one.
    #[must_use]
    pub fn is_zoomed(&self, node: NodeId) -> bool {
        self.regions.is_zoomed(node)
    }

    /// Zoom a pane to fill the area, or restore the layout.
    ///
    /// Zooming *in* is refused when `node` is not a pane holding a panel that
    /// allows it; zooming *out* is never refused, so a panel that stops offering
    /// the control while zoomed cannot strand the user with no way back.
    ///
    /// The pane is looked up wherever it lives: a dock's pane is not in the centre
    /// tree, and a zoom that only consulted the centre would refuse every dock
    /// panel — which reads as "maximize does not work".
    #[must_use]
    pub fn set_zoom(&mut self, node: NodeId, zoomed: bool) -> bool {
        if zoomed {
            let Some(panel) = self
                .tree_of(node)
                .and_then(|tree| tree.kind(node))
                .and_then(|k| match k {
                    NodeKind::Pane(p) => p.active.or_else(|| p.tabs.first().copied()),
                    _ => None,
                })
            else {
                return false;
            };
            if !self.panel_can_zoom(panel) {
                return false;
            }
        }
        if self.regions.is_zoomed(node) == zoomed {
            return false;
        }
        self.regions.set_zoomed(zoomed.then_some(node));
        self.layout_dirty = true;
        true
    }

    /// Flip the zoom between `node` and the rest of the layout.
    #[must_use]
    pub fn toggle_zoom(&mut self, node: NodeId) -> bool {
        self.set_zoom(node, !self.regions.is_zoomed(node))
    }
}

impl<K: Clone> DockWidgetState<K> {
    /// Close a panel wherever it lives. Returns whether it was closed.
    ///
    /// A panel in a dock is not reachable from the centre tree, so the tree it
    /// belongs to is resolved first and the edit applied there.
    pub fn close_panel(&mut self, panel: NodeId) -> bool {
        let Some(tree) = self.tree_of_mut(panel) else {
            return false;
        };
        let closed = Factory.close(tree, panel).is_ok();
        if closed {
            self.layout_dirty = true;
            self.sync_index();
            self.regions.prune_zoom(&self.layout);
        }
        closed
    }

    /// Activate a tab, wherever its pane lives.
    ///
    /// A pane in a dock is not in the centre tree, so the edit is applied to the tree
    /// that owns it and the session is told which pane now has focus.
    pub fn set_active_panel(&mut self, pane: NodeId, panel: NodeId) -> bool {
        let Some(tree) = self.tree_of_mut(pane) else {
            return false;
        };
        Factory.set_active_panel(tree, pane, panel);
        self.layout_dirty = true;
        self.focus(pane);
        true
    }

    /// Reorder or move a tab, wherever its pane lives.
    ///
    /// The insertion is resolved against the target pane, which may be in another
    /// region than the source — a drag from a dock into the centre is exactly that.
    fn apply_tab_insert(&mut self, session: DragSession, pane: NodeId, index: usize) -> bool {
        // Same tree: the manager can reorder in place.
        let same_tree = self
            .tree_of(session.source_pane)
            .zip(self.tree_of(pane))
            .is_some_and(|(a, b)| std::ptr::eq(a, b));
        if same_tree {
            let Some(tree) = self.tree_of_mut(pane) else {
                return false;
            };
            return DockManager
                .execute_tab_insert(tree, session, pane, index)
                .is_ok();
        }

        // Across trees: detach from the source and insert into the target, which is
        // the same shape a cross-pane fill takes.
        self.move_panel_between(session.source_panel, pane, Some(index))
    }

    /// Undo a completed cross-tree move: take the panel out of `target_pane`'s
    /// tree and put it back in `original_owner`.
    ///
    /// Used when a split refused *after* the panel had already travelled: without
    /// this, an edge drop that validated one way and applied another would leave
    /// the panel merged into the target pane. The panel was the target pane's last
    /// tab at that point, so undoing the move leaves that pane empty, and
    /// `move_panel_between` collapses it the way any removal does.
    fn undo_move_between(&mut self, panel: NodeId, target_pane: NodeId, original_owner: NodeId) {
        let _ = self.move_panel_between(panel, original_owner, None);
        let _ = target_pane;
    }

    /// Put a detached panel back into its old tree, exactly where it came from.
    ///
    /// The rollback half of a cross-tree move: the node re-enters under the id it
    /// already had, re-parented to the owner it was taken from. A failed insert
    /// must return the panel to its pane, never strand it in neither tree.
    fn restore_detached(&mut self, panel: NodeId, entry: NodeEntry<K>, old_owner: Option<NodeId>) {
        let Some(tree) = self.tree_of_mut(panel) else {
            return;
        };
        let mut entry = entry;
        entry.owner = old_owner;
        tree.insert_with_id(panel, entry);
        if let Some(owner) = old_owner {
            if let Some(NodeKind::Pane(ref mut p)) = tree.get_mut(owner).map(|e| &mut e.kind) {
                if !p.tabs.contains(&panel) {
                    p.tabs.push(panel);
                }
            }
        }
    }

    /// Move a panel to a pane, possibly in another region.
    ///
    /// Detaching and inserting happen as one operation so a failure part-way cannot
    /// leave the panel in neither tree — the same bargain the reference makes, where
    /// a move is a detach plus an insert and a failed insert is a silent no-op with
    /// the panel back where it started.
    fn move_panel_between(
        &mut self,
        panel: NodeId,
        target_pane: NodeId,
        index: Option<usize>,
    ) -> bool {
        let factory = Factory;

        // The node is *moved between arenas*, not just re-parented: the centre and
        // each dock are separate trees, and an insert is only valid within one of
        // them. Taking the entry out of its own tree and putting it into the target
        // under the same id is what keeps the panel's identity — which the index, a
        // zoom and an open drag all hold references to.
        let Some(source) = self.tree_of_mut(panel) else {
            return false;
        };
        let old_owner = source.get(panel).and_then(|e| e.owner);
        let detached = factory.remove_from_parent(source, panel).is_ok();
        if !detached {
            return false;
        }
        let Some(entry) = source.remove(panel) else {
            // `remove_from_parent` succeeded, so the node was there; a `None` here
            // would mean the tree changed underneath us. Put nothing back and report
            // the failure rather than operating on a half-moved tree.
            return false;
        };

        // The target is named before the panel is staged anywhere: a pane that
        // resolves to no tree must not strand the panel mid-move.
        let Some(target_key) = self.tree_id_of(target_pane) else {
            self.restore_detached(panel, entry, old_owner);
            return false;
        };
        let Some(target) = self.tree_mut(target_key) else {
            self.restore_detached(panel, entry, old_owner);
            return false;
        };
        // Cloned rather than moved: the entry is what a failed insert puts back
        // into the source tree, so it has to survive this one.
        target.insert_with_id(panel, entry.clone_for_insert());

        let inserted = match index {
            Some(index) => factory.insert_panel_at(target, target_pane, panel, index).is_ok(),
            None => factory.add_panel_to_pane(target, target_pane, panel).is_ok(),
        };
        if !inserted {
            self.restore_detached(panel, entry, old_owner);
            return false;
        }

        // The pane the panel left may be empty now, and an empty group draws as a
        // blank region. Collapsing it is what makes "drag a dock's last panel into
        // the centre" leave the dock tidy rather than hollow — the same tidy-up
        // `Factory::close` does for its own removals.
        if let Some(owner) = old_owner {
            if let Some(source) = self.tree_mut(target_key) {
                factory.collapse_owner_of(source, owner);
            }
        }

        self.layout_dirty = true;
        true
    }

    /// Which tree holds `node`, as a key rather than a borrow.
    ///
    /// `tree_of` returns a reference, which cannot be held across the mutable
    /// edits in between; a region key can.
    fn tree_id_of(&self, node: NodeId) -> Option<RegionKey> {
        if self.layout.contains_key(node) {
            return Some(RegionKey::Center);
        }
        self.regions
            .iter()
            .find(|(_, region)| region.tree.contains_key(node))
            .map(|(placement, _)| RegionKey::Dock(placement))
    }

    fn tree_mut(&mut self, key: RegionKey) -> Option<&mut DockLayout<K>> {
        match key {
            RegionKey::Center => Some(&mut self.layout),
            RegionKey::Dock(placement) => Some(&mut self.regions.region_mut(placement)?.tree),
        }
    }

    fn apply_fill(&mut self, panel: NodeId, target_pane: NodeId) -> bool {
        let same_tree = self
            .tree_of(panel)
            .zip(self.tree_of(target_pane))
            .is_some_and(|(a, b)| std::ptr::eq(a, b));
        if same_tree {
            let Some(tree) = self.tree_of_mut(target_pane) else {
                return false;
            };
            return Factory.dock_fill(tree, panel, target_pane).is_ok();
        }
        self.move_panel_between(panel, target_pane, None)
    }

    /// Split a pane, placing a panel beside it, across regions.
    fn apply_split(
        &mut self,
        session: DragSession,
        target: NodeId,
        operation: crate::dock::model::DockOperation,
    ) -> bool {
        let factory = Factory;
        let same_tree = self
            .tree_of(session.source_pane)
            .zip(self.tree_of(target))
            .is_some_and(|(a, b)| std::ptr::eq(a, b));

        if !same_tree {
            // Across trees the panel must travel *into* the target's tree before
            // anything splits: the split helpers only rearrange nodes that are
            // already in the tree they are handed. Detaching first and splitting
            // after had it backwards — the split then failed on a panel with no
            // owner, and the drop left the panel in neither tree.
            //
            // The rollback rule is the reference's: a move is a detach plus an
            // insert, and an insert that places nothing is a no-op with the panel
            // back where it started.
            let original_owner = self
                .tree_of(session.source_panel)
                .and_then(|tree| tree.get(session.source_panel))
                .and_then(|entry| entry.owner);
            let moved_to_target = self.move_panel_between(session.source_panel, target, None);
            if !moved_to_target {
                return false;
            }

            // The panel now sits as the last tab of the target pane. Edge drop
            // means "beside", so it splits out of that pane into its own group —
            // the same shape the reference's `InsertTarget::Split` produces.
            let split = match self.tree_of_mut(target) {
                Some(tree) => factory
                    .split_cross_pane_edge(tree, target, session.source_panel, target, operation)
                    .is_ok(),
                None => false,
            };
            if split {
                self.layout_dirty = true;
            } else if let Some(owner) = original_owner {
                // The split refused, so undo the move rather than leave the panel
                // merged into a pane the user asked to split.
                self.undo_move_between(session.source_panel, target, owner);
            }
            return split;
        }

        let Some(tree) = self.tree_of_mut(target) else {
            return false;
        };
        let result = if session.source_pane == target {
            factory.split_same_pane_edge(tree, target, session.source_panel, operation)
        } else {
            factory.split_cross_pane_edge(
                tree,
                session.source_pane,
                session.source_panel,
                target,
                operation,
            )
        };
        let ok = result.is_ok();
        if ok {
            self.layout_dirty = true;
        }
        ok
    }
}

/// End an active drag at `cursor`, applying a drop when valid.
pub fn finish_drag<K: Clone>(state: &mut DockWidgetState<K>, cursor: Option<iced::Point>) -> bool {
    let Some(cursor) = cursor else {
        let had_drag = state.drag.is_some();
        state.drag = None;
        return had_drag;
    };

    // The geometry of the frame the user is looking at. The panes record it while
    // drawing, and `draw` is the last thing a frame does — so between frames these
    // hold exactly the boxes that were on screen, which is what a drop is measured
    // against.
    let drop_targets = state.drop_targets.clone();
    let tab_bar_targets = state.tab_bar_targets.clone();
    let Some(mut session) = state.drag.take() else {
        return false;
    };

    DockManager::update_drag_hover_full(&mut session, cursor, &drop_targets, &tab_bar_targets);
    let mut changed = false;
    if let Some((pane, index)) = session.tab_insert {
        changed = state.apply_tab_insert(session, pane, index);
    } else if let Some(target) = session.hover_target {
        match session.operation {
            Some(crate::dock::model::DockOperation::Fill) => {
                changed = state.apply_fill(session.source_panel, target);
            }
            Some(operation) => {
                changed = state.apply_split(session, target, operation);
            }
            None => {}
        }
    }
    if changed {
        state.sync_index();
        state.regions.prune_zoom(&state.layout);
    }
    changed
}

/// Apply a [`DockAction`] to dock state (programmatic / session API).
///
/// Does not emit [`DockEvent`](crate::dock::DockEvent) values. After a successful structural change, call
/// [`DockWidgetState::sync_index`] or rely on the widget's next layout pass.
///
/// `K: Clone` because a cross-tree drop stages a panel's entry into the target
/// tree while keeping a copy for rollback; a dock's own keys are `Copy`, so this
/// is never a cost in practice.
pub fn dispatch_action<K: Clone>(state: &mut DockWidgetState<K>, action: DockAction) -> bool {
    let factory = Factory;
    let mut changed = false;

    match action {
        DockAction::Tab(tab_msg) => match tab_msg {
            TabAction::Select { pane, panel } => {
                changed = state.set_active_panel(pane, panel);
            }
            TabAction::Close { panel } => {
                // The key goes first: once the panel is out of its tree the index no
                // longer resolves it, and the index is what the key came from.
                let id = state
                    .index
                    .panels
                    .iter()
                    .find_map(|(s, &n)| (n == panel).then(|| s.clone()));
                if state.close_panel(panel) {
                    if let Some(id) = id {
                        state.index.panels.remove(&id);
                    }
                    changed = true;
                }
            }
            TabAction::DragStarted {
                source_pane,
                source_panel,
                drop_edge_fraction,
            } => {
                state.drag = Some(DragSession::new(
                    source_pane,
                    source_panel,
                    drop_edge_fraction,
                ));
                state.layout_dirty = true;
                changed = true;
            }
            TabAction::DragEnded { cursor } => {
                if finish_drag(state, Some(cursor)) {
                    changed = true;
                }
            }
            TabAction::DragMoved { cursor } => {
                // This frame's geometry, recorded by the panes while drawing.
                let drop_targets = state.drop_targets.clone();
                let tab_bar_targets = state.tab_bar_targets.clone();
                if let Some(ref mut session) = state.drag {
                    DockManager::update_drag_hover_full(
                        session,
                        cursor,
                        &drop_targets,
                        &tab_bar_targets,
                    );
                }
            }
            TabAction::DragCancelled => {
                state.drag = None;
                state.layout_dirty = true;
                changed = true;
            }
        },
        DockAction::PaneFocused { pane, panel } => {
            if let Some(panel_node) = panel {
                let tab_changed = state
                    .pane(pane)
                    .is_some_and(|p| p.active != Some(panel_node));
                if tab_changed {
                    changed = state.set_active_panel(pane, panel_node);
                }
            }
            if state.focus(pane) {
                changed = true;
            }
        }
        DockAction::SplitDrag {
            group,
            splitter_index,
            pair_ratio,
        } => {
            // The group's tree is resolved rather than assumed: a split inside a dock
            // is not in the centre tree.
            if state
                .tree_of_mut(group)
                .is_some_and(|tree| factory.adjust_splitter(tree, group, splitter_index, pair_ratio).is_ok())
            {
                state.layout_dirty = true;
                changed = true;
            }
        }
        DockAction::ToggleZoom { pane } => {
            if state.toggle_zoom(pane) {
                changed = true;
            }
        }
        DockAction::ToggleDock { placement } => {
            if state.toggle_dock(placement) {
                changed = true;
            }
        }
        DockAction::DockResize { placement, cursor } => {
            if state.resize_dock_to(placement, cursor) {
                changed = true;
            }
        }
    }
    if changed && state.layout_dirty {
        state.sync_index();
        state.regions.prune_zoom(&state.layout);
    }
    changed
}
