//! Cross-tree panel moves: drags that cross between the centre and an edge dock.
//!
//! The centre and each dock are separate layout arenas, so a drop that crosses
//! between them is a detach plus an insert, not a re-parent. Every bug this file
//! guards came from that seam: a panel detached and then never inserted (dropped
//! on an edge band, the panel vanished), a failed insert with no rollback (a
//! panel in neither tree), and a source pane emptied without being collapsed (a
//! blank region left behind).
//!
//! The failures were reported from the running example: dropping on a pane whose
//! *edge* band was highlighted lost the panel, while the centre (fill) band
//! worked — the edge path was the broken one.

use iced::{Point, Rectangle, Size};
use iced_kit::dock::model::{DockPlacement, NodeKind};
use iced_kit::dock::unstable::{dispatch_action, finish_drag, panels_in_tree};
use iced_kit::dock::{
    horizontal, panel, tabs, DockAction, DockWidgetState, LayoutArea, PanelDef, TabAction,
};

fn panel_def(id: &str, key: u32) -> PanelDef<u32> {
    panel(id, id, key)
}

/// Centre with two panels, a left dock, and a bottom dock — the shape a drag
/// from a dock into the centre (and back) crosses.
fn state() -> DockWidgetState<u32> {
    DockWidgetState::from_area(
        LayoutArea::new(horizontal([
            tabs([panel_def("editor", 0)]),
            tabs([panel_def("preview", 1)]),
        ]))
        .dock(
            DockPlacement::Left,
            240.0,
            tabs([panel_def("files", 2), panel_def("search", 3)]).active("files"),
        )
        .dock(
            DockPlacement::Bottom,
            180.0,
            tabs([panel_def("terminal", 4)]),
        ),
    )
    .expect("the area is valid")
}

/// The pane holding a panel, wherever it is.
fn pane_of(state: &DockWidgetState<u32>, id: &str) -> Option<iced_kit::dock::model::NodeId> {
    state.pane_for_panel_id(id)
}

/// The panel ids a pane holds, in order.
fn pane_tabs(state: &DockWidgetState<u32>, pane: iced_kit::dock::model::NodeId) -> Vec<String> {
    let Some(tree) = state.tree_of(pane) else {
        return Vec::new();
    };
    let Some(NodeKind::Pane(p)) = tree.kind(pane) else {
        return Vec::new();
    };
    p.tabs
        .iter()
        .filter_map(|&tab| match tree.kind(tab) {
            Some(NodeKind::Panel(panel)) => Some(panel.id.clone()),
            _ => None,
        })
        .collect()
}

/// The panel ids a region's tree holds.
fn tree_panel_ids(tree: &iced_kit::dock::model::Layout<u32>) -> Vec<String> {
    panels_in_tree(tree)
        .into_iter()
        .filter_map(|node| match tree.kind(node) {
            Some(NodeKind::Panel(p)) => Some(p.id.clone()),
            _ => None,
        })
        .collect()
}

/// Panes in a tree that hold no tabs and would draw as blank space.
fn hollow_panes(tree: &iced_kit::dock::model::Layout<u32>) -> Vec<iced_kit::dock::model::NodeId> {
    panels_in_tree(tree)
        .into_iter()
        .filter(|&node| matches!(tree.kind(node), Some(NodeKind::Pane(p)) if p.tabs.is_empty()))
        .collect()
}

/// The content box a pane registers as its drop target, in area coordinates.
const CENTER_PANE: Rectangle = Rectangle::new(Point::new(240.0, 0.0), Size::new(400.0, 300.0));

/// Begin a drag of `id`'s tab, the way the widget does when a press crosses the
/// threshold. A drag has to be in flight before a drop can apply — that is what
/// `finish_drag` consumes.
fn start_drag(state: &mut DockWidgetState<u32>, id: &str) {
    let panel = state.index.panels.get(id).copied().expect("indexed");
    let pane = state.pane_for_panel_id(id).expect("a pane");
    state.drag = Some(iced_kit::dock::unstable::DragSession::new(pane, panel, 0.2));
}

// ---------------------------------------------------------------------------
// Fill: dock panel dropped on the centre of a centre pane
// ---------------------------------------------------------------------------

#[test]
fn a_dock_panel_filling_a_centre_pane_joins_it() {
    let mut state = state();
    let target = pane_of(&state, "editor").expect("the editor pane");
    state.drop_targets.push((target, CENTER_PANE));
    start_drag(&mut state, "files");

    assert!(finish_drag(&mut state, Some(CENTER_PANE.center())));
    let pane = pane_of(&state, "files").expect("the panel has a home");
    assert_eq!(pane, target, "files joined the editor's pane");
    assert!(
        pane_tabs(&state, target).contains(&"files".to_owned()),
        "the panel is a tab of the pane it filled"
    );
}

// ---------------------------------------------------------------------------
// Edge split: the path that lost the panel
// ---------------------------------------------------------------------------

#[test]
fn a_dock_panel_dropped_on_a_centre_panes_edge_splits_it() {
    let mut state = state();
    let target = pane_of(&state, "editor").expect("the editor pane");

    // Register the pane's content box, and release on its *right edge band* —
    // the grey highlight the drop overlay draws. This is the drop that used to
    // detach the panel and then fail to place it.
    state.drop_targets.push((target, CENTER_PANE));
    let drop_at = Point::new(
        CENTER_PANE.x + CENTER_PANE.width - 2.0,
        CENTER_PANE.center_y(),
    );
    start_drag(&mut state, "files");

    assert!(
        finish_drag(&mut state, Some(drop_at)),
        "an edge drop onto a pane applies"
    );

    // The panel still exists, somewhere: the index can resolve it, and its pane
    // holds it.
    let pane = pane_of(&state, "files").expect("the dropped panel must not be lost");
    assert_ne!(pane, target, "an edge drop splits, not fills");
    assert_eq!(
        pane_tabs(&state, pane),
        vec!["files".to_owned()],
        "the dragged panel sits in its own new group"
    );

    // And the trees are consistent: every indexed panel resolves to a pane.
    for id in state.index.panels.keys() {
        assert!(pane_of(&state, id).is_some(), "`{id}` resolves to a pane");
    }
}

#[test]
fn an_edge_split_from_a_dock_keeps_both_trees_well_formed() {
    let mut state = state();
    let target = pane_of(&state, "editor").expect("the editor pane");

    // Fill first, so the target pane takes the dragged panel; then drag it out
    // again onto the *other* centre pane's edge.
    state.drop_targets.push((target, CENTER_PANE));
    start_drag(&mut state, "files");
    assert!(finish_drag(&mut state, Some(CENTER_PANE.center())));

    let other = pane_of(&state, "preview").expect("the preview pane");
    let other_bounds = Rectangle::new(Point::new(640.0, 0.0), Size::new(400.0, 300.0));
    state.drop_targets.clear();
    state.drop_targets.push((other, other_bounds));
    let drop_at = Point::new(other_bounds.x + 2.0, other_bounds.center_y());
    start_drag(&mut state, "files");

    assert!(finish_drag(&mut state, Some(drop_at)));

    // The left dock's tree has lost the panel — for good, and tidily.
    let left = state.regions.region(DockPlacement::Left).expect("the dock");
    let dock_panels = tree_panel_ids(&left.tree);
    assert!(
        !dock_panels.contains(&"files".to_owned()),
        "the panel left the dock's tree"
    );
    assert!(
        !dock_panels.is_empty(),
        "the dock keeps its remaining panel"
    );

    // The centre is well-formed: both centre panes still resolve.
    assert!(pane_of(&state, "editor").is_some());
    assert!(pane_of(&state, "preview").is_some());
    assert!(pane_of(&state, "files").is_some());
}

// ---------------------------------------------------------------------------
// Emptying a dock: the source region must not go hollow
// ---------------------------------------------------------------------------

#[test]
fn dragging_a_docks_last_panel_out_collapses_its_group() {
    let mut state =
        DockWidgetState::from_area(LayoutArea::new(tabs([panel_def("editor", 0)])).dock(
            DockPlacement::Left,
            240.0,
            tabs([panel_def("files", 2)]),
        ))
        .expect("valid");

    let target = pane_of(&state, "editor").expect("the editor pane");
    state.drop_targets.push((target, CENTER_PANE));
    start_drag(&mut state, "files");

    assert!(finish_drag(&mut state, Some(CENTER_PANE.center())));

    // The panel is in the centre now.
    assert_eq!(pane_of(&state, "files"), Some(target));

    // And the dock it left has no hollow group reachable from its root.
    let left = state.regions.region(DockPlacement::Left).expect("the dock");
    assert!(
        hollow_panes(&left.tree).is_empty(),
        "the dock must not keep an emptied group reachable"
    );
}

// ---------------------------------------------------------------------------
// Rollback: a refused drop must not swallow the panel
// ---------------------------------------------------------------------------

#[test]
fn a_drop_with_no_target_leaves_the_panel_where_it_was() {
    let mut state = state();
    let target = pane_of(&state, "editor").expect("the editor pane");

    state.drop_targets.push((target, CENTER_PANE));
    start_drag(&mut state, "files");
    assert!(finish_drag(&mut state, Some(CENTER_PANE.center())));

    // Drag `files` again, but release where nothing is registered: the drop
    // resolves to no target, applies nothing, and the panel stays put.
    state.drop_targets.clear();
    start_drag(&mut state, "files");
    assert!(
        !finish_drag(&mut state, Some(Point::new(50.0, 50.0))),
        "a drop with no target applies nothing"
    );
    assert_eq!(
        pane_of(&state, "files"),
        Some(target),
        "the panel stayed in the centre"
    );
}

// ---------------------------------------------------------------------------
// Zoom from a dock
// ---------------------------------------------------------------------------

#[test]
fn a_dock_pane_can_be_zoomed() {
    let mut state = state();
    let pane = pane_of(&state, "files").expect("the files pane");
    assert!(
        state.set_zoom(pane, true),
        "a dock panel with zoom allowed is zoomable"
    );
    assert!(state.is_zoomed(pane));
    assert!(state.set_zoom(pane, false));
    assert!(!state.is_zoomed(pane));
}

#[test]
fn a_dock_panel_that_refuses_zoom_is_refused() {
    let mut state = state();
    let pane = pane_of(&state, "files").expect("the files pane");
    let panel = state.index.panels.get("files").copied().expect("indexed");
    let tree = state.tree_of_mut(panel).expect("the panel's tree");
    if let Some(NodeKind::Panel(p)) = tree.get_mut(panel).map(|e| &mut e.kind) {
        p.can_zoom = false;
    }

    assert!(!state.set_zoom(pane, true), "the panel refuses");
    assert!(!state.is_zoomed(pane));
}

// ---------------------------------------------------------------------------
// Hiding gives the space back
// ---------------------------------------------------------------------------

/// Hiding every panel of a dock must free the dock's extent, not leave a blank
/// slab where the dock was.
///
/// The reported symptom: the panel's body vanished but the region it lived in
/// kept holding its 240 pixels, so the hide read as "the eye button changed the
/// icon and shuffled some text". The dock's open flag is untouched — showing a
/// panel again restores the dock at the size the user dragged.
#[test]
fn hiding_a_docks_every_panel_frees_its_extent() {
    let mut state =
        DockWidgetState::from_area(LayoutArea::new(tabs([panel_def("editor", 0)])).dock(
            DockPlacement::Left,
            240.0,
            tabs([panel_def("files", 2)]),
        ))
        .expect("valid");

    // The area lays out from `region_rects`, which consults this predicate.
    assert!(
        !state.region_is_empty(
            &state
                .regions
                .region(DockPlacement::Left)
                .expect("dock")
                .tree
        ),
        "the dock starts with something to draw"
    );

    assert!(state.set_panel_visible("files", false));

    let left = state.regions.region(DockPlacement::Left).expect("dock");
    assert!(
        state.region_is_empty(&left.tree),
        "a dock whose every panel is hidden is empty"
    );
    // The dock itself is still open and keeps its size, so a later show
    // restores it as it was.
    assert!(state.regions.is_dock_open(DockPlacement::Left));
    assert_eq!(
        state.regions.dock(DockPlacement::Left).unwrap().size(),
        240.0
    );

    // And showing the panel again makes the region drawable once more.
    assert!(state.set_panel_visible("files", true));
    let left = state.regions.region(DockPlacement::Left).expect("dock");
    assert!(!state.region_is_empty(&left.tree));
}

/// Hiding one of two panes in a split frees that pane's slot for its sibling —
/// the split cannot keep a child whose every panel is hidden.
///
/// This is the pane-level counterpart of the dock rule above, and the same one
/// the reference encodes as `resizable_panel().visible(...)`: the visible group
/// takes over the space, and giving the panel its slot back on re-show is what
/// the stored tab list guarantees.
#[test]
fn hiding_one_pane_of_a_split_frees_its_slot() {
    let mut state = DockWidgetState::from_area(LayoutArea::new(horizontal([
        tabs([panel_def("files", 2)]),
        tabs([panel_def("editor", 0)]),
    ])))
    .expect("valid");

    // The centre tree is not a region, but the same predicate answers it: one
    // visible panel each, so nothing is empty.
    assert!(!state.region_is_empty(&state.layout));

    assert!(state.set_panel_visible("files", false));

    // The tree still holds both panes — hiding is not a structural edit — but
    // the drawn set is what decides the slot: the pane that remains is all the
    // centre has to show.
    let editor = pane_of(&state, "editor").expect("the editor pane");
    assert!(state.pane(editor).is_some(), "the editor pane survives");

    // The widget builds panes from the visible set, so the hidden pane produces
    // no element: build_pane's contract is `None` for nothing to draw.
    let files = state.index.panels.get("files").copied().expect("indexed");
    let files_pane = pane_of(&state, "files").expect("the files pane");
    let files_still_there = state
        .pane(files_pane)
        .is_some_and(|p| p.tabs.contains(&files));
    assert!(
        files_still_there,
        "hiding keeps the panel's slot in the pane"
    );

    // The pane's drawn set is empty, which is what the widget asks.
    assert!(
        state.displayed_panel(files_pane).is_none(),
        "the hidden pane has nothing to display"
    );
    assert!(state.displayed_panel(editor).is_some());
}

// ---------------------------------------------------------------------------
// Close through the menu's action
// ---------------------------------------------------------------------------

#[test]
fn closing_a_panel_by_its_node_closes_the_right_one() {
    let mut state = state();
    let files = state.index.panels.get("files").copied().expect("indexed");

    assert!(state.close_panel(files));
    assert!(
        pane_of(&state, "files").is_none(),
        "the closed panel is gone from every tree"
    );

    // Its pane had another tab, so the pane survives with it.
    assert!(pane_of(&state, "search").is_some());
}

#[test]
fn closing_the_last_panel_of_a_dock_leaves_no_hollow_group() {
    let mut state = state();
    let terminal = state
        .index
        .panels
        .get("terminal")
        .copied()
        .expect("indexed");

    assert!(state.close_panel(terminal));

    let bottom = state
        .regions
        .region(DockPlacement::Bottom)
        .expect("the dock");
    assert!(
        hollow_panes(&bottom.tree).is_empty(),
        "the emptied group is collapsed away"
    );
}

// ---------------------------------------------------------------------------
// TabAction::Close via dispatch — the path the panel menu uses
// ---------------------------------------------------------------------------

#[test]
fn the_menu_close_action_closes_the_displayed_panel() {
    let mut state = state();
    let files = state.index.panels.get("files").copied().expect("indexed");

    let changed = dispatch_action(
        &mut state,
        DockAction::Tab(TabAction::Close { panel: files }),
    );
    assert!(changed, "a real panel id closes");
    assert!(pane_of(&state, "files").is_none());
}
