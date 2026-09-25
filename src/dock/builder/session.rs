// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::dock::builder::compile::{
    active_panel_in_pane, first_pane, insert_panel_into_state, pane_for_panel, BuiltLayout,
};
use crate::dock::builder::spec::{LayoutTree, PanelDef};
use crate::dock::factory::Factory;
use crate::dock::manager::DockManager;
use crate::dock::model::{DockOperation, NodeId, NodeKind};
use crate::dock::spatial::{adjacent_pane, pane_bounds_map, Direction};
use crate::dock::widget::{dispatch_action, DockAction, DockWidgetState, TabAction};
use crate::dock::{Error, Result};

/// Target pane for opening a new panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneTarget {
    /// Open in the pane registered with [`TabsNode::named`](crate::dock::builder::TabsNode::named).
    Named(String),
    /// Open in the pane that last received focus.
    Active,
    /// Open in the first pane encountered in a preorder tree walk.
    First,
}

/// Which pane receives focus when building a [`DockSession`].
#[derive(Debug, Clone, Default)]
pub enum InitialFocus<'a> {
    /// First pane in preorder tree walk.
    #[default]
    FirstPane,
    /// Pane registered with [`TabsNode::named`](crate::dock::builder::TabsNode::named).
    NamedPane(Cow<'a, str>),
    /// Pane that owns the panel with this id (active tab comes from `.active()` at compile time).
    NamedPanel(Cow<'a, str>),
}

/// Direction to cycle the active tab within the focused pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelCycle {
    Next,
    Prev,
}

/// High-level handle for a dock layout and runtime panel operations.
pub struct DockSession<K> {
    inner: Rc<RefCell<DockWidgetState<K>>>,
}

impl<K> DockSession<K>
where
    K: Copy,
{
    /// Build a session from a declarative layout tree.
    ///
    /// The tree becomes the centre; the session has no edge docks. Use
    /// [`from_area`](Self::from_area) to describe docks as well.
    pub fn from_tree(tree: LayoutTree<K>) -> Result<Self> {
        Self::from_tree_with_focus(tree, InitialFocus::default())
    }

    /// Build a session and set initial pane focus.
    pub fn from_tree_with_focus(tree: LayoutTree<K>, focus: InitialFocus<'_>) -> Result<Self> {
        Self::from_area_with_focus(crate::dock::LayoutArea::new(tree), focus)
    }

    /// Build a session from a whole area: a centre tree plus its edge docks.
    pub fn from_area(area: crate::dock::LayoutArea<K>) -> Result<Self> {
        Self::from_area_with_focus(area, InitialFocus::default())
    }

    /// Build a session from a whole area and set initial pane focus.
    pub fn from_area_with_focus(
        area: crate::dock::LayoutArea<K>,
        focus: InitialFocus<'_>,
    ) -> Result<Self> {
        let (built, regions) = crate::dock::builder::compile::build_area(&area)?;
        let focused_pane = resolve_initial_focus(&built, focus)?;
        let mut state = DockWidgetState::from_built(built, focused_pane);
        state.regions = regions;
        state.layout_dirty = true;
        Ok(Self {
            inner: Rc::new(RefCell::new(state)),
        })
    }

    /// Build a session from a compiled layout and index.
    #[must_use]
    pub fn from_built(built: BuiltLayout<K>, focused_pane: Option<NodeId>) -> Self {
        let state = DockWidgetState::from_built(built, focused_pane);
        Self {
            inner: Rc::new(RefCell::new(state)),
        }
    }

    /// Shared widget state for the iced dock builder.
    #[must_use]
    pub fn state(&self) -> Rc<RefCell<DockWidgetState<K>>> {
        Rc::clone(&self.inner)
    }

    /// Whether the tab bar should offer panel `id`.
    #[must_use]
    pub fn is_panel_visible(&self, id: &str) -> bool {
        self.inner.borrow().is_panel_visible(id)
    }

    /// Hide or show a panel, without touching the layout.
    ///
    /// Visibility is application state that changes without the layout changing, so
    /// this is the dock's own answer to it rather than a re-install: a hidden panel
    /// keeps its node and its tab slot, is left out of the strip, and is passed over
    /// when the displayed tab is resolved. Showing it again restores the order.
    ///
    /// Calling this is what makes hiding work — a presentation that only draws a
    /// different icon changes nothing on its own.
    #[expect(clippy::must_use_candidate)]
    pub fn set_panel_visible(&self, id: &str, visible: bool) -> bool {
        let mut state = self.inner.borrow_mut();
        let changed = state.set_panel_visible(id, visible);
        if changed {
            let pane = state
                .index
                .panels
                .get(id)
                .copied()
                .and_then(|panel| state.layout.get(panel).and_then(|e| e.owner));
            if let Some(pane) = pane {
                state.focus(pane);
            }
        }
        changed
    }

    /// Save the whole workspace: the centre, every edge dock, and the zoom.
    #[must_use]
    pub fn capture(&self, version: Option<usize>) -> crate::dock::DockAreaState<K>
    where
        K: Clone,
    {
        let state = self.inner.borrow();
        crate::dock::DockAreaState::capture(&state.layout, &state.regions, version)
    }

    /// Replace the whole workspace with a new area.
    ///
    /// The dock sizes and open flags the user dragged are carried across by
    /// placement, so installing a preset does not reset a dock the user resized to
    /// suit their screen. Use [`restore`](Self::restore) to load a saved workspace
    /// instead, which sets those deliberately.
    pub fn set_area(&self, area: crate::dock::LayoutArea<K>) -> Result
    where
        K: Copy,
    {
        self.inner.borrow_mut().set_area(area)
    }

    /// Replace the workspace with a saved one.
    ///
    /// Docks the saved state does not mention are dropped, and the zoom is restored
    /// by looking its panel key up — a key nothing holds any more simply opens
    /// unzoomed. Both are deliberate: a workspace restored from a file describes the
    /// whole layout, and a panel a later build removed should not block the load.
    pub fn restore(&self, saved: &crate::dock::DockAreaState<K>) -> Result
    where
        K: Clone,
    {
        let mut state = self.inner.borrow_mut();
        state.regions.restore(&saved.docks)?;
        state.layout = saved.center.clone();
        state.drag = None;
        state.sync_index();
        let focused = first_pane(&state.layout);
        state.focused_pane = focused;
        state.focus_frame_pane = focused;
        state.hidden.clear();
        // The saved zoom is a panel key; the pane holding it is what a zoom is
        // recorded against, so the key is resolved through this session's index.
        let zoomed_pane = saved.zoomed.as_deref().and_then(|key| {
            crate::dock::builder::compile::pane_for_panel(&state.layout, &state.index, key)
        });
        state.regions.restore_zoom(zoomed_pane);
        state.layout_dirty = true;
        Ok(())
    }

    /// Apply a [`DockAction`] programmatically (not for widget-originated input).
    #[expect(clippy::must_use_candidate)]
    pub fn dispatch(&self, action: DockAction) -> bool {
        dispatch_action(&mut self.inner.borrow_mut(), action)
    }

    /// Open a panel in the given pane target and activate it.
    pub fn open_panel(&self, target: PaneTarget, panel: impl Into<PanelDef<K>>) -> Result {
        let def = panel.into();
        let pane_id = self.resolve_pane(&target)?;
        let factory = Factory;
        let mut state = self.inner.borrow_mut();
        let panel_id = insert_panel_into_state(&factory, &mut *state, &def)?;
        factory.add_panel_to_pane(&mut state.layout, pane_id, panel_id)?;
        factory.set_active_panel(&mut state.layout, pane_id, panel_id);
        state.layout_dirty = true;
        state.focus(pane_id);
        state.sync_index();
        Ok(())
    }

    /// Activate a panel by string id and focus its pane.
    pub fn select_panel(&self, panel_id: &str) -> Result {
        let panel_node = self
            .panel_node(panel_id)
            .ok_or_else(|| Error::UnknownPanel(panel_id.into()))?;
        let pane_id = self.pane_for_panel(panel_id).ok_or(Error::InvalidTarget)?;
        self.dispatch(DockAction::Tab(TabAction::Select {
            pane: pane_id,
            panel: panel_node,
        }));
        Ok(())
    }

    /// Close a panel by its string id.
    pub fn close_panel(&self, panel_id: &str) -> Result {
        let panel_node = self
            .panel_node(panel_id)
            .ok_or_else(|| Error::UnknownPanel(panel_id.into()))?;
        if self.dispatch(DockAction::Tab(TabAction::Close { panel: panel_node })) {
            Ok(())
        } else {
            Err(Error::NoOwner { panel: panel_node })
        }
    }

    /// All known panel ids.
    #[must_use]
    pub fn panel_ids(&self) -> Vec<String> {
        self.inner.borrow().index.panel_ids().cloned().collect()
    }

    /// Panel node id for a string panel id.
    pub fn panel_node(&self, panel_id: &str) -> Option<NodeId> {
        self.inner.borrow().index.panel_node(panel_id)
    }

    /// Pane that owns a panel identified by string id.
    #[must_use]
    pub fn pane_for_panel(&self, panel_id: &str) -> Option<NodeId> {
        self.inner.borrow().pane_for_panel_id(panel_id)
    }

    /// Pane that last received focus, if any.
    #[must_use]
    pub fn focused_pane(&self) -> Option<NodeId> {
        self.inner.borrow().focused_pane
    }

    /// Whether the given pane currently has global focus.
    #[must_use]
    pub fn is_pane_focused(&self, pane: NodeId) -> bool {
        self.focused_pane() == Some(pane)
    }

    /// Pane that currently draws the focus frame, if any.
    ///
    /// Equal to [`Self::focused_pane`] unless
    /// [`DockBuilder::focus_frame_groups`](crate::dock::widget::DockBuilder::focus_frame_groups)
    /// restricts the frame to a subset of tab groups.
    #[must_use]
    pub fn focus_frame_pane(&self) -> Option<NodeId> {
        self.inner.borrow().focus_frame_pane
    }

    /// Focus a pane by id (does not change the active tab).
    pub fn focus_pane(&self, pane: NodeId) -> Result {
        if !matches!(
            self.inner.borrow().layout.kind(pane),
            Some(NodeKind::Pane(_))
        ) {
            return Err(Error::InvalidTarget);
        }
        self.dispatch(DockAction::PaneFocused { pane, panel: None });
        Ok(())
    }

    /// Move focus to the nearest pane in `direction`.
    ///
    /// Requires at least one draw pass so [`DockWidgetState::pane_bounds`] is populated
    /// (run the dock widget once or wait for the first frame).
    /// Returns `true` if focus moved to a neighbor.
    ///
    /// When
    /// [`DockBuilder::focus_frame_groups`](crate::dock::widget::DockBuilder::focus_frame_groups)
    /// restricts the focus frame, only panes in those groups are candidates — ineligible
    /// panes are skipped rather than stepped through.
    #[expect(
        clippy::must_use_candidate,
        reason = "This is a command-style API; callers may intentionally ignore failed movement."
    )]
    pub fn focus_adjacent(&self, direction: Direction) -> bool {
        let state = self.inner.borrow();
        let Some(pane) = state.focused_pane else {
            return false;
        };
        // The current pane stays in the map as the origin rectangle even when it is not
        // itself an eligible target (e.g. focus sits on a tool pane).
        let bounds = pane_bounds_map(&state.pane_bounds)
            .into_iter()
            .filter(|&(id, _)| id == pane || state.frame_eligible(id))
            .collect();
        drop(state);
        let Some(adjacent) = adjacent_pane(pane, direction, &bounds) else {
            return false;
        };
        let _ = self.focus_pane(adjacent);
        true
    }

    /// Move the active tab to the nearest pane in `direction`.
    ///
    /// Uses the same spatial lookup as [`Self::focus_adjacent`], so it also requires
    /// at least one draw pass for [`DockWidgetState::pane_bounds`] to be populated.
    /// Returns `true` if the active tab moved to a neighbor.
    #[expect(
        clippy::must_use_candidate,
        reason = "This is a command-style API; callers may intentionally ignore failed movement."
    )]
    pub fn move_active_panel_adjacent(&self, direction: Direction) -> bool {
        let state = self.inner.borrow();
        let Some(source_pane) = state.focused_pane else {
            return false;
        };
        let Some(NodeKind::Pane(pane_state)) = state.layout.kind(source_pane) else {
            return false;
        };
        let Some(source_panel) = pane_state.active.or(pane_state.tabs.first().copied()) else {
            return false;
        };
        let bounds = pane_bounds_map(&state.pane_bounds);
        let Some(target_pane) = adjacent_pane(source_pane, direction, &bounds) else {
            return false;
        };
        drop(state);

        let factory = Factory;
        let manager = DockManager;
        let mut state = self.inner.borrow_mut();
        if !manager.groups_compatible(&state.layout, source_panel, target_pane) {
            return false;
        }
        if factory
            .dock_fill(&mut state.layout, source_panel, target_pane)
            .is_err()
        {
            return false;
        }
        state.layout_dirty = true;
        state.focus(target_pane);
        state.sync_index();
        true
    }

    /// Split the focused pane in `direction` and move the active tab into the new pane.
    ///
    /// Returns `false` without changing the layout when the focused pane has fewer than
    /// two tabs.
    #[expect(
        clippy::must_use_candidate,
        reason = "This is a command-style API; callers may intentionally ignore failed movement."
    )]
    pub fn split_active_panel(&self, direction: Direction) -> bool {
        let state = self.inner.borrow();
        let Some(source_pane) = state.focused_pane else {
            return false;
        };
        let Some(NodeKind::Pane(pane_state)) = state.layout.kind(source_pane) else {
            return false;
        };
        if pane_state.tabs.len() <= 1 {
            return false;
        }
        let Some(source_panel) = pane_state.active.or(pane_state.tabs.first().copied()) else {
            return false;
        };
        drop(state);

        let factory = Factory;
        let mut state = self.inner.borrow_mut();
        if factory
            .split_same_pane_edge(
                &mut state.layout,
                source_pane,
                source_panel,
                operation_for_direction(direction),
            )
            .is_err()
        {
            return false;
        }

        let Some(target_pane) = state.layout.get(source_panel).and_then(|e| e.owner) else {
            state.layout_dirty = true;
            state.sync_index();
            return false;
        };
        if !matches!(state.layout.kind(target_pane), Some(NodeKind::Pane(_))) {
            state.layout_dirty = true;
            state.sync_index();
            return false;
        }

        state.layout_dirty = true;
        state.focus(target_pane);
        state.sync_index();
        true
    }

    /// Clear global pane focus (and the focus frame) without changing active tabs.
    pub fn clear_focus(&self) {
        let mut state = self.inner.borrow_mut();
        if state.focused_pane.is_some() || state.focus_frame_pane.is_some() {
            state.focused_pane = None;
            state.focus_frame_pane = None;
            state.focus_dirty = true;
        }
    }

    /// Cycle the active tab in the focused pane (wraps at ends).
    pub fn cycle_panel(&self, cycle: PanelCycle) -> Result {
        let pane = self.focused_pane().ok_or(Error::InvalidTarget)?;
        let state = self.inner.borrow();
        let NodeKind::Pane(pane_state) = state.layout.kind(pane).ok_or(Error::InvalidTarget)?
        else {
            return Err(Error::InvalidTarget);
        };
        if pane_state.tabs.is_empty() {
            return Err(Error::InvalidTarget);
        }
        let current = pane_state
            .active
            .or(pane_state.tabs.first().copied())
            .ok_or(Error::InvalidTarget)?;
        let current_index = pane_state
            .tabs
            .iter()
            .position(|&id| id == current)
            .unwrap_or(0);
        let len = pane_state.tabs.len();
        let next_index = match cycle {
            PanelCycle::Next => (current_index + 1) % len,
            PanelCycle::Prev => (current_index + len - 1) % len,
        };
        let panel = pane_state.tabs[next_index];
        drop(state);
        self.dispatch(DockAction::Tab(TabAction::Select { pane, panel }));
        Ok(())
    }

    /// Currently focused panel id (active tab in the focused pane), if any.
    #[must_use]
    pub fn active_panel(&self) -> Option<String> {
        let state = self.inner.borrow();
        let pane = state.focused_pane?;
        let tree = state.tree_of(pane).unwrap_or(&state.layout);
        active_panel_in_pane(tree, &state.index, pane)
    }

    /// Active panel id string in a specific pane (regardless of global focus).
    #[must_use]
    pub fn active_panel_in_pane(&self, pane: NodeId) -> Option<String> {
        let state = self.inner.borrow();
        let tree = state.tree_of(pane).unwrap_or(&state.layout);
        active_panel_in_pane(tree, &state.index, pane)
    }

    fn resolve_pane(&self, target: &PaneTarget) -> Result<NodeId> {
        match target {
            PaneTarget::Named(name) => self
                .inner
                .borrow()
                .index
                .pane_node(name)
                .ok_or_else(|| Error::UnknownPane(name.clone())),
            PaneTarget::Active => self.inner.borrow().focused_pane.ok_or(Error::InvalidTarget),
            PaneTarget::First => {
                first_pane(&self.inner.borrow().layout).ok_or(Error::InvalidTarget)
            }
        }
    }
}

fn operation_for_direction(direction: Direction) -> DockOperation {
    match direction {
        Direction::Left => DockOperation::Left,
        Direction::Right => DockOperation::Right,
        Direction::Up => DockOperation::Top,
        Direction::Down => DockOperation::Bottom,
    }
}

fn resolve_initial_focus<K>(
    built: &BuiltLayout<K>,
    focus: InitialFocus<'_>,
) -> Result<Option<NodeId>> {
    match focus {
        InitialFocus::FirstPane => Ok(first_pane(&built.layout)),
        InitialFocus::NamedPane(name) => built
            .index
            .pane_node(name.as_ref())
            .ok_or_else(|| Error::UnknownPane(name.to_string()))
            .map(Some),
        InitialFocus::NamedPanel(panel_id) => {
            pane_for_panel(&built.layout, &built.index, panel_id.as_ref())
                .ok_or_else(|| Error::UnknownPanel(panel_id.to_string()))
                .map(Some)
        }
    }
}

impl<K> fmt::Debug for DockSession<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DockSession").finish_non_exhaustive()
    }
}
